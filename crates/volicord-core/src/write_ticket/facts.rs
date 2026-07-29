use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId};
use volicord_types::product_path::path_is_within;
use volicord_types::values::{
    UserActionKind, UserActionRequiredFor, UtcTimestamp, WriteDecisionCategory,
};

use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, StoredUserActionRecordSet, TaskRecord,
};

use super::{
    WriteTicketDecisionCode, WriteTicketDecisionReason, WriteTicketPlanningError,
    WriteTicketRelatedRecord,
};
use crate::pipeline::{CoreResult, GitWorkspaceContext};
use crate::task_state::StoredScope;
use crate::write_ticket::write_decision_reason;
use volicord_user_action_service::{current_sensitive_approval, SensitiveApprovalRequirement};

pub(crate) fn load_prepare_write_task(
    store: &CoreProjectStore,
    task_id: &TaskId,
    task_is_current: bool,
) -> Result<(TaskRecord, Vec<WriteTicketDecisionReason>), WriteTicketPlanningError> {
    let task = store
        .task_record(task_id)?
        .ok_or(WriteTicketPlanningError::NoActiveTask)?;

    let mut reasons = Vec::new();
    if !task_is_current {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            WriteTicketDecisionCode::ScopeNotCurrent,
            "The addressed Task is not the current Task.",
            vec![WriteTicketRelatedRecord::Task(task_id.clone())],
        ));
    }

    Ok((task, reasons))
}

pub(crate) fn validate_prepare_write_change_unit(
    requested_change_unit_id: Option<&ChangeUnitId>,
    task_id: &TaskId,
    current_change_unit: &ChangeUnitRecord,
    reasons: &mut Vec<WriteTicketDecisionReason>,
) {
    if requested_change_unit_id
        .is_some_and(|change_unit_id| change_unit_id.as_str() != current_change_unit.change_unit_id)
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            WriteTicketDecisionCode::ScopeNotCurrent,
            "The addressed Change Unit is not the current Change Unit.",
            vec![WriteTicketRelatedRecord::CurrentChangeUnit {
                task_id: task_id.clone(),
                change_unit_id: ChangeUnitId::new(current_change_unit.change_unit_id.clone()),
            }],
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
    git_workspace_context: Option<&GitWorkspaceContext>,
) -> bool {
    match (
        change_unit.write_basis.git_workspace_context.as_ref(),
        git_workspace_context,
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
    }
}

pub(crate) fn paths_match_current_change_unit(
    intended_paths: &[String],
    change_unit: &ChangeUnitRecord,
) -> bool {
    if intended_paths.is_empty() {
        return true;
    }
    if change_unit.bounded_paths.is_empty() {
        return false;
    }
    let bounded_paths = &change_unit.bounded_paths;
    !bounded_paths.is_empty()
        && intended_paths.iter().all(|path| {
            bounded_paths
                .iter()
                .any(|scope| path_is_within(path, scope))
        })
}

pub(crate) struct SensitiveApprovalSearch<'a> {
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) task_id: &'a TaskId,
    pub(crate) task: &'a TaskRecord,
    pub(crate) change_unit: &'a ChangeUnitRecord,
    pub(crate) baseline_ref: &'a BaselineRef,
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
        task_id,
        task,
        change_unit,
        baseline_ref,
        intended_operation,
        normalized_paths,
        sensitive_categories,
        now,
    } = search;
    let records =
        store.resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)?;
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let requirement = SensitiveApprovalRequirement {
        task_id,
        change_unit_id: &change_unit_id,
        scope_revision: task.scope_revision,
        operation: intended_operation,
        normalized_paths,
        sensitive_categories,
        baseline_ref: Some(baseline_ref),
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
