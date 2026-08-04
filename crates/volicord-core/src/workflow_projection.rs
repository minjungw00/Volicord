use std::collections::{BTreeMap, BTreeSet};

use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ShapingCheckpointGapRecord, ShapingCheckpointRecord,
    ShapingDecisionApplicationRecord, ShapingGapApplication, StoredUserActionRecordSet, TaskRecord,
};
use volicord_types::ids::{
    shaping_decision_application_id, BaselineRef, ChangeUnitId, ProjectId, TaskId,
    UserActionResolutionId,
};
use volicord_types::schema::{
    PersistedUserActionRequestMetadata, RequiredNullable, ShapingCheckpointGap,
    ShapingCheckpointSummary, ShapingDecisionRecoveryRequirement, StateRecordRef,
    UserActionResolutionBody, WorkflowProjection, WorkflowRejectionUserAction,
};
use volicord_types::values::{
    evaluate_shaping_decision_authority, ActorSource, AuthorityNextActor, MethodName,
    ShapingCheckpointReadiness, ShapingDecisionApplicationAuthorityStatus,
    ShapingDecisionApplicationOwner, ShapingDecisionAuthorityFacts, ShapingDecisionAuthorityState,
    ShapingGapStatus, StateRecordKind, TaskLifecyclePhase, TaskMode, UserActionRequiredFor,
    UserActionStatus, UtcTimestamp, WorkPhase, WorkflowBlockingReason,
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
        self.blocking_facts()
            .filter(|fact| seen.insert(fact.request_ref.record_id.as_str().to_owned()))
            .map(|fact| fact.request_ref.clone())
            .collect()
    }

    pub(crate) fn has_blockers(&self) -> bool {
        !self.awaiting_user.is_empty()
            || !self.accepted_unapplied.is_empty()
            || !self.recovery_required.is_empty()
            || !self.stale.is_empty()
            || !self.inconsistent.is_empty()
    }

    pub(crate) fn blocks_advance_application(&self) -> bool {
        !self.awaiting_user.is_empty()
            || !self.recovery_required.is_empty()
            || !self.stale.is_empty()
            || !self.inconsistent.is_empty()
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
    let graph = store
        .current_shaping_authority_graph(&task_id, now)
        .map_err(CorePipelineError::from)?;
    let projecting_next_state = state_version
        > store
            .project_state()
            .map_err(CorePipelineError::from)?
            .state_version;
    let projected_checkpoint_replaces_stored = checkpoint.is_some_and(|checkpoint| {
        graph
            .current_checkpoint
            .as_ref()
            .is_none_or(|stored| stored.shaping_checkpoint_id != checkpoint.shaping_checkpoint_id)
    });
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
    let mut assessment = TaskWideShapingAuthority::default();
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
    let summary = checkpoint.map(|value| {
        checkpoint_summary(
            project_id,
            &task_id,
            state_version,
            value,
            task.mode,
            task_wide_authority,
        )
    });
    let mut refs = vec![task_ref];
    if let Some(summary) = summary.as_ref() {
        refs.push(summary.checkpoint_ref.clone());
        for application_ref in &summary.current_application_refs {
            if !refs.contains(application_ref) {
                refs.push(application_ref.clone());
            }
        }
        for request_ref in &summary.pending_decision_refs {
            if !refs.contains(request_ref) {
                refs.push(request_ref.clone());
            }
        }
        for gap in &summary.gaps {
            if let Some(resolution_ref) = gap.user_action_resolution_ref.as_ref() {
                if !refs.contains(resolution_ref) {
                    refs.push(resolution_ref.clone());
                }
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
    if !task_wide_authority.recovery_required.is_empty() {
        return WorkflowProjection::DecisionRecoveryRequired {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::RecordShapingCheckpoint),
            allowed_actions: vec![MethodName::RecordShapingCheckpoint, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(
                WorkflowBlockingReason::DecisionRecoveryRequired,
            ),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if !task_wide_authority.stale.is_empty() {
        return WorkflowProjection::ShapingRequired {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(if task.work_phase == WorkPhase::Shaping {
                MethodName::RecordShapingCheckpoint
            } else {
                MethodName::CloseTask
            }),
            allowed_actions: if task.work_phase == WorkPhase::Shaping {
                vec![MethodName::RecordShapingCheckpoint, MethodName::Status]
            } else {
                vec![MethodName::CloseTask, MethodName::Status]
            },
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(
                WorkflowBlockingReason::ApplicationAuthorityStale,
            ),
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
        if !task_wide_authority.awaiting_user.is_empty() {
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
            required_action: RequiredNullable::some(MethodName::RecordShapingCheckpoint),
            allowed_actions: vec![MethodName::RecordShapingCheckpoint, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(WorkflowBlockingReason::NoCurrentCheckpoint),
            checkpoint: RequiredNullable::null(),
        };
    };
    let has_pending_user = !task_wide_authority.awaiting_user.is_empty()
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
        gap.status == ShapingGapStatus::Accepted
            && gap
                .gap_kind
                .decision_policy_for_mode(task.mode)
                .is_some_and(|policy| {
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
                WorkflowBlockingReason::AcceptedDecisionsNotApplied,
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
            required_action: RequiredNullable::some(MethodName::RecordShapingCheckpoint),
            allowed_actions: vec![MethodName::RecordShapingCheckpoint, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(WorkflowBlockingReason::ShapingGapsCurrent),
            checkpoint: RequiredNullable::new(summary),
        };
    }
    if task.mode == TaskMode::Advisor && checkpoint.readiness == ShapingCheckpointReadiness::Ready {
        let current_basis = task.close_basis.as_ref().is_some_and(|basis| {
            basis.task_id.as_str() == task.task_id
                && basis.scope_revision == task.scope_revision
                && basis.close_basis_revision == task.close_basis_revision
                && basis.baseline_ref.as_ref() == task.shaping.baseline_ref.as_ref()
                && basis
                    .shaping_checkpoint_ref
                    .as_ref()
                    .is_some_and(|reference| {
                        reference.record_kind == StateRecordKind::ShapingCheckpoint
                            && reference.record_id.as_str() == checkpoint.shaping_checkpoint_id
                    })
                && current_change_unit.is_some_and(|change_unit| {
                    change_unit.change_unit_id == basis.change_unit_id.as_str()
                })
        });
        if current_basis {
            return WorkflowProjection::CloseReview {
                next_actor: AuthorityNextActor::Agent,
                required_action: RequiredNullable::some(MethodName::CheckClose),
                allowed_actions: vec![
                    MethodName::CheckClose,
                    MethodName::CloseTask,
                    MethodName::FinalizeAdvice,
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
        return WorkflowProjection::ReadyToFinalizeAdvice {
            next_actor: AuthorityNextActor::Agent,
            required_action: RequiredNullable::some(MethodName::FinalizeAdvice),
            allowed_actions: vec![MethodName::FinalizeAdvice, MethodName::Status],
            required_refs: refs,
            expected_state_version: state_version,
            blocking_reason: RequiredNullable::some(
                WorkflowBlockingReason::AdvisorFinalizationRequired,
            ),
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
