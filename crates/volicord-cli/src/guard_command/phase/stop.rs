use std::path::Path;

use serde_json::{json, Value};
use volicord_core::{
    validate_authority_status, AuthorityStatusExpectation, CoreService, InvocationContext,
};
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{
    ActorSource, ChangeUnitId, ErrorCode, GuardDecision, OperationCategory, ProjectId, RequestId,
    ResponseKind, StatusInclude, StatusRequest, StatusResult, TaskId, ToolEnvelope,
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
    invocation_binding_basis: &str,
) -> Result<GuardPhaseResult, GuardCommandError> {
    let summary = guard_state_summary(runtime_home, project, envelope, input)?;
    let (decision, reasons, close_status) = stop_decision(
        runtime_home,
        project,
        envelope,
        &summary,
        invocation_binding_basis,
    )?;
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
    invocation_binding_basis: &str,
) -> Result<(GuardDecision, Vec<GuardReason>, Value), GuardCommandError> {
    let Some(task_id) = summary.active_task_id.as_deref() else {
        return Ok((
            GuardDecision::Allow,
            Vec::new(),
            json!({"active_task": null, "close_blockers": []}),
        ));
    };
    let mut invocation = InvocationContext::new(
        ProjectId::new(&project.project_id),
        ActorSource::agent_connection(envelope.connection_id.clone()),
        OperationCategory::Read,
        invocation_binding_basis,
    );
    if let Some(session_id) = envelope.session_id.as_deref() {
        invocation = invocation.with_session_id(session_id);
    }
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
                pending_user_actions: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        invocation,
    )?;
    let mut reasons = Vec::new();
    if summary.pending_user_action_count > 0 {
        reasons.push(GuardReason {
            code: "pending_user_actions",
            message: "User-owned actions are still pending for the active task.".to_owned(),
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
    let expectation =
        AuthorityStatusExpectation::new(ProjectId::new(&project.project_id), TaskId::new(task_id))
            .with_state_version(summary.state_version)
            .with_current_change_unit(
                summary
                    .active_change_unit_id
                    .as_deref()
                    .map(ChangeUnitId::new),
            );
    let result = validate_authority_status(response, &expectation)
        .ok()?
        .into_status();
    if summary.pending_user_action_count != result.pending_user_action_summaries.len()
        || summary.active_blocker_count != result.blocker_refs.len()
    {
        return None;
    }
    Some(result)
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

        let expectation = AuthorityStatusExpectation::new(
            ProjectId::new("project_malformed"),
            TaskId::new("task_malformed"),
        );
        let validation_error = validate_authority_status(&malformed, &expectation)
            .expect_err("malformed status must not be authoritative");
        assert!(!validation_error.to_string().contains("sensitive"));
        assert_eq!(recognized_response_kind(&malformed), None);
        assert_eq!(public_error_codes(&malformed), ["MCP_UNAVAILABLE"]);
    }

    #[test]
    fn incomplete_result_refresh_is_not_authoritative() {
        let incomplete = json!({
            "base": {"response_kind": "result"}
        });

        let expectation = AuthorityStatusExpectation::new(
            ProjectId::new("project_incomplete"),
            TaskId::new("task_incomplete"),
        );
        assert!(validate_authority_status(&incomplete, &expectation).is_err());
        assert_eq!(
            recognized_response_kind(&incomplete),
            Some(ResponseKind::Result)
        );
    }
}
