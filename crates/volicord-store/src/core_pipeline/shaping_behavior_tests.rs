use std::error::Error;

use volicord_types::ids::{
    shaping_decision_application_id, AgentConnectionId, BaselineRef, IdempotencyKey, ProjectId,
    RequestHash, ShapingCheckpointId, ShapingGapId, TaskId, UserActionOptionId,
    UserActionResolutionId,
};
use volicord_types::schema::{
    PersistedUserActionRequest, PersistedUserActionRequestMetadata,
    PersistedUserActionShapingMetadata, RequiredNullable, ShapingCheckpointOperation,
    UserActionBasis, UserActionBasisCoordinates, UserActionChoiceBasis,
    UserActionChoiceRequestBody, UserActionContext, UserActionOption, UserActionRequestBody,
    UserActionResolutionBody,
};
use volicord_types::values::{
    ActorSource, JudgmentKind, JudgmentPresentation, JudgmentResolutionOutcome, MethodName,
    ShapingCheckpointReadiness, ShapingDecisionApplicationOwner, ShapingGapKind, ShapingGapStatus,
    UserActionBasisStatus, UserActionChannelKind, UserActionKind, UserActionOptionAction,
    UserActionVerificationBasis, UtcTimestamp, WorkPhase,
};

use super::{
    ShapingAdvanceApplication, ShapingCheckpointGapInsert, ShapingCheckpointInsert,
    ShapingCheckpointMutation, ShapingCheckpointUserActionInsert, ShapingGapApplication,
};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, ChangeUnitInsert, ChangeUnitMutation, CoreStorageMutation,
    StoredChangeUnitLifecycle, StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis,
    TaskMutation, UserActionMutation, UserActionRequestInsert, UserActionResolutionInsert,
};
use crate::StoreError;

fn application_id(resolution_id: &str, owner: ShapingDecisionApplicationOwner) -> String {
    shaping_decision_application_id(&UserActionResolutionId::new(resolution_id), owner)
        .expect("application identity")
        .into_inner()
}

#[test]
fn selected_owner_updates_are_exact_and_failed_advance_rolls_back() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_shaping_exact_application";
    let checkpoint_id = "checkpoint_shaping_exact_application";
    let change_unit_id = "cu_shaping_exact_application";
    let product_gap_id = "gap_product_exact_application";
    let scope_gap_id = "gap_scope_exact_application";
    let product_request_id = "request_product_exact_application";
    let scope_request_id = "request_scope_exact_application";
    let product_resolution_id = "resolution_product_exact_application";
    let scope_resolution_id = "resolution_scope_exact_application";
    let baseline = BaselineRef::new("baseline_shaping_exact_application");

    let mut task = task_insert(task_id);
    task.shaping.baseline_ref = Some(baseline.clone());
    let initial = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordShaping,
        Some(&IdempotencyKey::new("idem_shaping_exact_initial")),
        &RequestHash::new("sha256:shaping-exact-initial"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("shaping_exact_initial", task_id)],
    );
    let initial_mutations = vec![
        CoreStorageMutation::Task(TaskMutation::insert(task)),
        CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(ChangeUnitInsert {
            change_unit_id: change_unit_id.to_owned(),
            task_id: task_id.to_owned(),
            scope_summary: StoredChangeUnitScopeSummary {
                scope_summary: Some("Exact shaping application scope.".to_owned()),
                affected_areas: Vec::new(),
                constraints: Vec::new(),
            },
            bounded_paths: vec!["src/lib.rs".to_owned()],
            write_basis: StoredChangeUnitWriteBasis {
                baseline_ref: Some(baseline.clone()),
                git_workspace_context: None,
            },
            effect_contract: None,
            lifecycle: StoredChangeUnitLifecycle {
                recovery_required: false,
            },
        })),
        CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(shaping_request(
            product_request_id,
            task_id,
            change_unit_id,
            checkpoint_id,
            product_gap_id,
            baseline.clone(),
            ShapingGapKind::UserProductDecisionRequired,
        ))),
        CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(shaping_request(
            scope_request_id,
            task_id,
            change_unit_id,
            checkpoint_id,
            scope_gap_id,
            baseline.clone(),
            ShapingGapKind::UserScopeDecisionRequired,
        ))),
        CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(shaping_resolution(
            product_resolution_id,
            product_request_id,
            UserActionKind::ProductDecision,
        ))),
        CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(shaping_resolution(
            scope_resolution_id,
            scope_request_id,
            UserActionKind::ScopeDecision,
        ))),
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::Record(Box::new(
            ShapingCheckpointInsert {
                shaping_checkpoint_id: checkpoint_id.to_owned(),
                checkpoint_operation: ShapingCheckpointOperation::CreateInitial,
                task_id: task_id.to_owned(),
                scope_revision: 0,
                baseline_ref: Some(baseline.clone()),
                summary: "Two independently owned decisions are resolved.".to_owned(),
                implementation_boundary: Some("Apply each decision by its owner.".to_owned()),
                readiness: ShapingCheckpointReadiness::Blocked,
                source_refs: Vec::new(),
                evidence_refs: Vec::new(),
                created_at: UtcTimestamp::parse("2026-01-01T00:00:01Z")?,
                retired_user_action_request_ids: Vec::new(),
                carry_forward_application_ids: Vec::new(),
                gaps: vec![
                    shaping_gap(
                        product_gap_id,
                        product_request_id,
                        ShapingGapKind::UserProductDecisionRequired,
                    ),
                    shaping_gap(
                        scope_gap_id,
                        scope_request_id,
                        ShapingGapKind::UserScopeDecisionRequired,
                    ),
                ],
            },
        ))),
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::ResolveLinkedGap {
            user_action_request_id: product_request_id.to_owned(),
            user_action_resolution_id: product_resolution_id.to_owned(),
            disposition: ShapingGapStatus::Accepted,
        }),
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::ResolveLinkedGap {
            user_action_request_id: scope_request_id.to_owned(),
            user_action_resolution_id: scope_resolution_id.to_owned(),
            disposition: ShapingGapStatus::Accepted,
        }),
    ];
    store.commit_mutation(initial, &initial_mutations, response_json)?;

    let wrong_scope = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_shaping_wrong_scope")),
        &RequestHash::new("sha256:shaping-wrong-scope"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(1),
        vec![pending_event_for_task("shaping_wrong_scope", task_id)],
    );
    let error = store
        .commit_mutation(
            wrong_scope,
            &[CoreStorageMutation::Shaping(
                ShapingCheckpointMutation::ApplyScopeAndRebaseCurrent {
                    task_id: task_id.to_owned(),
                    shaping_checkpoint_id: checkpoint_id.to_owned(),
                    scope_revision: 1,
                    baseline_ref: Some(baseline.clone()),
                    change_unit_id: Some(change_unit_id.to_owned()),
                    applications: vec![ShapingGapApplication {
                        shaping_decision_application_id: application_id(
                            scope_resolution_id,
                            ShapingDecisionApplicationOwner::UpdateScope,
                        ),
                        shaping_gap_id: scope_gap_id.to_owned(),
                        user_action_resolution_id: scope_resolution_id.to_owned(),
                    }],
                },
            )],
            response_json,
        )
        .expect_err("scope application must match the current Task revision");
    assert!(matches!(error, StoreError::InvalidInput { .. }));
    assert_eq!(store.project_state()?.state_version, 1);
    assert_eq!(
        store
            .current_shaping_checkpoint(&TaskId::new(task_id))?
            .expect("checkpoint survives rollback")
            .gaps
            .iter()
            .find(|gap| gap.shaping_gap_id == scope_gap_id)
            .expect("scope gap")
            .status,
        ShapingGapStatus::Accepted
    );

    let apply_scope = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_shaping_exact_scope")),
        &RequestHash::new("sha256:shaping-exact-scope"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(1),
        vec![pending_event_for_task("shaping_exact_scope", task_id)],
    );
    store.commit_mutation(
        apply_scope,
        &[CoreStorageMutation::Shaping(
            ShapingCheckpointMutation::ApplyScopeAndRebaseCurrent {
                task_id: task_id.to_owned(),
                shaping_checkpoint_id: checkpoint_id.to_owned(),
                scope_revision: 0,
                baseline_ref: Some(baseline.clone()),
                change_unit_id: Some(change_unit_id.to_owned()),
                applications: vec![ShapingGapApplication {
                    shaping_decision_application_id: application_id(
                        scope_resolution_id,
                        ShapingDecisionApplicationOwner::UpdateScope,
                    ),
                    shaping_gap_id: scope_gap_id.to_owned(),
                    user_action_resolution_id: scope_resolution_id.to_owned(),
                }],
            },
        )],
        response_json,
    )?;
    let checkpoint = store
        .current_shaping_checkpoint(&TaskId::new(task_id))?
        .expect("current checkpoint");
    assert_eq!(
        checkpoint
            .gaps
            .iter()
            .find(|gap| gap.shaping_gap_id == scope_gap_id)
            .expect("scope gap")
            .status,
        ShapingGapStatus::Applied
    );
    assert_eq!(
        checkpoint
            .gaps
            .iter()
            .find(|gap| gap.shaping_gap_id == product_gap_id)
            .expect("product gap")
            .status,
        ShapingGapStatus::Accepted
    );

    let duplicate_application = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_shaping_duplicate_application")),
        &RequestHash::new("sha256:shaping-duplicate-application"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(2),
        vec![pending_event_for_task(
            "shaping_duplicate_application",
            task_id,
        )],
    );
    let error = store
        .commit_mutation(
            duplicate_application,
            &[CoreStorageMutation::Shaping(
                ShapingCheckpointMutation::ApplyScopeAndRebaseCurrent {
                    task_id: task_id.to_owned(),
                    shaping_checkpoint_id: checkpoint_id.to_owned(),
                    scope_revision: 0,
                    baseline_ref: Some(baseline.clone()),
                    change_unit_id: Some(change_unit_id.to_owned()),
                    applications: vec![ShapingGapApplication {
                        shaping_decision_application_id: application_id(
                            scope_resolution_id,
                            ShapingDecisionApplicationOwner::UpdateScope,
                        ),
                        shaping_gap_id: scope_gap_id.to_owned(),
                        user_action_resolution_id: scope_resolution_id.to_owned(),
                    }],
                },
            )],
            response_json,
        )
        .expect_err("one accepted resolution cannot create a duplicate application");
    assert!(matches!(error, StoreError::InvalidInput { .. }));
    assert_eq!(store.project_state()?.state_version, 2);

    let failed_advance = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::AdvanceTask,
        Some(&IdempotencyKey::new("idem_shaping_exact_failed_advance")),
        &RequestHash::new("sha256:shaping-exact-failed-advance"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(2),
        vec![pending_event_for_task(
            "shaping_exact_failed_advance",
            task_id,
        )],
    );
    let error = store
        .commit_mutation(
            failed_advance,
            &[CoreStorageMutation::Shaping(
                ShapingCheckpointMutation::ApplyAdvanceAndTransition(ShapingAdvanceApplication {
                    task_id: task_id.to_owned(),
                    shaping_checkpoint_id: checkpoint_id.to_owned(),
                    change_unit_id: change_unit_id.to_owned(),
                    scope_revision: 0,
                    baseline_ref: baseline.clone(),
                    applications: vec![
                        ShapingGapApplication {
                            shaping_decision_application_id: application_id(
                                product_resolution_id,
                                ShapingDecisionApplicationOwner::AdvanceTask,
                            ),
                            shaping_gap_id: product_gap_id.to_owned(),
                            user_action_resolution_id: product_resolution_id.to_owned(),
                        },
                        ShapingGapApplication {
                            shaping_decision_application_id: application_id(
                                scope_resolution_id,
                                ShapingDecisionApplicationOwner::AdvanceTask,
                            ),
                            shaping_gap_id: scope_gap_id.to_owned(),
                            user_action_resolution_id: scope_resolution_id.to_owned(),
                        },
                    ],
                }),
            )],
            response_json,
        )
        .expect_err("advance must reject a gap owned by update_scope");
    assert!(matches!(error, StoreError::InvalidInput { .. }));
    assert_eq!(store.project_state()?.state_version, 2);
    assert_eq!(
        store
            .current_shaping_checkpoint(&TaskId::new(task_id))?
            .expect("checkpoint survives rollback")
            .gaps
            .iter()
            .find(|gap| gap.shaping_gap_id == product_gap_id)
            .expect("product gap")
            .status,
        ShapingGapStatus::Accepted
    );
    assert_eq!(
        store
            .task_record(&TaskId::new(task_id))?
            .expect("Task survives rollback")
            .work_phase,
        WorkPhase::Shaping
    );

    let successful_advance = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::AdvanceTask,
        Some(&IdempotencyKey::new("idem_shaping_exact_advance")),
        &RequestHash::new("sha256:shaping-exact-advance"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(2),
        vec![pending_event_for_task("shaping_exact_advance", task_id)],
    );
    store.commit_mutation(
        successful_advance,
        &[CoreStorageMutation::Shaping(
            ShapingCheckpointMutation::ApplyAdvanceAndTransition(ShapingAdvanceApplication {
                task_id: task_id.to_owned(),
                shaping_checkpoint_id: checkpoint_id.to_owned(),
                change_unit_id: change_unit_id.to_owned(),
                scope_revision: 0,
                baseline_ref: baseline,
                applications: vec![ShapingGapApplication {
                    shaping_decision_application_id: application_id(
                        product_resolution_id,
                        ShapingDecisionApplicationOwner::AdvanceTask,
                    ),
                    shaping_gap_id: product_gap_id.to_owned(),
                    user_action_resolution_id: product_resolution_id.to_owned(),
                }],
            }),
        )],
        response_json,
    )?;
    assert_eq!(
        store
            .task_record(&TaskId::new(task_id))?
            .expect("Task advanced")
            .work_phase,
        WorkPhase::Implementation
    );
    assert!(store
        .current_shaping_checkpoint(&TaskId::new(task_id))?
        .expect("checkpoint remains current")
        .gaps
        .iter()
        .all(|gap| gap.status == ShapingGapStatus::Applied));
    let product_application_id = application_id(
        product_resolution_id,
        ShapingDecisionApplicationOwner::AdvanceTask,
    );
    assert_eq!(
        store
            .shaping_decision_applications_for_task(&TaskId::new(task_id))?
            .len(),
        2
    );
    drop(store);

    let corruption = rusqlite::Connection::open(harness.state_database_path())?;
    let trigger_sql: String = corruption.query_row(
        "SELECT sql FROM sqlite_master
          WHERE type = 'trigger'
            AND name = 'trg_shaping_checkpoint_application_delete_forbidden'",
        [],
        |row| row.get(0),
    )?;
    corruption.execute_batch("DROP TRIGGER trg_shaping_checkpoint_application_delete_forbidden")?;
    let detached = corruption.execute(
        "DELETE FROM shaping_checkpoint_applications
          WHERE project_id = ?1
            AND task_id = ?2
            AND shaping_checkpoint_id = ?3
            AND shaping_decision_application_id = ?4",
        rusqlite::params![PROJECT_ID, task_id, checkpoint_id, product_application_id],
    )?;
    assert_eq!(detached, 1, "fixture must detach one current application");
    corruption.execute_batch(&trigger_sql)?;
    drop(corruption);

    let store = harness.store()?;
    let error = store
        .shaping_decision_applications_for_task(&TaskId::new(task_id))
        .expect_err("a current application without current checkpoint lineage is corrupt");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table,
            logical_column,
            ..
        } if table == "shaping_decision_applications" && logical_column == "authority_status"
    ));
    Ok(())
}

fn shaping_gap(
    gap_id: &str,
    request_id: &str,
    gap_kind: ShapingGapKind,
) -> ShapingCheckpointGapInsert {
    let policy = gap_kind.decision_policy().expect("test gap is user-owned");
    ShapingCheckpointGapInsert {
        shaping_gap_id: gap_id.to_owned(),
        gap_kind,
        summary: "Exact decision application test gap.".to_owned(),
        affected_refs: Vec::new(),
        user_action: Some(ShapingCheckpointUserActionInsert {
            user_action_request_id: request_id.to_owned(),
            action_kind: policy.user_action_kind,
        }),
    }
}

fn shaping_request(
    request_id: &str,
    task_id: &str,
    change_unit_id: &str,
    checkpoint_id: &str,
    gap_id: &str,
    baseline: BaselineRef,
    gap_kind: ShapingGapKind,
) -> UserActionRequestInsert {
    let policy = gap_kind.decision_policy().expect("test gap is user-owned");
    let judgment_kind = match policy.user_action_kind {
        UserActionKind::ProductDecision => JudgmentKind::ProductDecision,
        UserActionKind::ScopeDecision => JudgmentKind::ScopeDecision,
        _ => unreachable!("focused Store test uses product and scope decisions"),
    };
    let required_for = policy.required_for.to_vec();
    UserActionRequestInsert {
        user_action_request_id: request_id.to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: Some(change_unit_id.to_owned()),
        action_kind: policy.user_action_kind,
        request: PersistedUserActionRequest {
            body: UserActionRequestBody::Choice(Box::new(UserActionChoiceRequestBody {
                judgment_kind,
                presentation: JudgmentPresentation::Short,
                question: "Apply this exact shaping decision?".to_owned(),
                options: vec![
                    UserActionOption {
                        option_id: UserActionOptionId::new("accept"),
                        label: "Accept".to_owned(),
                        description: "Accept this exact decision.".to_owned(),
                        consequence: "The semantic owner may apply it.".to_owned(),
                        machine_action: UserActionOptionAction::Accept,
                        resolution_outcome: JudgmentResolutionOutcome::Accepted,
                        is_default: true,
                    },
                    UserActionOption {
                        option_id: UserActionOptionId::new("reject"),
                        label: "Reject".to_owned(),
                        description: "Reject this exact decision.".to_owned(),
                        consequence: "The decision grants no authority.".to_owned(),
                        machine_action: UserActionOptionAction::Reject,
                        resolution_outcome: JudgmentResolutionOutcome::Rejected,
                        is_default: false,
                    },
                    UserActionOption {
                        option_id: UserActionOptionId::new("defer"),
                        label: "Defer".to_owned(),
                        description: "Defer this exact decision.".to_owned(),
                        consequence: "The decision grants no authority.".to_owned(),
                        machine_action: UserActionOptionAction::Defer,
                        resolution_outcome: JudgmentResolutionOutcome::Deferred,
                        is_default: false,
                    },
                ],
                context: UserActionContext {
                    summary: "Exact shaping application Store test.".to_owned(),
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                    visible_risks: Vec::new(),
                    constraints: Vec::new(),
                },
                affected_refs: Vec::new(),
                sensitive_action_scope: RequiredNullable::null(),
            })),
            required_for: required_for.clone(),
            expires_at: RequiredNullable::null(),
        },
        basis: UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
            coordinates: UserActionBasisCoordinates {
                task_id: TaskId::new(task_id),
                change_unit_id: RequiredNullable::some(change_unit_id.into()),
                scope_revision: 0,
                baseline_ref: RequiredNullable::some(baseline),
                created_at_state_version: 0,
                compatibility_status: UserActionBasisStatus::Current,
            },
            close_basis_revision: RequiredNullable::null(),
            result_refs: Vec::new(),
            residual_risk_ids: Vec::new(),
            sensitive_action_scope: RequiredNullable::null(),
        })),
        basis_status: UserActionBasisStatus::Current,
        required_for,
        requested_by_actor_source: ActorSource::AgentConnection(AgentConnectionId::new(
            CONNECTION_ID,
        )),
        source_method: MethodName::RecordShaping,
        source_idempotency_key: format!("idem_{request_id}"),
        requested_at: UtcTimestamp::parse("2026-01-01T00:00:00Z").expect("test timestamp"),
        expires_at: None,
        metadata: PersistedUserActionRequestMetadata::Shaping(PersistedUserActionShapingMetadata {
            created_by: MethodName::RecordShaping,
            shaping_checkpoint_id: ShapingCheckpointId::new(checkpoint_id),
            shaping_gap_id: ShapingGapId::new(gap_id),
        }),
    }
}

fn shaping_resolution(
    resolution_id: &str,
    request_id: &str,
    action_kind: UserActionKind,
) -> UserActionResolutionInsert {
    shaping_resolution_with_outcome(
        resolution_id,
        request_id,
        action_kind,
        UserActionOptionAction::Accept,
        JudgmentResolutionOutcome::Accepted,
    )
}

fn shaping_resolution_with_outcome(
    resolution_id: &str,
    request_id: &str,
    action_kind: UserActionKind,
    machine_action: UserActionOptionAction,
    resolution_outcome: JudgmentResolutionOutcome,
) -> UserActionResolutionInsert {
    UserActionResolutionInsert {
        user_action_resolution_id: resolution_id.to_owned(),
        user_action_request_id: request_id.to_owned(),
        action_kind,
        channel_kind: UserActionChannelKind::Cli,
        channel_submission_id: format!("submission_{resolution_id}"),
        resolution: UserActionResolutionBody::Choice {
            selected_option_id: UserActionOptionId::new(match machine_action {
                UserActionOptionAction::Accept => "accept",
                UserActionOptionAction::Reject => "reject",
                UserActionOptionAction::Defer => "defer",
            }),
            machine_action,
            resolution_outcome,
            note: RequiredNullable::null(),
            accepted_risk_ids: Vec::new(),
        },
        resolved_by_actor_source: ActorSource::LocalUser,
        resolved_verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
        resolved_assurance_level: "local_user_channel".to_owned(),
        resolved_at: UtcTimestamp::parse("2026-01-01T00:00:02Z").expect("test timestamp"),
    }
}

#[test]
fn outcome_specific_gap_resolution_is_atomic_and_only_accepted_can_apply(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_shaping_rejected_atomic";
    let checkpoint_id = "checkpoint_shaping_rejected_atomic";
    let change_unit_id = "cu_shaping_rejected_atomic";
    let gap_id = "gap_shaping_rejected_atomic";
    let request_id = "request_shaping_rejected_atomic";
    let resolution_id = "resolution_shaping_rejected_atomic";
    let baseline = BaselineRef::new("baseline_shaping_rejected_atomic");
    let mut task = task_insert(task_id);
    task.shaping.baseline_ref = Some(baseline.clone());

    store.commit_mutation(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordShaping,
            Some(&IdempotencyKey::new("idem_shaping_rejected_atomic_initial")),
            &RequestHash::new("sha256:shaping-rejected-atomic-initial"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "shaping_rejected_atomic_initial",
                task_id,
            )],
        ),
        &[
            CoreStorageMutation::Task(TaskMutation::insert(task)),
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(ChangeUnitInsert {
                change_unit_id: change_unit_id.to_owned(),
                task_id: task_id.to_owned(),
                scope_summary: StoredChangeUnitScopeSummary {
                    scope_summary: Some("Outcome-specific shaping test.".to_owned()),
                    affected_areas: Vec::new(),
                    constraints: Vec::new(),
                },
                bounded_paths: vec!["src/lib.rs".to_owned()],
                write_basis: StoredChangeUnitWriteBasis {
                    baseline_ref: Some(baseline.clone()),
                    git_workspace_context: None,
                },
                effect_contract: None,
                lifecycle: StoredChangeUnitLifecycle {
                    recovery_required: false,
                },
            })),
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(shaping_request(
                request_id,
                task_id,
                change_unit_id,
                checkpoint_id,
                gap_id,
                baseline.clone(),
                ShapingGapKind::UserScopeDecisionRequired,
            ))),
            CoreStorageMutation::Shaping(ShapingCheckpointMutation::Record(Box::new(
                ShapingCheckpointInsert {
                    shaping_checkpoint_id: checkpoint_id.to_owned(),
                    checkpoint_operation: ShapingCheckpointOperation::CreateInitial,
                    task_id: task_id.to_owned(),
                    scope_revision: 0,
                    baseline_ref: Some(baseline.clone()),
                    summary: "One exact scope decision is pending.".to_owned(),
                    implementation_boundary: Some(
                        "Only accepted authority may be applied.".to_owned(),
                    ),
                    readiness: ShapingCheckpointReadiness::Blocked,
                    source_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                    created_at: UtcTimestamp::parse("2026-01-01T00:00:01Z")?,
                    retired_user_action_request_ids: Vec::new(),
                    carry_forward_application_ids: Vec::new(),
                    gaps: vec![shaping_gap(
                        gap_id,
                        request_id,
                        ShapingGapKind::UserScopeDecisionRequired,
                    )],
                },
            ))),
        ],
        response_json,
    )?;

    let rejected_resolution = shaping_resolution_with_outcome(
        resolution_id,
        request_id,
        UserActionKind::ScopeDecision,
        UserActionOptionAction::Reject,
        JudgmentResolutionOutcome::Rejected,
    );
    let inconsistent = store
        .commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new(
                    "idem_shaping_rejected_atomic_inconsistent",
                )),
                &RequestHash::new("sha256:shaping-rejected-atomic-inconsistent"),
                Some(replay_context(CONNECTION_ID, "user_only")),
                Some(1),
                vec![pending_event_for_task(
                    "shaping_rejected_atomic_inconsistent",
                    task_id,
                )],
            ),
            &[
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    rejected_resolution.clone(),
                )),
                CoreStorageMutation::Shaping(ShapingCheckpointMutation::ResolveLinkedGap {
                    user_action_request_id: request_id.to_owned(),
                    user_action_resolution_id: resolution_id.to_owned(),
                    disposition: ShapingGapStatus::Accepted,
                }),
            ],
            response_json,
        )
        .expect_err("rejected resolution cannot back an accepted gap");
    assert!(matches!(inconsistent, StoreError::InvalidInput { .. }));
    assert_eq!(store.project_state()?.state_version, 1);
    assert_eq!(
        store
            .current_shaping_checkpoint(&TaskId::new(task_id))?
            .expect("checkpoint remains current")
            .gaps[0]
            .status,
        ShapingGapStatus::Current
    );
    let conn = rusqlite::Connection::open(harness.state_database_path())?;
    let resolution_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM user_action_resolutions WHERE user_action_resolution_id = ?1",
        [resolution_id],
        |row| row.get(0),
    )?;
    assert_eq!(resolution_count, 0);

    store.commit_mutation(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_shaping_rejected_atomic_exact")),
            &RequestHash::new("sha256:shaping-rejected-atomic-exact"),
            Some(replay_context(CONNECTION_ID, "user_only")),
            Some(1),
            vec![pending_event_for_task(
                "shaping_rejected_atomic_exact",
                task_id,
            )],
        ),
        &[
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                rejected_resolution,
            )),
            CoreStorageMutation::Shaping(ShapingCheckpointMutation::ResolveLinkedGap {
                user_action_request_id: request_id.to_owned(),
                user_action_resolution_id: resolution_id.to_owned(),
                disposition: ShapingGapStatus::Rejected,
            }),
        ],
        response_json,
    )?;
    assert_eq!(
        store
            .current_shaping_checkpoint(&TaskId::new(task_id))?
            .expect("checkpoint remains current")
            .gaps[0]
            .status,
        ShapingGapStatus::Rejected
    );

    let application = store
        .commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::UpdateScope,
                Some(&IdempotencyKey::new("idem_shaping_rejected_atomic_apply")),
                &RequestHash::new("sha256:shaping-rejected-atomic-apply"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(2),
                vec![pending_event_for_task(
                    "shaping_rejected_atomic_apply",
                    task_id,
                )],
            ),
            &[CoreStorageMutation::Shaping(
                ShapingCheckpointMutation::ApplyScopeAndRebaseCurrent {
                    task_id: task_id.to_owned(),
                    shaping_checkpoint_id: checkpoint_id.to_owned(),
                    scope_revision: 0,
                    baseline_ref: Some(baseline),
                    change_unit_id: Some(change_unit_id.to_owned()),
                    applications: vec![ShapingGapApplication {
                        shaping_decision_application_id: application_id(
                            resolution_id,
                            ShapingDecisionApplicationOwner::UpdateScope,
                        ),
                        shaping_gap_id: gap_id.to_owned(),
                        user_action_resolution_id: resolution_id.to_owned(),
                    }],
                },
            )],
            response_json,
        )
        .expect_err("rejected authority cannot be applied");
    assert!(matches!(application, StoreError::InvalidInput { .. }));
    assert_eq!(store.project_state()?.state_version, 2);
    assert_eq!(
        store
            .current_shaping_checkpoint(&TaskId::new(task_id))?
            .expect("checkpoint remains current")
            .gaps[0]
            .status,
        ShapingGapStatus::Rejected
    );

    conn.execute_batch(
        "DROP TRIGGER trg_shaping_gap_disposition_transition;
         DROP TRIGGER trg_shaping_gap_disposition_requires_matching_user_resolution;",
    )?;
    let corrupted = conn.execute(
        "UPDATE shaping_checkpoint_gaps
            SET status = 'accepted'
          WHERE project_id = ?1 AND shaping_gap_id = ?2",
        rusqlite::params![PROJECT_ID, gap_id],
    )?;
    assert_eq!(corrupted, 1);
    let error = store
        .current_shaping_checkpoint(&TaskId::new(task_id))
        .expect_err("accepted disposition backed by rejection is corrupt current data");
    assert!(matches!(error, StoreError::CorruptOwnerStateValue { .. }));
    Ok(())
}

#[test]
fn policy_owner_names_remain_exact() {
    assert_eq!(
        ShapingGapKind::UserScopeDecisionRequired
            .decision_policy()
            .expect("scope policy")
            .application_owner,
        ShapingDecisionApplicationOwner::UpdateScope
    );
}

#[test]
fn current_checkpoint_read_rejects_persisted_detached_user_action_authority(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_shaping_detached_authority";
    let checkpoint_id = "checkpoint_shaping_detached_authority";
    let change_unit_id = "cu_shaping_detached_authority";
    let gap_id = "gap_shaping_detached_authority";
    let request_id = "request_shaping_detached_authority";
    let baseline = BaselineRef::new("baseline_shaping_detached_authority");
    let mut task = task_insert(task_id);
    task.shaping.baseline_ref = Some(baseline.clone());

    store.commit_mutation(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordShaping,
            Some(&IdempotencyKey::new("idem_shaping_detached_authority")),
            &RequestHash::new("sha256:shaping-detached-authority"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "shaping_detached_authority",
                task_id,
            )],
        ),
        &[
            CoreStorageMutation::Task(TaskMutation::insert(task)),
            CoreStorageMutation::ChangeUnit(ChangeUnitMutation::InsertCurrent(ChangeUnitInsert {
                change_unit_id: change_unit_id.to_owned(),
                task_id: task_id.to_owned(),
                scope_summary: StoredChangeUnitScopeSummary {
                    scope_summary: Some("Detached authority corruption fixture.".to_owned()),
                    affected_areas: Vec::new(),
                    constraints: Vec::new(),
                },
                bounded_paths: vec!["src/lib.rs".to_owned()],
                write_basis: StoredChangeUnitWriteBasis {
                    baseline_ref: Some(baseline.clone()),
                    git_workspace_context: None,
                },
                effect_contract: None,
                lifecycle: StoredChangeUnitLifecycle {
                    recovery_required: false,
                },
            })),
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(shaping_request(
                request_id,
                task_id,
                change_unit_id,
                checkpoint_id,
                gap_id,
                baseline.clone(),
                ShapingGapKind::UserProductDecisionRequired,
            ))),
            CoreStorageMutation::Shaping(ShapingCheckpointMutation::Record(Box::new(
                ShapingCheckpointInsert {
                    shaping_checkpoint_id: checkpoint_id.to_owned(),
                    checkpoint_operation: ShapingCheckpointOperation::CreateInitial,
                    task_id: task_id.to_owned(),
                    scope_revision: 0,
                    baseline_ref: Some(baseline),
                    summary: "Current user authority is durably linked.".to_owned(),
                    implementation_boundary: Some("The link cannot disappear silently.".to_owned()),
                    readiness: ShapingCheckpointReadiness::Blocked,
                    source_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                    created_at: UtcTimestamp::parse("2026-01-01T00:00:01Z")?,
                    retired_user_action_request_ids: Vec::new(),
                    carry_forward_application_ids: Vec::new(),
                    gaps: vec![shaping_gap(
                        gap_id,
                        request_id,
                        ShapingGapKind::UserProductDecisionRequired,
                    )],
                },
            ))),
        ],
        response_json,
    )?;
    drop(store);

    let corruption = rusqlite::Connection::open(harness.state_database_path())?;
    let trigger_sql: String = corruption.query_row(
        "SELECT sql FROM sqlite_master
          WHERE type = 'trigger' AND name = 'trg_shaping_checkpoint_live_user_action_not_detached'",
        [],
        |row| row.get(0),
    )?;
    corruption
        .execute_batch("DROP TRIGGER trg_shaping_checkpoint_live_user_action_not_detached")?;
    let detached = corruption.execute(
        "UPDATE shaping_checkpoints
            SET readiness = 'superseded', superseded_at = '2026-01-01T00:00:02Z'
          WHERE project_id = ?1 AND shaping_checkpoint_id = ?2",
        rusqlite::params![PROJECT_ID, checkpoint_id],
    )?;
    assert_eq!(detached, 1, "fixture must detach one current authority");
    corruption.execute_batch(&trigger_sql)?;
    drop(corruption);

    let store = harness.store()?;
    let error = store
        .current_shaping_checkpoint(&TaskId::new(task_id))
        .expect_err("a detached current UserAction must be rejected as corrupt owner state");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue { table, logical_column, .. }
            if table == "user_action_requests" && logical_column == "metadata_json"
    ));
    Ok(())
}
