//! Post-mutation authoritative status refresh and validation.

use crate::adapter::McpAdapter;
use crate::binding::managed_agent_session_binding;
use crate::errors::McpAdapterError;
use crate::lifecycle::SessionRuntime;
use volicord_core::pipeline::PipelineResponse;
use volicord_core::{validate_authority_status, AuthorityStatusExpectation};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::schema::{AuthorityReceipt, NextActionSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MutationRefreshContext {
    pub(crate) project_id: ProjectId,
    pub(crate) task_id: TaskId,
}

impl MutationRefreshContext {
    pub(crate) fn from_pipeline_response(response: &PipelineResponse) -> Option<Self> {
        Some(Self {
            project_id: response.verified_invocation.as_ref()?.project_id.clone(),
            task_id: response.resolved_task_id.clone()?,
        })
    }
}

pub(crate) fn refresh_authority_status(
    mutation_context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    state: &SessionRuntime,
    context: &MutationRefreshContext,
) -> Result<PipelineResponse, McpAdapterError> {
    let binding = managed_agent_session_binding(&state.codex_binding, &state.runtime_session_id);
    let coordinates = binding
        .as_ref()
        .map(|binding| {
            adapter.ensure_agent_session_binding(mutation_context, &context.project_id, binding)
        })
        .transpose()?;
    adapter.refresh_authority_status(
        mutation_context,
        &context.project_id,
        &context.task_id,
        coordinates.as_ref().map(|value| value.borrowed()),
    )
}

pub(crate) fn validated_authority_refresh(
    context: &MutationRefreshContext,
    response: &PipelineResponse,
) -> Result<(AuthorityReceipt, Vec<NextActionSummary>), ()> {
    validate_authority_status(
        &response.response_value,
        &AuthorityStatusExpectation::new(context.project_id.clone(), context.task_id.clone()),
    )
    .map_err(|_| ())
    .map(|validated| validated.into_authority_projection())
}
