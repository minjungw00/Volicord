use volicord_store::core_pipeline::{ChangeUnitRecord, ShapingCheckpointRecord, TaskRecord};
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::schema::{
    RequiredNullable, ShapingCheckpointGap, ShapingCheckpointSummary, WorkflowProjection,
};
use volicord_types::values::{
    AuthorityNextActor, MethodName, ShapingCheckpointReadiness, ShapingGapStatus, StateRecordKind,
    TaskLifecyclePhase, TaskMode, WorkPhase, WorkflowBlockingReason,
};

use crate::record_refs::state_ref;

fn checkpoint_summary(
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
    checkpoint: &ShapingCheckpointRecord,
) -> ShapingCheckpointSummary {
    let gaps = checkpoint
        .gaps
        .iter()
        .map(|gap| ShapingCheckpointGap {
            shaping_gap_id: volicord_types::ids::ShapingGapId::new(gap.shaping_gap_id.clone()),
            gap_kind: gap.gap_kind,
            summary: gap.summary.clone(),
            affected_refs: gap.affected_refs.clone(),
            status: gap.status,
            user_action_request_ref: RequiredNullable::new(gap.user_action.as_ref().map(|link| {
                state_ref(
                    StateRecordKind::UserActionRequest,
                    &link.user_action_request_id,
                    project_id,
                    Some(task_id),
                    Some(state_version),
                )
            })),
            user_action_resolution_ref: RequiredNullable::new(gap.user_action.as_ref().and_then(
                |link| {
                    link.user_action_resolution_id
                        .as_ref()
                        .map(|resolution_id| {
                            state_ref(
                                StateRecordKind::UserActionResolution,
                                resolution_id,
                                project_id,
                                Some(task_id),
                                Some(state_version),
                            )
                        })
                },
            )),
        })
        .collect::<Vec<_>>();
    let pending_decision_refs = gaps
        .iter()
        .filter(|gap| gap.status == ShapingGapStatus::Current)
        .filter_map(|gap| gap.user_action_request_ref.as_ref().cloned())
        .collect();
    ShapingCheckpointSummary {
        checkpoint_ref: state_ref(
            StateRecordKind::ShapingCheckpoint,
            &checkpoint.shaping_checkpoint_id,
            project_id,
            Some(task_id),
            Some(state_version),
        ),
        readiness: checkpoint.readiness,
        scope_revision: checkpoint.scope_revision,
        baseline_ref: RequiredNullable::new(checkpoint.baseline_ref.clone()),
        implementation_boundary: RequiredNullable::new(checkpoint.implementation_boundary.clone()),
        gaps,
        pending_decision_refs,
    }
}

pub(crate) fn workflow_projection(
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    checkpoint: Option<&ShapingCheckpointRecord>,
) -> WorkflowProjection {
    let task_id = TaskId::new(task.task_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task.task_id,
        project_id,
        Some(&task_id),
        Some(state_version),
    );
    let summary =
        checkpoint.map(|value| checkpoint_summary(project_id, &task_id, state_version, value));
    let mut refs = vec![task_ref];
    if let Some(summary) = summary.as_ref() {
        refs.push(summary.checkpoint_ref.clone());
    }
    if let Some(change_unit) = current_change_unit {
        refs.push(state_ref(
            StateRecordKind::ChangeUnit,
            &change_unit.change_unit_id,
            project_id,
            Some(&task_id),
            Some(change_unit.basis_state_version),
        ));
    }

    let terminal = matches!(
        task.lifecycle_phase,
        TaskLifecyclePhase::Completed
            | TaskLifecyclePhase::Cancelled
            | TaskLifecyclePhase::Superseded
    );
    if terminal {
        return WorkflowProjection::Terminal {
            next_actor: AuthorityNextActor::None,
            required_action: RequiredNullable::null(),
            allowed_actions: vec![MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if task.work_phase == WorkPhase::Implementation {
        return WorkflowProjection::Implementation {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::null(),
            allowed_actions: vec![
                MethodName::UpdateScope,
                MethodName::PrepareWrite,
                MethodName::RecordRun,
                MethodName::CheckClose,
            ],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    let Some(checkpoint) = checkpoint else {
        return WorkflowProjection::ShapingRequired {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::RecordShaping),
            allowed_actions: vec![MethodName::RecordShaping, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(WorkflowBlockingReason::NoCurrentCheckpoint),
            checkpoint: RequiredNullable::null(),
        };
    };
    let has_pending_user = checkpoint
        .gaps
        .iter()
        .any(|gap| gap.status == ShapingGapStatus::Current && gap.user_action.is_some());
    if has_pending_user {
        return WorkflowProjection::AwaitingUserAction {
            next_actor: AuthorityNextActor::User,
            required_action: RequiredNullable::some(MethodName::ResolveUserAction),
            allowed_actions: vec![MethodName::ResolveUserAction, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(WorkflowBlockingReason::UserActionPending),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if checkpoint
        .gaps
        .iter()
        .any(|gap| gap.status == ShapingGapStatus::Current)
    {
        return WorkflowProjection::ShapingRequired {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::RecordShaping),
            allowed_actions: vec![MethodName::RecordShaping, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(WorkflowBlockingReason::ShapingGapsCurrent),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if checkpoint
        .gaps
        .iter()
        .any(|gap| gap.status == ShapingGapStatus::Resolved)
    {
        return WorkflowProjection::ReadyToApplyDecisions {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::UpdateScope),
            allowed_actions: vec![MethodName::UpdateScope, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(
                WorkflowBlockingReason::ResolvedDecisionsNotApplied,
            ),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if task.mode == TaskMode::Advisor && checkpoint.readiness == ShapingCheckpointReadiness::Ready {
        return WorkflowProjection::CloseReview {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::CheckClose),
            allowed_actions: vec![
                MethodName::CheckClose,
                MethodName::CloseTask,
                MethodName::Status,
            ],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if current_change_unit.is_none() {
        return WorkflowProjection::ReadyForChangeUnit {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::UpdateScope),
            allowed_actions: vec![MethodName::UpdateScope, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(WorkflowBlockingReason::ChangeUnitRequired),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    WorkflowProjection::ReadyForImplementation {
        next_actor: AuthorityNextActor::Agent,
        required_action: RequiredNullable::some(MethodName::AdvanceTask),
        allowed_actions: vec![MethodName::AdvanceTask, MethodName::Status],
        required_refs: refs,
        expected_state_version: state_version,
        blocking_reason: RequiredNullable::some(WorkflowBlockingReason::ExplicitAdvanceRequired),
        checkpoint: RequiredNullable::new(summary),
    }
}
