use std::path::Path;

use serde_json::{json, Value};
use volicord_core::{
    validate_authority_status, AuthorityStatusExpectation, CoreService, InvocationContext,
};
use volicord_store::bootstrap::ProjectRecord;
use volicord_store::{core_pipeline::CoreProjectStore, workflow_records::SessionEndReceiptInsert};
use volicord_types::{
    ActorSource, AuthorityNextActor, ChangeUnitId, ErrorCode, GuardDecision, OperationCategory,
    ProjectId, RequestId, ResponseKind, SessionEndTaskState, StatusInclude, StatusRequest,
    StatusResult, TaskId, ToolEnvelope,
};

use super::GuardPhaseResult;
use crate::guard_command::{
    args::GuardInput,
    context::{guard_state_summary, GuardReason, GuardStateSummary},
    core_current_timestamp,
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
    let summary = match guard_state_summary(runtime_home, project, envelope, input) {
        Ok(summary) => summary,
        Err(_) => {
            let mut disposition = refresh_failure_disposition(
                "Volicord could not read current project authority for this Stop event.",
                Value::Null,
            );
            disposition.session_end_receipt_persisted =
                persist_stop_receipt(runtime_home, project, envelope, &disposition);
            return Ok(stop_result(disposition, Value::Null));
        }
    };
    let mut disposition = stop_decision(
        runtime_home,
        project,
        envelope,
        &summary,
        invocation_binding_basis,
    );
    disposition.session_end_receipt_persisted =
        persist_stop_receipt(runtime_home, project, envelope, &disposition);
    Ok(stop_result(disposition, context_json(&summary)))
}

fn stop_result(disposition: StopDisposition, context: Value) -> GuardPhaseResult {
    GuardPhaseResult::new(
        GuardDecision::Allow,
        json!({
            "decision": "allow",
            "allowed": true,
            "completion_claim_allowed": disposition.completion_claim_allowed,
            "task_state": disposition.task_state.as_str(),
            "reasons": reasons_json(&disposition.reasons),
            "next_actor": disposition.next_actor.as_str(),
            "authoritative_refresh_succeeded": disposition.authoritative_refresh_succeeded,
            "session_end_receipt_persisted": disposition.session_end_receipt_persisted,
            "close_status": disposition.close_status,
            "context": context,
            "enforcement_level": "cooperative_detective"
        }),
    )
}

struct StopDisposition {
    reasons: Vec<GuardReason>,
    close_status: Value,
    close_blocker_codes: Vec<String>,
    completion_claim_allowed: bool,
    task_state: SessionEndTaskState,
    next_actor: AuthorityNextActor,
    authoritative_refresh_succeeded: bool,
    session_end_receipt_persisted: bool,
}

fn refresh_failure_disposition(message: &str, _context: Value) -> StopDisposition {
    StopDisposition {
        reasons: vec![GuardReason {
            code: "authoritative_refresh_failed",
            message: message.to_owned(),
            severity: "incomplete",
        }],
        close_status: json!({
            "active_task": null,
            "authoritative_refresh": {"error_codes": []}
        }),
        close_blocker_codes: Vec::new(),
        completion_claim_allowed: false,
        task_state: SessionEndTaskState::AuthorityUnknown,
        next_actor: AuthorityNextActor::None,
        authoritative_refresh_succeeded: false,
        session_end_receipt_persisted: false,
    }
}

fn stop_decision(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    invocation_binding_basis: &str,
) -> StopDisposition {
    let Some(task_id) = summary.active_task_id.as_deref() else {
        return StopDisposition {
            reasons: Vec::new(),
            close_status: json!({"active_task": null, "close_blockers": []}),
            close_blocker_codes: Vec::new(),
            completion_claim_allowed: false,
            task_state: SessionEndTaskState::None,
            next_actor: AuthorityNextActor::None,
            authoritative_refresh_succeeded: true,
            session_end_receipt_persisted: false,
        };
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
    let response = match CoreService::new(runtime_home).status(
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
    ) {
        Ok(response) => response,
        Err(_) => {
            return StopDisposition {
                reasons: vec![GuardReason {
                    code: "authoritative_refresh_failed",
                    message: "Volicord could not confirm current authoritative status for the active task.".to_owned(),
                    severity: "incomplete",
                }],
                close_status: json!({
                    "active_task": task_id,
                    "authoritative_refresh": {"error_codes": []}
                }),
                close_blocker_codes: Vec::new(),
                completion_claim_allowed: false,
                task_state: SessionEndTaskState::AuthorityUnknown,
                next_actor: AuthorityNextActor::None,
                authoritative_refresh_succeeded: false,
                session_end_receipt_persisted: false,
            };
        }
    };
    let mut reasons = Vec::new();
    if summary.pending_user_action_count > 0 {
        reasons.push(GuardReason {
            code: "pending_user_actions",
            message: "User-owned actions are still pending for the active task.".to_owned(),
            severity: "incomplete",
        });
    }
    if summary.unresolved_unrecorded_change_count > 0 {
        reasons.push(GuardReason {
            code: "unresolved_unrecorded_changes",
            message: "Observed Product Repository changes still need reconciliation.".to_owned(),
            severity: "incomplete",
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
                severity: "incomplete",
            },
        );
        return StopDisposition {
            reasons,
            close_status: json!({
                "active_task": task_id,
                "authoritative_refresh": {
                    "response_kind": response_kind.map(response_kind_label),
                    "error_codes": public_error_codes(&response.response_value)
                }
            }),
            close_blocker_codes: Vec::new(),
            completion_claim_allowed: false,
            task_state: SessionEndTaskState::AuthorityUnknown,
            next_actor: AuthorityNextActor::None,
            authoritative_refresh_succeeded: false,
            session_end_receipt_persisted: false,
        };
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
                severity: "incomplete",
            },
        );
    }
    let completion_claim_allowed = authority_receipt.completion_claim_allowed
        && close_blockers.is_empty()
        && reasons.is_empty();
    let next_actor = authority_receipt.next_actor;
    let close_blocker_codes = close_blockers
        .iter()
        .map(|blocker| blocker.code.clone())
        .collect::<Vec<_>>();
    StopDisposition {
        reasons,
        close_status: json!({
            "active_task": task_id,
            "status_summary": status_result.status_summary,
            "close_state": status_result.close_state,
            "close_blockers": close_blockers,
            "authority_receipt": authority_receipt
        }),
        close_blocker_codes,
        completion_claim_allowed,
        task_state: if completion_claim_allowed {
            SessionEndTaskState::Ready
        } else {
            SessionEndTaskState::Blocked
        },
        next_actor,
        authoritative_refresh_succeeded: true,
        session_end_receipt_persisted: false,
    }
}

fn persist_stop_receipt(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    disposition: &StopDisposition,
) -> bool {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return false;
    };
    let Ok(close_blocker_codes_json) = serde_json::to_string(&disposition.close_blocker_codes)
    else {
        return false;
    };
    let Ok(mut store) = CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id))
    else {
        return false;
    };
    let Ok(created_at) = core_current_timestamp(&store) else {
        return false;
    };
    store
        .insert_session_end_receipt(SessionEndReceiptInsert {
            session_end_receipt_id: stable_id(
                "session_end_receipt",
                &[&project.project_id, session_id, &envelope.event_id],
            ),
            managed_session_id: session_id.to_owned(),
            active_task_id: disposition
                .close_status
                .get("active_task")
                .and_then(Value::as_str)
                .map(str::to_owned),
            task_state: disposition.task_state,
            close_blocker_codes_json,
            next_actor: disposition.next_actor,
            completion_claim_allowed: disposition.completion_claim_allowed,
            authority_refresh_succeeded: disposition.authoritative_refresh_succeeded,
            created_at: created_at.to_string(),
        })
        .is_ok()
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
