use serde_json::{json, Value};
use volicord_types::{canonical_json_string, AuthorityReceipt, GuardDecision};

use crate::disclosure::{
    cooperative_host_decision_disclosure_json, COOPERATIVE_DECISION_DISCLOSURE_TEXT,
};

use super::{
    args::{GuardPhase, HostOutputMode, OutputFormat},
    context::{ActiveWriteTicketSummary, GuardReason, GuardStateSummary},
    json_error,
    mutation::PathAssessment,
    prompt_capture::GuardPendingJudgmentSummary,
    tool_observation::ToolObservation,
    write_ticket::WriteTicketCoverage,
    GuardCommandError,
};

const MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES: usize = 8 * 1024;
const AUTHORITY_RECEIPT_SYSTEM_MESSAGE_PREFIX: &str = "Volicord fresh AuthorityReceipt: ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RenderedGuardOutput {
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) exit_code: i32,
}

pub(super) fn render_guard_output(
    phase: GuardPhase,
    decision: GuardDecision,
    envelope: &super::envelope::GuardEnvelope,
    result: Value,
    output: OutputFormat,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    match output {
        OutputFormat::VolicordJson => Ok(RenderedGuardOutput {
            stdout: format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({
                    "phase": phase.event_kind(),
                    "decision": decision.as_str(),
                    "allowed": decision != GuardDecision::Deny,
                    "disclosure": cooperative_host_decision_disclosure_json(),
                    "guard_event_id": envelope.event_id,
                    "session_id": envelope.session_id,
                    "result": result
                }))
                .map_err(json_error)?
            ),
            stderr: String::new(),
            exit_code: if decision == GuardDecision::Deny {
                1
            } else {
                0
            },
        }),
        OutputFormat::Text => {
            let allowed = if decision == GuardDecision::Deny {
                "blocked"
            } else {
                "allowed"
            };
            let watcher_text = guard_watcher_scan_text(&result);
            Ok(RenderedGuardOutput {
                stdout: format!(
                    "Volicord host-hook {}: {} ({})\n{}{}\n",
                    phase.command_name(),
                    decision.as_str(),
                    allowed,
                    watcher_text,
                    COOPERATIVE_DECISION_DISCLOSURE_TEXT
                ),
                stderr: String::new(),
                exit_code: if decision == GuardDecision::Deny {
                    1
                } else {
                    0
                },
            })
        }
        OutputFormat::HostNative(host) => render_host_native_output(host, phase, decision, result),
    }
}

fn guard_watcher_scan_text(result: &Value) -> String {
    let Some(summary) = result
        .get("context")
        .and_then(|context| context.get("session_watch_scan_summary"))
        .filter(|summary| summary.is_object())
    else {
        return String::new();
    };
    let degraded_reasons = summary
        .get("degraded_reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let degraded_reasons = if degraded_reasons.is_empty() {
        "none".to_owned()
    } else {
        degraded_reasons.join(",")
    };
    format!(
        "watcher_scan: files_scanned={}; files_skipped={}; unreadable_paths={}; degraded_reasons={}\nwatcher_note: not full filesystem monitoring\n",
        summary
            .get("files_scanned")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("files_skipped")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("unreadable_paths_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        degraded_reasons,
    )
}

pub(super) fn render_host_native_output(
    host: HostOutputMode,
    phase: GuardPhase,
    decision: GuardDecision,
    result: Value,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    let event_name = host_hook_event_name(phase);
    let value = match phase {
        GuardPhase::SessionStart => context_output(event_name, guard_context_message(&result)),
        GuardPhase::PreTool => match decision {
            GuardDecision::Deny => Some(json!({
                "hookSpecificOutput": {
                    "hookEventName": event_name,
                    "permissionDecision": "deny",
                    "permissionDecisionReason": blocking_reason(phase, &result)
                }
            })),
            GuardDecision::Warn | GuardDecision::InjectContext => {
                context_output(event_name, guard_context_message(&result))
            }
            GuardDecision::Allow => None,
        },
        GuardPhase::PostTool => match decision {
            GuardDecision::Deny => Some(json!({
                "decision": "block",
                "reason": blocking_reason(phase, &result)
            })),
            GuardDecision::Warn | GuardDecision::InjectContext => {
                context_output(event_name, post_tool_context_message(&result))
            }
            GuardDecision::Allow => None,
        },
        GuardPhase::PromptCapture => match decision {
            GuardDecision::Deny => Some(json!({
                "decision": "block",
                "reason": blocking_reason(phase, &result)
            })),
            GuardDecision::InjectContext | GuardDecision::Warn => {
                context_output(event_name, prompt_context_message(&result))
            }
            GuardDecision::Allow => prompt_context_message(&result)
                .filter(|message| !message.trim().is_empty())
                .and_then(|message| context_output(event_name, Some(message))),
        },
        GuardPhase::Stop => Some(stop_output(decision, &result)),
    };
    let stdout = match value {
        Some(value) => format!("{}\n", serde_json::to_string(&value).map_err(json_error)?),
        None => String::new(),
    };
    Ok(RenderedGuardOutput {
        stdout,
        stderr: String::new(),
        exit_code: host_success_exit_code(host),
    })
}

fn stop_output(decision: GuardDecision, result: &Value) -> Value {
    let mut output = match decision {
        GuardDecision::Deny => json!({
            "decision": "block",
            "reason": blocking_reason(GuardPhase::Stop, result)
        }),
        GuardDecision::Allow | GuardDecision::Warn | GuardDecision::InjectContext => {
            json!({ "continue": true })
        }
    };
    if let Some(system_message) = stop_authority_system_message(result) {
        output
            .as_object_mut()
            .expect("Stop host output must be a JSON object")
            .insert("systemMessage".to_owned(), Value::String(system_message));
    }
    output
}

fn stop_authority_system_message(result: &Value) -> Option<String> {
    let active_task = result
        .pointer("/close_status/active_task")
        .and_then(Value::as_str)?;
    let receipt = result
        .pointer("/close_status/authority_receipt")
        .cloned()
        .and_then(|value| serde_json::from_value::<AuthorityReceipt>(value).ok());

    if let Some(receipt) = receipt.as_ref() {
        if let Ok(canonical_receipt) = canonical_json_string(receipt) {
            let message = format!("{AUTHORITY_RECEIPT_SYSTEM_MESSAGE_PREFIX}{canonical_receipt}");
            if message.len() <= MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES {
                return Some(message);
            }
        }
        return Some(stop_authority_fallback_message(
            "Volicord refreshed AuthorityReceipt exceeds the host UI byte budget; no partial receipt JSON is shown.",
            receipt.project_id.as_str(),
            receipt.task_ref.record_id.as_str(),
            Some(receipt.state_version),
        ));
    }

    let project_id = result
        .pointer("/context/project_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let state_version = result
        .pointer("/context/state_version")
        .and_then(Value::as_u64);
    Some(stop_authority_fallback_message(
        "Volicord could not display a fresh AuthorityReceipt from this Stop status refresh.",
        project_id,
        active_task,
        state_version,
    ))
}

fn stop_authority_fallback_message(
    notice: &str,
    project_id: &str,
    task_id: &str,
    state_version: Option<u64>,
) -> String {
    let state_version = state_version
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let message = format!(
        "{notice} project_id={project_id}; task_id={task_id}; state_version={state_version}. Inspect current authority with `volicord status --task {task_id} --json`."
    );
    if message.len() <= MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES {
        return message;
    }
    format!(
        "{notice} project_id=<omitted: host UI byte budget>; task_id=<omitted: host UI byte budget>; state_version={state_version}. Inspect current authority with `volicord status --task active --json`."
    )
}

fn host_success_exit_code(_host: HostOutputMode) -> i32 {
    0
}

fn host_hook_event_name(phase: GuardPhase) -> &'static str {
    match phase {
        GuardPhase::SessionStart => "SessionStart",
        GuardPhase::PreTool => "PreToolUse",
        GuardPhase::PostTool => "PostToolUse",
        GuardPhase::PromptCapture => "UserPromptSubmit",
        GuardPhase::Stop => "Stop",
    }
}

fn context_output(event_name: &str, message: Option<String>) -> Option<Value> {
    let message = message.filter(|message| !message.trim().is_empty())?;
    Some(json!({
        "hookSpecificOutput": {
            "hookEventName": event_name,
            "additionalContext": host_native_message_with_disclosure(&message)
        }
    }))
}

fn blocking_reason(phase: GuardPhase, result: &Value) -> String {
    let reason = first_reason_message(result).unwrap_or_else(|| match phase {
        GuardPhase::SessionStart => "Volicord session context could not be prepared.".to_owned(),
        GuardPhase::PreTool => "Volicord requested a host denial for this tool call.".to_owned(),
        GuardPhase::PostTool => {
            "Volicord requested a host denial for normal handling of this tool result.".to_owned()
        }
        GuardPhase::PromptCapture => {
            "Volicord requested a host denial for this user prompt.".to_owned()
        }
        GuardPhase::Stop => "Volicord needs more work before this session stops.".to_owned(),
    });
    host_native_message_with_disclosure(&reason)
}

fn host_native_message_with_disclosure(message: &str) -> String {
    format!("{message} {COOPERATIVE_DECISION_DISCLOSURE_TEXT}.")
}

fn first_reason_message(result: &Value) -> Option<String> {
    result
        .get("reasons")
        .and_then(Value::as_array)
        .and_then(|reasons| reasons.first())
        .and_then(|reason| {
            let message = reason.get("message").and_then(Value::as_str)?;
            let code = reason.get("code").and_then(Value::as_str);
            Some(match code {
                Some(code) if !code.trim().is_empty() => format!("{message} ({code})"),
                _ => message.to_owned(),
            })
        })
        .or_else(|| {
            result
                .get("model_context")
                .and_then(Value::as_str)
                .filter(|message| !message.trim().is_empty())
                .map(str::to_owned)
        })
}

fn guard_context_message(result: &Value) -> Option<String> {
    let context = result.get("context")?;
    let project_name = context.get("project_name").and_then(Value::as_str)?;
    let state_version = context.get("state_version").and_then(Value::as_u64)?;
    let active_task = context
        .get("active_task_id")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let write_tickets = context
        .get("current_write_ticket_ids")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let pending_judgments = context
        .get("pending_user_judgment_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let unresolved_changes = context
        .get("unresolved_unrecorded_change_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(format!(
        "Volicord context: project `{project_name}`, state_version {state_version}, active_task {active_task}, current_write_tickets {write_tickets}, pending_user_judgments {pending_judgments}, unresolved_unrecorded_changes {unresolved_changes}."
    ))
}

fn post_tool_context_message(result: &Value) -> Option<String> {
    let changes = result
        .get("unrecorded_changes")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if changes == 0 {
        return guard_context_message(result);
    }
    Some(format!(
        "Volicord observed {changes} unresolved Product Repository change finding(s) after this tool call. Reconcile them before close."
    ))
}

fn prompt_context_message(result: &Value) -> Option<String> {
    result
        .get("model_context")
        .and_then(Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| guard_context_message(result))
}

pub(super) fn context_json(summary: &GuardStateSummary) -> Value {
    json!({
        "project_id": summary.project_id,
        "project_name": summary.project_name,
        "repo_root": summary.repo_root,
        "state_version": summary.state_version,
        "active_task_id": summary.active_task_id,
        "active_change_unit_id": summary.active_change_unit_id,
        "prompt_capture_status": summary.prompt_capture_status.as_str(),
        "prompt_capture_enabled": summary.prompt_capture_enabled,
        "current_write_ticket_ids": summary.current_write_ticket_ids,
        "stale_write_ticket_ids": summary.stale_write_ticket_ids,
        "active_write_tickets": summary.active_write_tickets
            .iter()
            .map(active_write_ticket_json)
            .collect::<Vec<_>>(),
        "pending_user_judgment_count": summary.pending_user_judgment_count,
        "pending_user_judgments": summary.pending_user_judgments
            .iter()
            .map(pending_judgment_summary_json)
            .collect::<Vec<_>>(),
        "active_blocker_count": summary.active_blocker_count,
        "unresolved_unrecorded_change_count": summary.unresolved_unrecorded_change_count,
        "session_watch_scan_summary": summary.session_watch_scan_summary
    })
}

fn active_write_ticket_json(ticket: &ActiveWriteTicketSummary) -> Value {
    json!({
        "write_ticket_id": ticket.write_ticket_id,
        "change_unit_id": ticket.change_unit_id,
        "intended_paths": ticket.intended_paths,
        "expires_at": ticket.expires_at
    })
}

pub(super) fn write_ticket_backing_json(coverage: WriteTicketCoverage) -> Value {
    match coverage {
        WriteTicketCoverage::NotWriteLike => json!({
            "status": "not_write_like",
            "ticket_backed": false,
            "observed_paths": []
        }),
        WriteTicketCoverage::TicketBacked {
            ticket,
            observed_paths,
        } => json!({
            "status": "ticket_backed",
            "ticket_backed": true,
            "write_ticket_id": ticket.write_ticket_id.clone(),
            "write_ticket_ids": [ticket.write_ticket_id.clone()],
            "observed_paths": observed_paths,
            "scope": {
                "change_unit_id": ticket.change_unit_id,
                "intended_paths": ticket.intended_paths,
                "expires_at": ticket.expires_at
            },
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::NoObservedPaths => json!({
            "status": "scope_indeterminate",
            "ticket_backed": false,
            "observed_paths": [],
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::NoActiveTickets { observed_paths } => json!({
            "status": "missing_ticket",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::OutOfScope {
            observed_paths,
            active_ticket_ids,
        } => json!({
            "status": "out_of_scope",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "active_write_ticket_ids": active_ticket_ids,
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
        WriteTicketCoverage::Ambiguous {
            observed_paths,
            matching_ticket_ids,
        } => json!({
            "status": "ambiguous",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "matching_write_ticket_ids": matching_ticket_ids,
            "disclosure": "Volicord reports cooperative host-hook detection only; this is not OS-level enforcement and does not prove who changed a file."
        }),
    }
}

pub(super) fn pending_judgment_summary_json(summary: &GuardPendingJudgmentSummary) -> Value {
    json!({
        "chat_id": summary.chat_id,
        "verification_code": summary.verification_code,
        "judgment_kind": summary.judgment_kind,
        "question": summary.question,
        "answer_instruction": summary.answer_instruction,
        "note_instruction": summary.note_instruction,
        "options": summary.options.iter().map(|option| {
            json!({
                "selector": option.selector,
                "option_id": option.option_id,
                "label": option.label,
                "machine_action": option.machine_action,
                "resolution_outcome": option.resolution_outcome,
                "instruction": option.instruction
            })
        }).collect::<Vec<_>>()
    })
}

pub(super) fn tool_observation_json(observation: &ToolObservation) -> Value {
    json!({
        "tool_name": observation.tool_name,
        "host_invocation_id": observation.host_invocation_id,
        "command": observation.command,
        "classification": observation.classification.as_str(),
        "paths": path_assessments_json(&observation.paths),
        "changed_paths": path_assessments_json(&observation.changed_paths),
        "explicit_write_attempt": observation.explicit_write_attempt,
        "exit_code": observation.exit_code,
        "success": observation.success,
        "status": observation.status
    })
}

fn path_assessments_json(paths: &[PathAssessment]) -> Vec<Value> {
    paths
        .iter()
        .map(|path| {
            json!({
                "raw": path.raw,
                "normalized": path.normalized,
                "inside_repo": path.inside_repo
            })
        })
        .collect()
}

pub(super) fn reasons_json(reasons: &[GuardReason]) -> Vec<Value> {
    reasons
        .iter()
        .map(|reason| {
            json!({
                "code": reason.code,
                "message": reason.message,
                "severity": reason.severity
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_receipt(blocker_message: Option<String>) -> Value {
        let close_blockers = blocker_message
            .map(|message| {
                vec![json!({
                    "category": "task",
                    "code": "test_blocker",
                    "message": message,
                    "related_refs": [],
                    "next_actions": []
                })]
            })
            .unwrap_or_default();
        json!({
            "project_id": "project_render",
            "state_version": 7,
            "task_ref": {
                "record_kind": "task",
                "record_id": "task_render",
                "project_id": "project_render",
                "task_id": "task_render",
                "produced_at_state_version": 7
            },
            "change_unit_ref": null,
            "scope_revision": 1,
            "latest_run_ref": null,
            "product_file_write_observed": false,
            "evidence_gate": null,
            "close_state": "blocked",
            "close_blockers": close_blockers,
            "next_actor": "agent",
            "next_action": null
        })
    }

    fn stop_result(receipt: Option<Value>) -> Value {
        json!({
            "reasons": [{
                "code": "close_readiness_blocked",
                "message": "Close readiness has blockers for the active task."
            }],
            "close_status": {
                "active_task": "task_render",
                "authority_receipt": receipt
            },
            "context": {
                "project_id": "project_render",
                "state_version": 7
            }
        })
    }

    fn rendered_stop_value(host: HostOutputMode, decision: GuardDecision, result: Value) -> Value {
        let rendered = render_host_native_output(host, GuardPhase::Stop, decision, result)
            .expect("Stop host output should render");
        serde_json::from_str(rendered.stdout.trim()).expect("Stop host output should be JSON")
    }

    #[test]
    fn stop_output_renders_complete_receipt_for_allow_and_deny() {
        let receipt = authority_receipt(None);
        for host in [HostOutputMode::Codex, HostOutputMode::ClaudeCode] {
            for decision in [GuardDecision::Allow, GuardDecision::Deny] {
                let value = rendered_stop_value(host, decision, stop_result(Some(receipt.clone())));
                match decision {
                    GuardDecision::Allow => assert_eq!(value["continue"], true),
                    GuardDecision::Deny => assert_eq!(value["decision"], "block"),
                    GuardDecision::Warn | GuardDecision::InjectContext => unreachable!(),
                }
                let message = value["systemMessage"]
                    .as_str()
                    .expect("fresh receipt should use the host system message");
                let rendered_receipt = message
                    .strip_prefix(AUTHORITY_RECEIPT_SYSTEM_MESSAGE_PREFIX)
                    .expect("system message should identify the fresh receipt");
                assert_eq!(
                    serde_json::from_str::<Value>(rendered_receipt)
                        .expect("complete receipt should remain valid JSON"),
                    receipt
                );
                assert!(message.len() <= MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES);
            }
        }
    }

    #[test]
    fn stop_output_uses_bounded_fallback_for_oversized_receipt() {
        let receipt = authority_receipt(Some(
            "oversized_receipt_marker".repeat(MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES),
        ));
        let value = rendered_stop_value(
            HostOutputMode::Codex,
            GuardDecision::Deny,
            stop_result(Some(receipt)),
        );
        let message = value["systemMessage"]
            .as_str()
            .expect("oversized receipt should use a status fallback");

        assert!(message.contains("no partial receipt JSON is shown"));
        assert!(message.contains("project_id=project_render"));
        assert!(message.contains("task_id=task_render"));
        assert!(message.contains("state_version=7"));
        assert!(message.contains("volicord status --task task_render --json"));
        assert!(!message.contains("oversized_receipt_marker"));
        assert!(!message.contains("\"close_blockers\""));
        assert!(message.len() <= MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES);
    }

    #[test]
    fn stop_output_uses_status_fallback_when_fresh_receipt_is_unavailable() {
        let result = json!({
            "reasons": [{
                "code": "authoritative_refresh_failed",
                "message": "Volicord could not confirm current authoritative status."
            }],
            "close_status": {
                "active_task": "task_render",
                "authoritative_refresh": {
                    "response_kind": "rejected",
                    "error_codes": ["MCP_UNAVAILABLE"]
                }
            },
            "context": {
                "project_id": "project_render",
                "state_version": 7
            }
        });
        let value = rendered_stop_value(HostOutputMode::ClaudeCode, GuardDecision::Deny, result);
        let message = value["systemMessage"]
            .as_str()
            .expect("refresh failure should use a status fallback");

        assert!(message.contains("could not display a fresh AuthorityReceipt"));
        assert!(message.contains("project_id=project_render"));
        assert!(message.contains("task_id=task_render"));
        assert!(message.contains("state_version=7"));
        assert!(message.contains("volicord status --task task_render --json"));
        assert!(message.len() <= MAX_HOST_AUTHORITY_SYSTEM_MESSAGE_BYTES);
    }
}
