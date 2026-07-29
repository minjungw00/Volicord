use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId};
use volicord_types::methods::PrepareWriteRequest;
use volicord_types::product_path::path_is_within;
use volicord_types::schema::{ChangeUnitEffectContract, WriteDecisionReason};
use volicord_types::values::{
    StateRecordKind, UserActionKind, UserActionRequiredFor, UtcTimestamp, WriteDecisionCategory,
};

use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ProjectStateHeader, StoredUserActionRecordSet, TaskRecord,
};

use super::WriteTicketPlanningError;
use crate::pipeline::{CorePipelineError, CoreResult, VerifiedInvocationContext};
use crate::record_refs::{change_unit_ref, state_ref};
use crate::task_state::StoredScope;
use crate::write_ticket::write_decision_reason;
use volicord_user_action_service::{current_sensitive_approval, SensitiveApprovalRequirement};

pub(crate) fn resolve_prepare_write_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
) -> Result<(TaskId, TaskRecord, Vec<WriteDecisionReason>), WriteTicketPlanningError> {
    let task_id = request
        .task_id
        .clone()
        .or_else(|| request.envelope.task_id.as_ref().cloned())
        .or_else(|| project_state.active_task_id.clone().map(TaskId::new))
        .ok_or(WriteTicketPlanningError::NoActiveTask)?;
    let task = store
        .task_record(&task_id)
        .map_err(CorePipelineError::from)?
        .ok_or(WriteTicketPlanningError::NoActiveTask)?;

    let mut reasons = Vec::new();
    if project_state
        .active_task_id
        .as_deref()
        .is_some_and(|active_task_id| active_task_id != task_id.as_str())
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "scope_not_current",
            "The addressed Task is not the current Task.",
            vec![state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )],
        ));
    }

    Ok((task_id, task, reasons))
}

pub(crate) fn validate_prepare_write_change_unit(
    request: &PrepareWriteRequest,
    task_id: &TaskId,
    current_change_unit: &ChangeUnitRecord,
    reasons: &mut Vec<WriteDecisionReason>,
) {
    if request
        .change_unit_id
        .as_ref()
        .is_some_and(|change_unit_id| change_unit_id.as_str() != current_change_unit.change_unit_id)
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "scope_not_current",
            "The addressed Change Unit is not the current Change Unit.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                task_id,
                current_change_unit,
                current_change_unit.basis_state_version,
            )],
        ));
    }
}

pub(crate) fn baseline_matches(
    change_unit: &ChangeUnitRecord,
    task: &TaskRecord,
    baseline_ref: &BaselineRef,
) -> CoreResult<bool> {
    let task_baseline = StoredScope::from_task(task)?.baseline_ref;
    Ok(change_unit
        .write_basis
        .baseline_ref
        .as_ref()
        .map(BaselineRef::as_str)
        == Some(baseline_ref.as_str())
        && task_baseline.as_deref() == Some(baseline_ref.as_str()))
}

pub(crate) fn workspace_context_matches(
    change_unit: &ChangeUnitRecord,
    verified_invocation: &VerifiedInvocationContext,
) -> CoreResult<bool> {
    Ok(
        match (
            change_unit.write_basis.git_workspace_context.as_ref(),
            verified_invocation.git_workspace_context.as_ref(),
        ) {
            (None, None) => true,
            (Some(stored), Some(current)) => {
                stored.git_common_dir == current.git_common_dir
                    && stored.worktree_id == current.worktree_id
                    && stored.branch_ref == current.branch_ref
                    && stored.head_sha == current.head_sha
                    && stored.workspace_fingerprint == current.workspace_fingerprint
            }
            (None, Some(_)) | (Some(_), None) => false,
        },
    )
}

pub(crate) fn paths_match_current_change_unit(
    intended_paths: &[String],
    change_unit: &ChangeUnitRecord,
) -> CoreResult<bool> {
    if intended_paths.is_empty() {
        return Ok(true);
    }
    if change_unit.bounded_paths.is_empty() {
        return Ok(false);
    }
    let bounded_paths = &change_unit.bounded_paths;
    Ok(!bounded_paths.is_empty()
        && intended_paths.iter().all(|path| {
            bounded_paths
                .iter()
                .any(|scope| path_is_within(path, scope))
        }))
}

pub(crate) fn change_unit_effect_contract(
    change_unit: &ChangeUnitRecord,
) -> CoreResult<Option<ChangeUnitEffectContract>> {
    Ok(change_unit.effect_contract.clone())
}

pub(crate) struct SensitiveApprovalSearch<'a> {
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) request: &'a PrepareWriteRequest,
    pub(crate) task_id: &'a TaskId,
    pub(crate) task: &'a TaskRecord,
    pub(crate) change_unit: &'a ChangeUnitRecord,
    pub(crate) intended_operation: &'a str,
    pub(crate) normalized_paths: &'a [String],
    pub(crate) sensitive_categories: &'a [String],
    pub(crate) now: &'a UtcTimestamp,
}

pub(crate) fn matching_sensitive_approval(
    search: SensitiveApprovalSearch<'_>,
) -> Result<Option<StoredUserActionRecordSet>, WriteTicketPlanningError> {
    let SensitiveApprovalSearch {
        store,
        request,
        task_id,
        task,
        change_unit,
        intended_operation,
        normalized_paths,
        sensitive_categories,
        now,
    } = search;
    let records = store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)
        .map_err(CorePipelineError::from)?;
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let requirement = SensitiveApprovalRequirement {
        task_id,
        change_unit_id: &change_unit_id,
        scope_revision: task.scope_revision,
        operation: intended_operation,
        normalized_paths,
        sensitive_categories,
        baseline_ref: Some(&request.baseline_ref),
        required_for: UserActionRequiredFor::PrepareWrite,
        now,
    };

    for record in records {
        let authority = volicord_user_action_service::user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            return Ok(Some(record));
        }
    }

    Ok(None)
}
