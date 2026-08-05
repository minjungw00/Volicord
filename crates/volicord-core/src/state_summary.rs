use crate::pipeline::{CorePipelineError, CoreResult};
use crate::record_refs::state_ref;
use crate::task_state::StoredScope;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, ProjectStateHeader, ShapingCheckpointRecord, TaskRecord,
};
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::schema::{
    AcceptanceCriterion, CloseReadinessBlocker, EvidenceGateSummary, EvidenceSummary,
    GuaranteeDisplay, ProjectWorkflowPolicySummary, StateRecordRef, TaskLifecycleState,
    TaskLineageSummary, WorkflowProjection, WorkspaceContext, WriteTicketStateSummary,
};
use volicord_types::values::{
    CloseState, StateRecordKind, TaskLifecyclePhase, TaskResult, WorkspaceVcs,
};

pub(crate) struct StateSummaryInput<'a> {
    pub(crate) project_id: &'a ProjectId,
    pub(crate) state_version: u64,
    pub(crate) task: &'a TaskRecord,
    pub(crate) current_change_unit: Option<&'a ChangeUnitRecord>,
    pub(crate) shaping_checkpoint: Option<&'a ShapingCheckpointRecord>,
    pub(crate) task_wide_shaping_authority:
        &'a crate::workflow_projection::TaskWideShapingAuthority,
    pub(crate) project_policy: Option<ProjectWorkflowPolicySummary>,
    pub(crate) acceptance_criteria: Vec<AcceptanceCriterion>,
    pub(crate) pending_user_action_refs: Vec<StateRecordRef>,
    pub(crate) blocker_refs: Vec<StateRecordRef>,
    pub(crate) write_ticket_summary: Option<WriteTicketStateSummary>,
    pub(crate) evidence_summary: Option<EvidenceSummary>,
    pub(crate) evidence_gate: Option<EvidenceGateSummary>,
    pub(crate) close_state: Option<CloseState>,
    pub(crate) close_blockers: Vec<CloseReadinessBlocker>,
    pub(crate) guarantee_display: Option<GuaranteeDisplay>,
}

pub(crate) fn state_summary(
    input: StateSummaryInput<'_>,
) -> CoreResult<volicord_types::schema::StateSummary> {
    let StateSummaryInput {
        project_id,
        state_version,
        task,
        current_change_unit,
        shaping_checkpoint,
        task_wide_shaping_authority,
        project_policy,
        acceptance_criteria,
        pending_user_action_refs,
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate,
        close_state,
        close_blockers,
        guarantee_display,
    } = input;
    let task_id = TaskId::new(task.task_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task.task_id,
        project_id,
        Some(&task_id),
        Some(state_version),
    );
    let active_change_unit_ref = current_change_unit.map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            project_id,
            Some(&task_id),
            Some(record.basis_state_version),
        )
    });
    let effect_contract = current_change_unit.and_then(|record| record.effect_contract.clone());
    let workspace_context = current_change_unit
        .and_then(|record| record.write_basis.git_workspace_context.as_ref())
        .map(|workspace| WorkspaceContext {
            vcs: WorkspaceVcs::Git,
            git_common_dir: workspace.git_common_dir.clone(),
            worktree_id: workspace.worktree_id.clone(),
            branch_ref: workspace.branch_ref.clone(),
            head_sha: workspace.head_sha.clone(),
            workspace_fingerprint: workspace.workspace_fingerprint.clone(),
        });
    let lineage = match (
        task.predecessor_task_id.as_ref(),
        task.lineage_relation,
        task.lineage_reason.as_ref(),
    ) {
        (Some(predecessor_task_id), Some(relation), Some(creation_reason)) => {
            Some(TaskLineageSummary {
                predecessor_task_ref: state_ref(
                    StateRecordKind::Task,
                    predecessor_task_id,
                    project_id,
                    Some(&TaskId::new(predecessor_task_id.clone())),
                    Some(state_version),
                ),
                relation,
                creation_reason: creation_reason.clone(),
                carry_forward: task.carry_forward.clone(),
            })
        }
        (None, None, None) => None,
        _ => {
            return Err(CorePipelineError::Invariant {
                detail: "typed Store facts violate the Core `tasks.lineage` invariant".to_owned(),
            })
        }
    };
    let scope = StoredScope::from_task(task)?;
    let change_unit_scope =
        current_change_unit.and_then(|record| record.scope_summary.scope_summary.clone());
    let workflow = crate::workflow_projection::workflow_projection(
        project_id,
        state_version,
        task,
        current_change_unit,
        shaping_checkpoint,
        task_wide_shaping_authority,
    );
    let lifecycle_phase = if task.lifecycle_phase == TaskLifecyclePhase::WaitingUser
        && matches!(
            &workflow,
            WorkflowProjection::DecisionRecoveryRequired { .. }
        ) {
        TaskLifecyclePhase::Ready
    } else {
        task.lifecycle_phase
    };
    Ok(volicord_types::schema::StateSummary {
        project_id: project_id.clone(),
        state_version,
        task_ref: Some(task_ref),
        mode: Some(task.mode),
        requested_control_level: Some(task.requested_control_level),
        effective_control_level: Some(task.effective_control_level),
        control_level_reason: Some(task.control_level_reason.clone()),
        project_policy,
        work_phase: Some(task.work_phase),
        acceptance_policy: Some(task.acceptance_policy),
        acceptance_policy_reason: Some(task.acceptance_policy_reason.clone()),
        lineage,
        lifecycle: Some(TaskLifecycleState {
            lifecycle_phase,
            close_reason: task.close_summary.close_reason,
            result: task.result.unwrap_or(TaskResult::None),
            closed_at: task.closed_at.clone(),
        }),
        scope_revision: task.scope_revision,
        goal_summary: scope.goal_summary,
        scope_summary: change_unit_scope.or(scope.scope_summary),
        non_goals: scope.non_goals,
        acceptance_criteria,
        autonomy_boundary: scope.autonomy_boundary,
        active_change_unit_ref,
        effect_contract,
        baseline_ref: scope.baseline_ref,
        workspace_context,
        workflow,
        pending_user_action_summaries:
            volicord_user_action_service::agent_safe_pending_user_action_summaries(
                pending_user_action_refs,
            ),
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate,
        close_state,
        close_blockers,
        guarantee_display,
    })
}

pub(crate) fn project_state_header(
    project_state: &ProjectStateHeader,
    state_version: u64,
    active_task_id: Option<String>,
) -> ProjectStateHeader {
    ProjectStateHeader {
        project_id: project_state.project_id.clone(),
        state_version,
        active_task_id,
        updated_at: project_state.updated_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_pure_summary_signature(
        _projection: for<'a> fn(
            StateSummaryInput<'a>,
        ) -> CoreResult<volicord_types::schema::StateSummary>,
    ) {
    }

    #[test]
    fn state_summary_projection_accepts_typed_facts_without_store_access() {
        assert_pure_summary_signature(state_summary);
    }
}
