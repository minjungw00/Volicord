use serde_json::{json, Value};
use volicord_types::GuardDecision;

use crate::disclosure::{
    cooperative_host_decision_disclosure_json, COOPERATIVE_DECISION_DISCLOSURE_TEXT,
};

use super::{
    args::{GuardPhase, OutputFormat},
    context::{ActiveWriteTicketSummary, GuardReason, GuardStateSummary},
    json_error,
    mutation::PathAssessment,
    prompt_capture::GuardPendingUserActionSummary,
    tool_observation::ToolObservation,
    write_ticket::WriteTicketCoverage,
    GuardCommandError,
};

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
    let exit_code = i32::from(decision == GuardDecision::Deny);
    match output {
        OutputFormat::VolicordJson => Ok(RenderedGuardOutput {
            stdout: format!(
                "{}\\n",
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
            exit_code,
        }),
        OutputFormat::Text => Ok(RenderedGuardOutput {
            stdout: format!(
                "Volicord host-hook {}: {} ({})\\n{}\\n",
                phase.command_name(),
                decision.as_str(),
                if decision == GuardDecision::Deny {
                    "blocked"
                } else {
                    "allowed"
                },
                COOPERATIVE_DECISION_DISCLOSURE_TEXT
            ),
            stderr: String::new(),
            exit_code,
        }),
        OutputFormat::HostNative => render_codex_output(phase, decision, result),
    }
}

fn render_codex_output(
    phase: GuardPhase,
    decision: GuardDecision,
    result: Value,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    let event_name = match phase {
        GuardPhase::PreTool => "PreToolUse",
        GuardPhase::PostTool => "PostToolUse",
        GuardPhase::PromptCapture => "UserPromptSubmit",
    };
    let context = match phase {
        GuardPhase::PromptCapture => result
            .get("model_context")
            .and_then(Value::as_str)
            .map(str::to_owned),
        GuardPhase::PostTool if decision != GuardDecision::Allow => Some(
            "Volicord recorded a post-tool Guard warning. Inspect the hook result and reconcile unrecorded Product Repository changes before close."
                .to_owned(),
        ),
        GuardPhase::PreTool if matches!(decision, GuardDecision::Warn | GuardDecision::InjectContext) => {
            first_reason_message(&result)
        }
        _ => None,
    };
    let value = if phase == GuardPhase::PreTool && decision == GuardDecision::Deny {
        Some(json!({
            "hookSpecificOutput": {
                "hookEventName": event_name,
                "permissionDecision": "deny",
                "permissionDecisionReason": native_message(
                    &first_reason_message(&result)
                        .unwrap_or_else(|| "Volicord denied this write attempt.".to_owned())
                )
            }
        }))
    } else {
        context
            .filter(|message| !message.trim().is_empty())
            .map(|message| {
                json!({
                    "hookSpecificOutput": {
                        "hookEventName": event_name,
                        "additionalContext": native_message(&message)
                    }
                })
            })
    };
    Ok(RenderedGuardOutput {
        stdout: value
            .map(|value| serde_json::to_string(&value).map(|text| format!("{text}\n")))
            .transpose()
            .map_err(json_error)?
            .unwrap_or_default(),
        stderr: String::new(),
        exit_code: 0,
    })
}

fn first_reason_message(result: &Value) -> Option<String> {
    result
        .get("reasons")
        .and_then(Value::as_array)
        .and_then(|reasons| reasons.first())
        .and_then(|reason| reason.get("message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn native_message(message: &str) -> String {
    format!("{message} {COOPERATIVE_DECISION_DISCLOSURE_TEXT}.")
}

pub(super) fn context_json(summary: &GuardStateSummary) -> Value {
    json!({
        "project_id": summary.project_id,
        "project_name": summary.project_name,
        "repo_root": summary.repo_root,
        "state_version": summary.state_version,
        "active_task_id": summary.active_task_id,
        "active_task_effective_control_level": summary.active_task_effective_control_level,
        "policy_control_reevaluation": summary.policy_control_reevaluation.as_ref().map(|mark| json!({
            "required": true,
            "required_effective_control_level": mark.required_effective_control_level,
            "required_acceptance_policy": mark.required_acceptance_policy,
            "prepare_write_required": true
        })),
        "active_change_unit_id": summary.active_change_unit_id,
        "prompt_capture_status": summary.prompt_capture_status.as_str(),
        "prompt_capture_operational": summary.prompt_capture_operational,
        "current_write_ticket_ids": summary.current_write_ticket_ids,
        "stale_write_ticket_ids": summary.stale_write_ticket_ids,
        "uncertain_write_ticket_ids": summary.uncertain_write_ticket_ids,
        "active_write_tickets": summary.active_write_tickets
            .iter()
            .map(active_write_ticket_json)
            .collect::<Vec<_>>(),
        "pending_user_action_count": summary.pending_user_action_count,
        "pending_user_actions": summary.pending_user_actions
            .iter()
            .map(pending_user_action_summary_json)
            .collect::<Vec<_>>(),
        "active_blocker_count": summary.active_blocker_count,
        "unresolved_unrecorded_change_count": summary.unresolved_unrecorded_change_count,
        "suspected_unrecorded_change_count": summary.suspected_unrecorded_change_count
    })
}

fn active_write_ticket_json(ticket: &ActiveWriteTicketSummary) -> Value {
    json!({
        "write_ticket_id": ticket.write_ticket_id,
        "change_unit_id": ticket.change_unit_id,
        "allowed_path_prefixes": ticket.intended_paths,
        "denied_path_prefixes": ticket.denied_paths,
        "idle_expires_at": ticket.idle_expires_at,
        "workspace_validity_uncertain": ticket.workspace_validity_uncertain
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
                "allowed_path_prefixes": ticket.intended_paths,
                "denied_path_prefixes": ticket.denied_paths,
                "idle_expires_at": ticket.idle_expires_at
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
        WriteTicketCoverage::PolicyAuthorityStale {
            observed_paths,
            stale_ticket_ids,
        } => json!({
            "status": "policy_authority_stale",
            "ticket_backed": false,
            "observed_paths": observed_paths,
            "stale_write_ticket_ids": stale_ticket_ids,
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

pub(super) fn pending_user_action_summary_json(summary: &GuardPendingUserActionSummary) -> Value {
    json!({
        "user_action_request_id": summary.user_action_request_id,
        "status": summary.status,
        "next_actor": summary.next_actor
    })
}

pub(super) fn tool_observation_json(observation: &ToolObservation) -> Value {
    json!({
        "tool_name": observation.tool_name,
        "host_invocation_id": observation.host_invocation_id,
        "command": observation.command,
        "classification": observation.classification.as_str(),
        "effect": observation.effect(),
        "confidence": observation.confidence(),
        "paths": path_assessments_json(&observation.paths),
        "structured_paths": path_assessments_json(&observation.structured_paths),
        "changed_paths": path_assessments_json(&observation.changed_paths),
        "explicit_write_attempt": observation.explicit_write_attempt,
        "reported_effect": observation.reported_effect,
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
