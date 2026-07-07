use std::path::Path;

use serde_json::{json, Value};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{
    ActorSource, GuardDecision, OperationCategory, ProjectId, RequestId, StatusInclude,
    StatusRequest, TaskId, ToolEnvelope, VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
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
    let close_blockers = response
        .response_value
        .get("close_blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut reasons = Vec::new();
    if !close_blockers.is_empty() {
        reasons.push(GuardReason {
            code: "close_readiness_blocked",
            message: "Close readiness has blockers for the active task.".to_owned(),
            severity: "deny",
        });
    }
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
            "status_summary": response.response_value.get("status_summary").cloned().unwrap_or(Value::Null),
            "close_state": response.response_value.get("close_state").cloned().unwrap_or(Value::Null),
            "close_blockers": close_blockers
        }),
    ))
}
