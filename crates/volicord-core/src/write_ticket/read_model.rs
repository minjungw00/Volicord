use volicord_store::{
    core_pipeline::{CoreProjectStore, StoredWriteTicket},
    StoreError,
};
use volicord_types::ids::TaskId;
use volicord_types::schema::StateRecordRef;
use volicord_types::values::{TaskControlLevel, UserActionKind, UtcTimestamp};
use volicord_user_action_service::{user_action_authority_from_record, UserActionAuthority};

use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::workflow::{project_workflow_policy, resolve_task_control_authority};
use crate::record_refs::state_ref_from_stored;

use super::semantic::StoredWriteTicketFacts;

/// Task facts acquired for Write Ticket approval and current-validity policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketTaskFacts {
    pub(crate) scope_revision: u64,
    pub(crate) effective_control_level: TaskControlLevel,
    pub(crate) pending_policy_reevaluation: bool,
}

/// Task facts consumed by active stored-ticket current-validity policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketCurrentTaskFacts {
    pub(crate) pending_policy_reevaluation: bool,
}

/// Workflow facts needed by current Write Ticket validity policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketWorkflowFacts {
    pub(crate) write_authority_fingerprint: String,
}

/// Current typed facts consumed by pure Write Ticket validity policy.
#[derive(Debug, Clone)]
pub(crate) struct WriteTicketCurrentFacts {
    pub(crate) task: WriteTicketCurrentTaskFacts,
    pub(crate) workflow: WriteTicketWorkflowFacts,
}

/// Evidence facts consumed by pure Write Ticket summary projection.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct WriteTicketEvidenceFacts {
    pub(crate) observation_refs: Vec<StateRecordRef>,
}

pub(crate) fn load_write_ticket_candidates(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<Vec<StoredWriteTicketFacts>> {
    store
        .write_tickets_for_task(task_id)
        .map_err(CorePipelineError::from)
        .map(|records| {
            records
                .iter()
                .map(StoredWriteTicketFacts::from_record)
                .collect()
        })
}

pub(crate) fn load_write_ticket_control_facts(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<(WriteTicketTaskFacts, WriteTicketWorkflowFacts)> {
    let task = store
        .task_record(task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            CorePipelineError::Store(StoreError::NotFound {
                entity: "task",
                id: task_id.as_str().to_owned(),
            })
        })?;
    let workflow = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let resolved =
        resolve_task_control_authority(&task, &workflow).map_err(CorePipelineError::from)?;
    Ok((
        WriteTicketTaskFacts {
            scope_revision: task.scope_revision,
            effective_control_level: resolved.effective_control_level,
            pending_policy_reevaluation: resolved.pending_policy_reevaluation,
        },
        WriteTicketWorkflowFacts {
            write_authority_fingerprint: workflow.write_authority_fingerprint,
        },
    ))
}

pub(crate) fn load_sensitive_approval_facts(
    store: &CoreProjectStore,
    task_id: &TaskId,
    observed_at: &UtcTimestamp,
) -> CoreResult<Vec<UserActionAuthority>> {
    store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, observed_at)
        .map_err(CorePipelineError::from)?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<Result<Vec<_>, _>>()
        .map_err(CorePipelineError::from)
}

pub(crate) fn load_write_ticket_evidence_facts(
    store: &CoreProjectStore,
    task_id: &TaskId,
    consumed_by_run_id: Option<&volicord_types::ids::RunId>,
    state_version: u64,
) -> CoreResult<WriteTicketEvidenceFacts> {
    let Some(run_id) = consumed_by_run_id else {
        return Ok(WriteTicketEvidenceFacts::default());
    };
    let observation_refs = store
        .evidence_observation_refs_for_run(task_id, run_id.as_str(), state_version)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(state_ref_from_stored)
        .collect();
    Ok(WriteTicketEvidenceFacts { observation_refs })
}

pub(crate) fn stored_write_ticket_facts(record: &StoredWriteTicket) -> StoredWriteTicketFacts {
    StoredWriteTicketFacts::from_record(record)
}
