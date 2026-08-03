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
    enable_record_run_capabilities(&harness)?;
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
    assert_eq!(
        scoped.response_value["state"]["work_phase"], "shaping",
        "{}",
        scoped.response_value
    );

    let before_shortcuts = harness.counts()?;
    let premature_advance = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_shaping_advance_without_checkpoint",
                Some("idem_shaping_advance_without_checkpoint"),
                false,
                Some(2),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_not_created"),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        premature_advance.response_value["errors"][0]["code"],
        "SHAPING_CHECKPOINT_REQUIRED"
    );
    assert_eq!(
        premature_advance.response_value["errors"][0]["details"]["recovery"]["owner_method"],
        "volicord.record_shaping"
    );

    let premature_run = harness.service.record_run(
        record_run_request(
            "req_shaping_run_before_advance",
            "idem_shaping_run_before_advance",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        premature_run.response_value["errors"][0]["code"],
        "TASK_PHASE_TRANSITION_REQUIRED"
    );
    assert_eq!(
        premature_run.response_value["errors"][0]["details"]["recovery"]["owner_method"],
        "volicord.record_shaping"
    );

    let shaping_close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_shaping_close_before_advance",
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
    assert_eq!(shaping_close.response_value["close_state"], "blocked");
    assert_eq!(
        shaping_close.response_value["state"]["workflow"]["kind"],
        "shaping_required"
    );
    assert_eq!(harness.counts()?, before_shortcuts);

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
            operation: volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
                checkpoint_operation:
                    volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
                scope_revision: 1,
                baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
                summary: "The implementation boundary is ready.".to_owned(),
                implementation_boundary: RequiredNullable::some(
                    "Implement only the current export boundary.".to_owned(),
                ),
                gaps: Vec::new(),
                source_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
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

    let before_stale = harness.counts()?;
    let stale_checkpoint = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_shaping_stale_checkpoint",
                Some("idem_shaping_stale_checkpoint"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_superseded"),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        stale_checkpoint.response_value["errors"][0]["code"],
        "SHAPING_CHECKPOINT_STALE"
    );

    let stale_change_unit = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_shaping_stale_change_unit",
                Some("idem_shaping_stale_change_unit"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
            change_unit_id: ChangeUnitId::new("change_unit_superseded"),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        stale_change_unit.response_value["errors"][0]["code"],
        "CHANGE_UNIT_STALE"
    );
    assert_eq!(harness.counts()?, before_stale);

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
            operation: volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
                checkpoint_operation:
                    volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
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
            },
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
            operation: volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
                checkpoint_operation:
                    volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
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
            },
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

    let checkpoint_id = response.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .expect("blocked checkpoint id");
    let unresolved = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_shaping_gap_advance",
                Some("idem_shaping_gap_advance"),
                false,
                Some(after.state_version),
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
    assert_eq!(
        unresolved.response_value["errors"][0]["code"],
        "USER_DECISION_UNRESOLVED"
    );
    assert_eq!(
        unresolved.response_value["errors"][0]["details"]["recovery"]["owner_method"],
        "volicord.resolve_user_action"
    );
    assert_eq!(harness.counts()?, after);
    Ok(())
}

#[test]
fn non_authorizing_shaping_decisions_require_exact_recovery_and_successor_identity(
) -> Result<(), Box<dyn Error>> {
    for (label, gap_kind, judgment_kind, selected_option, expected_disposition) in [
        (
            "rejected_scope",
            ShapingGapKind::UserScopeDecisionRequired,
            JudgmentKind::ScopeDecision,
            "reject",
            "rejected",
        ),
        (
            "rejected_sensitive",
            ShapingGapKind::SensitiveApprovalRequired,
            JudgmentKind::SensitiveApproval,
            "reject",
            "rejected",
        ),
        (
            "deferred_scope",
            ShapingGapKind::UserScopeDecisionRequired,
            JudgmentKind::ScopeDecision,
            "defer",
            "deferred",
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = shaping_task(&harness, label)?;
        let shaped = record_user_owned_gap(
            &harness,
            label,
            &task_id,
            &change_unit_id,
            gap_kind,
            judgment_kind,
        )?;
        let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
        let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
            .as_str()
            .expect("shaping request id")
            .to_owned();

        let resolved = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!("req_{label}_terminal"),
                &format!("submission_{label}_terminal"),
                None,
                &task_id,
                &request_id,
                selected_option,
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        assert_eq!(resolved.response_value["base"]["response_kind"], "result");
        assert_eq!(
            resolved.response_value["user_action_resolution"]["body"]["resolution_outcome"],
            expected_disposition
        );
        let workflow = &resolved.response_value["state"]["workflow"];
        assert_eq!(workflow["kind"], "decision_recovery_required", "{label}");
        assert_eq!(workflow["next_actor"], "agent", "{label}");
        assert_eq!(workflow["required_action"], "volicord.record_shaping");
        assert_eq!(
            workflow["checkpoint"]["decision_recovery_requirements"][0]["disposition"],
            expected_disposition
        );
        assert_eq!(
            workflow["checkpoint"]["gaps"][0]["status"],
            expected_disposition
        );
        assert_eq!(workflow["checkpoint"]["readiness"], "ready");
        let retirement_ref: StateRecordRef = serde_json::from_value(
            workflow["checkpoint"]["decision_recovery_requirements"][0]["user_action_request_ref"]
                .clone(),
        )?;

        let before_application = harness.counts()?;
        let application = if gap_kind.decision_policy().is_some_and(|policy| {
            policy.application_owner == ShapingDecisionApplicationOwner::AdvanceTask
        }) {
            harness.service.advance_task(
                AdvanceTaskRequest {
                    envelope: envelope(
                        &format!("req_{label}_invalid_application"),
                        Some(&format!("idem_{label}_invalid_application")),
                        false,
                        Some(before_application.state_version),
                        Some(&task_id),
                    ),
                    task_id: TaskId::new(&task_id),
                    shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    change_unit_id: ChangeUnitId::new(&change_unit_id),
                    scope_revision: 1,
                    baseline_ref: BaselineRef::new("baseline_test"),
                    user_action_resolution_ids: Vec::new(),
                },
                invocation(OperationCategory::AgentWorkflow),
            )?
        } else {
            harness.service.update_scope(
                update_scope_request(
                    &format!("req_{label}_invalid_application"),
                    &format!("idem_{label}_invalid_application"),
                    false,
                    Some(before_application.state_version),
                    &task_id,
                    ChangeUnitOperation::KeepCurrent,
                    "A non-authorizing outcome cannot update scope.",
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?
        };
        assert_eq!(
            application.response_value["base"]["response_kind"],
            "rejected"
        );
        assert_eq!(
            application.response_value["errors"][0]["details"]["workflow"]["kind"],
            "decision_recovery_required"
        );
        assert_eq!(harness.counts()?, before_application);

        if selected_option == "defer" {
            let duplicate = harness.service.resolve_user_action(
                resolve_user_action_request(
                    &format!("req_{label}_duplicate"),
                    &format!("submission_{label}_duplicate"),
                    None,
                    &task_id,
                    &request_id,
                    "accept",
                ),
                invocation(OperationCategory::UserOnly),
            )?;
            assert_eq!(
                duplicate.response_value["base"]["response_kind"],
                "rejected"
            );
            assert_eq!(harness.counts()?, before_application);
        }

        let omitted = harness.service.record_shaping(
            ready_shaping_request(
                &format!("req_{label}_omitted_retirement"),
                &format!("idem_{label}_omitted_retirement"),
                before_application.state_version,
                &task_id,
                ShapingCheckpointOperation::ReplaceCurrent {
                    expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    retired_user_action_request_refs: Vec::new(),
                },
                "Recovery requires the exact predecessor decision ref.",
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(omitted.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, before_application);

        let mut successor = ready_shaping_request(
            &format!("req_{label}_successor"),
            &format!("idem_{label}_successor"),
            before_application.state_version,
            &task_id,
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                retired_user_action_request_refs: vec![retirement_ref],
            },
            "The revised plan asks for a fresh decision.",
        );
        let successor_action = user_action_request(
            "unused",
            "unused",
            false,
            Some(before_application.state_version),
            &task_id,
            Some(&change_unit_id),
            judgment_kind,
        )
        .action;
        let RecordShapingOperation::RecordCheckpoint { gaps, .. } = &mut successor.operation else {
            unreachable!();
        };
        *gaps = vec![ShapingGapInput {
            gap_kind,
            summary: "The revised plan still requires this user judgment.".to_owned(),
            affected_refs: Vec::new(),
            user_action: RequiredNullable::some(ShapingUserActionDraft {
                action: successor_action,
                expires_at: RequiredNullable::null(),
            }),
        }];
        let replaced = harness
            .service
            .record_shaping(successor, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(replaced.response_value["base"]["response_kind"], "result");
        assert_eq!(
            replaced.response_value["workflow"]["kind"],
            "awaiting_user_action"
        );
        let successor_request_id = replaced.response_value["created_user_action_request_refs"][0]
            ["record_id"]
            .as_str()
            .expect("successor request id");
        assert_ne!(successor_request_id, request_id);
        assert_eq!(user_action_status(&harness, &request_id)?, "superseded");
        assert_eq!(
            user_action_status(&harness, successor_request_id)?,
            "pending"
        );
    }
    Ok(())
}

#[test]
fn expired_shaping_request_routes_to_read_only_recovery_and_can_be_reissued(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at(DEFAULT_METHOD_TEST_CLOCK);
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = shaping_task(&harness, "expired_recovery")?;
    let mut shaping = ready_shaping_request(
        "req_expired_recovery_shaping",
        "idem_expired_recovery_shaping",
        2,
        &task_id,
        ShapingCheckpointOperation::CreateInitial,
        "The current plan requires a time-bounded scope decision.",
    );
    let action = user_action_request(
        "unused",
        "unused",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ScopeDecision,
    )
    .action;
    let RecordShapingOperation::RecordCheckpoint { gaps, .. } = &mut shaping.operation else {
        unreachable!();
    };
    *gaps = vec![ShapingGapInput {
        gap_kind: ShapingGapKind::UserScopeDecisionRequired,
        summary: "The request expires at its fixed authority boundary.".to_owned(),
        affected_refs: Vec::new(),
        user_action: RequiredNullable::some(ShapingUserActionDraft {
            action,
            expires_at: RequiredNullable::some(UtcTimestamp::parse("2026-06-18T00:01:00Z")?),
        }),
    }];
    let shaped = harness
        .service
        .record_shaping(shaping, invocation(OperationCategory::AgentWorkflow))?;
    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
        .as_str()
        .expect("expiring request id")
        .to_owned();
    let before_expiry_read = harness.counts()?;
    clock.advance(Duration::minutes(2));

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_expired_recovery_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
            continuity_page: None,
        },
        invocation(OperationCategory::Read),
    )?;
    let workflow = &status.response_value["active_task"]["workflow"];
    assert_eq!(workflow["kind"], "decision_recovery_required");
    assert_eq!(workflow["next_actor"], "agent");
    assert_eq!(workflow["required_action"], "volicord.record_shaping");
    assert_eq!(
        workflow["checkpoint"]["decision_recovery_requirements"][0]["reason"],
        "expired"
    );
    assert_eq!(
        workflow["checkpoint"]["decision_recovery_requirements"][0]["user_action_resolution_ref"],
        Value::Null
    );
    assert_eq!(harness.counts()?, before_expiry_read);

    let rejected_resolution = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_expired_recovery_resolve",
            "submission_expired_recovery_resolve",
            None,
            &task_id,
            &request_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(
        rejected_resolution.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(harness.counts()?, before_expiry_read);

    let retirement_ref: StateRecordRef = serde_json::from_value(
        workflow["checkpoint"]["decision_recovery_requirements"][0]["user_action_request_ref"]
            .clone(),
    )?;
    let mut successor = ready_shaping_request(
        "req_expired_recovery_successor",
        "idem_expired_recovery_successor",
        before_expiry_read.state_version,
        &task_id,
        ShapingCheckpointOperation::ReplaceCurrent {
            expected_current_checkpoint_id: ShapingCheckpointId::new(checkpoint_id),
            retired_user_action_request_refs: vec![retirement_ref],
        },
        "The revised plan creates a fresh bounded request.",
    );
    let successor_action = user_action_request(
        "unused",
        "unused",
        false,
        Some(before_expiry_read.state_version),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ScopeDecision,
    )
    .action;
    let RecordShapingOperation::RecordCheckpoint { gaps, .. } = &mut successor.operation else {
        unreachable!();
    };
    *gaps = vec![ShapingGapInput {
        gap_kind: ShapingGapKind::UserScopeDecisionRequired,
        summary: "The successor request has an independent immutable identity.".to_owned(),
        affected_refs: Vec::new(),
        user_action: RequiredNullable::some(ShapingUserActionDraft {
            action: successor_action,
            expires_at: RequiredNullable::some(UtcTimestamp::parse("2026-06-18T00:05:00Z")?),
        }),
    }];
    let replaced = harness
        .service
        .record_shaping(successor, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        replaced.response_value["workflow"]["kind"],
        "awaiting_user_action"
    );
    let successor_request_id = replaced.response_value["created_user_action_request_refs"][0]
        ["record_id"]
        .as_str()
        .expect("successor request id");
    assert_ne!(successor_request_id, request_id);
    assert_eq!(user_action_status(&harness, &request_id)?, "superseded");
    assert_eq!(
        harness
            .store()?
            .user_action_record(
                successor_request_id,
                &UtcTimestamp::parse("2026-06-18T00:02:00Z")?,
            )?
            .expect("successor user action")
            .status(),
        UserActionStatus::Pending
    );
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
        advisor_update_scope_request(
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
    assert_eq!(
        scoped.response_value["state"]["work_phase"], "shaping",
        "{}",
        scoped.response_value
    );
    let before_write_capable_scope = harness.counts()?;
    let write_capable_scope = harness.service.update_scope(
        update_scope_request(
            "req_advisor_write_capable_scope",
            "idem_advisor_write_capable_scope",
            false,
            Some(before_write_capable_scope.state_version),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Advisor mode cannot own a write-capable Change Unit.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        write_capable_scope.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(harness.counts()?, before_write_capable_scope);
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
            operation: volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
                checkpoint_operation:
                    volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
                scope_revision: 1,
                baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
                summary: "The advice is complete.".to_owned(),
                implementation_boundary: RequiredNullable::some(
                    "Advice is limited to the current export boundary.".to_owned(),
                ),
                gaps: Vec::new(),
                source_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        shaped.response_value["workflow"]["kind"],
        "ready_to_finalize_advice"
    );
    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let pre_finalize_close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_advisor_pre_finalize_check",
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
    let pre_finalize_json = serde_json::to_string(&pre_finalize_close.response_value)?;
    assert!(pre_finalize_json.contains("volicord.record_shaping"));
    assert!(!pre_finalize_json.contains("volicord.record_run"));

    enable_record_run_capabilities(&harness)?;
    let before_forbidden_mutations = harness.counts()?;
    let advisor_write = harness.service.prepare_write(
        prepare_write_request(
            "req_advisor_public_write",
            "idem_advisor_public_write",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        advisor_write.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(harness.counts()?, before_forbidden_mutations);
    let advisor_run = harness.service.record_run(
        record_run_request(
            "req_advisor_public_run",
            "idem_advisor_public_run",
            false,
            Some(3),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        advisor_run.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(harness.counts()?, before_forbidden_mutations);
    assert!(!product_repo_root(&harness)?.join("src").exists());
    let finalize_request = RecordShapingRequest {
        envelope: envelope(
            "req_advisor_shaping_finalize",
            Some("idem_advisor_shaping_finalize"),
            false,
            Some(3),
            Some(&task_id),
        ),
        task_id: TaskId::new(&task_id),
        operation: volicord_types::methods::RecordShapingOperation::FinalizeAdvice {
            shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
            result_summary: "The requested advice is complete.".to_owned(),
            result_refs: Vec::new(),
            evidence_refs: Vec::new(),
            residual_risks: Vec::new(),
            recovery_constraints: Vec::new(),
        },
    };
    let finalized = harness.service.record_shaping(
        finalize_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(finalized.response_value["workflow"]["kind"], "close_review");
    assert!(finalized.response_value["workflow"]["allowed_actions"]
        .as_array()
        .is_some_and(|actions| actions
            .iter()
            .any(|action| action == "volicord.record_shaping")));
    let finalized_task = task_revision(&harness, &task_id)?;
    let finalized_basis = finalized_task
        .current_close_basis
        .as_ref()
        .expect("advisor finalization close basis");
    assert_eq!(
        finalized_basis
            .shaping_checkpoint_ref
            .as_ref()
            .expect("advisor checkpoint ref")
            .record_id
            .as_str(),
        checkpoint_id
    );
    assert!(finalized_basis.source_run_ref.as_ref().is_none());
    let replay = harness.service.record_shaping(
        finalize_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(replay.response_value, finalized.response_value);
    let mut conflict_request = finalize_request;
    if let RecordShapingOperation::FinalizeAdvice { result_summary, .. } =
        &mut conflict_request.operation
    {
        *result_summary = "A conflicting advice result.".to_owned();
    }
    let conflict = harness.service.record_shaping(
        conflict_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(conflict.response_value["base"]["response_kind"], "rejected");
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
        close.response_value["close_state"], "blocked",
        "{}",
        close.response_value
    );
    harness.conn()?.execute(
        "UPDATE tasks
            SET close_basis_json = json_set(
                close_basis_json,
                '$.shaping_checkpoint_ref.record_id',
                'checkpoint_wrong'
            )
          WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
    )?;
    assert!(matches!(
        harness.store()?.task_record(&TaskId::new(&task_id)),
        Err(volicord_store::StoreError::SchemaInvariant { .. })
    ));
    Ok(())
}

#[test]
fn advisor_close_basis_is_invalidated_by_checkpoint_replacement() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "advisor_checkpoint_invalidation",
        RequestedMode::Advisor,
    )?;
    let shaped = harness.service.record_shaping(
        ready_shaping_request(
            "req_advisor_checkpoint_invalidation_record",
            "idem_advisor_checkpoint_invalidation_record",
            2,
            &task_id,
            ShapingCheckpointOperation::CreateInitial,
            "Current bounded advice.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let finalized = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_advisor_checkpoint_invalidation_finalize",
                Some("idem_advisor_checkpoint_invalidation_finalize"),
                false,
                Some(3),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            operation: RecordShapingOperation::FinalizeAdvice {
                shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                change_unit_id: ChangeUnitId::new(&change_unit_id),
                scope_revision: 1,
                baseline_ref: BaselineRef::new("baseline_test"),
                user_action_resolution_ids: Vec::new(),
                result_summary: "Current bounded advice result.".to_owned(),
                result_refs: Vec::new(),
                evidence_refs: Vec::new(),
                residual_risks: Vec::new(),
                recovery_constraints: Vec::new(),
            },
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(finalized.response_value["workflow"]["kind"], "close_review");
    let before = task_revision(&harness, &task_id)?;
    assert!(before.current_close_basis.is_some());

    let replaced = harness.service.record_shaping(
        ready_shaping_request(
            "req_advisor_checkpoint_invalidation_replace",
            "idem_advisor_checkpoint_invalidation_replace",
            4,
            &task_id,
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                retired_user_action_request_refs: Vec::new(),
            },
            "Replacement bounded advice.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        replaced.response_value["workflow"]["kind"],
        "ready_to_finalize_advice"
    );
    assert_ne!(
        shaping_checkpoint_id(&replaced.response_value),
        checkpoint_id
    );
    let after = task_revision(&harness, &task_id)?;
    assert!(after.current_close_basis.is_none());
    assert_eq!(after.close_basis_revision, before.close_basis_revision + 1);
    Ok(())
}

#[test]
fn stale_advisor_resolution_blocks_finalization() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "advisor_stale_resolution",
        RequestedMode::Advisor,
    )?;
    let shaped = record_user_owned_gaps(
        &harness,
        "advisor_stale_resolution",
        &task_id,
        Some(&change_unit_id),
        &[(
            ShapingGapKind::UserProductDecisionRequired,
            JudgmentKind::ProductDecision,
        )],
    )?;
    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
        .as_str()
        .expect("advisor request id");
    let resolved = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_advisor_stale_resolution_resolve",
            "submission_advisor_stale_resolution",
            None,
            &task_id,
            request_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let resolution_id = resolved.response_value["user_action_resolution_ref"]["record_id"]
        .as_str()
        .expect("advisor resolution id")
        .to_owned();

    let scoped = harness.service.update_scope(
        advisor_update_scope_request(
            "req_advisor_stale_resolution_scope_change",
            "idem_advisor_stale_resolution_scope_change",
            false,
            Some(4),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Materially changed advisor analysis scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        scoped.response_value["base"]["response_kind"], "result",
        "{}",
        scoped.response_value
    );

    let stale_finalize = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_advisor_stale_resolution_finalize",
                Some("idem_advisor_stale_resolution_finalize"),
                false,
                Some(5),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            operation: RecordShapingOperation::FinalizeAdvice {
                shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                change_unit_id: ChangeUnitId::new(&change_unit_id),
                scope_revision: 1,
                baseline_ref: BaselineRef::new("baseline_test"),
                user_action_resolution_ids: vec![UserActionResolutionId::new(&resolution_id)],
                result_summary: "Stale advice must not finalize.".to_owned(),
                result_refs: Vec::new(),
                evidence_refs: Vec::new(),
                residual_risks: Vec::new(),
                recovery_constraints: Vec::new(),
            },
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        stale_finalize.response_value["base"]["response_kind"],
        "rejected"
    );
    assert!(task_revision(&harness, &task_id)?
        .current_close_basis
        .is_none());
    Ok(())
}

#[test]
fn advisor_decision_kinds_apply_exactly_at_finalization_and_close() -> Result<(), Box<dyn Error>> {
    for (label, decisions) in [
        (
            "advisor_product",
            vec![(
                ShapingGapKind::UserProductDecisionRequired,
                JudgmentKind::ProductDecision,
            )],
        ),
        (
            "advisor_technical",
            vec![(
                ShapingGapKind::UserTechnicalDecisionRequired,
                JudgmentKind::TechnicalDecision,
            )],
        ),
        (
            "advisor_scope",
            vec![(
                ShapingGapKind::UserScopeDecisionRequired,
                JudgmentKind::ScopeDecision,
            )],
        ),
        (
            "advisor_sensitive",
            vec![(
                ShapingGapKind::SensitiveApprovalRequired,
                JudgmentKind::SensitiveApproval,
            )],
        ),
        (
            "advisor_product_technical",
            vec![
                (
                    ShapingGapKind::UserProductDecisionRequired,
                    JudgmentKind::ProductDecision,
                ),
                (
                    ShapingGapKind::UserTechnicalDecisionRequired,
                    JudgmentKind::TechnicalDecision,
                ),
            ],
        ),
        (
            "advisor_product_scope",
            vec![
                (
                    ShapingGapKind::UserProductDecisionRequired,
                    JudgmentKind::ProductDecision,
                ),
                (
                    ShapingGapKind::UserScopeDecisionRequired,
                    JudgmentKind::ScopeDecision,
                ),
            ],
        ),
        (
            "advisor_technical_scope",
            vec![
                (
                    ShapingGapKind::UserTechnicalDecisionRequired,
                    JudgmentKind::TechnicalDecision,
                ),
                (
                    ShapingGapKind::UserScopeDecisionRequired,
                    JudgmentKind::ScopeDecision,
                ),
            ],
        ),
        (
            "advisor_multiple",
            vec![
                (
                    ShapingGapKind::UserProductDecisionRequired,
                    JudgmentKind::ProductDecision,
                ),
                (
                    ShapingGapKind::UserTechnicalDecisionRequired,
                    JudgmentKind::TechnicalDecision,
                ),
                (
                    ShapingGapKind::UserScopeDecisionRequired,
                    JudgmentKind::ScopeDecision,
                ),
            ],
        ),
        (
            "advisor_all",
            vec![
                (
                    ShapingGapKind::UserProductDecisionRequired,
                    JudgmentKind::ProductDecision,
                ),
                (
                    ShapingGapKind::UserTechnicalDecisionRequired,
                    JudgmentKind::TechnicalDecision,
                ),
                (
                    ShapingGapKind::UserScopeDecisionRequired,
                    JudgmentKind::ScopeDecision,
                ),
                (
                    ShapingGapKind::SensitiveApprovalRequired,
                    JudgmentKind::SensitiveApproval,
                ),
            ],
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, initial_change_unit_id) =
            create_task_with_mode_and_change_unit(&harness, label, RequestedMode::Advisor)?;
        let shaped = record_user_owned_gaps(
            &harness,
            label,
            &task_id,
            Some(&initial_change_unit_id),
            &decisions,
        )?;
        assert_eq!(
            shaped.response_value["workflow"]["kind"],
            "awaiting_user_action"
        );
        let before_forbidden_mutations = harness.counts()?;
        let forbidden_write = harness.service.prepare_write(
            prepare_write_request(
                &format!("req_{label}_write"),
                &format!("idem_{label}_write"),
                Some(before_forbidden_mutations.state_version),
                Some(&task_id),
                Some(&initial_change_unit_id),
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            forbidden_write.response_value["base"]["response_kind"],
            "rejected"
        );
        assert_eq!(harness.counts()?, before_forbidden_mutations);
        let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
        let request_refs = shaped.response_value["created_user_action_request_refs"]
            .as_array()
            .expect("advisor decision requests")
            .clone();
        let pending_finalize = harness.service.record_shaping(
            RecordShapingRequest {
                envelope: envelope(
                    &format!("req_{label}_pending_finalize"),
                    Some(&format!("idem_{label}_pending_finalize")),
                    false,
                    Some(harness.counts()?.state_version),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                operation: RecordShapingOperation::FinalizeAdvice {
                    shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    change_unit_id: ChangeUnitId::new(&initial_change_unit_id),
                    scope_revision: 1,
                    baseline_ref: BaselineRef::new("baseline_test"),
                    user_action_resolution_ids: Vec::new(),
                    result_summary: "Advice result.".to_owned(),
                    result_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                    residual_risks: Vec::new(),
                    recovery_constraints: Vec::new(),
                },
            },
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            pending_finalize.response_value["base"]["response_kind"],
            "rejected"
        );

        let mut resolution_refs = Vec::new();
        for (index, request_ref) in request_refs.iter().enumerate() {
            let request_id = request_ref["record_id"].as_str().expect("request id");
            let resolved = harness.service.resolve_user_action(
                resolve_user_action_request(
                    &format!("req_{label}_resolve_{index}"),
                    &format!("submission_{label}_{index}"),
                    None,
                    &task_id,
                    request_id,
                    "accept",
                ),
                invocation(OperationCategory::UserOnly),
            )?;
            assert_eq!(
                resolved.response_value["base"]["response_kind"], "result",
                "{label}: {}",
                resolved.response_value
            );
            resolution_refs.push(serde_json::from_value::<StateRecordRef>(
                resolved.response_value["user_action_resolution_ref"].clone(),
            )?);
        }
        let has_scope = decisions
            .iter()
            .any(|(kind, _)| *kind == ShapingGapKind::UserScopeDecisionRequired);
        let (change_unit_id, scope_revision) = if has_scope {
            let scope_resolution_refs = decisions
                .iter()
                .zip(&resolution_refs)
                .filter(|((kind, _), _)| *kind == ShapingGapKind::UserScopeDecisionRequired)
                .map(|(_, reference)| reference.clone())
                .collect();
            let mut scope_request = advisor_update_scope_request(
                &format!("req_{label}_apply_scope"),
                &format!("idem_{label}_apply_scope"),
                false,
                Some(harness.counts()?.state_version),
                &task_id,
                ChangeUnitOperation::ReplaceCurrent,
                "Apply the exact advisor scope decision.",
            );
            scope_request.related_scope_decision_refs = scope_resolution_refs;
            let scope = harness
                .service
                .update_scope(scope_request, invocation(OperationCategory::AgentWorkflow))?;
            (
                response_record_id(&scope.response_value, "change_unit_ref"),
                2,
            )
        } else {
            (initial_change_unit_id, 1)
        };
        let current_checkpoint = harness
            .store()?
            .current_shaping_checkpoint(&TaskId::new(&task_id))?
            .expect("current advisor checkpoint");
        assert_eq!(current_checkpoint.shaping_checkpoint_id, checkpoint_id);
        let finalized = harness.service.record_shaping(
            RecordShapingRequest {
                envelope: envelope(
                    &format!("req_{label}_finalize"),
                    Some(&format!("idem_{label}_finalize")),
                    false,
                    Some(harness.counts()?.state_version),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                operation: RecordShapingOperation::FinalizeAdvice {
                    shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    change_unit_id: ChangeUnitId::new(&change_unit_id),
                    scope_revision,
                    baseline_ref: BaselineRef::new("baseline_test"),
                    user_action_resolution_ids: resolution_refs
                        .iter()
                        .map(|reference| UserActionResolutionId::new(reference.record_id.as_str()))
                        .collect(),
                    result_summary: "The bounded advisor decision is complete.".to_owned(),
                    result_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                    residual_risks: Vec::new(),
                    recovery_constraints: Vec::new(),
                },
            },
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(finalized.response_value["workflow"]["kind"], "close_review");
        assert_eq!(
            finalized.response_value["shaping_checkpoint"]["shaping_checkpoint_id"],
            checkpoint_id
        );
        assert!(finalized.response_value["created_user_action_request_refs"]
            .as_array()
            .expect("created request refs")
            .is_empty());
        let basis = &finalized.response_value["state"];
        assert_eq!(basis["close_state"], "blocked");
        let after_acceptance = record_final_acceptance(
            &harness,
            &task_id,
            &change_unit_id,
            finalized.response_value["base"]["state_version"]
                .as_u64()
                .expect("finalized state version"),
            label,
        )?;
        let closed = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: &format!("req_{label}_close"),
                idempotency_key: Some(&format!("idem_{label}_close")),
                dry_run: false,
                expected_state_version: Some(after_acceptance),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            closed.response_value["state"]["lifecycle"]["result"],
            "advice_only"
        );
        let final_counts = harness.counts()?;
        assert_eq!(final_counts.write_tickets, 0, "{label}");
        assert_eq!(final_counts.runs, 0, "{label}");
        assert!(
            !product_repo_root(&harness)?.join("src").exists(),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn direct_mode_records_a_mutation_without_shaping() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let repository = planning_only_repository_fixture(&harness, "direct_without_shaping")?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "direct_without_shaping",
        RequestedMode::Direct,
    )?;
    assert_eq!(
        harness
            .store()?
            .task_record(&TaskId::new(&task_id))?
            .expect("direct Task")
            .work_phase,
        WorkPhase::Implementation
    );
    assert!(harness
        .store()?
        .current_shaping_checkpoint(&TaskId::new(&task_id))?
        .is_none());

    let before = harness.counts()?;
    let ticket = harness.service.prepare_write(
        prepare_write_request(
            "req_direct_without_shaping_write",
            "idem_direct_without_shaping_write",
            Some(before.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(ticket.response_value["decision"], "allowed");
    assert!(repository.status_bytes()?.is_empty());
    let write_ticket_id = response_record_id(&ticket.response_value, "write_ticket_ref");
    repository.write(
        "src/export.rs",
        b"// authorized direct-mode implementation\n",
    )?;

    let mut request = product_write_record_run_request(
        "req_direct_without_shaping_run",
        "idem_direct_without_shaping_run",
        before.state_version + 1,
        &task_id,
        &change_unit_id,
        &write_ticket_id,
        "run_direct_without_shaping",
    );
    request.kind = RunKind::Direct;
    request.close_assessment = Some(close_assessment_with_risks(
        "The bounded direct operation is complete.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_typed_result_contract::<RecordRunResult>(&response);
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["state"]["mode"], "direct");
    let after = harness.counts()?;
    assert_eq!(after.state_version, before.state_version + 2);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_tickets, before.write_tickets + 1);
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");

    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after.state_version,
        "direct_without_shaping",
    )?;
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_direct_without_shaping_close",
            idempotency_key: Some("idem_direct_without_shaping_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");

    let tracked = MethodHarness::new()?;
    let intake = tracked.service.intake(
        intake_request(
            "req_tracked_work_still_shapes",
            "idem_tracked_work_still_shapes",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(intake.response_value["state"]["work_phase"], "shaping");
    assert_eq!(
        intake.response_value["state"]["workflow"]["kind"],
        "shaping_required"
    );
    Ok(())
}

#[test]
fn shaping_checkpoint_succession_is_explicit_linear_and_replayable() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _change_unit_id) = shaping_task(&harness, "succession")?;
    let initial_request = ready_shaping_request(
        "req_succession_initial",
        "idem_succession_initial",
        2,
        &task_id,
        ShapingCheckpointOperation::CreateInitial,
        "Initial current shaping authority.",
    );
    let initial = harness.service.record_shaping(
        initial_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let initial_id = shaping_checkpoint_id(&initial.response_value);
    assert_eq!(
        initial.response_value["shaping_checkpoint"]["predecessor_checkpoint_id"],
        Value::Null
    );
    let after_initial = harness.counts()?;

    let duplicate_initial = harness.service.record_shaping(
        ready_shaping_request(
            "req_succession_duplicate_initial",
            "idem_succession_duplicate_initial",
            after_initial.state_version,
            &task_id,
            ShapingCheckpointOperation::CreateInitial,
            "Duplicate initial authority.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        duplicate_initial.response_value["errors"][0]["code"],
        "SHAPING_CHECKPOINT_STALE"
    );

    let stale_replacement = harness.service.record_shaping(
        ready_shaping_request(
            "req_succession_stale",
            "idem_succession_stale",
            after_initial.state_version,
            &task_id,
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new("checkpoint_unknown"),
                retired_user_action_request_refs: Vec::new(),
            },
            "Stale replacement authority.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        stale_replacement.response_value["errors"][0]["code"],
        "SHAPING_CHECKPOINT_STALE"
    );
    assert_eq!(harness.counts()?, after_initial);

    let replacement_request = ready_shaping_request(
        "req_succession_replace",
        "idem_succession_replace",
        after_initial.state_version,
        &task_id,
        ShapingCheckpointOperation::ReplaceCurrent {
            expected_current_checkpoint_id: ShapingCheckpointId::new(&initial_id),
            retired_user_action_request_refs: Vec::new(),
        },
        "Exact successor authority.",
    );
    let replacement = harness.service.record_shaping(
        replacement_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_id = shaping_checkpoint_id(&replacement.response_value);
    assert_eq!(
        replacement.response_value["shaping_checkpoint"]["predecessor_checkpoint_id"],
        initial_id
    );
    let store = harness.store()?;
    let predecessor = store
        .shaping_checkpoint_record(&TaskId::new(&task_id), &initial_id)?
        .expect("predecessor must remain durable");
    let successor = store
        .shaping_checkpoint_record(&TaskId::new(&task_id), &replacement_id)?
        .expect("successor must be durable");
    assert_eq!(
        successor.predecessor_shaping_checkpoint_id.as_deref(),
        Some(initial_id.as_str())
    );
    assert_eq!(
        predecessor.readiness,
        ShapingCheckpointReadiness::Superseded
    );
    assert_eq!(
        predecessor.superseded_at.as_ref(),
        Some(&successor.created_at)
    );
    drop(store);

    let replay = harness.service.record_shaping(
        replacement_request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(replay.response_value, replacement.response_value);
    let after_replacement = harness.counts()?;

    let losing_competitor = harness.service.record_shaping(
        ready_shaping_request(
            "req_succession_competing_replace",
            "idem_succession_competing_replace",
            after_replacement.state_version,
            &task_id,
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new(&initial_id),
                retired_user_action_request_refs: Vec::new(),
            },
            "A competing replacement cannot displace the committed successor.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        losing_competitor.response_value["errors"][0]["code"],
        "SHAPING_CHECKPOINT_STALE"
    );
    assert_eq!(harness.counts()?, after_replacement);

    let mut conflict = replacement_request;
    if let volicord_types::methods::RecordShapingOperation::RecordCheckpoint { summary, .. } =
        &mut conflict.operation
    {
        *summary = "Conflicting replay payload.".to_owned();
    }
    let conflict = harness
        .service
        .record_shaping(conflict, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(conflict.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, after_replacement);
    Ok(())
}

#[test]
fn concurrent_checkpoint_replacements_commit_exactly_one_successor() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = shaping_task(&harness, "concurrent_succession")?;
    let initial = harness.service.record_shaping(
        ready_shaping_request(
            "req_concurrent_succession_initial",
            "idem_concurrent_succession_initial",
            2,
            &task_id,
            ShapingCheckpointOperation::CreateInitial,
            "Initial authority before concurrent succession.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let initial_id = shaping_checkpoint_id(&initial.response_value);
    let before = harness.counts()?;
    let first = ready_shaping_request(
        "req_concurrent_succession_first",
        "idem_concurrent_succession_first",
        before.state_version,
        &task_id,
        ShapingCheckpointOperation::ReplaceCurrent {
            expected_current_checkpoint_id: ShapingCheckpointId::new(&initial_id),
            retired_user_action_request_refs: Vec::new(),
        },
        "First competing successor.",
    );
    let second = ready_shaping_request(
        "req_concurrent_succession_second",
        "idem_concurrent_succession_second",
        before.state_version,
        &task_id,
        ShapingCheckpointOperation::ReplaceCurrent {
            expected_current_checkpoint_id: ShapingCheckpointId::new(&initial_id),
            retired_user_action_request_refs: Vec::new(),
        },
        "Second competing successor.",
    );
    let barrier = Arc::new(Barrier::new(3));
    let (first, second) = std::thread::scope(|scope| {
        let first_barrier = Arc::clone(&barrier);
        let first_service = &harness.service;
        let first = scope.spawn(move || {
            first_barrier.wait();
            first_service.record_shaping(first, invocation(OperationCategory::AgentWorkflow))
        });
        let second_barrier = Arc::clone(&barrier);
        let second_service = &harness.service;
        let second = scope.spawn(move || {
            second_barrier.wait();
            second_service.record_shaping(second, invocation(OperationCategory::AgentWorkflow))
        });
        barrier.wait();
        (
            first.join().expect("first replacement thread"),
            second.join().expect("second replacement thread"),
        )
    });
    let responses = [first?, second?];
    let result_count = responses
        .iter()
        .filter(|response| response.response_value["base"]["response_kind"] == "result")
        .count();
    assert_eq!(
        result_count, 1,
        "concurrent responses: {} / {}",
        responses[0].response_value, responses[1].response_value
    );
    let rejected = responses
        .iter()
        .find(|response| response.response_value["base"]["response_kind"] == "rejected")
        .expect("one concurrent replacement must lose");
    assert!(
        matches!(
            rejected.response_value["errors"][0]["code"].as_str(),
            Some("STATE_VERSION_CONFLICT" | "SHAPING_CHECKPOINT_STALE")
        ),
        "losing response: {}",
        rejected.response_value
    );
    let after = harness.counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    let current = harness
        .store()?
        .current_shaping_checkpoint(&TaskId::new(&task_id))?
        .expect("one current successor");
    assert_eq!(
        current.predecessor_shaping_checkpoint_id.as_deref(),
        Some(initial_id.as_str())
    );
    Ok(())
}

#[test]
fn every_live_user_owned_shaping_decision_blocks_replacement() -> Result<(), Box<dyn Error>> {
    for (label, gap_kind, judgment_kind) in [
        (
            "product",
            ShapingGapKind::UserProductDecisionRequired,
            JudgmentKind::ProductDecision,
        ),
        (
            "technical",
            ShapingGapKind::UserTechnicalDecisionRequired,
            JudgmentKind::TechnicalDecision,
        ),
        (
            "scope",
            ShapingGapKind::UserScopeDecisionRequired,
            JudgmentKind::ScopeDecision,
        ),
        (
            "sensitive",
            ShapingGapKind::SensitiveApprovalRequired,
            JudgmentKind::SensitiveApproval,
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = shaping_task(&harness, label)?;
        let shaped = record_user_owned_gap(
            &harness,
            label,
            &task_id,
            &change_unit_id,
            gap_kind,
            judgment_kind,
        )?;
        let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
        let request_ref = shaped.response_value["created_user_action_request_refs"][0].clone();
        let before = harness.counts()?;
        let replacement = harness.service.record_shaping(
            ready_shaping_request(
                &format!("req_{label}_replace"),
                &format!("idem_{label}_replace"),
                before.state_version,
                &task_id,
                ShapingCheckpointOperation::ReplaceCurrent {
                    expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    retired_user_action_request_refs: Vec::new(),
                },
                "A free-form replacement cannot remove decision authority.",
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            replacement.response_value["errors"][0]["code"], "USER_DECISION_UNRESOLVED",
            "{label}: {}",
            replacement.response_value
        );
        let details = &replacement.response_value["errors"][0]["details"];
        assert_eq!(details["state_change_applied"], false);
        assert!(details["blockers"][0]["required_refs"]
            .as_array()
            .expect("required refs")
            .iter()
            .any(|record| {
                record["record_kind"] == "shaping_checkpoint"
                    && record["record_id"] == checkpoint_id
            }));
        assert_eq!(
            details["blockers"][0]["user_actions"],
            json!([{
                "user_action_request_ref": request_ref,
                "effective_status": "pending",
                "required_owner_method": "volicord.resolve_user_action"
            }])
        );
        assert_eq!(harness.counts()?, before, "{label}");
    }
    Ok(())
}

#[test]
fn direct_store_mutation_cannot_detach_live_shaping_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = shaping_task(&harness, "direct_store_detach")?;
    let shaped = record_user_owned_gap(
        &harness,
        "direct_store_detach",
        &task_id,
        &change_unit_id,
        ShapingGapKind::UserProductDecisionRequired,
        JudgmentKind::ProductDecision,
    )?;
    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let before = harness.counts()?;
    let mut store = harness.mutation_store()?;
    let input = volicord_store::core_pipeline::commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RecordShaping,
        Some(&IdempotencyKey::new("idem_direct_store_detach")),
        &volicord_types::ids::RequestHash::new("sha256:direct-store-detach"),
        Some(volicord_store::core_pipeline::VerifiedReplayContext {
            actor_source: ActorSource::AgentConnection(AgentConnectionId::new(CONNECTION_ID)),
            operation_category: OperationCategory::AgentWorkflow,
            verification_basis: Some("direct_store_test".to_owned()),
            git_workspace_context: None,
        }),
        Some(before.state_version),
        vec![volicord_store::core_pipeline::PendingTaskEvent {
            event_id: "event_direct_store_detach".to_owned(),
            task_id: Some(task_id.clone()),
            change_unit_id: Some(change_unit_id),
            event_kind: "shaping_checkpoint_recorded".to_owned(),
            event_payload_json: "{}".to_owned(),
        }],
    );
    let mutation = volicord_store::core_pipeline::CoreStorageMutation::Shaping(
        volicord_store::core_pipeline::ShapingCheckpointMutation::Record(
            volicord_store::core_pipeline::ShapingCheckpointInsert {
                shaping_checkpoint_id: "checkpoint_direct_store_successor".to_owned(),
                checkpoint_operation: ShapingCheckpointOperation::ReplaceCurrent {
                    expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    retired_user_action_request_refs: Vec::new(),
                },
                task_id: task_id.clone(),
                scope_revision: 1,
                baseline_ref: Some(BaselineRef::new("baseline_test")),
                summary: "Direct Store successor attempt.".to_owned(),
                implementation_boundary: Some("Exact current scope only.".to_owned()),
                readiness: ShapingCheckpointReadiness::Ready,
                source_refs: Vec::new(),
                evidence_refs: Vec::new(),
                created_at: UtcTimestamp::parse("2026-06-18T00:00:01Z")?,
                retired_user_action_request_ids: Vec::new(),
                gaps: Vec::new(),
            },
        ),
    );
    let error = store
        .commit_mutation(input, &[mutation], |_| Ok("{}".to_owned()))
        .expect_err("Store must reject direct detachment of live UserAction authority");
    assert!(matches!(
        error,
        volicord_store::StoreError::InvalidInput { .. }
    ));
    drop(store);
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        harness
            .store()?
            .current_shaping_checkpoint(&TaskId::new(&task_id))?
            .expect("original checkpoint remains current")
            .shaping_checkpoint_id,
        checkpoint_id
    );
    Ok(())
}

#[test]
fn resolved_decision_blocks_until_scope_authority_applies_and_invalidates_it(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = shaping_task(&harness, "resolved_scope")?;
    let shaped = record_user_owned_gap(
        &harness,
        "resolved_scope",
        &task_id,
        &change_unit_id,
        ShapingGapKind::UserScopeDecisionRequired,
        JudgmentKind::ScopeDecision,
    )?;
    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
        .as_str()
        .expect("request id")
        .to_owned();
    let resolved = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_resolved_scope_resolve",
            "submission_resolved_scope",
            None,
            &task_id,
            &request_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(
        resolved.response_value["base"]["response_kind"], "result",
        "{}",
        resolved.response_value
    );
    let resolution_ref: StateRecordRef =
        serde_json::from_value(resolved.response_value["user_action_resolution_ref"].clone())?;
    let before_rejected = harness.counts()?;
    let rejected = harness.service.record_shaping(
        ready_shaping_request(
            "req_resolved_scope_replace_early",
            "idem_resolved_scope_replace_early",
            before_rejected.state_version,
            &task_id,
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                retired_user_action_request_refs: Vec::new(),
            },
            "Resolved authority is not yet applied.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        rejected.response_value["errors"][0]["details"]["blockers"][0]["user_actions"][0]
            ["effective_status"],
        "resolved"
    );
    assert_eq!(harness.counts()?, before_rejected);

    let mut scope_request = update_scope_request(
        "req_resolved_scope_apply",
        "idem_resolved_scope_apply",
        false,
        Some(before_rejected.state_version),
        &task_id,
        ChangeUnitOperation::ReplaceCurrent,
        "Scope authority applies the resolved shaping decision.",
    );
    scope_request.related_scope_decision_refs = vec![resolution_ref.clone()];
    let applied = harness
        .service
        .update_scope(scope_request, invocation(OperationCategory::AgentWorkflow))?;
    let current = harness
        .store()?
        .current_shaping_checkpoint(&TaskId::new(&task_id))?
        .expect("applied checkpoint remains current");
    assert_eq!(current.shaping_checkpoint_id, checkpoint_id);
    assert!(current
        .gaps
        .iter()
        .all(|gap| gap.status == ShapingGapStatus::Applied));
    let mut later_request = ready_shaping_request(
        "req_resolved_scope_replace_after_apply",
        "idem_resolved_scope_replace_after_apply",
        applied.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version"),
        &task_id,
        ShapingCheckpointOperation::ReplaceCurrent {
            expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
            retired_user_action_request_refs: Vec::new(),
        },
        "Applied authority permits exact checkpoint succession.",
    );
    if let volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
        scope_revision,
        ..
    } = &mut later_request.operation
    {
        *scope_revision = 2;
    }
    let later = harness
        .service
        .record_shaping(later_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        later.response_value["base"]["response_kind"], "result",
        "{}",
        later.response_value
    );
    assert_eq!(
        later.response_value["shaping_checkpoint"]["predecessor_checkpoint_id"],
        checkpoint_id
    );
    let before_cross_checkpoint = harness.counts()?;
    let cross_checkpoint = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_resolved_scope_cross_checkpoint",
                Some("idem_resolved_scope_cross_checkpoint"),
                false,
                Some(before_cross_checkpoint.state_version),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(shaping_checkpoint_id(
                &later.response_value,
            )),
            change_unit_id: ChangeUnitId::new(response_record_id(
                &applied.response_value,
                "change_unit_ref",
            )),
            scope_revision: 2,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: vec![UserActionResolutionId::new(
                resolution_ref.record_id.as_str(),
            )],
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        cross_checkpoint.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(harness.counts()?, before_cross_checkpoint);
    Ok(())
}

#[test]
fn shaping_decision_owner_matrix_routes_and_applies_only_exact_gaps() -> Result<(), Box<dyn Error>>
{
    let product = (
        ShapingGapKind::UserProductDecisionRequired,
        JudgmentKind::ProductDecision,
    );
    let technical = (
        ShapingGapKind::UserTechnicalDecisionRequired,
        JudgmentKind::TechnicalDecision,
    );
    let scope = (
        ShapingGapKind::UserScopeDecisionRequired,
        JudgmentKind::ScopeDecision,
    );
    let sensitive = (
        ShapingGapKind::SensitiveApprovalRequired,
        JudgmentKind::SensitiveApproval,
    );
    let cases = vec![
        ("none", vec![]),
        ("product", vec![product]),
        ("technical", vec![technical]),
        ("scope", vec![scope]),
        ("sensitive", vec![sensitive]),
        ("product_technical", vec![product, technical]),
        ("product_scope", vec![product, scope]),
        ("technical_scope", vec![technical, scope]),
        ("product_technical_scope", vec![product, technical, scope]),
        ("all", vec![product, technical, scope, sensitive]),
    ];

    for (label, decisions) in cases {
        let harness = MethodHarness::new()?;
        let repository = planning_only_repository_fixture(&harness, label)?;
        let (task_id, change_unit_id) = shaping_task(&harness, label)?;
        let shaped =
            record_user_owned_gaps(&harness, label, &task_id, Some(&change_unit_id), &decisions)?;
        let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
        assert_eq!(
            shaped.response_value["shaping_checkpoint"]["readiness"],
            if decisions.is_empty() {
                "ready"
            } else {
                "blocked"
            },
            "{label} checkpoint readiness follows exact unresolved gaps"
        );
        let request_refs = shaped.response_value["created_user_action_request_refs"]
            .as_array()
            .expect("created request refs");
        assert_eq!(request_refs.len(), decisions.len(), "{label}");
        let stored_checkpoint = harness
            .store()?
            .current_shaping_checkpoint(&TaskId::new(&task_id))?
            .expect("current shaping checkpoint");
        assert_eq!(stored_checkpoint.gaps.len(), decisions.len(), "{label}");
        for (expected_kind, _) in &decisions {
            assert_eq!(
                stored_checkpoint
                    .gaps
                    .iter()
                    .filter(|gap| gap.gap_kind == *expected_kind)
                    .count(),
                1,
                "{label} exact gap kind"
            );
        }
        assert!(
            stored_checkpoint
                .gaps
                .iter()
                .all(|gap| gap.status == ShapingGapStatus::Current),
            "{label}"
        );
        if decisions.is_empty() {
            assert_eq!(
                shaped.response_value["workflow"]["kind"],
                "ready_for_implementation"
            );
        } else {
            assert_eq!(
                shaped.response_value["workflow"]["kind"],
                "awaiting_user_action"
            );
        }

        assert!(
            !product_repo_root(&harness)?.join("src").exists(),
            "{label}"
        );
        assert!(repository.status_bytes()?.is_empty(), "{label}");
        let before_early_write = harness.counts()?;
        let early_write = harness.service.prepare_write(
            prepare_write_request(
                &format!("req_{label}_early_write"),
                &format!("idem_{label}_early_write"),
                Some(before_early_write.state_version),
                Some(&task_id),
                Some(&change_unit_id),
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            early_write.response_value["base"]["response_kind"], "rejected",
            "{label} shaping cannot issue write authority"
        );
        assert_eq!(harness.counts()?, before_early_write, "{label}");
        assert!(repository.status_bytes()?.is_empty(), "{label}");

        if !decisions.is_empty() {
            let before_pending_replace = harness.counts()?;
            let pending_replace = harness.service.record_shaping(
                ready_shaping_request(
                    &format!("req_{label}_pending_replace"),
                    &format!("idem_{label}_pending_replace"),
                    before_pending_replace.state_version,
                    &task_id,
                    ShapingCheckpointOperation::ReplaceCurrent {
                        expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                        retired_user_action_request_refs: Vec::new(),
                    },
                    "Pending user authority remains attached to the current checkpoint.",
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                pending_replace.response_value["base"]["response_kind"], "rejected",
                "{label}"
            );
            assert_eq!(harness.counts()?, before_pending_replace, "{label}");
        }

        let mut resolved = Vec::new();
        for (index, ((gap_kind, _), request_ref)) in
            decisions.iter().zip(request_refs.iter()).enumerate()
        {
            let request_id = request_ref["record_id"].as_str().expect("request id");
            let before_chat = harness.counts()?;
            let chat_attempt = harness.service.resolve_user_action(
                resolve_user_action_request(
                    &format!("req_{label}_chat_{index}"),
                    &format!("chat_submission_{label}_{index}"),
                    None,
                    &task_id,
                    request_id,
                    "accept",
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                chat_attempt.response_value["base"]["response_kind"], "rejected",
                "{label} chat/agent acceptance has no resolution authority"
            );
            assert_eq!(harness.counts()?, before_chat, "{label}");
            assert_eq!(
                user_action_status(&harness, request_id)?,
                "pending",
                "{label}"
            );

            let response = harness.service.resolve_user_action(
                resolve_user_action_request(
                    &format!("req_{label}_resolve_{index}"),
                    &format!("submission_{label}_{index}"),
                    None,
                    &task_id,
                    request_id,
                    "accept",
                ),
                invocation(OperationCategory::UserOnly),
            )?;
            assert_verified_invocation(&response, OperationCategory::UserOnly);
            assert_eq!(
                response.response_value["base"]["response_kind"], "result",
                "{label}"
            );
            let resolution_ref: StateRecordRef = serde_json::from_value(
                response.response_value["user_action_resolution_ref"].clone(),
            )?;
            resolved.push((*gap_kind, resolution_ref));
            if index + 1 < decisions.len() {
                assert_eq!(
                    response.response_value["state"]["workflow"]["kind"], "awaiting_user_action",
                    "{label} partial resolution"
                );
            }
        }

        let has_scope = decisions
            .iter()
            .any(|(gap_kind, _)| *gap_kind == ShapingGapKind::UserScopeDecisionRequired);
        if !decisions.is_empty() {
            let status = harness.service.status(
                StatusRequest {
                    envelope: envelope(
                        &format!("req_{label}_resolved_status"),
                        None,
                        false,
                        None,
                        Some(&task_id),
                    ),
                    include: status_include(),
                    continuity_page: None,
                },
                invocation(OperationCategory::Read),
            )?;
            assert_eq!(
                status.response_value["active_task"]["workflow"]["kind"],
                if has_scope {
                    "ready_to_apply_decisions"
                } else {
                    "ready_for_implementation"
                },
                "{label} all resolved"
            );
            assert_eq!(
                status.response_value["active_task"]["workflow"]["required_action"],
                if has_scope {
                    "volicord.update_scope"
                } else {
                    "volicord.advance_task"
                },
                "{label} owner"
            );
            assert_eq!(
                status.response_value["active_task"]["workflow"]["checkpoint"]["readiness"],
                "ready",
                "{label} structural readiness"
            );

            let before_resolved_replace = harness.counts()?;
            let resolved_replace = harness.service.record_shaping(
                ready_shaping_request(
                    &format!("req_{label}_resolved_replace"),
                    &format!("idem_{label}_resolved_replace"),
                    before_resolved_replace.state_version,
                    &task_id,
                    ShapingCheckpointOperation::ReplaceCurrent {
                        expected_current_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                        retired_user_action_request_refs: Vec::new(),
                    },
                    "Resolved authority remains live until its semantic owner applies it.",
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                resolved_replace.response_value["base"]["response_kind"], "rejected",
                "{label}"
            );
            assert_eq!(harness.counts()?, before_resolved_replace, "{label}");
        }

        let mut scope_revision = 1;
        let scope_refs = resolved
            .iter()
            .filter(|(gap_kind, _)| *gap_kind == ShapingGapKind::UserScopeDecisionRequired)
            .map(|(_, resolution_ref)| resolution_ref.clone())
            .collect::<Vec<_>>();
        let advance_owned_refs = resolved
            .iter()
            .filter(|(gap_kind, _)| {
                gap_kind.decision_policy().is_some_and(|policy| {
                    policy.application_owner == ShapingDecisionApplicationOwner::AdvanceTask
                })
            })
            .map(|(_, resolution_ref)| resolution_ref.clone())
            .collect::<Vec<_>>();

        if !advance_owned_refs.is_empty() {
            let before_wrong_scope_owner = harness.counts()?;
            let mut wrong_scope_owner = update_scope_request(
                &format!("req_{label}_wrong_scope_owner"),
                &format!("idem_{label}_wrong_scope_owner"),
                false,
                Some(before_wrong_scope_owner.state_version),
                &task_id,
                ChangeUnitOperation::KeepCurrent,
                "A non-scope decision cannot be consumed by update_scope.",
            );
            wrong_scope_owner.related_scope_decision_refs = advance_owned_refs.clone();
            let rejected = harness.service.update_scope(
                wrong_scope_owner,
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                rejected.response_value["base"]["response_kind"], "rejected",
                "{label}"
            );
            assert_eq!(harness.counts()?, before_wrong_scope_owner, "{label}");
        }

        if has_scope {
            let before_wrong_advance_owner = harness.counts()?;
            let wrong_advance_owner = harness.service.advance_task(
                AdvanceTaskRequest {
                    envelope: envelope(
                        &format!("req_{label}_wrong_advance_owner"),
                        Some(&format!("idem_{label}_wrong_advance_owner")),
                        false,
                        Some(before_wrong_advance_owner.state_version),
                        Some(&task_id),
                    ),
                    task_id: TaskId::new(&task_id),
                    shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    change_unit_id: ChangeUnitId::new(&change_unit_id),
                    scope_revision,
                    baseline_ref: BaselineRef::new("baseline_test"),
                    user_action_resolution_ids: resolved
                        .iter()
                        .map(|(_, reference)| {
                            UserActionResolutionId::new(reference.record_id.as_str())
                        })
                        .collect(),
                },
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                wrong_advance_owner.response_value["base"]["response_kind"], "rejected",
                "{label}"
            );
            assert_eq!(harness.counts()?, before_wrong_advance_owner, "{label}");
        }

        if has_scope {
            let before = harness.counts()?;
            let mut update = update_scope_request(
                &format!("req_{label}_apply_scope"),
                &format!("idem_{label}_apply_scope"),
                false,
                Some(before.state_version),
                &task_id,
                ChangeUnitOperation::KeepCurrent,
                &format!("Apply the exact {label} scope decision."),
            );
            update.related_scope_decision_refs = scope_refs.clone();
            let applied = harness
                .service
                .update_scope(update, invocation(OperationCategory::AgentWorkflow))?;
            assert_eq!(
                applied.response_value["base"]["response_kind"], "result",
                "{label}"
            );
            assert_eq!(
                applied.response_value["applied_scope_decision_refs"],
                serde_json::to_value(&scope_refs)?,
                "{label} scope refs"
            );
            assert_eq!(
                applied.response_value["applied_shaping_gap_refs"]
                    .as_array()
                    .expect("applied scope gap refs")
                    .len(),
                1,
                "{label} exact scope gap"
            );
            assert_eq!(
                applied.response_value["state"]["workflow"]["kind"], "ready_for_implementation",
                "{label} no workflow loop"
            );
            scope_revision = 2;
            let checkpoint = harness
                .store()?
                .current_shaping_checkpoint(&TaskId::new(&task_id))?
                .expect("current checkpoint");
            for gap in checkpoint.gaps {
                let expected = if gap.gap_kind == ShapingGapKind::UserScopeDecisionRequired {
                    ShapingGapStatus::Applied
                } else {
                    ShapingGapStatus::Accepted
                };
                assert_eq!(gap.status, expected, "{label}: {:?}", gap.gap_kind);
            }
        }

        let advance_resolution_ids = advance_owned_refs
            .iter()
            .map(|resolution_ref| UserActionResolutionId::new(resolution_ref.record_id.as_str()))
            .collect::<Vec<_>>();
        if !advance_resolution_ids.is_empty() {
            let before = harness.counts()?;
            let missing = harness.service.advance_task(
                AdvanceTaskRequest {
                    envelope: envelope(
                        &format!("req_{label}_advance_missing"),
                        Some(&format!("idem_{label}_advance_missing")),
                        false,
                        Some(before.state_version),
                        Some(&task_id),
                    ),
                    task_id: TaskId::new(&task_id),
                    shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                    change_unit_id: ChangeUnitId::new(&change_unit_id),
                    scope_revision,
                    baseline_ref: BaselineRef::new("baseline_test"),
                    user_action_resolution_ids: Vec::new(),
                },
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                missing.response_value["base"]["response_kind"], "rejected",
                "{label}"
            );
            assert_eq!(harness.counts()?, before, "{label} atomic rejection");
            assert_eq!(
                harness
                    .store()?
                    .task_record(&TaskId::new(&task_id))?
                    .expect("Task")
                    .work_phase,
                WorkPhase::Shaping,
                "{label} remains shaping"
            );
        }

        let expected_state = harness.counts()?.state_version;
        let advance_request = AdvanceTaskRequest {
            envelope: envelope(
                &format!("req_{label}_advance"),
                Some(&format!("idem_{label}_advance")),
                false,
                Some(expected_state),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: advance_resolution_ids.clone(),
        };
        let advanced = harness.service.advance_task(
            advance_request.clone(),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            advanced.response_value["base"]["response_kind"], "result",
            "{label}"
        );
        assert_eq!(
            advanced.response_value["state"]["work_phase"], "implementation",
            "{label}"
        );
        assert_eq!(
            advanced.response_value["applied_user_action_resolution_refs"]
                .as_array()
                .expect("advance resolution refs")
                .len(),
            advance_resolution_ids.len(),
            "{label} exact advance refs"
        );
        let replay = harness.service.advance_task(
            advance_request,
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            replay.response_value, advanced.response_value,
            "{label} replay"
        );

        let before_second_advance = harness.counts()?;
        let second_advance = harness.service.advance_task(
            AdvanceTaskRequest {
                envelope: envelope(
                    &format!("req_{label}_second_advance"),
                    Some(&format!("idem_{label}_second_advance")),
                    false,
                    Some(before_second_advance.state_version),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                change_unit_id: ChangeUnitId::new(&change_unit_id),
                scope_revision,
                baseline_ref: BaselineRef::new("baseline_test"),
                user_action_resolution_ids: advance_resolution_ids,
            },
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            second_advance.response_value["base"]["response_kind"], "rejected",
            "{label} cannot advance twice"
        );
        assert_eq!(harness.counts()?, before_second_advance, "{label}");

        let applied_checkpoint = harness
            .store()?
            .current_shaping_checkpoint(&TaskId::new(&task_id))?
            .expect("applied current checkpoint");
        assert!(
            applied_checkpoint
                .gaps
                .iter()
                .all(|gap| gap.status == ShapingGapStatus::Applied),
            "{label} no decision may remain detached or ignored"
        );

        enable_record_run_capabilities(&harness)?;
        let before_ticket = harness.counts()?;
        let mut prepare = prepare_write_request(
            &format!("req_{label}_write"),
            &format!("idem_{label}_write"),
            Some(before_ticket.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let has_sensitive = decisions
            .iter()
            .any(|(gap_kind, _)| *gap_kind == ShapingGapKind::SensitiveApprovalRequired);
        if has_sensitive {
            prepare.sensitive_categories = vec!["network".to_owned()];
        }
        let ticket = harness
            .service
            .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(ticket.response_value["decision"], "allowed", "{label}");
        assert_eq!(
            harness.counts()?.write_tickets,
            before_ticket.write_tickets + 1,
            "{label}"
        );
        assert!(repository.status_bytes()?.is_empty(), "{label}");
        let write_ticket_id = response_record_id(&ticket.response_value, "write_ticket_ref");

        repository.write(
            "src/export.rs",
            format!("// authorized planning implementation for {label}\n").as_bytes(),
        )?;
        assert!(!repository.status_bytes()?.is_empty(), "{label}");
        let before_run = harness.counts()?;
        let mut run = product_write_record_run_request(
            &format!("req_{label}_run"),
            &format!("idem_{label}_run"),
            before_run.state_version,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            &format!("run_{label}"),
        );
        if has_sensitive {
            run.observed_changes.sensitive_categories = vec!["network".to_owned()];
        }
        let mut close_assessment = close_assessment_with_risks(
            "The authorized planning implementation is complete.",
            Vec::new(),
        );
        if has_sensitive {
            close_assessment.sensitive_categories = vec!["network".to_owned()];
        }
        run.close_assessment = Some(close_assessment).into();
        let recorded = harness
            .service
            .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(
            recorded.response_value["base"]["response_kind"], "result",
            "{label}: {}",
            recorded.response_value
        );
        assert_eq!(harness.counts()?.runs, before_run.runs + 1, "{label}");
        assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");

        let after_run = recorded.response_value["base"]["state_version"]
            .as_u64()
            .expect("recorded state version");
        let after_final =
            record_final_acceptance(&harness, &task_id, &change_unit_id, after_run, label)?;
        let closed = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: &format!("req_{label}_close"),
                idempotency_key: Some(&format!("idem_{label}_close")),
                dry_run: false,
                expected_state_version: Some(after_final),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(closed.response_value["close_state"], "closed", "{label}");
        assert_eq!(
            closed.response_value["authority_receipt"]["completion_claim_allowed"], true,
            "{label}"
        );
    }
    Ok(())
}

fn planning_only_repository_fixture(
    harness: &MethodHarness,
    label: &str,
) -> Result<IsolatedGitRepository, Box<dyn Error>> {
    let repository = IsolatedGitRepository::initialize_at(product_repo_root(harness)?)?;
    repository.write(
        "plans/product.md",
        b"# Product plan\n\nThe user owns product judgments.\n",
    )?;
    repository.write(
        "plans/technical.md",
        b"# Technical plan\n\nImplementation starts only after explicit authority.\n",
    )?;
    repository.commit_all("seed generic planning-only repository")?;
    record_guard_installation(harness, &format!("shaping_matrix_{label}"))?;
    assert!(repository.status_bytes()?.is_empty());
    assert!(!repository.root().join("src").exists());
    assert!(!repository.root().join("tests").exists());
    Ok(repository)
}

#[test]
fn product_and_technical_resolutions_need_no_scope_ref_before_change_unit_creation(
) -> Result<(), Box<dyn Error>> {
    for (label, decision) in [
        (
            "product_without_cu",
            (
                ShapingGapKind::UserProductDecisionRequired,
                JudgmentKind::ProductDecision,
            ),
        ),
        (
            "technical_without_cu",
            (
                ShapingGapKind::UserTechnicalDecisionRequired,
                JudgmentKind::TechnicalDecision,
            ),
        ),
    ] {
        let harness = MethodHarness::new()?;
        let intake = harness.service.intake(
            intake_request(
                &format!("req_{label}_task"),
                &format!("idem_{label}_task"),
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let task_id = response_record_id(&intake.response_value, "task_ref");
        let scoped = harness.service.update_scope(
            update_scope_request(
                &format!("req_{label}_scope"),
                &format!("idem_{label}_scope"),
                false,
                Some(1),
                &task_id,
                ChangeUnitOperation::KeepCurrent,
                "Current scope without a Change Unit.",
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert!(scoped.response_value["change_unit_ref"].is_null());
        let shaped = record_user_owned_gaps(&harness, label, &task_id, None, &[decision])?;
        let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
            .as_str()
            .expect("request id");
        let resolved = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!("req_{label}_resolve"),
                &format!("submission_{label}"),
                None,
                &task_id,
                request_id,
                "accept",
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        assert_eq!(
            resolved.response_value["state"]["workflow"]["kind"],
            "ready_for_change_unit"
        );
        assert_eq!(
            resolved.response_value["state"]["workflow"]["required_action"],
            "volicord.update_scope"
        );
        let before = harness.counts()?;
        let mut create_change_unit = update_scope_request(
            &format!("req_{label}_create_cu"),
            &format!("idem_{label}_create_cu"),
            false,
            Some(before.state_version),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Create the compatible current Change Unit.",
        );
        create_change_unit.goal_summary = RequiredNullable::null();
        create_change_unit.scope_update = RequiredNullable::null();
        create_change_unit.scope_boundary = RequiredNullable::null();
        create_change_unit.non_goals = RequiredNullable::null();
        create_change_unit.acceptance_criteria = RequiredNullable::null();
        create_change_unit.autonomy_boundary = RequiredNullable::null();
        create_change_unit.baseline_ref = RequiredNullable::null();
        let created = harness.service.update_scope(
            create_change_unit,
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(created.response_value["base"]["response_kind"], "result");
        assert_eq!(
            created.response_value["applied_scope_decision_refs"],
            json!([])
        );
        assert_eq!(
            created.response_value["applied_shaping_gap_refs"],
            json!([])
        );
        assert_eq!(
            created.response_value["state"]["workflow"]["kind"],
            "ready_for_implementation"
        );
    }
    Ok(())
}

#[test]
fn task_wide_advance_authority_blocks_a_gap_free_checkpoint_and_all_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = shaping_task(&harness, "task_wide")?;
    let mut action_request = user_action_request(
        "req_task_wide_action",
        "idem_task_wide_action",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    action_request.required_for = vec![UserActionRequiredFor::AdvanceTask];
    let requested = harness
        .service
        .request_user_action(action_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        requested.response_value["state"]["workflow"]["kind"],
        "awaiting_user_action"
    );
    let request_id = response_record_id(&requested.response_value, "user_action_request_ref");
    let shaped = harness.service.record_shaping(
        ready_shaping_request(
            "req_task_wide_shaping",
            "idem_task_wide_shaping",
            3,
            &task_id,
            ShapingCheckpointOperation::CreateInitial,
            "Gap-free checkpoint subject to Task-wide authority.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        shaped.response_value["workflow"]["kind"],
        "awaiting_user_action"
    );
    assert!(shaped.response_value["workflow"]["required_refs"]
        .as_array()
        .expect("workflow refs")
        .iter()
        .any(|record| record["record_id"] == request_id));

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_task_wide_status", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(
        status.response_value["active_task"]["workflow"]["kind"],
        "awaiting_user_action"
    );
    assert_eq!(
        status.response_value["active_task"]["workflow"]["blocking_reason"],
        "inconsistent_authority_state"
    );

    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let before = harness.counts()?;
    let rejected = harness.service.advance_task(
        AdvanceTaskRequest {
            envelope: envelope(
                "req_task_wide_advance",
                Some("idem_task_wide_advance"),
                false,
                Some(before.state_version),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
            change_unit_id: ChangeUnitId::new(&change_unit_id),
            scope_revision: 1,
            baseline_ref: BaselineRef::new("baseline_test"),
            user_action_resolution_ids: Vec::new(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(rejected.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "USER_DECISION_UNRESOLVED"
    );
    assert_eq!(
        rejected.response_value["errors"][0]["details"]["blockers"][0]["user_actions"][0]
            ["user_action_request_ref"]["record_id"],
        request_id
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(before.write_tickets, 0);
    assert_eq!(before.runs, 0);
    assert_eq!(
        harness
            .store()?
            .task_record(&TaskId::new(&task_id))?
            .expect("task")
            .work_phase,
        WorkPhase::Shaping
    );
    Ok(())
}

#[test]
fn task_wide_advisor_authority_blocks_gap_free_finalization_and_all_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "task_wide_advisor",
        RequestedMode::Advisor,
    )?;
    let mut action_request = user_action_request(
        "req_task_wide_advisor_action",
        "idem_task_wide_advisor_action",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    action_request.required_for = vec![UserActionRequiredFor::FinalizeAdvice];
    let requested = harness
        .service
        .request_user_action(action_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        requested.response_value["state"]["workflow"]["kind"],
        "shaping_required"
    );
    let request_id = response_record_id(&requested.response_value, "user_action_request_ref");
    let shaped = harness.service.record_shaping(
        ready_shaping_request(
            "req_task_wide_advisor_shaping",
            "idem_task_wide_advisor_shaping",
            3,
            &task_id,
            ShapingCheckpointOperation::CreateInitial,
            "Gap-free advisor checkpoint subject to Task-wide authority.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        shaped.response_value["workflow"]["kind"],
        "awaiting_user_action"
    );
    assert_eq!(
        shaped.response_value["workflow"]["blocking_reason"],
        "inconsistent_authority_state"
    );
    assert!(shaped.response_value["workflow"]["required_refs"]
        .as_array()
        .expect("workflow refs")
        .iter()
        .any(|record| record["record_id"] == request_id));

    let checkpoint_id = shaping_checkpoint_id(&shaped.response_value);
    let before = harness.counts()?;
    let rejected = harness.service.record_shaping(
        RecordShapingRequest {
            envelope: envelope(
                "req_task_wide_advisor_finalize",
                Some("idem_task_wide_advisor_finalize"),
                false,
                Some(before.state_version),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            operation: RecordShapingOperation::FinalizeAdvice {
                shaping_checkpoint_id: ShapingCheckpointId::new(&checkpoint_id),
                change_unit_id: ChangeUnitId::new(&change_unit_id),
                scope_revision: 1,
                baseline_ref: BaselineRef::new("baseline_test"),
                user_action_resolution_ids: Vec::new(),
                result_summary: "Detached authority must prevent advisor finalization.".to_owned(),
                result_refs: Vec::new(),
                evidence_refs: Vec::new(),
                residual_risks: Vec::new(),
                recovery_constraints: Vec::new(),
            },
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(rejected.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "USER_DECISION_UNRESOLVED"
    );
    assert_eq!(
        rejected.response_value["errors"][0]["details"]["workflow"]["blocking_reason"],
        "inconsistent_authority_state"
    );
    assert_eq!(harness.counts()?, before);
    assert!(task_revision(&harness, &task_id)?
        .current_close_basis
        .is_none());
    Ok(())
}

fn shaping_task(harness: &MethodHarness, label: &str) -> Result<(String, String), Box<dyn Error>> {
    let intake = harness.service.intake(
        intake_request(
            &format!("req_{label}_task"),
            &format!("idem_{label}_task"),
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(intake.response_value["state"]["work_phase"], "shaping");
    assert_eq!(
        intake.response_value["state"]["workflow"]["kind"],
        "shaping_required"
    );
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scoped = harness.service.update_scope(
        update_scope_request(
            &format!("req_{label}_scope"),
            &format!("idem_{label}_scope"),
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Current shaping authority test scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(scoped.response_value["state"]["work_phase"], "shaping");
    assert_eq!(
        scoped.response_value["state"]["workflow"]["kind"],
        "shaping_required"
    );
    Ok((
        task_id,
        response_record_id(&scoped.response_value, "change_unit_ref"),
    ))
}

fn shaping_checkpoint_id(response: &Value) -> String {
    response["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .unwrap_or_else(|| panic!("shaping checkpoint id: {response}"))
        .to_owned()
}

fn ready_shaping_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: u64,
    task_id: &str,
    checkpoint_operation: ShapingCheckpointOperation,
    summary: &str,
) -> RecordShapingRequest {
    RecordShapingRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            false,
            Some(expected_state_version),
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        operation: volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
            checkpoint_operation,
            scope_revision: 1,
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_test")),
            summary: summary.to_owned(),
            implementation_boundary: RequiredNullable::some(
                "Keep implementation inside the exact current scope.".to_owned(),
            ),
            gaps: Vec::new(),
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
        },
    }
}

fn record_user_owned_gap(
    harness: &MethodHarness,
    label: &str,
    task_id: &str,
    change_unit_id: &str,
    gap_kind: ShapingGapKind,
    judgment_kind: JudgmentKind,
) -> CoreResult<PipelineResponse> {
    let action = user_action_request(
        "unused",
        "unused",
        false,
        Some(2),
        task_id,
        Some(change_unit_id),
        judgment_kind,
    )
    .action;
    let mut request = ready_shaping_request(
        &format!("req_{label}_shaping"),
        &format!("idem_{label}_shaping"),
        2,
        task_id,
        ShapingCheckpointOperation::CreateInitial,
        "User-owned shaping authority is required.",
    );
    let gaps = vec![ShapingGapInput {
        gap_kind,
        summary: "The exact User Channel decision remains required.".to_owned(),
        affected_refs: Vec::new(),
        user_action: RequiredNullable::some(ShapingUserActionDraft {
            action,
            expires_at: RequiredNullable::null(),
        }),
    }];
    if let volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
        gaps: request_gaps,
        ..
    } = &mut request.operation
    {
        *request_gaps = gaps;
    }
    harness
        .service
        .record_shaping(request, invocation(OperationCategory::AgentWorkflow))
}

fn record_user_owned_gaps(
    harness: &MethodHarness,
    label: &str,
    task_id: &str,
    change_unit_id: Option<&str>,
    decisions: &[(ShapingGapKind, JudgmentKind)],
) -> CoreResult<PipelineResponse> {
    let mut request = ready_shaping_request(
        &format!("req_{label}_matrix_shaping"),
        &format!("idem_{label}_matrix_shaping"),
        2,
        task_id,
        ShapingCheckpointOperation::CreateInitial,
        "Each User Channel decision has one semantic application owner.",
    );
    let gaps = decisions
        .iter()
        .map(|(gap_kind, judgment_kind)| {
            let action = user_action_request(
                "unused",
                "unused",
                false,
                Some(2),
                task_id,
                change_unit_id,
                *judgment_kind,
            )
            .action;
            ShapingGapInput {
                gap_kind: *gap_kind,
                summary: format!("Apply {gap_kind:?} only through its semantic owner."),
                affected_refs: Vec::new(),
                user_action: RequiredNullable::some(ShapingUserActionDraft {
                    action,
                    expires_at: RequiredNullable::null(),
                }),
            }
        })
        .collect();
    if let volicord_types::methods::RecordShapingOperation::RecordCheckpoint {
        gaps: request_gaps,
        ..
    } = &mut request.operation
    {
        *request_gaps = gaps;
    }
    harness
        .service
        .record_shaping(request, invocation(OperationCategory::AgentWorkflow))
}
