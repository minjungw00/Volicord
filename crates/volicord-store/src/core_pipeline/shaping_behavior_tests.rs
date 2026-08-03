use std::error::Error;

use volicord_types::ids::{
    AgentConnectionId, BaselineRef, IdempotencyKey, ProjectId, RequestHash, ShapingCheckpointId,
    ShapingGapId, TaskId, UserActionOptionId,
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
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::Record(ShapingCheckpointInsert {
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
        })),
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::ResolveLinkedGap {
            user_action_request_id: product_request_id.to_owned(),
            user_action_resolution_id: product_resolution_id.to_owned(),
        }),
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::ResolveLinkedGap {
            user_action_request_id: scope_request_id.to_owned(),
            user_action_resolution_id: scope_resolution_id.to_owned(),
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
                    applications: vec![ShapingGapApplication {
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
        ShapingGapStatus::Resolved
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
                applications: vec![ShapingGapApplication {
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
        ShapingGapStatus::Resolved
    );

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
                            shaping_gap_id: product_gap_id.to_owned(),
                            user_action_resolution_id: product_resolution_id.to_owned(),
                        },
                        ShapingGapApplication {
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
        ShapingGapStatus::Resolved
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
                options: vec![UserActionOption {
                    option_id: UserActionOptionId::new("accept"),
                    label: "Accept".to_owned(),
                    description: "Accept this exact decision.".to_owned(),
                    consequence: "The semantic owner may apply it.".to_owned(),
                    machine_action: UserActionOptionAction::Accept,
                    resolution_outcome: JudgmentResolutionOutcome::Accepted,
                    is_default: true,
                }],
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
    UserActionResolutionInsert {
        user_action_resolution_id: resolution_id.to_owned(),
        user_action_request_id: request_id.to_owned(),
        action_kind,
        channel_kind: UserActionChannelKind::Cli,
        channel_submission_id: format!("submission_{resolution_id}"),
        resolution: UserActionResolutionBody::Choice {
            selected_option_id: UserActionOptionId::new("accept"),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
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
fn policy_owner_names_remain_exact() {
    assert_eq!(
        ShapingGapKind::UserScopeDecisionRequired
            .decision_policy()
            .expect("scope policy")
            .application_owner,
        ShapingDecisionApplicationOwner::UpdateScope
    );
}
