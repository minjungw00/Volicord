//! Post-mutation authoritative status refresh and validation.

use crate::adapter::McpAdapter;
use crate::binding::managed_agent_session_binding;
use crate::errors::McpAdapterError;
use crate::lifecycle::SessionRuntime;
use volicord_core::pipeline::PipelineResponse;
use volicord_core::{validate_authority_status, AuthorityStatusExpectation};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ProjectId, RecordId, TaskId};
use volicord_types::schema::{AuthorityReceipt, WorkflowProjection};
use volicord_types::schema::{RequiredNullable, StateRecordRef};
use volicord_types::values::{StateRecordKind, TaskMode, WorkPhase};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValidatedMutationAuthority {
    pub(crate) receipt: AuthorityReceipt,
    pub(crate) workflow: WorkflowProjection,
    pub(crate) task_mode: TaskMode,
    pub(crate) work_phase: WorkPhase,
    pub(crate) pending_user_action_refs: Vec<StateRecordRef>,
}

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
) -> Result<ValidatedMutationAuthority, ()> {
    let validated = validate_authority_status(
        &response.response_value,
        &AuthorityStatusExpectation::new(context.project_id.clone(), context.task_id.clone()),
    )
    .map_err(|_| ())?;
    let mut status = validated.into_status();
    let active_task = status.active_task.take().ok_or(())?;
    let task_mode = active_task.mode.ok_or(())?;
    let work_phase = active_task.work_phase.ok_or(())?;
    let state_version = active_task.state_version;
    let pending_user_action_refs = active_task
        .pending_user_action_summaries
        .into_iter()
        .map(|summary| StateRecordRef {
            record_kind: StateRecordKind::UserActionRequest,
            record_id: RecordId::new(summary.user_action_request_id.as_str()),
            project_id: context.project_id.clone(),
            task_id: RequiredNullable::some(context.task_id.clone()),
            produced_at_state_version: RequiredNullable::some(state_version),
        })
        .collect();
    Ok(ValidatedMutationAuthority {
        receipt: status.authority_receipt.take().ok_or(())?,
        workflow: active_task.workflow,
        task_mode,
        work_phase,
        pending_user_action_refs,
    })
}
