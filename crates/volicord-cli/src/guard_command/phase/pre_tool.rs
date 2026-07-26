use chrono::Duration as ChronoDuration;
use serde_json::{json, Value};
use volicord_store::{
    bootstrap::ProjectRecord,
    guards::{insert_expected_write, ExpectedWriteInsert},
    RuntimeHomeMutationContext,
};
use volicord_types::guard_outcome::GuardPolicyDecision;

use super::GuardPhaseResult;
use crate::guard_command::{
    args::GuardInput,
    context::{guard_state_summary, ActiveWriteTicketSummary, GuardReason, GuardStateSummary},
    envelope::{event_time, GuardEnvelope},
    format_timestamp,
    mutation::ToolClassification,
    render::{context_json, reasons_json, tool_observation_json, write_ticket_backing_json},
    stable_id,
    tool_observation::{host_invocation_id, tool_observation, ToolObservation},
    write_ticket::{normalized_observed_paths, write_ticket_coverage, WriteTicketCoverage},
    GuardCommandError, EXPECTED_WRITE_TTL_MINUTES,
};

#[derive(Debug, Clone)]
pub(in crate::guard_command) struct ExpectedWriteCandidate {
    pub(super) insert: ExpectedWriteInsert,
    expected_paths: Vec<String>,
    write_ticket: ActiveWriteTicketSummary,
}

pub(in crate::guard_command) fn handle_pre_tool(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardPhaseResult, GuardCommandError> {
    let summary = guard_state_summary(context, project, envelope, input)?;
    let observation = tool_observation(&input.raw_value, &project.repo_root);
    let (decision, reasons) = pre_tool_decision(&summary, &observation);
    let write_ticket_backing = if tool_attempts_product_write(&observation) {
        write_ticket_backing_json(write_ticket_coverage(&summary, &observation))
    } else {
        write_ticket_backing_json(WriteTicketCoverage::NotWriteLike)
    };
    let expected_write =
        expected_write_candidate(project, envelope, &summary, &observation, input, decision)?;
    let expected_write_json = expected_write
        .as_ref()
        .map(expected_write_candidate_json)
        .unwrap_or(Value::Null);
    Ok(GuardPhaseResult::with_expected_write(
        decision,
        json!({
            "decision": decision.as_str(),
            "allowed": decision != GuardPolicyDecision::Deny,
            "reasons": reasons_json(&reasons),
            "tool": tool_observation_json(&observation),
            "write_ticket_backing": write_ticket_backing,
            "expected_write": expected_write_json,
            "context": context_json(&summary),
            "enforcement_level": "cooperative_guard"
        }),
        expected_write,
    ))
}

fn pre_tool_decision(
    summary: &GuardStateSummary,
    observation: &ToolObservation,
) -> (GuardPolicyDecision, Vec<GuardReason>) {
    let mut reasons = Vec::new();
    let product_file_write_attempt = observation.deterministic_product_write_attempt();
    if observation.deterministic_write_attempt()
        && observation
            .structured_paths
            .iter()
            .any(|path| !path.inside_repo)
    {
        reasons.push(GuardReason {
            code: "target_outside_project_allowlist",
            message: "One or more target paths are outside the selected Product Repository."
                .to_owned(),
            severity: "deny",
        });
    }
    if product_file_write_attempt {
        if summary.active_task_id.is_none() {
            reasons.push(GuardReason {
                code: "no_active_task",
                message: "Product-file writes require an active Volicord task.".to_owned(),
                severity: "deny",
            });
        } else {
            let coverage = write_ticket_coverage(summary, observation);
            match coverage {
                WriteTicketCoverage::PolicyAuthorityStale { .. } => reasons.push(GuardReason {
                    code: "write_ticket_policy_changed",
                    message: "Project workflow policy changed after the previously covering write ticket was issued, or that ticket lacks a current policy-authority binding. Run `volicord.prepare_write` again to reevaluate the paths and obtain current authorization before this Product Repository write.".to_owned(),
                    severity: "deny",
                }),
                _ if summary.policy_control_reevaluation.is_some() => reasons.push(GuardReason {
                    code: "policy_control_reevaluation_required",
                    message: "Project policy now requires the active Task's control level or acceptance policy to be reevaluated. Run `volicord.prepare_write` again before this Product Repository write so Core can apply the stronger policy and issue a current write ticket.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::NotWriteLike => {}
                WriteTicketCoverage::TicketBacked { ticket, .. } => {
                    if ticket.workspace_validity_uncertain {
                        reasons.push(GuardReason {
                            code: "write_ticket_workspace_unverified",
                            message: "Volicord could not refresh the Git workspace coordinate for this otherwise compatible write ticket. The cooperative guard is allowing the attempt with degraded workspace verification.".to_owned(),
                            severity: "warn",
                        });
                    }
                }
                WriteTicketCoverage::NoObservedPaths => reasons.push(GuardReason {
                    code: "write_ticket_scope_indeterminate",
                    message: "The host hook did not expose a deterministic Product Repository path for this write-like operation. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::NoActiveTickets { .. } => reasons.push(GuardReason {
                    code: "write_ticket_missing",
                    message: "No active write ticket covers this Product Repository write-like operation. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::OutOfScope { .. } => reasons.push(GuardReason {
                    code: "write_ticket_path_scope_violation",
                    message: "The observed Product Repository path is outside the active write ticket scope. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "deny",
                }),
                WriteTicketCoverage::Ambiguous { .. } => reasons.push(GuardReason {
                    code: "write_ticket_ambiguous",
                    message: "More than one active write ticket could cover this Product Repository path, so Volicord cannot deterministically link the operation. This is a cooperative Volicord host decision, not OS-level enforcement.".to_owned(),
                    severity: "warn",
                }),
            }
        }
    }
    if matches!(
        observation.classification,
        ToolClassification::UnknownMutationRisk | ToolClassification::Mutating
    ) && !observation.deterministic_write_attempt()
        && observation.structured_reported_effect().is_none()
    {
        reasons.push(GuardReason {
            code: "unknown_effect_warning",
            message: "Volicord cannot determine this invocation's write paths from structured host facts; it is allowed with a warning and must be checked against actual post-tool changes.".to_owned(),
            severity: "warn",
        });
    }
    let decision = if reasons.iter().any(|reason| reason.severity == "deny") {
        GuardPolicyDecision::Deny
    } else if reasons.iter().any(|reason| reason.severity == "warn") {
        GuardPolicyDecision::ContinueWithWarning
    } else {
        GuardPolicyDecision::Continue
    };
    (decision, reasons)
}

fn tool_attempts_product_write(observation: &ToolObservation) -> bool {
    observation.explicit_write_attempt
        || observation.structured_reported_effect() == Some("product_file_write")
        || observation.classification == ToolClassification::Mutating
        || tool_name_implies_write(observation.tool_name.as_deref())
}

fn confidently_expects_product_write(observation: &ToolObservation) -> bool {
    observation.deterministic_product_write_attempt()
}

fn tool_name_implies_write(tool_name: Option<&str>) -> bool {
    tool_name
        .map(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "edit" | "write" | "write_file" | "apply_patch" | "patch" | "notebook_edit"
            )
        })
        .unwrap_or(false)
}

fn expected_write_candidate(
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    input: &GuardInput,
    decision: GuardPolicyDecision,
) -> Result<Option<ExpectedWriteCandidate>, GuardCommandError> {
    if decision == GuardPolicyDecision::Deny || !confidently_expects_product_write(observation) {
        return Ok(None);
    }
    let Some(task_id) = summary.active_task_id.clone() else {
        return Ok(None);
    };
    if observation
        .structured_paths
        .iter()
        .any(|path| !path.inside_repo)
    {
        return Ok(None);
    }
    let expected_paths = normalized_observed_paths(observation.structured_paths.iter());
    if expected_paths.is_empty() {
        return Ok(None);
    }
    let write_ticket = match write_ticket_coverage(summary, observation) {
        WriteTicketCoverage::TicketBacked { ticket, .. } => ticket,
        _ => return Ok(None),
    };
    let created_at = event_time(&envelope.occurred_at)?;
    let expires_at = created_at + ChronoDuration::minutes(EXPECTED_WRITE_TTL_MINUTES);
    let host_invocation_id = host_invocation_id(&input.raw_value);
    let expected_write_id = stable_id(
        "expected_write",
        &[
            &project.project_id,
            &envelope.connection_id,
            envelope.session_id.as_deref().unwrap_or(""),
            &envelope.event_id,
            host_invocation_id.as_deref().unwrap_or(""),
            &expected_paths.join("|"),
            &write_ticket.write_ticket_id,
        ],
    );
    let write_ticket_ids = vec![write_ticket.write_ticket_id.clone()];
    Ok(Some(ExpectedWriteCandidate {
        insert: ExpectedWriteInsert {
            expected_write_id,
            correlation: envelope.correlation.clone(),
            connection_internal_id: envelope.connection_id.clone(),
            guard_installation_id: envelope.guard_installation_id.clone(),
            pre_tool_guard_event_id: envelope.event_id.clone(),
            host_invocation_id,
            tool_name: observation.tool_name.clone(),
            command_kind: observation.classification.as_str().to_owned(),
            path_policy: "exact_paths".to_owned(),
            expected_paths: expected_paths.clone(),
            task_id,
            change_unit_id: write_ticket.change_unit_id.clone(),
            write_ticket_ids: write_ticket_ids.clone(),
            basis_state_version: summary.state_version,
            created_at: format_timestamp(created_at),
            expires_at: format_timestamp(expires_at),
            metadata_json: json!({
                "source": "volicord_guard_pre_tool",
                "raw_event_sha256": input.raw_sha256,
                "ticket_backed": true,
                "write_ticket_ids": write_ticket_ids
            })
            .to_string(),
        },
        expected_paths,
        write_ticket,
    }))
}

pub(in crate::guard_command) fn persist_expected_write(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    candidate: ExpectedWriteCandidate,
) -> Result<(), GuardCommandError> {
    insert_expected_write(context, &project.project_id, candidate.insert)?;
    Ok(())
}

fn expected_write_candidate_json(candidate: &ExpectedWriteCandidate) -> Value {
    json!({
        "expected_write_id": candidate.insert.expected_write_id,
        "host_invocation_id": candidate.insert.host_invocation_id,
        "tool_name": candidate.insert.tool_name,
        "command_kind": candidate.insert.command_kind,
        "path_policy": candidate.insert.path_policy,
        "expected_paths": candidate.expected_paths,
        "task_id": candidate.insert.task_id,
        "change_unit_id": candidate.insert.change_unit_id,
        "ticket_backed": true,
        "write_ticket_id": candidate.write_ticket.write_ticket_id,
        "write_ticket_ids": candidate.insert.write_ticket_ids,
        "basis_state_version": candidate.insert.basis_state_version,
        "expires_at": candidate.insert.expires_at
    })
}
