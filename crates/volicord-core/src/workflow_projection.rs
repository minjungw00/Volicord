use std::collections::{BTreeMap, BTreeSet};

use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ShapingCheckpointGapRecord, ShapingCheckpointRecord,
    ShapingDecisionApplicationRecord, ShapingGapApplication, StoredUserActionRecordSet, TaskRecord,
};
use volicord_types::ids::{
    shaping_decision_application_id, BaselineRef, ChangeUnitId, ProjectId, ShapingCheckpointId,
    TaskId, UserActionResolutionId,
};
use volicord_types::schema::{
    PersistedUserActionRequestMetadata, RequiredNullable, ShapingCheckpointGap,
    ShapingCheckpointSummary, ShapingDecisionRecoveryRequirement, StateRecordRef,
    TransitionDescriptor, UserActionResolutionBody, WorkflowActionAuthorityCoordinates,
    WorkflowActionKey, WorkflowActionRole, WorkflowCheckpointActionCoordinates,
    WorkflowCloseReadiness, WorkflowProjection, WorkflowRejectionUserAction,
    WorkflowTransitionCatalog,
};
use volicord_types::values::{
    evaluate_shaping_decision_authority, ActorSource, AuthorityNextActor, ChangeUnitOperation,
    MethodName, RunKind, ShapingCheckpointReadiness, ShapingDecisionApplicationAuthorityStatus,
    ShapingDecisionApplicationOwner, ShapingDecisionAuthorityFacts, ShapingDecisionAuthorityState,
    ShapingGapStatus, StateRecordKind, TaskLifecyclePhase, TaskMode, UserActionRequiredFor,
    UserActionStatus, UtcTimestamp, WorkPhase, WorkflowActionSemanticVariant,
    WorkflowAgentInputRequirement, WorkflowBlockingReason, WorkflowExpectedResultState,
    WorkflowStateKind, WorkflowTransitionActor, WorkflowTransitionEffectClass,
};

use crate::pipeline::{CorePipelineError, CoreResult};
use crate::record_refs::state_ref;

pub(crate) fn apply_projected_shaping_applications(
    checkpoint: &mut ShapingCheckpointRecord,
    applications: &[ShapingGapApplication],
    owner: ShapingDecisionApplicationOwner,
    scope_revision: u64,
    baseline_ref: &BaselineRef,
    change_unit_id: Option<&ChangeUnitId>,
    applied_at: &UtcTimestamp,
) -> CoreResult<()> {
    for existing in &mut checkpoint.applications {
        if existing.authority_status == ShapingDecisionApplicationAuthorityStatus::Current
            && (existing.applied_scope_revision != scope_revision
                || &existing.applied_baseline_ref != baseline_ref
                || existing.applied_change_unit_id.as_ref() != change_unit_id)
        {
            existing.authority_status = ShapingDecisionApplicationAuthorityStatus::Stale;
            existing.stale_at = Some(applied_at.clone());
            existing.superseded_at = None;
        }
    }
    checkpoint.scope_revision = scope_revision;
    checkpoint.baseline_ref = Some(baseline_ref.clone());
    for application in applications {
        let gap = checkpoint
            .gaps
            .iter_mut()
            .find(|gap| gap.shaping_gap_id == application.shaping_gap_id)
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "a projected shaping application references a missing gap".to_owned(),
            })?;
        let link = gap
            .user_action
            .as_ref()
            .ok_or_else(|| CorePipelineError::Invariant {
                detail:
                    "a projected shaping application references a gap without UserAction authority"
                        .to_owned(),
            })?;
        let judgment_kind =
            gap.gap_kind
                .judgment_kind()
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "a projected shaping application references a non-judgment gap"
                        .to_owned(),
                })?;
        if link.user_action_resolution_id.as_deref()
            != Some(application.user_action_resolution_id.as_str())
        {
            return Err(CorePipelineError::Invariant {
                detail: "a projected shaping application does not match the gap resolution"
                    .to_owned(),
            });
        }
        if checkpoint.applications.iter().any(|existing| {
            existing.shaping_decision_application_id == application.shaping_decision_application_id
        }) {
            return Err(CorePipelineError::Invariant {
                detail: "a projected shaping application identity is duplicated".to_owned(),
            });
        }
        gap.status = ShapingGapStatus::Applied;
        checkpoint
            .applications
            .push(ShapingDecisionApplicationRecord {
                project_id: checkpoint.project_id.clone(),
                shaping_decision_application_id: application
                    .shaping_decision_application_id
                    .clone(),
                task_id: checkpoint.task_id.clone(),
                source_checkpoint_id: checkpoint.shaping_checkpoint_id.clone(),
                source_gap_id: application.shaping_gap_id.clone(),
                user_action_request_id: link.user_action_request_id.clone(),
                user_action_resolution_id: application.user_action_resolution_id.clone(),
                judgment_kind,
                application_owner: owner,
                applied_scope_revision: scope_revision,
                applied_baseline_ref: baseline_ref.clone(),
                applied_change_unit_id: change_unit_id.cloned(),
                applied_at: applied_at.clone(),
                authority_status: ShapingDecisionApplicationAuthorityStatus::Current,
                stale_at: None,
                superseded_at: None,
                linked_checkpoint_id: Some(checkpoint.shaping_checkpoint_id.clone()),
                carried_from_checkpoint_id: None,
            });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkflowUserActionFact {
    pub(crate) request_ref: StateRecordRef,
    pub(crate) resolution_ref: Option<StateRecordRef>,
    pub(crate) status: UserActionStatus,
    pub(crate) authority_state: ShapingDecisionAuthorityState,
    pub(crate) required_owner_method: MethodName,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TaskWideShapingAuthority {
    pub(crate) awaiting_user: Vec<WorkflowUserActionFact>,
    pub(crate) accepted_unapplied: Vec<WorkflowUserActionFact>,
    pub(crate) recovery_required: Vec<WorkflowUserActionFact>,
    pub(crate) applied: Vec<WorkflowUserActionFact>,
    pub(crate) stale: Vec<WorkflowUserActionFact>,
    pub(crate) inconsistent: Vec<WorkflowUserActionFact>,
    pub(crate) current_resolution_ids: BTreeSet<String>,
    pub(crate) stale_application_refs: Vec<StateRecordRef>,
}

impl TaskWideShapingAuthority {
    fn all_facts(&self) -> impl Iterator<Item = &WorkflowUserActionFact> {
        self.awaiting_user
            .iter()
            .chain(self.accepted_unapplied.iter())
            .chain(self.recovery_required.iter())
            .chain(self.applied.iter())
            .chain(self.stale.iter())
            .chain(self.inconsistent.iter())
    }

    fn blocking_facts(&self) -> impl Iterator<Item = &WorkflowUserActionFact> {
        self.awaiting_user
            .iter()
            .chain(self.accepted_unapplied.iter())
            .chain(self.recovery_required.iter())
            .chain(self.stale.iter())
            .chain(self.inconsistent.iter())
    }

    pub(crate) fn blocking_request_refs(&self) -> Vec<StateRecordRef> {
        let mut seen = BTreeSet::new();
        let mut refs = self
            .blocking_facts()
            .filter(|fact| seen.insert(fact.request_ref.record_id.as_str().to_owned()))
            .map(|fact| fact.request_ref.clone())
            .collect::<Vec<_>>();
        refs.sort();
        refs
    }

    pub(crate) fn blocks_advance_application(&self) -> bool {
        !self.awaiting_user.is_empty()
            || !self.recovery_required.is_empty()
            || !self.stale.is_empty()
            || !self.inconsistent.is_empty()
    }

    pub(crate) fn blocking_user_actions(&self) -> Vec<WorkflowRejectionUserAction> {
        let mut seen = BTreeSet::new();
        let mut actions = self
            .blocking_facts()
            .filter(|fact| seen.insert(fact.request_ref.record_id.as_str().to_owned()))
            .map(|fact| WorkflowRejectionUserAction {
                user_action_request_ref: fact.request_ref.clone(),
                effective_status: fact.status,
                required_owner_method: fact.required_owner_method,
            })
            .collect::<Vec<_>>();
        actions.sort_by(|left, right| {
            left.user_action_request_ref
                .cmp(&right.user_action_request_ref)
        });
        actions
    }

    fn resolvable_user_action_refs(&self) -> Vec<StateRecordRef> {
        let mut seen = BTreeSet::new();
        let mut refs = self
            .all_facts()
            .filter(|fact| fact.status == UserActionStatus::Pending)
            .filter(|fact| seen.insert(fact.request_ref.record_id.as_str().to_owned()))
            .map(|fact| fact.request_ref.clone())
            .collect::<Vec<_>>();
        refs.sort();
        refs
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
    let graph = store
        .current_shaping_authority_graph(&task_id, now)
        .map_err(CorePipelineError::from)?;
    let stale_application_refs = graph
        .stale_recovery_obligations
        .iter()
        .map(|authority| {
            state_ref(
                StateRecordKind::ShapingDecisionApplication,
                &authority.application.shaping_decision_application_id,
                project_id,
                Some(&task_id),
                Some(state_version),
            )
        })
        .collect::<Vec<_>>();
    let projecting_next_state = state_version
        > store
            .project_state()
            .map_err(CorePipelineError::from)?
            .state_version;
    let projected_checkpoint_replaces_stored = projecting_next_state
        && match (checkpoint, graph.current_checkpoint.as_ref()) {
            (None, Some(_)) => true,
            (Some(projected), Some(stored)) => {
                stored.shaping_checkpoint_id != projected.shaping_checkpoint_id
            }
            (Some(_), None) | (None, None) => false,
        };
    let projected_application_ids = checkpoint
        .map(|checkpoint| {
            checkpoint
                .applications
                .iter()
                .map(|application| application.shaping_decision_application_id.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut records_by_request_id = BTreeMap::<String, StoredUserActionRecordSet>::new();
    for decision in graph.current_gap_decisions {
        let represented_by_projection = !projected_checkpoint_replaces_stored
            || checkpoint.is_some_and(|checkpoint| {
                checkpoint.gaps.iter().any(|gap| {
                    gap.user_action.as_ref().is_some_and(|link| {
                        link.user_action_request_id
                            == decision.user_action.request().user_action_request_id()
                    })
                })
            });
        if represented_by_projection {
            records_by_request_id.insert(
                decision
                    .user_action
                    .request()
                    .user_action_request_id()
                    .to_owned(),
                decision.user_action,
            );
        }
    }
    let mut application_sources = BTreeMap::<String, (String, ShapingCheckpointGapRecord)>::new();
    let mut applications = Vec::new();
    for authority in graph
        .current_applications
        .into_iter()
        .filter(|authority| {
            !projected_checkpoint_replaces_stored
                || projected_application_ids.contains(
                    authority
                        .application
                        .shaping_decision_application_id
                        .as_str(),
                )
        })
        .chain(graph.stale_recovery_obligations)
    {
        records_by_request_id.insert(
            authority
                .user_action
                .request()
                .user_action_request_id()
                .to_owned(),
            authority.user_action,
        );
        application_sources.insert(
            authority
                .application
                .shaping_decision_application_id
                .clone(),
            (
                authority.application.source_checkpoint_id.clone(),
                authority.source_gap,
            ),
        );
        applications.push(authority.application);
    }
    if let Some(checkpoint) = checkpoint {
        for projected in &checkpoint.applications {
            applications.retain(|stored| {
                stored.shaping_decision_application_id != projected.shaping_decision_application_id
            });
            if !application_sources.contains_key(&projected.shaping_decision_application_id) {
                let source_gap = (projected.source_checkpoint_id
                    == checkpoint.shaping_checkpoint_id)
                    .then(|| {
                        checkpoint
                            .gaps
                            .iter()
                            .find(|gap| gap.shaping_gap_id == projected.source_gap_id)
                            .cloned()
                    })
                    .flatten()
                    .ok_or_else(|| CorePipelineError::Invariant {
                        detail: "a projected shaping application has no Store-validated source"
                            .to_owned(),
                    })?;
                application_sources.insert(
                    projected.shaping_decision_application_id.clone(),
                    (projected.source_checkpoint_id.clone(), source_gap),
                );
            }
            applications.push(projected.clone());
        }
    }
    let records = records_by_request_id.into_values().collect::<Vec<_>>();
    let mut assessment = TaskWideShapingAuthority {
        stale_application_refs,
        ..TaskWideShapingAuthority::default()
    };
    for record in records {
        let request = record.request();
        let resolution_id = record
            .resolution()
            .map(|resolution| resolution.user_action_resolution_id());
        let application = applications.iter().find(|application| {
            application.user_action_request_id == request.user_action_request_id()
                && resolution_id == Some(application.user_action_resolution_id.as_str())
        });
        let represented_gap = checkpoint.and_then(|checkpoint| {
            checkpoint.gaps.iter().find(|gap| {
                gap.user_action.as_ref().is_some_and(|link| {
                    link.user_action_request_id == request.user_action_request_id()
                })
            })
        });
        let application_is_relevant = application.is_some_and(|application| {
            matches!(
                application.authority_status,
                ShapingDecisionApplicationAuthorityStatus::Current
                    | ShapingDecisionApplicationAuthorityStatus::Stale
            )
        });
        let participates_in_progression = represented_gap.is_some()
            || application_is_relevant
            || if task.mode == TaskMode::Advisor {
                request
                    .required_for()
                    .contains(&UserActionRequiredFor::FinalizeAdvice)
                    || request
                        .required_for()
                        .contains(&UserActionRequiredFor::ScopeUpdate)
            } else {
                request
                    .required_for()
                    .contains(&UserActionRequiredFor::AdvanceTask)
            };
        if !participates_in_progression {
            continue;
        }
        let source = if let Some(application) = application {
            application_sources
                .get(&application.shaping_decision_application_id)
                .cloned()
        } else {
            checkpoint.and_then(|checkpoint| {
                represented_gap
                    .cloned()
                    .map(|gap| (checkpoint.shaping_checkpoint_id.clone(), gap))
            })
        };
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
        let origin_matches = match (request.metadata(), source.as_ref()) {
            (PersistedUserActionRequestMetadata::Shaping(metadata), Some((checkpoint_id, gap))) => {
                metadata.shaping_checkpoint_id.as_str() == checkpoint_id
                    && metadata.shaping_gap_id.as_str() == gap.shaping_gap_id
                    && metadata
                        .reauthorizes_application_id
                        .as_ref()
                        .map(|id| id.as_str())
                        == gap.reauthorizes_application_id.as_deref()
            }
            _ => false,
        };
        let basis = request.basis();
        let coordinates = basis.coordinates();
        let policy = source
            .as_ref()
            .and_then(|(_, gap)| gap.gap_kind.decision_policy_for_mode(task.mode));
        let (machine_action, resolution_outcome) = match record
            .resolution()
            .map(|resolution| resolution.resolution())
        {
            Some(UserActionResolutionBody::Choice {
                machine_action,
                resolution_outcome,
                ..
            }) => (Some(*machine_action), Some(*resolution_outcome)),
            _ => (None, None),
        };
        let resolution_identity_matches = source.as_ref().is_some_and(|(_, gap)| {
            gap.user_action
                .as_ref()
                .is_some_and(|link| link.user_action_resolution_id.as_deref() == resolution_id)
        });
        let verified_user_channel = record.resolution().is_some_and(|resolution| {
            resolution.resolved_by_actor_source() == &ActorSource::LocalUser
                && resolution.resolved_verification_basis()
                    == resolution.channel_kind().verification_basis()
                && !resolution.resolved_assurance_level().trim().is_empty()
        });
        let application_identity_matches = application.is_some_and(|application| {
            let expected_id = shaping_decision_application_id(
                &UserActionResolutionId::new(&application.user_action_resolution_id),
                application.application_owner,
            )
            .ok();
            source.as_ref().is_some_and(|(source_checkpoint_id, gap)| {
                application.project_id == project_id.as_str()
                    && application.task_id == task.task_id
                    && application.source_checkpoint_id == *source_checkpoint_id
                    && application.source_gap_id == gap.shaping_gap_id
                    && application.user_action_request_id == request.user_action_request_id()
                    && resolution_id == Some(application.user_action_resolution_id.as_str())
                    && gap.gap_kind.judgment_kind() == Some(application.judgment_kind)
                    && policy.is_some_and(|policy| {
                        policy.application_owner == application.application_owner
                    })
                    && expected_id.as_ref().is_some_and(|expected_id| {
                        expected_id.as_str() == application.shaping_decision_application_id
                    })
            })
        });
        let scope_revision_matches = application.map_or(
            projecting_next_state || coordinates.scope_revision == task.scope_revision,
            |application| application.applied_scope_revision == task.scope_revision,
        );
        let baseline_matches = application.map_or_else(
            || {
                if projecting_next_state {
                    return checkpoint.is_some_and(|checkpoint| {
                        checkpoint.baseline_ref.as_ref() == task.shaping.baseline_ref.as_ref()
                    });
                }
                coordinates
                    .baseline_ref
                    .as_ref()
                    .map(|baseline| baseline.as_str())
                    == task
                        .shaping
                        .baseline_ref
                        .as_ref()
                        .map(|baseline| baseline.as_str())
            },
            |application| {
                Some(application.applied_baseline_ref.as_str())
                    == task
                        .shaping
                        .baseline_ref
                        .as_ref()
                        .map(|baseline| baseline.as_str())
            },
        );
        let change_unit_matches = application.map_or_else(
            || {
                if projecting_next_state {
                    return true;
                }
                coordinates.change_unit_id.as_ref().map(|id| id.as_str())
                    == current_change_unit.map(|change_unit| change_unit.change_unit_id.as_str())
            },
            |application| {
                application
                    .applied_change_unit_id
                    .as_ref()
                    .map(|id| id.as_str())
                    == current_change_unit.map(|change_unit| change_unit.change_unit_id.as_str())
            },
        );
        let mut authority_state =
            evaluate_shaping_decision_authority(ShapingDecisionAuthorityFacts {
                effective_user_action_status: record.status(),
                resolution_present: record.resolution().is_some(),
                machine_action,
                resolution_outcome,
                request_basis_status: request.basis_status(),
                basis_compatibility_status: basis.compatibility_status(),
                checkpoint_identity_matches: origin_matches,
                gap_identity_matches: source.is_some(),
                resolution_identity_matches: !record.resolution().is_some()
                    || resolution_identity_matches,
                policy_matches: policy.is_some_and(|policy| {
                    policy.user_action_kind == request.action_kind()
                        && policy.required_for == request.required_for()
                }),
                verified_user_channel: !record.resolution().is_some() || verified_user_channel,
                task_mode_matches: policy.is_some(),
                scope_revision_matches: coordinates.task_id == task_id && scope_revision_matches,
                baseline_matches,
                change_unit_matches,
                gap_status: source
                    .as_ref()
                    .map_or(ShapingGapStatus::Current, |(_, gap)| gap.status),
                application_present: application.is_some(),
                application_authority_status: application
                    .map(|application| application.authority_status),
                application_identity_matches,
                application_lineage_current: application.is_some_and(|application| {
                    checkpoint.is_some_and(|checkpoint| {
                        application.linked_checkpoint_id.as_deref()
                            == Some(checkpoint.shaping_checkpoint_id.as_str())
                    })
                }),
            });
        if represented_gap.is_none()
            && record.status() == UserActionStatus::Pending
            && request.basis_status() == volicord_types::values::UserActionBasisStatus::Current
            && basis.compatibility_status()
                == volicord_types::values::UserActionBasisStatus::Current
            && coordinates.task_id == task_id
            && coordinates.scope_revision == task.scope_revision
        {
            authority_state = ShapingDecisionAuthorityState::AwaitingUser;
        }
        let fact = WorkflowUserActionFact {
            request_ref,
            resolution_ref,
            status: record.status(),
            authority_state,
            required_owner_method: match authority_state {
                ShapingDecisionAuthorityState::AwaitingUser => MethodName::ResolveUserAction,
                ShapingDecisionAuthorityState::AcceptedUnapplied => policy
                    .map_or(MethodName::Status, |policy| {
                        policy.application_owner.method()
                    }),
                ShapingDecisionAuthorityState::Rejected
                | ShapingDecisionAuthorityState::Deferred
                | ShapingDecisionAuthorityState::Expired => MethodName::RecordShapingCheckpoint,
                ShapingDecisionAuthorityState::Stale => {
                    if task.work_phase == WorkPhase::Shaping {
                        MethodName::RecordShapingCheckpoint
                    } else {
                        MethodName::CloseTask
                    }
                }
                ShapingDecisionAuthorityState::Applied
                | ShapingDecisionAuthorityState::Superseded
                | ShapingDecisionAuthorityState::Inconsistent => MethodName::Status,
            },
        };
        match authority_state {
            ShapingDecisionAuthorityState::AwaitingUser => {
                assessment.awaiting_user.push(fact.clone());
                if represented_gap.is_none() {
                    assessment.inconsistent.push(fact);
                }
            }
            ShapingDecisionAuthorityState::AcceptedUnapplied => {
                assessment.accepted_unapplied.push(fact)
            }
            ShapingDecisionAuthorityState::Rejected
            | ShapingDecisionAuthorityState::Deferred
            | ShapingDecisionAuthorityState::Expired => assessment.recovery_required.push(fact),
            ShapingDecisionAuthorityState::Applied => {
                assessment.applied.push(fact);
                if let Some(resolution) = record.resolution() {
                    assessment
                        .current_resolution_ids
                        .insert(resolution.user_action_resolution_id().to_owned());
                }
            }
            ShapingDecisionAuthorityState::Stale => assessment.stale.push(fact),
            ShapingDecisionAuthorityState::Superseded => {}
            ShapingDecisionAuthorityState::Inconsistent => assessment.inconsistent.push(fact),
        }
    }
    if projecting_next_state {
        if let Some(checkpoint) = checkpoint {
            for gap in checkpoint
                .gaps
                .iter()
                .filter(|gap| gap.status == ShapingGapStatus::Current)
            {
                let Some(link) = gap.user_action.as_ref() else {
                    continue;
                };
                if assessment
                    .all_facts()
                    .any(|fact| fact.request_ref.record_id.as_str() == link.user_action_request_id)
                {
                    continue;
                }
                assessment.awaiting_user.push(WorkflowUserActionFact {
                    request_ref: state_ref(
                        StateRecordKind::UserActionRequest,
                        &link.user_action_request_id,
                        project_id,
                        Some(&task_id),
                        Some(state_version),
                    ),
                    resolution_ref: None,
                    status: UserActionStatus::Pending,
                    authority_state: ShapingDecisionAuthorityState::AwaitingUser,
                    required_owner_method: MethodName::ResolveUserAction,
                });
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
    task_mode: TaskMode,
    task_wide_authority: &TaskWideShapingAuthority,
) -> ShapingCheckpointSummary {
    let gaps = checkpoint
        .gaps
        .iter()
        .map(|gap| {
            let authority_state = gap.user_action.as_ref().and_then(|link| {
                task_wide_authority
                    .all_facts()
                    .find(|fact| fact.request_ref.record_id.as_str() == link.user_action_request_id)
                    .map(|fact| fact.authority_state)
                    .or_else(|| {
                        (gap.status == ShapingGapStatus::Applied)
                            .then_some(ShapingDecisionAuthorityState::Applied)
                    })
                    .or_else(|| {
                        (gap.status == ShapingGapStatus::Current)
                            .then_some(ShapingDecisionAuthorityState::AwaitingUser)
                    })
            });
            ShapingCheckpointGap {
                shaping_gap_id: volicord_types::ids::ShapingGapId::new(gap.shaping_gap_id.clone()),
                gap_kind: gap.gap_kind,
                application_owner: RequiredNullable::new(
                    gap.gap_kind
                        .decision_policy_for_mode(task_mode)
                        .map(|policy| policy.application_owner),
                ),
                summary: gap.summary.clone(),
                affected_refs: gap.affected_refs.clone(),
                status: gap.status,
                decision_authority_state: RequiredNullable::new(authority_state),
                user_action_request_ref: RequiredNullable::new(gap.user_action.as_ref().map(
                    |link| {
                        state_ref(
                            StateRecordKind::UserActionRequest,
                            &link.user_action_request_id,
                            project_id,
                            Some(task_id),
                            Some(state_version),
                        )
                    },
                )),
                user_action_resolution_ref: RequiredNullable::new(
                    gap.user_action.as_ref().and_then(|link| {
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
                    }),
                ),
                reauthorizes_application_ref: RequiredNullable::new(
                    gap.reauthorizes_application_id
                        .as_ref()
                        .map(|application_id| {
                            state_ref(
                                StateRecordKind::ShapingDecisionApplication,
                                application_id,
                                project_id,
                                Some(task_id),
                                Some(state_version),
                            )
                        }),
                ),
            }
        })
        .collect::<Vec<_>>();
    let pending_decision_refs = gaps
        .iter()
        .filter(|gap| {
            gap.decision_authority_state.as_ref()
                == Some(&ShapingDecisionAuthorityState::AwaitingUser)
        })
        .filter_map(|gap| gap.user_action_request_ref.as_ref().cloned())
        .collect();
    let unresolved_application_owners = gaps
        .iter()
        .filter(|gap| gap.status == ShapingGapStatus::Accepted)
        .filter_map(|gap| gap.application_owner.as_ref().copied())
        .collect::<BTreeSet<ShapingDecisionApplicationOwner>>()
        .into_iter()
        .collect();
    let decision_recovery_requirements = gaps
        .iter()
        .filter_map(|gap| {
            let disposition = gap.decision_authority_state.as_ref().copied()?;
            let reason = disposition.recovery_reason()?;
            Some(ShapingDecisionRecoveryRequirement {
                shaping_gap_id: gap.shaping_gap_id.clone(),
                user_action_request_ref: gap.user_action_request_ref.as_ref()?.clone(),
                user_action_resolution_ref: gap.user_action_resolution_ref.clone(),
                disposition,
                reason,
            })
        })
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
        current_application_refs: checkpoint
            .applications
            .iter()
            .filter(|application| {
                application.authority_status == ShapingDecisionApplicationAuthorityStatus::Current
                    && application.linked_checkpoint_id.as_deref()
                        == Some(checkpoint.shaping_checkpoint_id.as_str())
            })
            .map(|application| {
                state_ref(
                    StateRecordKind::ShapingDecisionApplication,
                    &application.shaping_decision_application_id,
                    project_id,
                    Some(task_id),
                    Some(state_version),
                )
            })
            .collect(),
        gaps,
        pending_decision_refs,
        unresolved_application_owners,
        decision_recovery_requirements,
    }
}

/// One normalized current-authority snapshot consumed by the pure workflow machine.
///
/// Store owns graph construction and strict record decoding. This snapshot owns only
/// the current facts and explicit stale obligations needed for progression; it never
/// receives superseded history.
#[derive(Debug)]
pub(crate) struct WorkflowSnapshot<'a> {
    pub(crate) project_id: &'a ProjectId,
    pub(crate) state_version: u64,
    pub(crate) task: &'a TaskRecord,
    pub(crate) current_scope: &'a volicord_store::core_pipeline::TaskShapingFacts,
    pub(crate) baseline: Option<&'a BaselineRef>,
    pub(crate) current_change_unit: Option<&'a ChangeUnitRecord>,
    pub(crate) checkpoint: Option<&'a ShapingCheckpointRecord>,
    pub(crate) checkpoint_summary: Option<ShapingCheckpointSummary>,
    pub(crate) shaping_authority_graph: &'a TaskWideShapingAuthority,
    pub(crate) user_action_state: WorkflowUserActionState,
    pub(crate) write_ticket_state: WorkflowDelegatedAuthorityState,
    pub(crate) close_basis: Option<&'a volicord_types::schema::CurrentCloseBasis>,
    pub(crate) evidence_state: WorkflowAuthorityPresence,
    pub(crate) final_acceptance_state: WorkflowDelegatedAuthorityState,
    pub(crate) recovery_constraints: &'a [String],
    pub(crate) current_workspace_context:
        Option<&'a volicord_store::core_pipeline::StoredGitWorkspaceContext>,
    pub(crate) required_refs: Vec<StateRecordRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowAuthorityPresence {
    Absent,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowDelegatedAuthorityState {
    SeparateAssessmentOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowUserActionState {
    pub(crate) awaiting_user: usize,
    pub(crate) accepted_unapplied: usize,
    pub(crate) recovery_required: usize,
    pub(crate) inconsistent: usize,
}

impl<'a> WorkflowSnapshot<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        project_id: &'a ProjectId,
        state_version: u64,
        task: &'a TaskRecord,
        current_change_unit: Option<&'a ChangeUnitRecord>,
        checkpoint: Option<&'a ShapingCheckpointRecord>,
        shaping_authority_graph: &'a TaskWideShapingAuthority,
        checkpoint_summary: Option<ShapingCheckpointSummary>,
        mut required_refs: Vec<StateRecordRef>,
    ) -> CoreResult<Self> {
        let contradiction = |detail: &str| CorePipelineError::Invariant {
            detail: format!("workflow snapshot contradicts current authority: {detail}"),
        };
        if task.project_id != project_id.as_str() {
            return Err(contradiction("Task project identity"));
        }
        if shaping_authority_graph
            .all_facts()
            .map(|fact| fact.request_ref.record_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != shaping_authority_graph.all_facts().count()
        {
            return Err(contradiction("duplicate current UserAction authority"));
        }
        match (task.current_change_unit_id.as_deref(), current_change_unit) {
            (None, None) => {}
            (Some(expected), Some(change_unit))
                if change_unit.project_id == task.project_id
                    && change_unit.task_id == task.task_id
                    && change_unit.is_current
                    && change_unit.change_unit_id == expected => {}
            _ => return Err(contradiction("current Change Unit identity")),
        }
        if let Some(checkpoint) = checkpoint {
            if checkpoint.project_id != task.project_id
                || checkpoint.task_id != task.task_id
                || checkpoint.superseded_at.is_some()
            {
                return Err(contradiction("current checkpoint identity or status"));
            }
        }
        if checkpoint.is_some() != checkpoint_summary.is_some() {
            return Err(contradiction("checkpoint summary presence"));
        }
        let terminal = matches!(
            task.lifecycle_phase,
            TaskLifecyclePhase::Completed
                | TaskLifecyclePhase::Cancelled
                | TaskLifecyclePhase::Superseded
        );
        if terminal != task.closed_at.is_some() {
            return Err(contradiction("terminal lifecycle and closed_at"));
        }
        let close_basis = task.close_basis.as_ref();
        if let Some(basis) = close_basis {
            if basis.task_id.as_str() != task.task_id {
                return Err(contradiction("current close basis"));
            }
        }
        let recovery_constraints = close_basis
            .map(|basis| basis.recovery_constraints.as_slice())
            .unwrap_or_default();
        let evidence_state = if close_basis
            .and_then(|basis| basis.evidence_summary_ref.as_ref())
            .is_some()
        {
            WorkflowAuthorityPresence::Present
        } else {
            WorkflowAuthorityPresence::Absent
        };
        let current_workspace_context = current_change_unit
            .and_then(|change_unit| change_unit.write_basis.git_workspace_context.as_ref());
        required_refs.sort();
        required_refs.dedup();
        Ok(Self {
            project_id,
            state_version,
            task,
            current_scope: &task.shaping,
            baseline: task.shaping.baseline_ref.as_ref(),
            current_change_unit,
            checkpoint,
            checkpoint_summary,
            shaping_authority_graph,
            user_action_state: WorkflowUserActionState {
                awaiting_user: shaping_authority_graph.resolvable_user_action_refs().len(),
                accepted_unapplied: shaping_authority_graph.accepted_unapplied.len(),
                recovery_required: shaping_authority_graph.recovery_required.len(),
                inconsistent: shaping_authority_graph.inconsistent.len(),
            },
            write_ticket_state: WorkflowDelegatedAuthorityState::SeparateAssessmentOwner,
            close_basis,
            evidence_state,
            final_acceptance_state: WorkflowDelegatedAuthorityState::SeparateAssessmentOwner,
            recovery_constraints,
            current_workspace_context,
            required_refs,
        })
    }

    fn validate_separated_authority(&self) -> CoreResult<()> {
        let evidence_present = self
            .close_basis
            .and_then(|basis| basis.evidence_summary_ref.as_ref())
            .is_some();
        let recovery_constraints = self
            .close_basis
            .map(|basis| basis.recovery_constraints.as_slice())
            .unwrap_or_default();
        let workspace_context = self
            .current_change_unit
            .and_then(|change_unit| change_unit.write_basis.git_workspace_context.as_ref());
        if self.project_id.as_str() != self.task.project_id
            || !std::ptr::eq(self.current_scope, &self.task.shaping)
            || matches!(
                self.write_ticket_state,
                WorkflowDelegatedAuthorityState::SeparateAssessmentOwner
            ) != matches!(
                self.final_acceptance_state,
                WorkflowDelegatedAuthorityState::SeparateAssessmentOwner
            )
            || evidence_present != matches!(self.evidence_state, WorkflowAuthorityPresence::Present)
            || recovery_constraints != self.recovery_constraints
            || workspace_context != self.current_workspace_context
        {
            return Err(CorePipelineError::Invariant {
                detail: "workflow snapshot separated authority facts are inconsistent".to_owned(),
            });
        }
        Ok(())
    }
}

/// Complete result of evaluating one normalized snapshot.
#[derive(Debug)]
pub(crate) struct WorkflowEvaluation {
    pub(crate) workflow_kind: WorkflowStateKind,
    pub(crate) next_actor: AuthorityNextActor,
    pub(crate) transition_catalog: WorkflowTransitionCatalog,
    pub(crate) typed_blocking_reason: Option<WorkflowBlockingReason>,
    pub(crate) required_refs: Vec<StateRecordRef>,
    pub(crate) close_readiness: WorkflowCloseReadiness,
    pub(crate) checkpoint: Option<ShapingCheckpointSummary>,
}

/// Pure owner of current Task progression and transition admission.
pub(crate) struct WorkflowMachine;

fn transition_descriptor(
    method: MethodName,
    semantic_variant: WorkflowActionSemanticVariant,
    role: WorkflowActionRole,
    snapshot: &WorkflowSnapshot<'_>,
) -> CoreResult<TransitionDescriptor> {
    let action_key = WorkflowActionKey::new(method, semantic_variant).map_err(|detail| {
        CorePipelineError::Invariant {
            detail: detail.to_owned(),
        }
    })?;
    let state_version = snapshot.state_version;
    let task = snapshot.task;
    let current_change_unit = snapshot.current_change_unit;
    let checkpoint = snapshot.checkpoint_summary.as_ref();
    let task_wide_authority = snapshot.shaping_authority_graph;
    let required_refs = snapshot.required_refs.as_slice();
    let task_id = TaskId::new(&task.task_id);
    let baseline_ref = RequiredNullable::new(task.shaping.baseline_ref.clone());
    let resolution_refs_for = |owner: MethodName| {
        let mut refs = task_wide_authority
            .accepted_unapplied
            .iter()
            .filter(|fact| fact.required_owner_method == owner)
            .filter_map(|fact| fact.resolution_ref.clone())
            .collect::<Vec<_>>();
        refs.sort();
        refs.dedup();
        refs
    };
    let resolution_ids_for = |owner: MethodName| {
        let mut ids = task_wide_authority
            .accepted_unapplied
            .iter()
            .filter(|fact| fact.required_owner_method == owner)
            .filter_map(|fact| fact.resolution_ref.as_ref())
            .map(|reference| UserActionResolutionId::new(reference.record_id.as_str()))
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        ids
    };
    let fixed_authority_coordinates = match method {
        MethodName::RecordShapingCheckpoint => {
            let checkpoint_operation = checkpoint.map_or(
                WorkflowCheckpointActionCoordinates::CreateInitial,
                |checkpoint| WorkflowCheckpointActionCoordinates::ReplaceCurrent {
                    current_checkpoint_ref: checkpoint.checkpoint_ref.clone(),
                    predecessor_checkpoint_ref: checkpoint.predecessor_checkpoint_ref.clone(),
                    retired_non_authorizing_request_refs: task_wide_authority
                        .recovery_required
                        .iter()
                        .map(|fact| fact.request_ref.clone())
                        .collect(),
                    carry_forward_application_refs: checkpoint.current_application_refs.clone(),
                    stale_application_refs: task_wide_authority.stale_application_refs.clone(),
                },
            );
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                task_id,
                checkpoint_operation,
                scope_revision: task.scope_revision,
                baseline_ref,
            }
        }
        MethodName::UpdateScope => {
            let selected_change_unit_operation = semantic_variant
                .change_unit_operation()
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "update-scope transition has no Change Unit operation".to_owned(),
                })?;
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id,
                scope_revision: task.scope_revision,
                baseline_ref,
                current_change_unit_id: RequiredNullable::new(
                    current_change_unit
                        .map(|change_unit| ChangeUnitId::new(&change_unit.change_unit_id)),
                ),
                related_scope_decision_refs: resolution_refs_for(MethodName::UpdateScope),
                selected_change_unit_operation,
            }
        }
        MethodName::FinalizeAdvice => {
            let checkpoint = checkpoint.ok_or_else(|| CorePipelineError::Invariant {
                detail: "advisor-finalization transition requires a current checkpoint".to_owned(),
            })?;
            let change_unit = current_change_unit.ok_or_else(|| CorePipelineError::Invariant {
                detail: "advisor-finalization transition requires a current Change Unit".to_owned(),
            })?;
            WorkflowActionAuthorityCoordinates::FinalizeAdvice {
                task_id,
                shaping_checkpoint_id: ShapingCheckpointId::new(
                    checkpoint.checkpoint_ref.record_id.as_str(),
                ),
                change_unit_id: ChangeUnitId::new(&change_unit.change_unit_id),
                scope_revision: task.scope_revision,
                baseline_ref,
                user_action_resolution_ids: resolution_ids_for(MethodName::FinalizeAdvice),
            }
        }
        MethodName::AdvanceTask => {
            let checkpoint = checkpoint.ok_or_else(|| CorePipelineError::Invariant {
                detail: "advance transition requires a current checkpoint".to_owned(),
            })?;
            let change_unit = current_change_unit.ok_or_else(|| CorePipelineError::Invariant {
                detail: "advance transition requires a current Change Unit".to_owned(),
            })?;
            WorkflowActionAuthorityCoordinates::AdvanceTask {
                task_id,
                shaping_checkpoint_id: ShapingCheckpointId::new(
                    checkpoint.checkpoint_ref.record_id.as_str(),
                ),
                change_unit_id: ChangeUnitId::new(&change_unit.change_unit_id),
                scope_revision: task.scope_revision,
                baseline_ref,
                user_action_resolution_ids: resolution_ids_for(MethodName::AdvanceTask),
            }
        }
        MethodName::PrepareEvidenceCapture => {
            let change_unit = current_change_unit.ok_or_else(|| CorePipelineError::Invariant {
                detail: "evidence-capture transition requires a current Change Unit".to_owned(),
            })?;
            WorkflowActionAuthorityCoordinates::PrepareEvidenceCapture {
                task_id,
                change_unit_id: ChangeUnitId::new(&change_unit.change_unit_id),
                baseline_ref: task.shaping.baseline_ref.clone().ok_or_else(|| {
                    CorePipelineError::Invariant {
                        detail: "evidence-capture transition requires a current baseline"
                            .to_owned(),
                    }
                })?,
            }
        }
        MethodName::PrepareWrite => {
            let change_unit = current_change_unit.ok_or_else(|| CorePipelineError::Invariant {
                detail: "write-preparation transition requires a current Change Unit".to_owned(),
            })?;
            WorkflowActionAuthorityCoordinates::PrepareWrite {
                task_id,
                change_unit_id: ChangeUnitId::new(&change_unit.change_unit_id),
                baseline_ref: task.shaping.baseline_ref.clone().ok_or_else(|| {
                    CorePipelineError::Invariant {
                        detail: "write-preparation transition requires a current baseline"
                            .to_owned(),
                    }
                })?,
            }
        }
        MethodName::StageArtifact => WorkflowActionAuthorityCoordinates::StageArtifact { task_id },
        MethodName::RecordRun => {
            let change_unit = current_change_unit.ok_or_else(|| CorePipelineError::Invariant {
                detail: "run-recording transition requires a current Change Unit".to_owned(),
            })?;
            let run_kind = match task.mode {
                TaskMode::Direct => RunKind::Direct,
                TaskMode::Work => RunKind::Implementation,
                TaskMode::Advisor => {
                    return Err(CorePipelineError::Invariant {
                        detail: "advisor Task cannot expose a run-recording transition".to_owned(),
                    })
                }
            };
            WorkflowActionAuthorityCoordinates::RecordRun {
                task_id,
                change_unit_id: ChangeUnitId::new(&change_unit.change_unit_id),
                baseline_ref: task.shaping.baseline_ref.clone().ok_or_else(|| {
                    CorePipelineError::Invariant {
                        detail: "run-recording transition requires a current baseline".to_owned(),
                    }
                })?,
                run_kind,
            }
        }
        MethodName::RequestUserAction => WorkflowActionAuthorityCoordinates::RequestUserAction {
            task_id,
            change_unit_id: RequiredNullable::new(
                current_change_unit
                    .map(|change_unit| ChangeUnitId::new(&change_unit.change_unit_id)),
            ),
        },
        MethodName::ResolveUserAction => {
            let user_action_request_refs = task_wide_authority.resolvable_user_action_refs();
            WorkflowActionAuthorityCoordinates::ResolveUserAction {
                task_id,
                user_action_request_refs,
            }
        }
        MethodName::ReconcileChanges => {
            WorkflowActionAuthorityCoordinates::ReconcileChanges { task_id }
        }
        MethodName::CheckClose => WorkflowActionAuthorityCoordinates::CheckClose { task_id },
        MethodName::CloseTask => WorkflowActionAuthorityCoordinates::CloseTask { task_id },
        _ => {
            return Err(CorePipelineError::Invariant {
                detail: format!("{} is not a workflow transition method", method.as_str()),
            })
        }
    };
    let actor = if method == MethodName::ResolveUserAction {
        WorkflowTransitionActor::User
    } else {
        WorkflowTransitionActor::Agent
    };
    let mut agent_input_requirements = match method {
        MethodName::RecordShapingCheckpoint => {
            vec![WorkflowAgentInputRequirement::ShapingCheckpoint]
        }
        MethodName::UpdateScope => vec![WorkflowAgentInputRequirement::ScopeAndChangeUnit],
        MethodName::FinalizeAdvice => vec![WorkflowAgentInputRequirement::AdviceResult],
        MethodName::AdvanceTask | MethodName::CheckClose | MethodName::ResolveUserAction => {
            Vec::new()
        }
        MethodName::PrepareEvidenceCapture => {
            vec![WorkflowAgentInputRequirement::EvidenceCapture]
        }
        MethodName::PrepareWrite => vec![WorkflowAgentInputRequirement::ProposedWrite],
        MethodName::StageArtifact => vec![WorkflowAgentInputRequirement::Artifact],
        MethodName::RecordRun => vec![WorkflowAgentInputRequirement::RunObservation],
        MethodName::RequestUserAction => vec![WorkflowAgentInputRequirement::UserActionDraft],
        MethodName::ReconcileChanges => {
            vec![WorkflowAgentInputRequirement::ChangeReconciliation]
        }
        MethodName::CloseTask => vec![WorkflowAgentInputRequirement::CloseIntent],
        MethodName::Intake | MethodName::Status | MethodName::GetOperationResult => Vec::new(),
    };
    agent_input_requirements.sort();
    let effect_class = match method {
        MethodName::ResolveUserAction => WorkflowTransitionEffectClass::UserChannelMutation,
        MethodName::PrepareEvidenceCapture => WorkflowTransitionEffectClass::EvidenceCapture,
        MethodName::PrepareWrite => WorkflowTransitionEffectClass::WriteAuthorization,
        MethodName::StageArtifact => WorkflowTransitionEffectClass::ArtifactStaging,
        MethodName::RecordRun => WorkflowTransitionEffectClass::ExecutionRecording,
        MethodName::CheckClose => WorkflowTransitionEffectClass::ReadOnlyAssessment,
        MethodName::CloseTask => WorkflowTransitionEffectClass::TerminalMutation,
        _ => WorkflowTransitionEffectClass::CoreStateMutation,
    };
    let expected_result_state = match method {
        MethodName::AdvanceTask => WorkflowExpectedResultState::Implementation,
        MethodName::FinalizeAdvice | MethodName::CheckClose => {
            WorkflowExpectedResultState::CloseReview
        }
        MethodName::RequestUserAction => WorkflowExpectedResultState::AwaitingUserAction,
        MethodName::CloseTask => WorkflowExpectedResultState::Terminal,
        MethodName::PrepareEvidenceCapture
        | MethodName::PrepareWrite
        | MethodName::StageArtifact
        | MethodName::RecordRun
        | MethodName::ReconcileChanges => WorkflowExpectedResultState::Implementation,
        _ => WorkflowExpectedResultState::ReevaluateCurrentAuthority,
    };
    Ok(TransitionDescriptor {
        action_key,
        actor,
        role,
        expected_state_version: state_version,
        fixed_authority_coordinates,
        agent_input_requirements,
        effect_class,
        expected_result_state,
        required_refs: required_refs.to_vec(),
    })
}

pub(crate) fn workflow_transition_catalog(
    required_key: Option<WorkflowActionKey>,
    allowed_methods: &[MethodName],
    snapshot: &WorkflowSnapshot<'_>,
) -> CoreResult<WorkflowTransitionCatalog> {
    let mut methods = allowed_methods.to_vec();
    methods.sort_by_key(|method| method.as_str());
    methods.dedup();
    let mut transitions = Vec::new();
    for method in methods {
        let semantic_variants = match method {
            MethodName::RecordShapingCheckpoint => vec![if snapshot.checkpoint_summary.is_some() {
                WorkflowActionSemanticVariant::ReplaceCurrent
            } else {
                WorkflowActionSemanticVariant::CreateInitial
            }],
            MethodName::UpdateScope => {
                if snapshot.current_change_unit.is_none() {
                    vec![WorkflowActionSemanticVariant::for_change_unit_operation(
                        ChangeUnitOperation::CreateCurrent,
                    )]
                } else {
                    let mut variants =
                        vec![WorkflowActionSemanticVariant::for_change_unit_operation(
                            ChangeUnitOperation::KeepCurrent,
                        )];
                    let replacement_invalidates_current_implementation_authority =
                        snapshot.task.work_phase == WorkPhase::Implementation
                            && !snapshot.shaping_authority_graph.applied.is_empty();
                    if !replacement_invalidates_current_implementation_authority {
                        variants.push(WorkflowActionSemanticVariant::for_change_unit_operation(
                            ChangeUnitOperation::ReplaceCurrent,
                        ));
                    }
                    variants
                }
            }
            _ => WorkflowActionSemanticVariant::for_single_variant_method(method)
                .into_iter()
                .collect(),
        };
        for semantic_variant in semantic_variants {
            let key = WorkflowActionKey::new(method, semantic_variant).map_err(|detail| {
                CorePipelineError::Invariant {
                    detail: detail.to_owned(),
                }
            })?;
            let role = if required_key == Some(key) {
                WorkflowActionRole::Required
            } else {
                WorkflowActionRole::Allowed
            };
            transitions.push(transition_descriptor(
                method,
                semantic_variant,
                role,
                snapshot,
            )?);
        }
    }
    transitions.sort_by(|left, right| {
        left.action_key
            .method
            .as_str()
            .cmp(right.action_key.method.as_str())
            .then_with(|| {
                left.action_key
                    .semantic_variant
                    .as_str()
                    .cmp(right.action_key.semantic_variant.as_str())
            })
    });
    let catalog = WorkflowTransitionCatalog::new(transitions)
        .map_err(|detail| CorePipelineError::Invariant { detail })?;
    if required_key.is_some() && catalog.required_transition().is_none() {
        return Err(CorePipelineError::Invariant {
            detail: "required workflow transition is absent from its catalog".to_owned(),
        });
    }
    Ok(catalog)
}

impl WorkflowMachine {
    /// Evaluates one already-normalized snapshot without Store or adapter access.
    pub(crate) fn evaluate(snapshot: &WorkflowSnapshot<'_>) -> CoreResult<WorkflowEvaluation> {
        snapshot.validate_separated_authority()?;
        let task = snapshot.task;
        let authority = snapshot.shaping_authority_graph;
        let required = |method: MethodName, variant: WorkflowActionSemanticVariant| {
            WorkflowActionKey::new(method, variant).map_err(|detail| CorePipelineError::Invariant {
                detail: detail.to_owned(),
            })
        };
        let record_shaping_variant = if snapshot.checkpoint.is_some() {
            WorkflowActionSemanticVariant::ReplaceCurrent
        } else {
            WorkflowActionSemanticVariant::CreateInitial
        };
        let update_scope_variant = if snapshot.current_change_unit.is_some() {
            WorkflowActionSemanticVariant::KeepCurrentChangeUnit
        } else {
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit
        };

        if matches!(
            task.lifecycle_phase,
            TaskLifecyclePhase::Completed
                | TaskLifecyclePhase::Cancelled
                | TaskLifecyclePhase::Superseded
        ) {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::Terminal,
                AuthorityNextActor::None,
                None,
                &[],
                None,
            );
        }
        if snapshot.user_action_state.recovery_required > 0 {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::DecisionRecoveryRequired,
                AuthorityNextActor::Agent,
                Some(required(
                    MethodName::RecordShapingCheckpoint,
                    record_shaping_variant,
                )?),
                &[MethodName::RecordShapingCheckpoint],
                Some(WorkflowBlockingReason::DecisionRecoveryRequired),
            );
        }
        if !authority.stale.is_empty() {
            let (method, variant) = if task.work_phase == WorkPhase::Shaping {
                (MethodName::RecordShapingCheckpoint, record_shaping_variant)
            } else {
                (
                    MethodName::CloseTask,
                    WorkflowActionSemanticVariant::CloseTask,
                )
            };
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ShapingRequired,
                AuthorityNextActor::Agent,
                Some(required(method, variant)?),
                &[method],
                Some(WorkflowBlockingReason::ApplicationAuthorityStale),
            );
        }
        if task.work_phase == WorkPhase::Implementation {
            let execution_coordinates_available =
                snapshot.current_change_unit.is_some() && snapshot.baseline.is_some();
            let mut methods = vec![MethodName::UpdateScope, MethodName::StageArtifact];
            if execution_coordinates_available {
                methods.extend([MethodName::PrepareEvidenceCapture, MethodName::PrepareWrite]);
            }
            if execution_coordinates_available && task.mode != TaskMode::Advisor {
                methods.push(MethodName::RecordRun);
            }
            methods.extend([
                MethodName::RequestUserAction,
                MethodName::ReconcileChanges,
                MethodName::CheckClose,
                MethodName::CloseTask,
            ]);
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::Implementation,
                AuthorityNextActor::Agent,
                None,
                &methods,
                None,
            );
        }
        if snapshot.user_action_state.awaiting_user > 0 {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::AwaitingUserAction,
                AuthorityNextActor::User,
                Some(required(
                    MethodName::ResolveUserAction,
                    WorkflowActionSemanticVariant::ResolveUserAction,
                )?),
                &[MethodName::ResolveUserAction],
                Some(if snapshot.user_action_state.inconsistent > 0 {
                    WorkflowBlockingReason::InconsistentAuthorityState
                } else {
                    WorkflowBlockingReason::UserActionPending
                }),
            );
        }
        let Some(checkpoint) = snapshot.checkpoint else {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ShapingRequired,
                AuthorityNextActor::Agent,
                Some(required(
                    MethodName::RecordShapingCheckpoint,
                    record_shaping_variant,
                )?),
                &[MethodName::RecordShapingCheckpoint],
                Some(WorkflowBlockingReason::NoCurrentCheckpoint),
            );
        };
        if snapshot.user_action_state.inconsistent > 0 {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ShapingRequired,
                AuthorityNextActor::Agent,
                Some(required(
                    MethodName::RecordShapingCheckpoint,
                    record_shaping_variant,
                )?),
                &[MethodName::RecordShapingCheckpoint],
                Some(WorkflowBlockingReason::InconsistentAuthorityState),
            );
        }
        let has_scope_decisions_to_apply = checkpoint.gaps.iter().any(|gap| {
            gap.status == ShapingGapStatus::Accepted
                && gap
                    .gap_kind
                    .decision_policy_for_mode(task.mode)
                    .is_some_and(|policy| {
                        policy.application_owner == ShapingDecisionApplicationOwner::UpdateScope
                    })
        });
        if has_scope_decisions_to_apply {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ReadyToApplyDecisions,
                AuthorityNextActor::Agent,
                Some(required(MethodName::UpdateScope, update_scope_variant)?),
                &[MethodName::UpdateScope],
                Some(WorkflowBlockingReason::AcceptedDecisionsNotApplied),
            );
        }
        if checkpoint
            .gaps
            .iter()
            .any(|gap| gap.status == ShapingGapStatus::Current)
        {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ShapingRequired,
                AuthorityNextActor::Agent,
                Some(required(
                    MethodName::RecordShapingCheckpoint,
                    record_shaping_variant,
                )?),
                &[MethodName::RecordShapingCheckpoint],
                Some(WorkflowBlockingReason::ShapingGapsCurrent),
            );
        }
        if task.mode == TaskMode::Advisor
            && checkpoint.readiness == ShapingCheckpointReadiness::Ready
        {
            let current_basis = snapshot.close_basis.is_some_and(|basis| {
                basis.scope_revision == task.scope_revision
                    && basis.close_basis_revision == task.close_basis_revision
                    && basis.baseline_ref.as_ref() == snapshot.baseline
                    && basis
                        .shaping_checkpoint_ref
                        .as_ref()
                        .is_some_and(|reference| {
                            reference.record_kind == StateRecordKind::ShapingCheckpoint
                                && reference.record_id.as_str() == checkpoint.shaping_checkpoint_id
                        })
                    && snapshot.current_change_unit.is_some_and(|change_unit| {
                        change_unit.change_unit_id == basis.change_unit_id.as_str()
                    })
            });
            if current_basis {
                return Self::evaluation(
                    snapshot,
                    WorkflowStateKind::CloseReview,
                    AuthorityNextActor::Agent,
                    Some(required(
                        MethodName::CheckClose,
                        WorkflowActionSemanticVariant::CheckClose,
                    )?),
                    &[
                        MethodName::CheckClose,
                        MethodName::CloseTask,
                        MethodName::FinalizeAdvice,
                    ],
                    None,
                );
            }
            if snapshot.current_change_unit.is_none() {
                return Self::evaluation(
                    snapshot,
                    WorkflowStateKind::ReadyForChangeUnit,
                    AuthorityNextActor::Agent,
                    Some(required(MethodName::UpdateScope, update_scope_variant)?),
                    &[MethodName::UpdateScope],
                    Some(WorkflowBlockingReason::ChangeUnitRequired),
                );
            }
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ReadyToFinalizeAdvice,
                AuthorityNextActor::Agent,
                Some(required(
                    MethodName::FinalizeAdvice,
                    WorkflowActionSemanticVariant::FinalizeAdvice,
                )?),
                &[MethodName::FinalizeAdvice],
                Some(WorkflowBlockingReason::AdvisorFinalizationRequired),
            );
        }
        if snapshot.current_change_unit.is_none() {
            return Self::evaluation(
                snapshot,
                WorkflowStateKind::ReadyForChangeUnit,
                AuthorityNextActor::Agent,
                Some(required(MethodName::UpdateScope, update_scope_variant)?),
                &[MethodName::UpdateScope],
                Some(WorkflowBlockingReason::ChangeUnitRequired),
            );
        }
        Self::evaluation(
            snapshot,
            WorkflowStateKind::ReadyForImplementation,
            AuthorityNextActor::Agent,
            Some(required(
                MethodName::AdvanceTask,
                WorkflowActionSemanticVariant::AdvanceTask,
            )?),
            &[MethodName::AdvanceTask],
            Some(WorkflowBlockingReason::ExplicitAdvanceRequired),
        )
    }

    fn evaluation(
        snapshot: &WorkflowSnapshot<'_>,
        workflow_kind: WorkflowStateKind,
        next_actor: AuthorityNextActor,
        required_key: Option<WorkflowActionKey>,
        allowed_methods: &[MethodName],
        typed_blocking_reason: Option<WorkflowBlockingReason>,
    ) -> CoreResult<WorkflowEvaluation> {
        let transition_catalog =
            workflow_transition_catalog(required_key, allowed_methods, snapshot)?;
        let required_transition = transition_catalog.required_transition();
        if required_key.is_some() != required_transition.is_some() {
            return Err(CorePipelineError::Invariant {
                detail: "workflow required-transition membership is inconsistent".to_owned(),
            });
        }
        if let Some(required_transition) = required_transition {
            let actor_matches = matches!(
                (next_actor, required_transition.actor),
                (AuthorityNextActor::Agent, WorkflowTransitionActor::Agent)
                    | (AuthorityNextActor::User, WorkflowTransitionActor::User)
            );
            if !actor_matches {
                return Err(CorePipelineError::Invariant {
                    detail: "workflow next actor contradicts the required transition".to_owned(),
                });
            }
        }
        let terminal = workflow_kind == WorkflowStateKind::Terminal;
        if terminal && !transition_catalog.transitions.is_empty() {
            return Err(CorePipelineError::Invariant {
                detail: "terminal workflow exposes a transition".to_owned(),
            });
        }
        if !terminal
            && workflow_kind != WorkflowStateKind::NoActiveTask
            && transition_catalog.transitions.is_empty()
        {
            return Err(CorePipelineError::Invariant {
                detail: "nonterminal workflow has no executable transition".to_owned(),
            });
        }
        if next_actor == AuthorityNextActor::User
            && required_transition
                .is_none_or(|transition| transition.actor != WorkflowTransitionActor::User)
        {
            return Err(CorePipelineError::Invariant {
                detail: "User-owned workflow does not identify its exact required transition"
                    .to_owned(),
            });
        }
        Ok(WorkflowEvaluation {
            workflow_kind,
            next_actor,
            transition_catalog,
            typed_blocking_reason,
            required_refs: snapshot.required_refs.clone(),
            close_readiness: WorkflowCloseReadiness {
                assessment_required: workflow_kind == WorkflowStateKind::CloseReview,
                current_close_basis_present: snapshot.close_basis.is_some(),
            },
            checkpoint: snapshot.checkpoint_summary.clone(),
        })
    }
}

impl WorkflowEvaluation {
    fn into_projection(self, expected_state_version: u64) -> WorkflowProjection {
        let Self {
            workflow_kind,
            next_actor,
            transition_catalog,
            typed_blocking_reason,
            required_refs,
            close_readiness,
            checkpoint,
        } = self;
        macro_rules! projection {
            ($variant:ident) => {
                WorkflowProjection::$variant {
                    next_actor,
                    required_refs,
                    expected_state_version,
                    blocking_reason: RequiredNullable::new(typed_blocking_reason),
                    checkpoint: RequiredNullable::new(checkpoint),
                    transition_catalog,
                    close_readiness,
                }
            };
        }
        match workflow_kind {
            WorkflowStateKind::NoActiveTask => projection!(NoActiveTask),
            WorkflowStateKind::ShapingRequired => projection!(ShapingRequired),
            WorkflowStateKind::AwaitingUserAction => projection!(AwaitingUserAction),
            WorkflowStateKind::DecisionRecoveryRequired => projection!(DecisionRecoveryRequired),
            WorkflowStateKind::ReadyToApplyDecisions => projection!(ReadyToApplyDecisions),
            WorkflowStateKind::ReadyForChangeUnit => projection!(ReadyForChangeUnit),
            WorkflowStateKind::ReadyToFinalizeAdvice => projection!(ReadyToFinalizeAdvice),
            WorkflowStateKind::ReadyForImplementation => projection!(ReadyForImplementation),
            WorkflowStateKind::Implementation => projection!(Implementation),
            WorkflowStateKind::CloseReview => projection!(CloseReview),
            WorkflowStateKind::Terminal => projection!(Terminal),
        }
    }
}

/// Builds the normalized snapshot and projects the pure machine evaluation.
pub(crate) fn workflow_projection(
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    checkpoint: Option<&ShapingCheckpointRecord>,
    task_wide_authority: &TaskWideShapingAuthority,
) -> CoreResult<WorkflowProjection> {
    let task_id = TaskId::new(task.task_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task.task_id,
        project_id,
        Some(&task_id),
        Some(state_version),
    );
    let checkpoint_summary = checkpoint.map(|value| {
        checkpoint_summary(
            project_id,
            &task_id,
            state_version,
            value,
            task.mode,
            task_wide_authority,
        )
    });
    let mut required_refs = vec![task_ref];
    if let Some(summary) = checkpoint_summary.as_ref() {
        required_refs.push(summary.checkpoint_ref.clone());
        for reference in summary
            .current_application_refs
            .iter()
            .chain(summary.pending_decision_refs.iter())
            .chain(
                summary
                    .gaps
                    .iter()
                    .filter_map(|gap| gap.user_action_resolution_ref.as_ref()),
            )
        {
            if !required_refs.contains(reference) {
                required_refs.push(reference.clone());
            }
        }
    }
    if let Some(change_unit) = current_change_unit {
        required_refs.push(state_ref(
            StateRecordKind::ChangeUnit,
            &change_unit.change_unit_id,
            project_id,
            Some(&task_id),
            Some(change_unit.basis_state_version),
        ));
    }
    for request_ref in task_wide_authority.blocking_request_refs() {
        if !required_refs.contains(&request_ref) {
            required_refs.push(request_ref);
        }
    }
    let snapshot = WorkflowSnapshot::new(
        project_id,
        state_version,
        task,
        current_change_unit,
        checkpoint,
        task_wide_authority,
        checkpoint_summary,
        required_refs,
    )?;
    WorkflowMachine::evaluate(&snapshot).map(|evaluation| evaluation.into_projection(state_version))
}
