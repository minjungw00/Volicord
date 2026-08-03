use std::collections::BTreeSet;

use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ShapingCheckpointRecord, TaskRecord,
};
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::schema::{
    PersistedUserActionRequestMetadata, RequiredNullable, ShapingCheckpointGap,
    ShapingCheckpointSummary, StateRecordRef, WorkflowProjection, WorkflowRejectionUserAction,
};
use volicord_types::values::{
    AuthorityNextActor, MethodName, ShapingCheckpointReadiness, ShapingDecisionApplicationOwner,
    ShapingGapStatus, StateRecordKind, TaskLifecyclePhase, TaskMode, UserActionBasisStatus,
    UserActionRequiredFor, UserActionStatus, UtcTimestamp, WorkPhase, WorkflowBlockingReason,
};

use crate::pipeline::{CorePipelineError, CoreResult};
use crate::record_refs::state_ref;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkflowUserActionFact {
    pub(crate) request_ref: StateRecordRef,
    pub(crate) resolution_ref: Option<StateRecordRef>,
    pub(crate) status: UserActionStatus,
    pub(crate) required_owner_method: MethodName,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TaskWideShapingAuthority {
    pub(crate) pending: Vec<WorkflowUserActionFact>,
    pub(crate) resolved_unapplied: Vec<WorkflowUserActionFact>,
    pub(crate) inconsistent: Vec<WorkflowUserActionFact>,
    pub(crate) applied_resolution_ids: BTreeSet<String>,
}

impl TaskWideShapingAuthority {
    fn blocking_facts(&self) -> impl Iterator<Item = &WorkflowUserActionFact> {
        self.pending
            .iter()
            .chain(self.resolved_unapplied.iter())
            .chain(self.inconsistent.iter())
    }

    pub(crate) fn blocking_request_refs(&self) -> Vec<StateRecordRef> {
        let mut seen = BTreeSet::new();
        self.blocking_facts()
            .filter(|fact| seen.insert(fact.request_ref.record_id.as_str().to_owned()))
            .map(|fact| fact.request_ref.clone())
            .collect()
    }

    pub(crate) fn has_blockers(&self) -> bool {
        !self.pending.is_empty()
            || !self.resolved_unapplied.is_empty()
            || !self.inconsistent.is_empty()
    }

    pub(crate) fn blocks_advance_application(&self) -> bool {
        !self.pending.is_empty() || !self.inconsistent.is_empty()
    }

    pub(crate) fn blocking_user_actions(&self) -> Vec<WorkflowRejectionUserAction> {
        let mut seen = BTreeSet::new();
        self.blocking_facts()
            .filter(|fact| seen.insert(fact.request_ref.record_id.as_str().to_owned()))
            .map(|fact| WorkflowRejectionUserAction {
                user_action_request_ref: fact.request_ref.clone(),
                effective_status: fact.status,
                required_owner_method: fact.required_owner_method,
            })
            .collect()
    }
}

pub(crate) fn task_wide_shaping_authority(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    checkpoint: Option<&ShapingCheckpointRecord>,
    now: &UtcTimestamp,
) -> CoreResult<TaskWideShapingAuthority> {
    let task_id = TaskId::new(task.task_id.clone());
    let records = store
        .user_action_records_for_task(&task_id, now)
        .map_err(CorePipelineError::from)?;
    let mut assessment = TaskWideShapingAuthority::default();
    for record in records {
        let request = record.request();
        if !request
            .required_for()
            .contains(&UserActionRequiredFor::AdvanceTask)
        {
            continue;
        }
        let request_ref = state_ref(
            StateRecordKind::UserActionRequest,
            request.user_action_request_id(),
            project_id,
            Some(&task_id),
            Some(state_version),
        );
        let resolution_ref = record.resolution().map(|resolution| {
            state_ref(
                StateRecordKind::UserActionResolution,
                resolution.user_action_resolution_id(),
                project_id,
                Some(&task_id),
                Some(state_version),
            )
        });
        let represented_gap = checkpoint.and_then(|checkpoint| {
            checkpoint.gaps.iter().find(|gap| {
                gap.user_action.as_ref().is_some_and(|link| {
                    link.user_action_request_id == request.user_action_request_id()
                })
            })
        });
        let origin_matches = match (request.metadata(), checkpoint, represented_gap) {
            (
                PersistedUserActionRequestMetadata::Shaping(metadata),
                Some(checkpoint),
                Some(gap),
            ) => {
                metadata.shaping_checkpoint_id.as_str() == checkpoint.shaping_checkpoint_id
                    && metadata.shaping_gap_id.as_str() == gap.shaping_gap_id
            }
            _ => false,
        };
        let basis = request.basis();
        let coordinates = basis.coordinates();
        let current_basis_matches = request.basis_status() == UserActionBasisStatus::Current
            && basis.compatibility_status() == UserActionBasisStatus::Current
            && coordinates.task_id == task_id
            && coordinates.scope_revision == task.scope_revision
            && coordinates.change_unit_id.as_ref().map(|id| id.as_str())
                == current_change_unit.map(|change_unit| change_unit.change_unit_id.as_str())
            && coordinates
                .baseline_ref
                .as_ref()
                .map(|baseline| baseline.as_str())
                == task
                    .shaping
                    .baseline_ref
                    .as_ref()
                    .map(|baseline| baseline.as_str());
        let applied = represented_gap.is_some_and(|gap| {
            gap.status == ShapingGapStatus::Applied
                && gap.user_action.as_ref().is_some_and(|link| {
                    link.user_action_resolution_id.as_deref()
                        == record
                            .resolution()
                            .map(|resolution| resolution.user_action_resolution_id())
                })
        });
        let fact = WorkflowUserActionFact {
            request_ref,
            resolution_ref,
            status: record.status(),
            required_owner_method: match record.status() {
                UserActionStatus::Pending | UserActionStatus::Expired => {
                    MethodName::ResolveUserAction
                }
                UserActionStatus::Resolved if origin_matches => represented_gap
                    .and_then(|gap| gap.gap_kind.decision_policy())
                    .map_or(MethodName::Status, |policy| {
                        policy.application_owner.method()
                    }),
                UserActionStatus::Resolved => MethodName::Status,
                UserActionStatus::Stale | UserActionStatus::Superseded => MethodName::Status,
            },
        };
        match record.status() {
            UserActionStatus::Pending | UserActionStatus::Expired => {
                assessment.pending.push(fact.clone());
                if !origin_matches || !current_basis_matches {
                    assessment.inconsistent.push(fact);
                }
            }
            UserActionStatus::Resolved => {
                if applied && origin_matches && current_basis_matches {
                    if let Some(resolution) = record.resolution() {
                        assessment
                            .applied_resolution_ids
                            .insert(resolution.user_action_resolution_id().to_owned());
                    }
                } else {
                    assessment.resolved_unapplied.push(fact.clone());
                    if !origin_matches || !current_basis_matches {
                        assessment.inconsistent.push(fact);
                    }
                }
            }
            UserActionStatus::Stale | UserActionStatus::Superseded => {
                if applied && origin_matches {
                    if let Some(resolution) = record.resolution() {
                        assessment
                            .applied_resolution_ids
                            .insert(resolution.user_action_resolution_id().to_owned());
                    }
                } else if represented_gap.is_some() {
                    assessment.inconsistent.push(fact);
                }
            }
        }
    }
    Ok(assessment)
}

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
            application_owner: RequiredNullable::new(
                gap.gap_kind
                    .decision_policy()
                    .map(|policy| policy.application_owner),
            ),
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
    let unresolved_application_owners = gaps
        .iter()
        .filter(|gap| gap.status == ShapingGapStatus::Resolved)
        .filter_map(|gap| gap.application_owner.as_ref().copied())
        .collect::<BTreeSet<ShapingDecisionApplicationOwner>>()
        .into_iter()
        .collect();
    ShapingCheckpointSummary {
        checkpoint_ref: state_ref(
            StateRecordKind::ShapingCheckpoint,
            &checkpoint.shaping_checkpoint_id,
            project_id,
            Some(task_id),
            Some(state_version),
        ),
        predecessor_checkpoint_ref: RequiredNullable::new(
            checkpoint
                .predecessor_shaping_checkpoint_id
                .as_ref()
                .map(|predecessor_id| {
                    state_ref(
                        StateRecordKind::ShapingCheckpoint,
                        predecessor_id,
                        project_id,
                        Some(task_id),
                        Some(state_version),
                    )
                }),
        ),
        readiness: checkpoint.readiness,
        scope_revision: checkpoint.scope_revision,
        baseline_ref: RequiredNullable::new(checkpoint.baseline_ref.clone()),
        implementation_boundary: RequiredNullable::new(checkpoint.implementation_boundary.clone()),
        gaps,
        pending_decision_refs,
        unresolved_application_owners,
    }
}

pub(crate) fn workflow_projection(
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    checkpoint: Option<&ShapingCheckpointRecord>,
    task_wide_authority: &TaskWideShapingAuthority,
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
        for request_ref in &summary.pending_decision_refs {
            if !refs.contains(request_ref) {
                refs.push(request_ref.clone());
            }
        }
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
    for request_ref in task_wide_authority.blocking_request_refs() {
        if !refs.contains(&request_ref) {
            refs.push(request_ref);
        }
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
        if !task_wide_authority.pending.is_empty() {
            return WorkflowProjection::AwaitingUserAction {
                next_actor: AuthorityNextActor::User,
                required_action: RequiredNullable::some(MethodName::ResolveUserAction),
                allowed_actions: vec![MethodName::ResolveUserAction, MethodName::Status],
                required_refs: refs,
                expected_state_version: state_version,
                blocking_reason: RequiredNullable::some(
                    WorkflowBlockingReason::InconsistentAuthorityState,
                ),
                checkpoint: RequiredNullable::null(),
            };
        }
        if task_wide_authority.has_blockers() {
            return WorkflowProjection::ShapingRequired {
                next_actor: AuthorityNextActor::Agent,
                required_action: RequiredNullable::some(MethodName::Status),
                allowed_actions: vec![MethodName::Status],
                required_refs: refs,
                expected_state_version: state_version,
                blocking_reason: RequiredNullable::some(
                    WorkflowBlockingReason::InconsistentAuthorityState,
                ),
                checkpoint: RequiredNullable::null(),
            };
        }
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
    let has_pending_user = !task_wide_authority.pending.is_empty()
        || checkpoint
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
            blocking_reason: RequiredNullable::some(
                if task_wide_authority.inconsistent.is_empty() {
                    WorkflowBlockingReason::UserActionPending
                } else {
                    WorkflowBlockingReason::InconsistentAuthorityState
                },
            ),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if !task_wide_authority.inconsistent.is_empty() {
        return WorkflowProjection::ShapingRequired {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::Status),
            allowed_actions: vec![MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(
                WorkflowBlockingReason::InconsistentAuthorityState,
            ),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    let has_scope_decisions_to_apply = checkpoint.gaps.iter().any(|gap| {
        gap.status == ShapingGapStatus::Resolved
            && gap.gap_kind.decision_policy().is_some_and(|policy| {
                policy.application_owner == ShapingDecisionApplicationOwner::UpdateScope
            })
    });
    if has_scope_decisions_to_apply {
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
