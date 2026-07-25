use super::*;

#[test]
fn reused_request_id_does_not_collide_except_for_compatible_write_ticket_reuse(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request_id = "req_reused_for_generated_ids";

    let first_intake = harness.service.intake(
        intake_request(
            request_id,
            "idem_reused_intake_1",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_task_id = response_record_id(&first_intake.response_value, "task_ref");
    let first_event_id = response_event_id(&first_intake.response_value);

    let second_intake = harness.service.intake(
        intake_request(
            request_id,
            "idem_reused_intake_2",
            false,
            Some(1),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_task_id = response_record_id(&second_intake.response_value, "task_ref");
    let second_event_id = response_event_id(&second_intake.response_value);
    assert_ne!(first_task_id, second_task_id);
    assert_ne!(first_event_id, second_event_id);

    let first_scope = harness.service.update_scope(
        update_scope_request(
            request_id,
            "idem_reused_scope_1",
            false,
            Some(2),
            &second_task_id,
            ChangeUnitOperation::CreateCurrent,
            "First reused request scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_change_unit_id = response_record_id(&first_scope.response_value, "change_unit_ref");
    let first_scope_event_id = response_event_id(&first_scope.response_value);

    let second_scope = harness.service.update_scope(
        update_scope_request(
            request_id,
            "idem_reused_scope_2",
            false,
            Some(3),
            &second_task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Second reused request scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_change_unit_id = response_record_id(&second_scope.response_value, "change_unit_ref");
    let second_scope_event_id = response_event_id(&second_scope.response_value);
    assert_ne!(first_change_unit_id, second_change_unit_id);
    assert_ne!(first_scope_event_id, second_scope_event_id);

    let first_write = harness.service.prepare_write(
        prepare_write_request(
            request_id,
            "idem_reused_write_1",
            Some(4),
            Some(&second_task_id),
            Some(&second_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_write_id = response_record_id(&first_write.response_value, "write_ticket_ref");
    let first_write_event_id = response_event_id(&first_write.response_value);

    let second_write = harness.service.prepare_write(
        prepare_write_request(
            request_id,
            "idem_reused_write_2",
            Some(5),
            Some(&second_task_id),
            Some(&second_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_write_id = response_record_id(&second_write.response_value, "write_ticket_ref");
    let second_write_event_id = response_event_id(&second_write.response_value);
    assert_eq!(first_write_id, second_write_id);
    assert_eq!(second_write.response_value["write_ticket_effect"], "reused");
    assert_ne!(first_write_event_id, second_write_event_id);

    let first_judgment = harness.service.request_user_action(
        user_action_request(
            request_id,
            "idem_reused_judgment_1",
            false,
            Some(6),
            &second_task_id,
            Some(&second_change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_judgment_id =
        response_record_id(&first_judgment.response_value, "user_action_request_ref");
    let first_judgment_event_id = response_event_id(&first_judgment.response_value);

    let second_judgment = harness.service.request_user_action(
        user_action_request(
            request_id,
            "idem_reused_judgment_2",
            false,
            Some(7),
            &second_task_id,
            Some(&second_change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_judgment_id =
        response_record_id(&second_judgment.response_value, "user_action_request_ref");
    let second_judgment_event_id = response_event_id(&second_judgment.response_value);
    assert_ne!(first_judgment_id, second_judgment_id);
    assert_ne!(first_judgment_event_id, second_judgment_event_id);

    Ok(())
}

#[test]
fn reused_request_id_stage_artifact_returns_distinct_handles() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_reused_request")?;

    let first = harness.service.stage_artifact(
        stage_artifact_request("req_stage_reused", None, false, None, &task_id),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second = harness.service.stage_artifact(
        stage_artifact_request("req_stage_reused", None, false, None, &task_id),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let first_handle = first.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("first handle should be present");
    let second_handle = second.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("second handle should be present");
    assert_ne!(first_handle, second_handle);
    assert_eq!(harness.counts()?.artifact_staging, 2);
    Ok(())
}

#[test]
fn idempotent_replay_returns_original_generated_ids() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request = intake_request(
        "req_replay_generated_ids",
        "idem_replay_generated_ids",
        false,
        Some(0),
        RequestedMode::Work,
    );

    let first = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;

    assert!(second.replayed);
    assert_eq!(
        second
            .resolved_task_id
            .as_ref()
            .map(|task_id| task_id.as_str().to_owned()),
        Some(response_record_id(&first.response_value, "task_ref"))
    );
    assert_eq!(
        response_record_id(&first.response_value, "task_ref"),
        response_record_id(&second.response_value, "task_ref")
    );
    assert_eq!(
        response_event_id(&first.response_value),
        response_event_id(&second.response_value)
    );
    assert_eq!(harness.counts()?.tasks, 1);
    assert_eq!(harness.counts()?.authority_events, 1);
    Ok(())
}

#[test]
fn deterministic_generated_id_collision_retries_bounded_candidates() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    insert_superseding_task(&harness, "task_collision")?;
    harness.service.inner = CoreService::for_mutation_with_id_generator(
        &harness.service.context(),
        SequenceDurableIdGenerator::new(["collision", "fresh", "criterion", "event"]),
    );

    let response = harness.service.intake(
        intake_request(
            "req_collision_retry",
            "idem_collision_retry",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response_record_id(&response.response_value, "task_ref"),
        "task_fresh"
    );
    assert_eq!(response_event_id(&response.response_value), "evt_event");
    assert_eq!(harness.counts()?.tasks, 2);
    Ok(())
}
