use super::*;

#[test]
fn every_workflow_rejection_code_preserves_authority_without_an_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_workflow_rejection_task",
            "idem_workflow_rejection_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = TaskId::new(response_record_id(&intake.response_value, "task_ref"));
    let store = harness.store()?;
    let project_state = store.project_state()?;
    let before = harness.counts()?;

    for (index, code) in [
        ErrorCode::RunKindIncompatible,
        ErrorCode::TaskPhaseTransitionRequired,
        ErrorCode::ShapingCheckpointRequired,
        ErrorCode::ShapingCheckpointStale,
        ErrorCode::UserDecisionUnresolved,
        ErrorCode::ChangeUnitRequired,
        ErrorCode::ChangeUnitStale,
        ErrorCode::WorkspaceBasisStale,
        ErrorCode::WorkflowActionNotAllowed,
    ]
    .into_iter()
    .enumerate()
    {
        let request_id = format!("req_workflow_rejection_{index}");
        let response = crate::method_rejection::workflow_rejected_response(
            &store,
            &project_state,
            &envelope(
                &request_id,
                Some(&format!("idem_workflow_rejection_{index}")),
                false,
                Some(project_state.state_version),
                Some(task_id.as_str()),
            ),
            &task_id,
            code,
            "current workflow rejected the semantic request",
            MethodName::RecordRun,
            Some(RunKind::Direct),
            vec![RunKind::Implementation],
            code != ErrorCode::WorkflowActionNotAllowed,
            MethodName::Status,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(
            response.response_value["base"]["state_version"],
            project_state.state_version
        );
        assert_eq!(response.response_value["base"]["events"], json!([]));
        assert_eq!(response.response_value["errors"][0]["code"], code.as_str());
        assert_eq!(
            response.response_value["errors"][0]["retryable"],
            code != ErrorCode::WorkflowActionNotAllowed
        );
        let details: WorkflowRejectionDetails =
            serde_json::from_value(response.response_value["errors"][0]["details"].clone())?;
        assert_eq!(details.current_task_mode, TaskMode::Work);
        assert_eq!(details.current_work_phase, WorkPhase::Shaping);
        assert_eq!(details.received_action, MethodName::RecordRun);
        assert_eq!(details.received_run_kind.as_ref(), Some(&RunKind::Direct));
        assert_eq!(details.allowed_run_kinds, vec![RunKind::Implementation]);
        assert!(!details.allowed_actions.is_empty());
        assert_eq!(
            details.workflow,
            serde_json::from_value::<WorkflowProjection>(
                response.response_value["errors"][0]["details"]["workflow"].clone()
            )?
        );
        assert_eq!(
            details.corrected_retry_allowed,
            code != ErrorCode::WorkflowActionNotAllowed
        );
        assert_eq!(details.blockers.len(), 1);
        assert_eq!(details.blockers[0].code, code);
        assert_eq!(
            details.blockers[0].owner_method,
            details.recovery.owner_method
        );
    }

    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn work_requires_ready_checkpoint_and_explicit_advance_before_write() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_shaping_task",
            "idem_shaping_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    assert_eq!(intake.response_value["state"]["work_phase"], "shaping");
    assert_eq!(
        intake.response_value["state"]["workflow"]["kind"],
        "shaping_required"
    );

    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_shaping_scope",
            "idem_shaping_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Current implementation boundary.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    assert_eq!(scoped.response_value["state"]["work_phase"], "shaping");

    let denied = harness.service.prepare_write(
        prepare_write_request(
            "req_shaping_write_denied",
            "idem_shaping_write_denied",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(denied.response_value["base"]["response_kind"], "rejected");
    assert_eq!(denied.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(denied.response_value["base"]["state_version"], 2);
    assert_eq!(
        denied.response_value["errors"][0]["code"],
        "TASK_PHASE_TRANSITION_REQUIRED"
    );
    assert_eq!(
        denied.response_value["errors"][0]["details"]["current_task_mode"],
        "work"
    );
    assert_eq!(
        denied.response_value["errors"][0]["details"]["current_work_phase"],
        "shaping"
    );
    assert_eq!(
        denied.response_value["errors"][0]["details"]["workflow"]["kind"],
        "shaping_required"
    );
    assert_eq!(
        denied.response_value["errors"][0]["details"]["recovery"]["owner_method"],
        "volicord.record_shaping"
    );

    let shaped = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_shaping_record",
                Some("idem_shaping_record"),
                false,
                Some(2),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            scope_revision: 1,
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
            summary: "The implementation boundary is ready.".to_owned(),
            implementation_boundary: RequiredNullable::some(
                "Implement only the current export boundary.".to_owned(),
            ),
            gaps: Vec::new(),
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            close_assessment: RequiredNullable::null(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_typed_result_contract::<RecordShapingResult>(&shaped);
    assert_eq!(
        shaped.response_value["shaping_checkpoint"]["readiness"],
        "ready"
    );
    assert_eq!(
        shaped.response_value["workflow"]["kind"],
        "ready_for_implementation"
    );
    let checkpoint_id = shaped.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .expect("checkpoint id")
        .to_owned();

    let advanced = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_shaping_advance",
                Some("idem_shaping_advance"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(checkpoint_id),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_typed_result_contract::<AdvanceTaskResult>(&advanced);
    assert_eq!(
        advanced.response_value["state"]["work_phase"],
        "implementation"
    );
    assert_eq!(
        advanced.response_value["workflow"]["kind"],
        "implementation"
    );

    let replay = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_shaping_advance",
                Some("idem_shaping_advance"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(
                shaped.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
                    .as_str()
                    .expect("checkpoint id"),
            ),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(replay.response_value, advanced.response_value);
    Ok(())
}

#[test]
fn user_owned_shaping_gap_is_atomic_and_requires_an_exact_request() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_shaping_gap_task",
            "idem_shaping_gap_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_shaping_gap_scope",
            "idem_shaping_gap_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Current shaping decision boundary.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let before = harness.counts()?;
    let invalid = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_shaping_gap_invalid",
                Some("idem_shaping_gap_invalid"),
                false,
                Some(2),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            scope_revision: 1,
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
            summary: "A technical decision is required.".to_owned(),
            implementation_boundary: RequiredNullable::some("Current boundary.".to_owned()),
            gaps: vec![ShapingGapInput {
                gap_kind: ShapingGapKind::UserTechnicalDecisionRequired,
                summary: "Choose the current technical direction.".to_owned(),
                affected_refs: Vec::new(),
                user_action: RequiredNullable::null(),
            }],
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            close_assessment: RequiredNullable::null(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(invalid.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);

    let action = user_action_request(
        "unused",
        "unused",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    )
    .action;
    let response = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_shaping_gap",
                Some("idem_shaping_gap"),
                false,
                Some(2),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            scope_revision: 1,
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
            summary: "A technical decision is required.".to_owned(),
            implementation_boundary: RequiredNullable::some("Current boundary.".to_owned()),
            gaps: vec![ShapingGapInput {
                gap_kind: ShapingGapKind::UserTechnicalDecisionRequired,
                summary: "Choose the current technical direction.".to_owned(),
                affected_refs: Vec::new(),
                user_action: RequiredNullable::some(ShapingUserActionDraft {
                    action,
                    expires_at: RequiredNullable::null(),
                }),
            }],
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            close_assessment: RequiredNullable::null(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        response.response_value["workflow"]["kind"],
        "awaiting_user_action"
    );
    assert_eq!(
        response.response_value["created_user_action_request_refs"]
            .as_array()
            .expect("created refs")
            .len(),
        1
    );
    let after = harness.counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_action_requests, before.user_action_requests + 1);
    Ok(())
}

#[test]
fn ready_advisor_checkpoint_establishes_advice_close_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_advisor_shaping_task",
            "idem_advisor_shaping_task",
            false,
            Some(0),
            RequestedMode::Advisor,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_advisor_shaping_scope",
            "idem_advisor_shaping_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Read-only advice boundary.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(scoped.response_value["state"]["work_phase"], "shaping");
    let shaped = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_advisor_shaping_record",
                Some("idem_advisor_shaping_record"),
                false,
                Some(2),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            scope_revision: 1,
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
            summary: "The advice is complete.".to_owned(),
            implementation_boundary: RequiredNullable::some(
                "Advice is limited to the current export boundary.".to_owned(),
            ),
            gaps: Vec::new(),
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            close_assessment: RequiredNullable::some(CloseAssessmentInput {
                result_summary: "The requested advice is complete.".to_owned(),
                result_refs: Vec::new(),
                residual_risks: Vec::new(),
                sensitive_categories: Vec::new(),
                recovery_constraints: Vec::new(),
            }),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(shaped.response_value["workflow"]["kind"], "close_review");
    let close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_advisor_shaping_check",
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
        close.response_value["close_state"], "ready",
        "{}",
        close.response_value
    );
    Ok(())
}
