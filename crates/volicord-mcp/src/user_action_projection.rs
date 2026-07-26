//! Current UserAction reread and compound MCP result projection.

use crate::adapter::McpAdapter;
use crate::errors::McpAdapterError;
use crate::tool_dispatch::ToolCallOutput;
use volicord_core::pipeline::{CoreService, PipelineResponse};
use volicord_core::CurrentUserActionProjection;
use volicord_store::diagnostics::DiagnosticFallbackKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ProjectId, UserActionRequestId};
use volicord_types::methods::{
    McpRequestUserActionResponse, RequestUserActionResponse, RequestUserActionResult,
};
use volicord_types::values::UserActionStatus;

pub(crate) fn user_action_tool_output(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    pending_response: PipelineResponse,
) -> Result<ToolCallOutput, McpAdapterError> {
    let Some(coordinate) = pending_user_action_coordinate_from_response(&pending_response)? else {
        return ToolCallOutput::from_pipeline_response(&pending_response);
    };
    adapter.admitted_runtime_home(context)?;
    let current = current_user_action_projection_for_coordinate(context, &coordinate)?;
    let mut output = compound_user_action_output(&pending_response, &current)?;
    if current.status == UserActionStatus::Pending {
        output = output.with_user_action_fallback(cli_recovery_fallback());
    }
    Ok(output)
}

fn compound_user_action_output(
    pending_response: &PipelineResponse,
    current: &CurrentUserActionProjection,
) -> Result<ToolCallOutput, McpAdapterError> {
    let compound = McpRequestUserActionResponse {
        agent_workflow_result: serde_json::from_value::<RequestUserActionResponse>(
            pending_response.response_value.clone(),
        )
        .map_err(McpAdapterError::Json)?,
        agent_workflow_result_replayed: pending_response.replayed,
        current_projection_state_version: current.observed_state_version,
        current_projection_observed_at: current.observed_at.clone(),
        current_status: current.status,
        user_channel_resolution_ref: current.user_action_resolution_ref.clone().into(),
        user_channel_resolution: current.user_action_resolution.clone().into(),
        derived_refs: current.derived_refs.clone(),
    };
    let response_value = serde_json::to_value(compound).map_err(McpAdapterError::Json)?;
    let response_json = serde_json::to_string(&response_value).map_err(McpAdapterError::Json)?;
    Ok(ToolCallOutput::success(response_json)?
        .with_operation_result_ref(pending_response.operation_result_ref.clone())
        .with_pipeline_diagnostics(pending_response))
}

fn current_user_action_projection_for_coordinate(
    context: &RuntimeHomeMutationContext<'_>,
    coordinate: &PendingUserActionCoordinate,
) -> Result<CurrentUserActionProjection, McpAdapterError> {
    let current = CoreService::for_mutation(context)
        .current_user_action_projection(&coordinate.project_id, &coordinate.user_action_request_id)
        .map_err(McpAdapterError::Core)?
        .ok_or_else(|| {
            McpAdapterError::Protocol(
                "committed user-action request disappeared during current-state reread".to_owned(),
            )
        })?;
    if current.project_id != coordinate.project_id
        || current.user_action_request_id != coordinate.user_action_request_id
    {
        return Err(McpAdapterError::Protocol(
            "current user-action projection does not match the original request".to_owned(),
        ));
    }
    Ok(current)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingUserActionCoordinate {
    project_id: ProjectId,
    user_action_request_id: UserActionRequestId,
}

fn pending_user_action_coordinate_from_response(
    response: &PipelineResponse,
) -> Result<Option<PendingUserActionCoordinate>, McpAdapterError> {
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return Ok(None);
    }
    let result = serde_json::from_value::<RequestUserActionResult>(response.response_value.clone())
        .map_err(McpAdapterError::Json)?;
    let invocation = response.verified_invocation.as_ref().ok_or_else(|| {
        McpAdapterError::Protocol(
            "successful request_user_action response omitted verified invocation facts".to_owned(),
        )
    })?;
    Ok(Some(PendingUserActionCoordinate {
        project_id: invocation.project_id.clone(),
        user_action_request_id: result.user_action_request_summary.user_action_request_id,
    }))
}

pub(crate) struct UserActionFallback {
    pub(crate) texts: Vec<String>,
    pub(crate) kind: DiagnosticFallbackKind,
}

pub(crate) fn cli_recovery_fallback() -> UserActionFallback {
    UserActionFallback {
        texts: vec![cli_inbox_fallback_text()],
        kind: DiagnosticFallbackKind::CliInbox,
    }
}

fn cli_inbox_fallback_text() -> String {
    "A pending UserAction requires the user. Open `volicord inbox` and resume the existing request after the user completes it."
        .to_owned()
}
