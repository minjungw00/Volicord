use std::path::Path;

use serde_json::{json, Value};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{
    ActorSource, AuthorityReceipt, EffectKind, ErrorCode, GuardDecision, OperationCategory,
    ProjectId, RequestId, ResponseKind, StatusInclude, StatusRequest, StatusResult, TaskId,
    ToolEnvelope, VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
};

use super::GuardPhaseResult;
use crate::guard_command::{
    args::GuardInput,
    context::{guard_state_summary, GuardReason, GuardStateSummary},
    envelope::GuardEnvelope,
    render::{context_json, reasons_json},
    stable_id, GuardCommandError,
};

pub(in crate::guard_command) fn handle_stop(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardPhaseResult, GuardCommandError> {
    let summary = guard_state_summary(runtime_home, project, envelope, input)?;
    let (decision, reasons, close_status) =
        stop_decision(runtime_home, project, envelope, &summary)?;
    Ok(GuardPhaseResult::new(
        decision,
        json!({
            "decision": decision.as_str(),
            "allowed": decision != GuardDecision::Deny,
            "reasons": reasons_json(&reasons),
            "close_status": close_status,
            "context": context_json(&summary),
            "enforcement_level": "cooperative_detective"
        }),
    ))
}

fn stop_decision(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
) -> Result<(GuardDecision, Vec<GuardReason>, Value), GuardCommandError> {
    let Some(task_id) = summary.active_task_id.as_deref() else {
        return Ok((
            GuardDecision::Allow,
            Vec::new(),
            json!({"active_task": null, "close_blockers": []}),
        ));
    };
    let response = CoreService::new(runtime_home).status(
        StatusRequest {
            envelope: ToolEnvelope {
                project_id: ProjectId::new(&project.project_id),
                task_id: Some(TaskId::new(task_id)).into(),
                request_id: RequestId::new(stable_id(
                    "req_guard_stop_status",
                    &[&envelope.event_id, task_id],
                )),
                idempotency_key: None.into(),
                expected_state_version: None.into(),
                dry_run: false,
                locale: None.into(),
            },
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        InvocationContext::new(
            ProjectId::new(&project.project_id),
            ActorSource::agent_connection(envelope.connection_id.clone()),
            OperationCategory::Read,
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
        ),
    )?;
    let mut reasons = Vec::new();
    if summary.pending_user_judgment_count > 0 {
        reasons.push(GuardReason {
            code: "pending_user_judgments",
            message: "User-owned judgments are still pending for the active task.".to_owned(),
            severity: "deny",
        });
    }
    if summary.unresolved_unrecorded_change_count > 0 {
        reasons.push(GuardReason {
            code: "unresolved_unrecorded_changes",
            message: "Observed Product Repository changes still need reconciliation.".to_owned(),
            severity: "deny",
        });
    }
    let response_kind = recognized_response_kind(&response.response_value);
    let Some(status_result) =
        authoritative_status_result(&response.response_value, project, task_id, summary)
    else {
        reasons.insert(
            0,
            GuardReason {
                code: "authoritative_refresh_failed",
                message:
                    "Volicord could not confirm current authoritative status for the active task."
                        .to_owned(),
                severity: "deny",
            },
        );
        return Ok((
            GuardDecision::Deny,
            reasons,
            json!({
                "active_task": task_id,
                "authoritative_refresh": {
                    "response_kind": response_kind.map(response_kind_label),
                    "error_codes": public_error_codes(&response.response_value)
                }
            }),
        ));
    };
    let authority_receipt = status_result
        .authority_receipt
        .clone()
        .expect("validated authoritative status requires a receipt");
    let close_blockers = status_result
        .close_blockers
        .expect("authoritative status requires close blockers");
    if !close_blockers.is_empty() {
        reasons.insert(
            0,
            GuardReason {
                code: "close_readiness_blocked",
                message: "Close readiness has blockers for the active task.".to_owned(),
                severity: "deny",
            },
        );
    }
    let decision = if reasons.iter().any(|reason| reason.severity == "deny") {
        GuardDecision::Deny
    } else {
        GuardDecision::Allow
    };
    Ok((
        decision,
        reasons,
        json!({
            "active_task": task_id,
            "status_summary": status_result.status_summary,
            "close_state": status_result.close_state,
            "close_blockers": close_blockers,
            "authority_receipt": authority_receipt
        }),
    ))
}

fn authoritative_status_result(
    response: &Value,
    project: &ProjectRecord,
    task_id: &str,
    summary: &GuardStateSummary,
) -> Option<StatusResult> {
    let result = parse_authoritative_status_result(response)?;
    let receipt = result.authority_receipt.as_ref()?;
    if !authority_receipt_matches_fresh_status(receipt, &result, project, task_id, summary) {
        return None;
    }
    Some(result)
}

fn parse_authoritative_status_result(response: &Value) -> Option<StatusResult> {
    let result = serde_json::from_value::<StatusResult>(response.clone()).ok()?;
    if result.base.response_kind != ResponseKind::Result
        || result.base.effect_kind != EffectKind::ReadOnly
        || result.base.dry_run
        || result.close_state.is_none()
        || result.close_blockers.is_none()
    {
        return None;
    }
    Some(result)
}

fn authority_receipt_matches_fresh_status(
    receipt: &AuthorityReceipt,
    result: &StatusResult,
    project: &ProjectRecord,
    task_id: &str,
    summary: &GuardStateSummary,
) -> bool {
    let Some(state_version) = result.base.state_version else {
        return false;
    };
    let Some(active_task) = result.active_task.as_ref() else {
        return false;
    };
    let Some(active_task_ref) = active_task.task_ref.as_ref() else {
        return false;
    };
    let receipt_change_unit_id = receipt
        .change_unit_ref
        .as_ref()
        .map(|record| record.record_id.as_str());
    receipt.project_id.as_str() == project.project_id
        && receipt.task_ref.project_id.as_str() == project.project_id
        && receipt.task_ref.record_id.as_str() == task_id
        && receipt.task_ref.task_id.as_ref().map(TaskId::as_str) == Some(task_id)
        && receipt.task_ref.produced_at_state_version.as_ref() == Some(&state_version)
        && receipt.state_version == state_version
        && summary.state_version == state_version
        && summary.active_task_id.as_deref() == Some(task_id)
        && summary.active_change_unit_id.as_deref() == receipt_change_unit_id
        && summary.pending_user_judgment_count == result.pending_user_judgments.len()
        && summary.active_blocker_count == result.blocker_refs.len()
        && active_task.project_id.as_str() == project.project_id
        && active_task.state_version == state_version
        && active_task_ref == &receipt.task_ref
        && active_task.scope_revision == receipt.scope_revision
        && active_task.active_change_unit_ref == receipt.change_unit_ref
        && result.close_state == Some(receipt.close_state)
        && result.close_blockers.as_ref() == Some(&receipt.close_blockers)
        && result.evidence_gate.as_ref().and_then(|gate| gate.as_ref())
            == receipt.evidence_gate.as_ref()
        && receipt
            .next_action
            .as_ref()
            .is_none_or(|action| result.next_actions.contains(action))
}

fn recognized_response_kind(response: &Value) -> Option<ResponseKind> {
    serde_json::from_value(response.pointer("/base/response_kind")?.clone()).ok()
}

fn response_kind_label(response_kind: ResponseKind) -> &'static str {
    match response_kind {
        ResponseKind::Result => "result",
        ResponseKind::Rejected => "rejected",
        ResponseKind::DryRun => "dry_run",
    }
}

fn public_error_codes(response: &Value) -> Vec<String> {
    response
        .get("errors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|error| error.get("code"))
        .filter_map(|value| {
            serde_json::from_value::<ErrorCode>(value.clone())
                .ok()
                .and_then(|_| value.as_str().map(str::to_owned))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_refresh_is_not_authoritative_and_diagnostics_are_allowlisted() {
        let malformed = json!({
            "base": {"response_kind": "unexpected-secret-kind"},
            "errors": [
                {
                    "code": "MCP_UNAVAILABLE",
                    "message": "sensitive Core message",
                    "details": {"response_body": "sensitive body"}
                },
                {"code": "PRIVATE_INTERNAL_CODE"}
            ]
        });

        assert!(parse_authoritative_status_result(&malformed).is_none());
        assert_eq!(recognized_response_kind(&malformed), None);
        assert_eq!(public_error_codes(&malformed), ["MCP_UNAVAILABLE"]);
    }

    #[test]
    fn incomplete_result_refresh_is_not_authoritative() {
        let incomplete = json!({
            "base": {"response_kind": "result"}
        });

        assert!(parse_authoritative_status_result(&incomplete).is_none());
        assert_eq!(
            recognized_response_kind(&incomplete),
            Some(ResponseKind::Result)
        );
    }
}
