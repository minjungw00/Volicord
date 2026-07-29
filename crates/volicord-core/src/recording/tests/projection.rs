use super::*;
use crate::pipeline::VerifiedInvocationContext;

fn recording_verified_invocation() -> VerifiedInvocationContext {
    VerifiedInvocationContext {
        project_id: ProjectId::new(PROJECT_ID),
        actor_source: ActorSource::agent_connection(CONNECTION_ID),
        operation_category: OperationCategory::AgentWorkflow,
        verification_basis: VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned(),
        assurance_level: "test_fixture".to_owned(),
        session_id: None,
        git_workspace_context: None,
    }
}

fn recording_input(
    task_id: &str,
    change_unit_id: &str,
    dry_run: volicord_types::schema::DryRunIntent,
    summary: &str,
) -> crate::recording::RecordRunInput {
    crate::recording::RecordRunInput::new(
        ProjectId::new(PROJECT_ID),
        dry_run,
        TaskId::new(task_id),
        ChangeUnitId::new(change_unit_id),
        RunKind::Implementation,
        None,
        BaselineRef::new("baseline_test"),
        None,
        Some("local_sensitive_step".to_owned()),
        summary.to_owned(),
        ObservedChanges {
            changed_paths: Vec::new(),
            product_file_write_observed: false,
            sensitive_categories: Vec::new(),
            baseline_ref: Some(BaselineRef::new("baseline_test")).into(),
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
    )
}

#[test]
fn recording_plan_returns_semantic_operation_and_result_facts() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "semantic_recording_plan")?;
    let input = recording_input(
        &task_id,
        &change_unit_id,
        volicord_types::schema::DryRunIntent::NotRequested,
        "Recorded implementation run.",
    );
    let store = harness.store()?;
    let project_state = store.project_state()?;
    let operation_now = UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?;

    let plan = crate::recording::plan_record_run(
        &harness.service.inner,
        &store,
        &project_state,
        input,
        &recording_verified_invocation(),
        &operation_now,
    )
    .unwrap_or_else(|error| panic!("semantic Recording plan failed: {error:?}"));
    let (effect, result_facts) = plan.into_parts();

    assert_eq!(effect.task_id().as_str(), task_id);
    assert_eq!(effect.change_unit_id().as_str(), change_unit_id);
    assert_eq!(effect.event_payload()["task_id"], task_id);
    assert!(!effect.into_storage_mutations().is_empty());
    assert_eq!(result_facts.kind(), RunKind::Implementation);
    assert_eq!(result_facts.summary(), "Recorded implementation run.");
    assert_eq!(
        result_facts.run_ref().task_id.as_ref(),
        Some(&TaskId::new(task_id))
    );
    assert!(result_facts.registered_artifacts().is_empty());
    assert!(result_facts.evidence_observations().is_empty());
    assert!(result_facts.evidence_producers().is_empty());
    assert_eq!(result_facts.state().state_version, 3);
    Ok(())
}

#[test]
fn recording_plan_returns_typed_semantic_validation_error() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "semantic_recording_error")?;
    let input = recording_input(
        &task_id,
        &change_unit_id,
        volicord_types::schema::DryRunIntent::Requested,
        " ",
    );
    let store = harness.store()?;
    let project_state = store.project_state()?;
    let operation_now = UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?;

    let error = match crate::recording::plan_record_run(
        &harness.service.inner,
        &store,
        &project_state,
        input,
        &recording_verified_invocation(),
        &operation_now,
    ) {
        Ok(_) => panic!("blank summary must fail semantic validation"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        crate::recording::RecordingError::Rejected(
            crate::recording::RecordingRejection::Validation {
                field: "summary",
                message: "summary must not be empty"
            }
        )
    ));
    Ok(())
}

#[test]
fn record_run_post_commit_close_projection_matches_immediate_status() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_state_projection")?;
    let mut request = record_run_request(
        "req_run_state_projection",
        "idem_run_state_projection",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Close claim supported.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_run_state_projection_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["state"]["evidence_summary"],
        response.response_value["evidence_summary"]
    );
    assert_eq!(response.response_value["state"]["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value["state"],
        "missing_final_acceptance",
    );
    assert!(response.response_value["state"]["close_blockers"]
        .as_array()
        .is_some_and(|blockers| !blockers.is_empty()));
    assert_record_run_close_projection_matches_status(
        &response.response_value,
        &status.response_value,
    );
    Ok(())
}

#[test]
fn record_run_without_evidence_updates_separates_result_state_and_close_evidence(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_no_update_evidence_views")?;
    let (after_criteria, criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        2,
        "run_no_update_evidence_views",
        &[("Required retained evidence.", EvidenceRequirement::Required)],
    )?;
    let criterion_id = criteria[0].acceptance_criterion_id.clone();

    let mut evidence_run = record_run_request(
        "req_run_prior_evidence",
        "idem_run_prior_evidence",
        false,
        Some(after_criteria),
        &task_id,
        &change_unit_id,
    );
    evidence_run.evidence_updates = vec![evidence_update_for_acceptance_criterion(
        supported_evidence_update("Required retained evidence."),
        &criterion_id,
    )];
    evidence_run.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "First Run records current required evidence.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let first = harness
        .service
        .record_run(evidence_run, invocation(OperationCategory::AgentWorkflow))?;
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("first Run should commit");
    assert_eq!(
        first.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "supported"
    );

    let mut no_update_run = record_run_request(
        "req_run_without_evidence_update",
        "idem_run_without_evidence_update",
        false,
        Some(after_first),
        &task_id,
        &change_unit_id,
    );
    no_update_run.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Second Run records a basis without an evidence update.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let second = harness
        .service
        .record_run(no_update_run, invocation(OperationCategory::AgentWorkflow))?;
    assert!(second.response_value["evidence_summary"].is_null());
    assert!(second.response_value["current_close_basis"]["evidence_summary_ref"].is_null());

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_run_without_evidence_update_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_run_without_evidence_update_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        second.response_value["state"]["evidence_summary"],
        status.response_value["evidence_summary"]
    );
    assert_eq!(
        second.response_value["state"]["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "supported"
    );
    assert_eq!(
        second.response_value["state"]["close_state"],
        status.response_value["close_state"]
    );
    assert_eq!(
        second.response_value["state"]["close_blockers"],
        status.response_value["close_blockers"]
    );
    assert_eq!(
        status.response_value["close_blockers"],
        check.response_value["blockers"]
    );
    assert_eq!(
        second.response_value["state"]["evidence_gate"],
        status.response_value["evidence_gate"]
    );
    assert_eq!(
        status.response_value["evidence_gate"],
        check.response_value["evidence_gate"]
    );
    assert_close_blocker(&second.response_value["state"], "evidence_claim_missing");
    Ok(())
}

#[test]
fn record_run_promoted_artifact_close_projection_matches_immediate_status(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_artifact_projection")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_artifact_projection", 2)?;
    let claim = "The staged validation report supports close.";
    let mut request = record_run_request(
        "req_run_artifact_projection",
        "idem_run_artifact_projection",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_close_projection",
        handle,
        Some("validation_report"),
        Some(claim),
    )];
    request.evidence_updates = vec![supported_evidence_update(claim)];
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: claim.to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_run_artifact_projection_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        response.response_value["registered_artifacts"][0],
        response.response_value["evidence_summary"]["coverage_items"][0]
            ["supporting_artifact_refs"][0]
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["availability"],
        "available"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_no_close_blocker(&response.response_value["state"], "artifact_unavailable");
    assert_record_run_close_projection_matches_status(
        &response.response_value,
        &status.response_value,
    );
    Ok(())
}
