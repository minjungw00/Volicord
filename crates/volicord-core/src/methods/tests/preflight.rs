use super::*;

#[test]
fn invalid_stored_method_owned_json_routes_to_structured_unavailability(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_method_json")?;
    let judgment = harness.service.request_user_action(
        user_action_request(
            "req_bad_method_json_judgment",
            "idem_bad_method_json_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    harness.conn()?.execute(
        "UPDATE user_action_requests
                SET request_json = '{not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id],
    )?;
    let before = harness.counts()?;

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_bad_method_json_record",
            "idem_bad_method_json_record",
            None,
            &task_id,
            &judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_action_requests",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn public_methods_use_same_verified_invocation_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "verified_context")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_verified_status", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_verified_invocation(&status, OperationCategory::Read);

    let intake = harness.service.intake(
        intake_request(
            "req_verified_intake",
            "idem_verified_intake",
            true,
            Some(2),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&intake, OperationCategory::AgentWorkflow);

    let update_scope = harness.service.update_scope(
        update_scope_request(
            "req_verified_scope",
            "idem_verified_scope",
            true,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Initial current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&update_scope, OperationCategory::AgentWorkflow);

    let mut prepare_write = prepare_write_request(
        "req_verified_prepare",
        "idem_verified_prepare",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare_write.envelope.dry_run = true;
    let prepare_write = harness
        .service
        .prepare_write(prepare_write, invocation(OperationCategory::AgentWorkflow))?;
    assert_verified_invocation(&prepare_write, OperationCategory::AgentWorkflow);

    let stage_artifact = harness.service.stage_artifact(
        stage_artifact_request(
            "req_verified_stage",
            Some("idem_verified_stage"),
            true,
            Some(2),
            &task_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&stage_artifact, OperationCategory::AgentWorkflow);

    let record_run = harness.service.record_run(
        record_run_request(
            "req_verified_run",
            "idem_verified_run",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&record_run, OperationCategory::AgentWorkflow);

    let request_action = harness.service.request_user_action(
        user_action_request(
            "req_verified_judgment_preview",
            "idem_verified_judgment_preview",
            true,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_verified_invocation(&request_action, OperationCategory::AgentWorkflow);

    let pending_judgment = harness.service.request_user_action(
        user_action_request(
            "req_verified_judgment_pending",
            "idem_verified_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_action_request_ref");
    let mut resolve_action = resolve_user_action_request(
        "req_verified_resolve_user_action",
        "idem_verified_resolve_user_action",
        None,
        &task_id,
        &pending_judgment_id,
        "accept",
    );
    resolve_action.envelope.dry_run = true;
    let resolved_action = harness
        .service
        .resolve_user_action(resolve_action, invocation(OperationCategory::UserOnly))?;
    assert_verified_invocation(&resolved_action, OperationCategory::UserOnly);

    let close_check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_verified_close",
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
    assert_verified_invocation(&close_check, OperationCategory::Read);

    Ok(())
}
