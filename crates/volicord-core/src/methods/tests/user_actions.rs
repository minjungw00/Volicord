//! UserAction service construction and materialization coverage.

use super::super::user_actions::{
    construct_user_action, materialize_user_action_request, UserActionConstructionInput,
    UserActionIntent, UserActionMaterializationInput, UserActionOrigin,
};
use super::*;
use volicord_store::core_pipeline::UserActionMutation;

fn construct_from_request(
    harness: &MethodHarness,
    request: &RequestUserActionRequest,
) -> Result<super::super::user_actions::ConstructedUserAction, PlanError> {
    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))
            .expect("service test Store should open");
    let project_state = store
        .project_state()
        .expect("service test project state should load");
    let task = store
        .task_record(&request.task_id)
        .expect("service test Task lookup should succeed")
        .expect("service test Task should exist");
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .expect("service test Change Unit lookup should succeed");
    construct_user_action(UserActionConstructionInput {
        store: &store,
        project_state: &project_state,
        envelope: &request.envelope,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        operation_now: &UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)
            .expect("service test timestamp should parse"),
        intent: UserActionIntent {
            task_id: request.task_id.clone(),
            change_unit_id: request.change_unit_id.as_ref().cloned(),
            action: request.action.clone(),
            required_for: request.required_for.clone(),
            expires_at: request.expires_at.clone(),
        },
    })
}

fn assert_validation_field(error: PlanError, expected_field: &str) {
    let PlanError::Response(response) = error else {
        panic!("semantic validation should return a typed rejection")
    };
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        expected_field
    );
}

#[test]
fn service_constructs_canonical_typed_choice_body() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_action_service_body")?;
    let mut request = user_action_request(
        "req_user_action_service_body",
        "idem_user_action_service_body",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    let UserActionDraft::Choice(choice) = &mut request.action else {
        panic!("fixture should create a choice intent")
    };
    choice.question = "  Choose the current product direction.  ".to_owned();
    choice.context.summary = "  A current user-owned decision is required.  ".to_owned();

    let constructed = construct_from_request(&harness, &request)
        .unwrap_or_else(|_| panic!("valid semantic intent should construct"));

    assert_eq!(
        constructed
            .coordinate_change_unit_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(change_unit_id.as_str())
    );
    assert_eq!(
        constructed.basis.coordinates().task_id.as_str(),
        task_id.as_str()
    );
    assert_eq!(constructed.basis.coordinates().created_at_state_version, 2);
    let UserActionRequestBody::Choice(body) = &constructed.body else {
        panic!("choice intent should construct a typed choice body")
    };
    assert_eq!(body.question, "Choose the current product direction.");
    assert_eq!(
        body.context.summary,
        "  A current user-owned decision is required.  "
    );
    assert_eq!(body.options.len(), 2);
    assert!(body.options.iter().all(|option| {
        option.machine_action == UserActionOptionAction::Accept
            && option.resolution_outcome == JudgmentResolutionOutcome::Accepted
    }));
    assert_eq!(
        constructed.required_for,
        vec![UserActionRequiredFor::CloseComplete]
    );
    assert!(constructed.expires_at.is_none());
    Ok(())
}

#[test]
fn service_rejects_invalid_semantic_combinations() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_action_service_invalid")?;
    let base = user_action_request(
        "req_user_action_service_invalid",
        "idem_user_action_service_invalid",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );

    let mut empty_required_for = base.clone();
    empty_required_for.required_for.clear();
    assert_validation_field(
        construct_from_request(&harness, &empty_required_for)
            .expect_err("empty required_for should reject"),
        "required_for",
    );

    let mut duplicate_required_for = base.clone();
    duplicate_required_for.required_for = vec![
        UserActionRequiredFor::RecordRun,
        UserActionRequiredFor::RecordRun,
    ];
    assert_validation_field(
        construct_from_request(&harness, &duplicate_required_for)
            .expect_err("duplicate required_for should reject"),
        "required_for",
    );

    let mut incompatible_required_for = user_action_request(
        "req_user_action_service_incompatible",
        "idem_user_action_service_incompatible",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::Cancellation,
    );
    incompatible_required_for.required_for = vec![UserActionRequiredFor::PrepareWrite];
    assert_validation_field(
        construct_from_request(&harness, &incompatible_required_for)
            .expect_err("incompatible required_for should reject"),
        "required_for",
    );

    let mut invalid_sensitive_scope = base;
    let UserActionDraft::Choice(choice) = &mut invalid_sensitive_scope.action else {
        panic!("fixture should create a choice intent")
    };
    choice.sensitive_action_scope =
        sensitive_action_scope_for_kind(JudgmentKind::SensitiveApproval).into();
    assert_validation_field(
        construct_from_request(&harness, &invalid_sensitive_scope)
            .expect_err("non-sensitive action with sensitive scope should reject"),
        "action.sensitive_action_scope",
    );
    Ok(())
}

#[test]
fn service_materializes_canonical_store_mutation_and_identity() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_action_service_materialization")?;
    let generator = CountingDurableIdGenerator::new(["service_materialization"]);
    harness.use_generator_and_clock(
        generator.clone(),
        ManualClock::at(DEFAULT_METHOD_TEST_CLOCK),
    );
    let request = user_action_request(
        "req_user_action_service_materialization",
        "idem_user_action_service_materialization",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    let constructed = construct_from_request(&harness, &request)
        .unwrap_or_else(|_| panic!("valid semantic intent should construct"));
    let expected_body = constructed.body.clone();
    let expected_basis = constructed.basis.clone();
    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let project_state = store.project_state()?;
    let materialized = materialize_user_action_request(UserActionMaterializationInput {
        service: &harness.service.inner,
        store: &store,
        project_state: &project_state,
        verified_invocation: &VerifiedInvocationContext {
            project_id: ProjectId::new(PROJECT_ID),
            actor_source: ActorSource::agent_connection(CONNECTION_ID),
            operation_category: OperationCategory::AgentWorkflow,
            verification_basis: VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned(),
            assurance_level: "test_fixture".to_owned(),
            session_id: None,
            git_workspace_context: None,
        },
        envelope: &request.envelope,
        origin: UserActionOrigin::DirectRequest,
        constructed,
    })
    .unwrap_or_else(|_| panic!("valid construction should materialize"));

    let expected_request_id =
        prefixed_durable_id(DurableIdKind::UserActionRequest, "service_materialization");
    assert_eq!(
        materialized.public_request.user_action_request_id.as_str(),
        expected_request_id
    );
    assert_eq!(materialized.public_request.body, expected_body);
    assert_eq!(materialized.public_request.basis, expected_basis);
    assert_eq!(
        materialized.request_ref.record_id.as_str(),
        expected_request_id
    );
    assert_eq!(generator.count(DurableIdKind::UserActionRequest), 1);

    let CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(insert)) =
        &materialized.mutation
    else {
        panic!("materialization should return one typed UserAction insert")
    };
    assert_eq!(insert.user_action_request_id, expected_request_id);
    assert_eq!(insert.source_method, MethodName::RequestUserAction.as_str());
    assert_eq!(
        insert.source_idempotency_key,
        "idem_user_action_service_materialization"
    );
    assert_eq!(insert.metadata_json, "{}");
    let persisted: PersistedUserActionRequest = serde_json::from_str(&insert.request_json)?;
    let persisted_basis: UserActionBasis = serde_json::from_str(&insert.basis_json)?;
    assert_eq!(persisted.body, materialized.public_request.body);
    assert_eq!(
        persisted.required_for,
        materialized.public_request.required_for
    );
    assert_eq!(persisted_basis, materialized.public_request.basis);
    assert_eq!(
        materialized.effective.request.request_json,
        insert.request_json
    );
    Ok(())
}
