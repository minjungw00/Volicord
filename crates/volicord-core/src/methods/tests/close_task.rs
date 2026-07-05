use super::*;

#[test]
fn check_close_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check")?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_check",
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

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn check_close_dry_run_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check_dry")?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_check_dry",
            idempotency_key: Some("idem_close_check_dry"),
            dry_run: true,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_does_not_use_terminal_summary_as_current_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "terminal_summary_not_basis")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(
            r#"{"close_reason":"none","visible_risks":[{"risk_id":"risk_summary_only","summary":"Terminal summary risk."}]}"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_terminal_summary_not_basis",
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

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert!(response.response_value["current_close_basis"].is_null());
    assert_close_blocker(&response.response_value, "missing_current_close_basis");
    assert_no_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_completion_policy_rejects_close_check_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_policy_check")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_bad_policy_check",
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

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "completion_policy_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_completion_policy_rejects_close_complete_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_policy_complete")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_policy_complete",
            idempotency_key: Some("idem_bad_policy_complete"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "completion_policy_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn schema_invalid_close_summary_rejects_instead_of_hiding_residual_risk(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_close_summary")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(r#"{"residual_risks":"known-but-wrong-shape"}"#),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_bad_close_summary",
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

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_summary_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_close_basis_stops_close_readiness_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_close_basis")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_bad_close_basis",
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

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_lifecycle_state_does_not_default_close_phase() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_lifecycle")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "lifecycle_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_bad_lifecycle",
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

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "lifecycle_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_write_basis_rejects_prepare_write_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_write_basis")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "write_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_write_basis",
            "idem_bad_write_basis",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "write_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_bounded_paths_rejects_prepare_write_without_empty_scope() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_paths")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "bounded_paths_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_paths",
            "idem_bad_paths",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "bounded_paths_json",
        &harness.runtime_home_path,
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_dry_run_with_corrupt_owner_state_is_rejected_no_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_bad_owner")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "write_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;
    let mut request = prepare_write_request(
        "req_dry_bad_owner",
        "idem_dry_bad_owner",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = true;

    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "write_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_read_only_rejects_corrupt_owner_state_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_bad_owner")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_bad_owner", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_summary_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn resolved_judgment_null_resolution_json_is_owner_state_corruption() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "null_resolution")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "null_resolution",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_null_resolution_judgment",
            "idem_null_resolution_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(&harness, &judgment_id, None)?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_null_resolution_close",
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

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_optional_resolution_json_rejects_close_readiness() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_resolution")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_resolution",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_resolution_judgment",
            "idem_bad_resolution_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(&harness, &judgment_id, Some(corrupt_owner_json()))?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_bad_resolution_close",
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

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_judgment_request_wrong_field_type_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_request_type")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_request_type",
            "idem_bad_request_type",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let corrupt_request_json = r#"{"presentation":17,"question":"must not leak secret-request-path","required_for":["close_complete"],"expires_at":null}"#;
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_request_type",
            "idem_record_bad_request_type",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, "secret-request-path");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn request_user_judgment_rejects_expiration_at_clock_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_request_exact")?;
    let before = harness.counts()?;
    let mut request = user_judgment_request(
        "req_judgment_expiry_request_exact",
        "idem_judgment_expiry_request_exact",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")?).into();

    let response = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "expires_at"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_user_judgment_uses_semantic_expiry_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_before")?;
    let mut request = user_judgment_request(
        "req_judgment_expiry_before",
        "idem_judgment_expiry_before",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::UtcTimestamp::parse(
        "2026-06-18T09:00:01+09:00",
    )?)
    .into();
    let judgment = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_judgment_expiry_before",
            "idem_record_judgment_expiry_before",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");

    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_exact")?;
    let mut request = user_judgment_request(
        "req_judgment_expiry_exact",
        "idem_judgment_expiry_exact",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::UtcTimestamp::parse("2026-06-18T00:00:01Z")?).into();
    let judgment = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    clock.advance(Duration::seconds(1));
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_judgment_expiry_exact",
            "idem_record_judgment_expiry_exact",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_request_invalid_expiration_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_request_expiration")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_request_expiration",
            "idem_bad_request_expiration",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let corrupt_request_json = r#"{"presentation":"short","question":"must not leak secret-expiry-path","required_for":["close_complete"],"expires_at":"tomorrow"}"#;
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_request_expiration",
            "idem_record_bad_request_expiration",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, "secret-expiry-path");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_request_missing_required_field_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_request_missing")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_request_missing",
            "idem_bad_request_missing",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let corrupt_request_json =
        r#"{"presentation":"short","required_for":["close_complete"],"expires_at":null}"#;
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_request_missing",
            "idem_record_bad_request_missing",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, corrupt_request_json);
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_resolution_incompatible_branches_rejects_close_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_resolution_branch")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_resolution_branch",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_resolution_branch_judgment",
            "idem_bad_resolution_branch_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(
        &harness,
        &judgment_id,
        Some(
            r#"{
                "selected_option_id":"accept",
                "machine_action":"accept",
                "resolution_outcome":"accepted",
                "answer":{
                    "product_decision":{"judgment":{"decision":"accepted"}},
                    "technical_decision":null,
                    "scope_decision":null,
                    "sensitive_action_scope":null,
                    "final_acceptance":{"judgment":{"decision":"accepted"}},
                    "residual_risk_acceptance":null,
                    "cancellation":null
                },
                "note":null,
                "accepted_risks":[],
                "resolved_by_actor_source":"user"
            }"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_bad_resolution_branch",
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

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_judgment_basis_invalid_revision_type_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_basis_revision")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_basis_revision",
            "idem_bad_basis_revision",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_owner_json(
        &harness,
        &judgment_id,
        "basis_json",
        Some(
            &json!({
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "scope_revision": "not-a-revision",
                "close_basis_revision": null,
                "baseline_ref": null,
                "result_refs": [],
                "residual_risk_ids": [],
                "sensitive_action_scope": null,
                "created_at_state_version": 3,
                "compatibility_status": "current"
            })
            .to_string(),
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bad_basis_revision",
            "idem_record_bad_basis_revision",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_accepted_risk_missing_risk_id_rejects_close_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_accepted_risk")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_accepted_risk",
        vec![residual_risk_input("Risk requiring explicit acceptance.")],
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_accepted_risk_judgment",
            "idem_bad_accepted_risk_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(
        &harness,
        &judgment_id,
        Some(
            &json!({
                "selected_option_id": "accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "answer": {
                    "product_decision": null,
                    "technical_decision": null,
                    "scope_decision": null,
                    "sensitive_action_scope": null,
                    "final_acceptance": null,
                    "residual_risk_acceptance": { "risk_ids": risk_ids },
                    "cancellation": null
                },
                "note": null,
                "accepted_risks": [{
                    "summary": "Risk accepted without a persisted risk_id.",
                    "consequence": "The missing risk identity must fail closed.",
                    "related_refs": [],
                    "accepted_for_close": true
                }],
                "resolved_by_actor_source": "user"
            })
            .to_string(),
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_bad_accepted_risk",
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

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_artifact_producer_json_rejects_existing_artifact_run_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_producer")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "bad_producer")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    set_artifact_owner_json(
        &harness,
        &artifact_id,
        "producer_json",
        corrupt_owner_json(),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_reuse_bad_producer",
        "idem_reuse_bad_producer",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_bad_producer",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_rejection(
        &response,
        "artifacts",
        &artifact_id,
        "producer_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn artifact_provenance_missing_source_ref_rejects_close_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_provenance")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "bad_provenance")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let artifact_state_ref = StateRecordRef {
        record_kind: StateRecordKind::Artifact,
        record_id: RecordId::new(&artifact_id),
        project_id: ProjectId::new(PROJECT_ID),
        task_id: Some(TaskId::new(&task_id)).into(),
        state_version: Some(state_version).into(),
    };
    let mut basis_request = record_run_request(
        "req_basis_bad_provenance",
        "idem_basis_bad_provenance",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    basis_request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_bad_provenance_basis",
        artifact_ref,
    )];
    basis_request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Close basis references the registered artifact.".to_owned(),
        result_refs: vec![artifact_state_ref],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let basis_response = harness
        .service
        .record_run(basis_request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        basis_response.response_value["base"]["response_kind"],
        "result"
    );
    clear_artifact_source_staging_handle(&harness, &artifact_id)?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_bad_provenance",
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

    assert_owner_state_value_rejection(
        &response,
        "artifacts",
        &artifact_id,
        "source_staging_handle_id",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_evidence_coverage_rejects_status_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_evidence_coverage")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence_coverage",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let corrupt_coverage_json =
        r#"{"claim":"secret-evidence-coverage-path","coverage_state":"supported"}"#;
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "coverage_json",
        corrupt_coverage_json,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_bad_evidence_coverage",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "evidence_summaries",
        &evidence_summary_id,
        "coverage_json",
        &harness.runtime_home_path,
    );
    assert_public_response_omits(&response, "secret-evidence-coverage-path");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_evidence_source_refs_rejects_close_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_evidence_refs")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence_refs",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "supporting_refs_json",
        r#"{"record_kind":"run"}"#,
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_bad_evidence_refs",
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

    assert_owner_state_rejection(
        &response,
        "evidence_summaries",
        &evidence_summary_id,
        "supporting_refs_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_evidence_metadata_rejects_status_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_evidence_metadata")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence_metadata",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "metadata_json",
        r#"{"updated_by_run_id":123}"#,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_bad_evidence_metadata",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "evidence_summaries",
        &evidence_summary_id,
        "metadata_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn display_only_staged_artifact_metadata_corruption_falls_back() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "display_only_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "display_only_artifact", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    set_artifact_staging_artifact_json(&harness, &handle_id, corrupt_owner_json())?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_display_only_artifact",
        "idem_display_only_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_display_only",
        handle,
        Some("display_only"),
        Some("Display-only artifact metadata may fall back."),
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["registered_artifacts"][0]["display_name"],
        handle_id
    );
    assert_public_response_omits(&response, corrupt_owner_json());
    assert_eq!(harness.counts()?.state_version, before.state_version + 1);
    Ok(())
}

#[test]
fn close_task_complete_blocks_missing_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_no_final")?;
    let state_version =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "no_final", true)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_no_final",
            idempotency_key: Some("idem_close_no_final"),
            dry_run: false,
            expected_state_version: Some(state_version),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_complete_blocks_only_relevant_pending_judgments() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_pending_kind")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "pending_kind", true)?;
    let mut product_request = user_judgment_request(
        "req_close_product_pending",
        "idem_close_product_pending",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for = vec![volicord_types::JudgmentRequiredFor::CloseComplete];
    let requested = harness.service.request_user_judgment(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let requested_judgment_id = response_record_id(&requested.response_value, "user_judgment_ref");
    assert_eq!(
        requested.response_value["inbox_item"]["judgment_id"],
        requested_judgment_id.as_str()
    );
    assert_eq!(
        requested.response_value["inbox_item"]["choices"][0]["choice_id"],
        "accept"
    );
    assert_eq!(
        requested.response_value["inbox_item"]["preferred_capture_path"]["kind"],
        "cli"
    );
    let requested_availability =
        &requested.response_value["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(requested_availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(requested_availability, "prompt_capture")["available"],
        false
    );
    assert_eq!(
        channel_path(requested_availability, "local_web_consent")["available"],
        false
    );
    assert_eq!(
        channel_path(requested_availability, "cli")["available"],
        true
    );
    assert!(requested.response_value["inbox_item"]["fallbacks"]
        .as_array()
        .expect("fallbacks should be an array")
        .is_empty());
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "pending_kind",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_product_pending_attempt",
            idempotency_key: Some("idem_close_product_pending_attempt"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "pending_user_judgment");
    assert_close_blocker_category(
        &response.response_value,
        "pending_user_judgment",
        "pending_user_judgment",
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_complete_ignores_pending_cancellation_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_ignore_cancel")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "ignore_cancel",
        true,
    )?;
    harness.service.request_user_judgment(
        user_judgment_request(
            "req_close_cancel_pending",
            "idem_close_cancel_pending",
            false,
            Some(after_evidence),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "ignore_cancel",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_ignore_cancel",
            idempotency_key: Some("idem_close_ignore_cancel"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_no_close_blocker(&response.response_value, "pending_user_judgment");
    Ok(())
}

#[test]
fn close_task_complete_blocks_unsupported_evidence_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_bad_evidence")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence",
        false,
    )?;
    let after_final =
        record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "bad")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_evidence",
            idempotency_key: Some("idem_close_bad_evidence"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_unsupported");
    assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn unverified_claim_alone_cannot_satisfy_close_readiness() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_unverified_claim")?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "unverified_claim",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
        )],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "unverified_claim",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_unverified_claim",
            idempotency_key: Some("idem_close_unverified_claim"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn missing_evidence_and_insufficient_provenance_are_distinct_blockers() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_missing_and_weak_evidence")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(
            r#"{"evidence_required":true,"required_claims":["Close claim supported.","Missing close claim."]}"#,
        ),
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_and_weak_evidence",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
        )],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "missing_and_weak_evidence",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_missing_and_weak_evidence",
            idempotency_key: Some("idem_close_missing_and_weak_evidence"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_missing");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_claim_missing",
        "evidence_claim",
    );
    assert_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_provenance_insufficient",
        "evidence_provenance",
    );
    Ok(())
}

#[test]
fn cooperative_agent_report_only_blocks_when_stronger_evidence_is_required(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_agent_report_only")?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "agent_report_only",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
        )],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "agent_report_only",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_agent_report_only",
            idempotency_key: Some("idem_close_agent_report_only"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_agent_report_only");
    Ok(())
}

#[test]
fn external_tool_provenance_supports_the_attached_close_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_external_tool")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "external_tool",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "external_tool",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_external_tool",
            idempotency_key: Some("idem_close_external_tool"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(
        response.response_value["summary_card"]["evidence"],
        "accepted_for_close"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["evidence_state"],
        "accepted_for_close"
    );
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_no_close_blocker(&response.response_value, "evidence_agent_report_only");
    assert_no_close_blocker(&response.response_value, "session_watch_unavailable");
    assert_eq!(
        response.response_value["coverage_summary"]["active_profile"],
        "record"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["session_watcher_state"],
        "unsupported"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["unresolved_unrecorded_change_count"],
        0
    );
    Ok(())
}

#[test]
fn supported_evidence_without_provenance_cannot_satisfy_close_readiness(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_missing_provenance")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_provenance",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "missing_provenance",
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let coverage_json: String = harness.conn()?.query_row(
        "SELECT coverage_json
           FROM evidence_summaries
          WHERE project_id = ?1
            AND evidence_summary_id = ?2",
        rusqlite::params![PROJECT_ID, evidence_summary_id],
        |row| row.get(0),
    )?;
    let mut coverage: Value = serde_json::from_str(&coverage_json)?;
    coverage[0]["observation_refs"] = json!([]);
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "coverage_json",
        &serde_json::to_string(&coverage)?,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_missing_provenance",
            idempotency_key: Some("idem_close_missing_provenance"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_provenance_insufficient",
        "evidence_provenance",
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn external_tool_evidence_does_not_support_unattached_close_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_external_tool_scope")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(
            r#"{"evidence_required":true,"required_claims":["Close claim supported.","Other claim supported."]}"#,
        ),
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "external_tool_scope",
        vec![supported_evidence_update("Other claim supported.")],
        "Close claim supported.",
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "external_tool_scope",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_external_tool_scope",
            idempotency_key: Some("idem_close_external_tool_scope"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_missing");
    assert_close_blocker_category(
        &response.response_value,
        "evidence_claim_missing",
        "evidence_claim",
    );
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn user_observation_evidence_does_not_replace_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_user_observation")?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_observation",
        vec![supported_evidence_update_with_provenance(
            "Close claim supported.",
            EvidenceSourceKind::UserObservation,
            EvidenceAssuranceLevel::UserObserved,
        )],
        "Close claim supported.",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_user_observation",
            idempotency_key: Some("idem_close_user_observation"),
            dry_run: false,
            expected_state_version: Some(after_evidence),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    Ok(())
}

#[test]
fn stale_evidence_provenance_is_not_current_close_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_stale_provenance")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "stale_provenance",
        true,
    )?;
    let evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let coverage_json: String = harness.conn()?.query_row(
        "SELECT coverage_json
           FROM evidence_summaries
          WHERE project_id = ?1
            AND evidence_summary_id = ?2",
        rusqlite::params![PROJECT_ID, evidence_summary_id],
        |row| row.get(0),
    )?;
    let mut coverage: Value = serde_json::from_str(&coverage_json)?;
    coverage[0]["observation_refs"][0]["state_version"] = json!(after_evidence - 1);
    set_evidence_summary_owner_json(
        &harness,
        &evidence_summary_id,
        "coverage_json",
        &serde_json::to_string(&coverage)?,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "stale_provenance",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_stale_provenance",
            idempotency_key: Some("idem_close_stale_provenance"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_provenance_stale");
    Ok(())
}

#[test]
fn close_task_complete_success() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_success")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "success", true)?;
    let after_final =
        record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "ok")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_success",
            idempotency_key: Some("idem_close_success"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_authority_disclosure(&response.response_value);
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(
        response.response_value["base"]["state_version"],
        after_final + 1
    );
    assert_eq!(fields.lifecycle_phase, "completed");
    assert_eq!(fields.result.as_deref(), Some("completed"));
    assert_eq!(
        fields.close_summary["close_reason"],
        "completed_self_checked"
    );
    assert!(fields.closed_at.is_some());
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn guarded_close_complete_success_reports_guard_health() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "guarded_success", "detective", "active", "{}")?;
    let session_id = "session_guarded_success";
    initialize_full_watch_baseline(
        &harness,
        session_id,
        &guard_installation_id,
        "guarded_success",
    )?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_close_success")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_success",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_success",
    )?;

    let before_status = harness.counts()?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_success",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(harness.counts()?, before_status);
    assert_eq!(
        status.response_value["guard_health"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_installation_id"],
        guard_installation_id
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_installation_status"],
        "active"
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]
            ["cooperative_pre_tool_warning_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]
            ["cooperative_pre_tool_denial_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]["actor_identity_provable"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["generated_config_verified"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["native_host_output_adapter_verified"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["post_tool_correlation_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["bash_shell_mutation_coverage"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["direct_file_write_matcher_coverage"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["bypass_detection_active"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["local_web_consent_available"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_hook_observed"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["last_guard_observed_at"],
        "2026-06-30T00:02:00Z"
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_status"],
        "observed"
    );
    assert_eq!(
        status.response_value["guard_health"]["mcp_connection_healthy"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_eq!(
        status.response_value["coverage_summary"]["active_profile"],
        "detective"
    );
    assert_eq!(
        status.response_value["coverage_summary"]["host_hook_state"],
        "observed"
    );
    assert_eq!(
        status.response_value["coverage_summary"]["session_watcher_state"],
        "active"
    );
    assert_eq!(
        status.response_value["coverage_summary"]["coverage_started_at"],
        "2026-06-30T00:03:00Z"
    );
    assert_eq!(
        status.response_value["coverage_summary"]["last_snapshot_at"],
        "2026-06-30T00:03:00Z"
    );
    assert_eq!(
        status.response_value["coverage_summary"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_coverage_non_guarantees(&status.response_value["coverage_summary"]);
    assert_eq!(
        status.response_value["active_task"]["guard_health"]["selected_profile"],
        "detective"
    );
    let local_web_status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_success_local_web",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read).with_local_web_consent_available(true),
    )?;
    assert_eq!(
        local_web_status.response_value["guard_health"]["local_web_consent_available"],
        true
    );

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_success",
            idempotency_key: Some("idem_close_guarded_success"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        response.response_value["guard_health"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_eq!(
        response.response_value["coverage_summary"]["active_profile"],
        "detective"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["host_hook_state"],
        "observed"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["session_watcher_state"],
        "active"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_coverage_non_guarantees(&response.response_value["coverage_summary"]);
    Ok(())
}

#[test]
fn host_hook_strength_requires_native_output_adapter() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let mut capability = complete_guard_capability_value(&harness)?;
    capability["native_host_output_adapter_verified"] = json!(false);
    record_guard_installation(
        &harness,
        "native_output_unverified",
        "detective",
        "active",
        &capability.to_string(),
    )?;
    let (task_id, _, _) = create_close_ready_task(&harness, "native_output_unverified")?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_native_output_unverified",
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
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["native_host_output_adapter_verified"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        false
    );
    Ok(())
}

#[test]
fn host_hook_strength_requires_generated_config_verification() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let capability = complete_guard_capability_value(&harness)?;
    let wrapper_path = capability["files"]
        .as_array()
        .and_then(|files| {
            files
                .iter()
                .find(|file| file["kind"] == "host_hook_wrapper")
        })
        .and_then(|file| file["path"].as_str())
        .map(PathBuf::from)
        .expect("complete capability should include a wrapper path");
    fs::remove_file(wrapper_path)?;
    record_guard_installation(
        &harness,
        "generated_config_missing",
        "detective",
        "active",
        &capability.to_string(),
    )?;
    let (task_id, _, _) = create_close_ready_task(&harness, "generated_config_missing")?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_generated_config_missing",
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
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["generated_config_verified"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        false
    );
    Ok(())
}

#[test]
fn host_hook_strength_requires_hook_path_safety_and_recovers() -> Result<(), Box<dyn Error>> {
    let suffix = "hook_path_unsafe_observe";
    let harness = MethodHarness::new()?;
    let mut capability = complete_guard_capability_value(&harness)?;
    capability["host_hook_commands"][0]["command"] =
        json!(".codex/hooks/volicord-dispatch.sh session-start");
    capability["host_hook_commands"][0]["cwd_independent"] = json!(false);
    capability["host_hook_commands"][0]["subdirectory_safe"] = json!(false);
    capability["hook_path_safety"]["overall_status"] = json!("relative_path_unsafe");
    capability["hook_path_safety"]["all_cwd_independent"] = json!(false);
    capability["hook_path_safety"]["all_subdirectory_safe"] = json!(false);
    record_guard_installation(
        &harness,
        suffix,
        "detective",
        "active",
        &capability.to_string(),
    )?;
    let (task_id, _, _) = create_close_ready_task(&harness, suffix)?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: &format!("req_check_{suffix}"),
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
        response.response_value["guard_health"]["hook_path_safety"],
        "relative_path_unsafe"
    );
    assert_eq!(
        response.response_value["guard_health"]["hook_commands_cwd_independent"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["hook_commands_subdirectory_safe"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["generated_config_verified"],
        false
    );

    let safe_capability = complete_guard_capability_value(&harness)?;
    record_guard_installation(
        &harness,
        suffix,
        "detective",
        "active",
        &safe_capability.to_string(),
    )?;
    let recovered = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: &format!("req_check_{suffix}_recovered"),
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
        recovered.response_value["guard_health"]["hook_path_safety"],
        "ok"
    );
    assert_eq!(
        recovered.response_value["guard_health"]["generated_config_verified"],
        true
    );
    assert_eq!(
        recovered.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        true
    );
    assert_eq!(
        recovered.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    Ok(())
}

#[test]
fn host_hook_strength_requires_shell_and_direct_write_matcher_coverage(
) -> Result<(), Box<dyn Error>> {
    for (suffix, field) in [
        ("shell_matcher_unavailable", "bash_shell_mutation_coverage"),
        (
            "direct_write_matcher_unavailable",
            "direct_file_write_matcher_coverage",
        ),
    ] {
        let harness = MethodHarness::new()?;
        let mut capability = complete_guard_capability_value(&harness)?;
        capability[field] = json!(false);
        record_guard_installation(
            &harness,
            suffix,
            "detective",
            "active",
            &capability.to_string(),
        )?;
        let (task_id, _, _) = create_close_ready_task(&harness, suffix)?;

        let response = harness.service.check_close(
            check_close_request(CloseTaskFixture {
                request_id: &format!("req_check_{suffix}"),
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
            response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
            false
        );
        assert_eq!(
            response.response_value["guard_health"]["control_surface"]["os_enforced"],
            false
        );
        assert_eq!(response.response_value["guard_health"][field], false);
        assert_eq!(
            response.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
            false
        );
    }
    Ok(())
}

#[test]
fn guarded_close_blocks_unhealthy_guard_installation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "guarded_unhealthy",
        "detective",
        "reload_required",
        "{}",
    )?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_unhealthy")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_unhealthy",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_unhealthy",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_unhealthy",
            idempotency_key: Some("idem_close_guarded_unhealthy"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_reload_required");
    assert_close_blocker_category(
        &response.response_value,
        "guard_reload_required",
        "connection_capability",
    );
    assert_close_blocker_resolution(
        &response.response_value,
        "guard_reload_required",
        false,
        true,
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_status"],
        "reload_required"
    );
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_close_blocks_configured_guard_before_observation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "guarded_not_observed",
        "detective",
        "configured",
        "{}",
    )?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_not_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_not_observed",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_not_observed",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_not_observed",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["close_state"], "blocked");
    assert_close_blocker(&status.response_value, "guard_not_observed");
    assert_eq!(
        status.response_value["guard_health"]["guard_installation_status"],
        "configured"
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["guard_hook_observed"],
        false
    );
    assert_eq!(
        status.response_value["guard_health"]["last_guard_observed_at"],
        Value::Null
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(
        status.response_value["guard_health"]["prompt_capture_status"],
        "configured"
    );
    assert_eq!(harness.counts()?, before);

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_not_observed",
            idempotency_key: Some("idem_close_guarded_not_observed"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_not_observed");
    let blocker = close_blocker_by_code(&response.response_value, "guard_not_observed");
    assert_eq!(blocker["control_surface"]["host_hooks_active"], false);
    assert_eq!(blocker["control_surface"]["os_enforced"], false);
    assert_close_blocker_resolution(&response.response_value, "guard_not_observed", false, true);
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_configured_guard_becomes_effectively_active_after_valid_observation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id = record_guard_installation(
        &harness,
        "guarded_configured_observed",
        "detective",
        "configured",
        "{}",
    )?;
    let observed = observe_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationObservation {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            observed_policy_hash: "sha256:guardedfixture".to_owned(),
            observed_binary_version: Some("0.0.0-test".to_owned()),
            observed_phase: "session_start".to_owned(),
            observed_at: "2026-06-30T00:03:00Z".to_owned(),
        },
    )?
    .expect("matching observation should record guard activation");
    assert_eq!(observed.installation_status, "active");
    let session_id = "session_guarded_configured_observed";
    initialize_full_watch_baseline(
        &harness,
        session_id,
        &guard_installation_id,
        "guarded_configured_observed",
    )?;

    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_configured_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_configured_observed",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_configured_observed",
    )?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_configured_observed",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::Read, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "ready");
    assert_no_close_blocker(&response.response_value, "guard_not_observed");
    assert_eq!(
        response.response_value["guard_health"]["guard_configuration_status"],
        "configured"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_observation_status"],
        "observed"
    );
    assert_eq!(
        response.response_value["guard_health"]["effective_guard_status"],
        "active"
    );
    Ok(())
}

#[test]
fn guarded_degraded_installation_with_valid_event_still_blocks_missing_required_hooks(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let degraded_capability = json!({
        "schema": "volicord-host-hook-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_hook_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": ["pre_tool_hook"],
        "prompt_capture": true
    })
    .to_string();
    let guard_installation_id = record_guard_installation(
        &harness,
        "guarded_degraded_observed",
        "detective",
        "degraded",
        &degraded_capability,
    )?;
    let observed = observe_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationObservation {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            observed_policy_hash: "sha256:guardedfixture".to_owned(),
            observed_binary_version: Some("0.0.0-test".to_owned()),
            observed_phase: "session_start".to_owned(),
            observed_at: "2026-06-30T00:03:00Z".to_owned(),
        },
    )?
    .expect("matching degraded observation should record metadata");
    assert_eq!(observed.installation_status, "degraded");
    assert_eq!(observed.last_seen_phase.as_deref(), Some("session_start"));

    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_degraded_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_degraded_observed",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_degraded_observed",
    )?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_degraded_observed",
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

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_required_hooks_missing");
    assert_no_close_blocker(&response.response_value, "guard_not_observed");
    let blocker = close_blocker_by_code(&response.response_value, "guard_required_hooks_missing");
    assert!(blocker["message"]
        .as_str()
        .is_some_and(|message| message.contains("pre_tool_hook") && message.contains("codex")));
    assert_eq!(
        response.response_value["guard_health"]["guard_configuration_status"],
        "degraded"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_observation_status"],
        "observed"
    );
    assert_eq!(
        response.response_value["guard_health"]["effective_guard_status"],
        "degraded"
    );
    assert_eq!(
        response.response_value["guard_health"]["missing_required_hook_phases"],
        json!(["pre_tool_hook"])
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        false
    );
    let blocker = close_blocker_by_code(&response.response_value, "guard_required_hooks_missing");
    assert_eq!(blocker["control_surface"]["host_hooks_active"], false);
    assert_eq!(blocker["control_surface"]["os_enforced"], false);
    Ok(())
}

#[test]
fn guarded_partial_required_phase_configuration_with_event_still_blocks_missing_required_hooks(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let partial_capability = json!({
        "schema": "volicord-host-hook-capability-v1",
        "policy_hash": "sha256:guardedfixture",
        "host_capabilities": {
            "user_prompt_submit_hook": true
        },
        "required_hook_phases": ["session_start_hook"],
        "missing_required_hooks": [],
        "prompt_capture": true
    })
    .to_string();
    let guard_installation_id = record_guard_installation(
        &harness,
        "guarded_partial_observed",
        "detective",
        "configured",
        &partial_capability,
    )?;
    let observed = observe_guard_installation(
        &harness.runtime_home_path,
        GuardInstallationObservation {
            guard_installation_id: guard_installation_id.clone(),
            connection_internal_id: CONNECTION_ID.to_owned(),
            project_id: PROJECT_ID.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            observed_policy_hash: "sha256:guardedfixture".to_owned(),
            observed_binary_version: Some("0.0.0-test".to_owned()),
            observed_phase: "session_start".to_owned(),
            observed_at: "2026-06-30T00:03:00Z".to_owned(),
        },
    )?
    .expect("matching partial observation should record metadata");
    assert_eq!(observed.installation_status, "configured");

    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_partial_observed")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_partial_observed",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_partial_observed",
    )?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_partial_observed",
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

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_required_hooks_missing");
    assert_eq!(
        response.response_value["guard_health"]["guard_configuration_status"],
        "degraded"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_observation_status"],
        "observed"
    );
    assert_eq!(
        response.response_value["guard_health"]["effective_guard_status"],
        "degraded"
    );
    let missing = response.response_value["guard_health"]["missing_required_hook_phases"]
        .as_array()
        .expect("missing required hook phases should be an array");
    assert_eq!(missing.len(), 4);
    assert!(missing.iter().any(|phase| phase == "pre_tool_hook"));
    assert!(missing.iter().all(|phase| phase != "session_start_hook"));
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    Ok(())
}

#[test]
fn guarded_close_blocks_missing_guard_installation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    insert_guarded_agent_session(&harness, "guarded_missing_install", "detective")?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_missing_install")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_missing_install",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_missing_install",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_missing_install",
            idempotency_key: Some("idem_close_guarded_missing_install"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "guard_not_installed");
    assert_close_blocker_category(
        &response.response_value,
        "guard_not_installed",
        "connection_capability",
    );
    assert_close_blocker_resolution(&response.response_value, "guard_not_installed", false, true);
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_status"],
        "absent"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_id"],
        Value::Null
    );
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_close_blocks_stale_broken_and_degraded_guard_status() -> Result<(), Box<dyn Error>> {
    for (status, code) in [
        ("stale", "guard_stale"),
        ("broken", "guard_broken"),
        ("degraded", "guard_degraded"),
    ] {
        let harness = MethodHarness::new()?;
        record_guard_installation(
            &harness,
            &format!("guarded_{status}"),
            "detective",
            status,
            "{}",
        )?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("guarded_{status}"))?;
        let after_evidence = record_close_evidence(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            &format!("guarded_{status}"),
            true,
        )?;
        let after_final = record_final_acceptance(
            &harness,
            &task_id,
            &change_unit_id,
            after_evidence,
            &format!("guarded_{status}"),
        )?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: &format!("req_close_guarded_{status}"),
                idempotency_key: Some(&format!("idem_close_guarded_{status}")),
                dry_run: false,
                expected_state_version: Some(after_final),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(OperationCategory::AgentWorkflow),
        )?;

        assert_eq!(response.response_value["close_state"], "blocked");
        assert_close_blocker(&response.response_value, code);
        assert_close_blocker_category(&response.response_value, code, "connection_capability");
        assert_close_blocker_resolution(&response.response_value, code, false, true);
        assert_eq!(
            response.response_value["guard_health"]["guard_installation_status"],
            status
        );
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(harness.counts()?, before);
    }
    Ok(())
}

#[test]
fn guarded_close_blocks_unresolved_unrecorded_changes_and_check_is_read_only(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "guarded_unrecorded", "detective", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_unrecorded")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_unrecorded",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_unrecorded",
    )?;
    insert_guarded_unrecorded_change(&harness, &task_id, "guarded_unrecorded")?;
    let before = harness.counts()?;

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_unrecorded",
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

    assert_eq!(check.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_close_blocker(&check.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        check.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    assert!(!check.response_json.contains("src/export.rs"));
    assert_eq!(harness.counts()?, before);

    let complete = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_unrecorded",
            idempotency_key: Some("idem_close_guarded_unrecorded"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(complete.response_value["close_state"], "blocked");
    assert_close_blocker(&complete.response_value, "unresolved_unrecorded_changes");
    assert_eq!(complete.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_close_blocks_write_ticket_issue_from_guard_event() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "guarded_write_ready", "detective", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_write_ready")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_write_ready",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "guarded_write_ready",
    )?;
    insert_write_ticket_guard_event(&harness, &guard_installation_id, "guarded_write_ready")?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_write_ready",
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

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value,
        "guard_write_ticket_missing_or_stale",
    );
    assert_eq!(
        response.response_value["guard_health"]["missing_or_stale_write_ticket"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["last_guard_event_at"],
        "2026-06-30T00:06:00Z"
    );
    assert_eq!(harness.counts()?, before);

    let complete = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_write_ready",
            idempotency_key: Some("idem_close_guarded_write_ready"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(complete.response_value["close_state"], "blocked");
    assert_eq!(complete.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_close_blocks_write_ticket_path_scope_guard_event() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "guarded_path_scope", "detective", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_path_scope")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_path_scope",
        true,
    )?;
    insert_write_ticket_path_scope_guard_event(&harness, &guard_installation_id, "path_scope")?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_path_scope",
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

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value,
        "guard_write_ticket_path_scope_violation",
    );
    assert_eq!(
        response.response_value["guard_health"]["write_ticket_path_scope_violation"],
        true
    );
    assert_eq!(
        close_blocker_by_code(
            &response.response_value,
            "guard_write_ticket_path_scope_violation"
        )["control_surface"]["os_enforced"],
        false
    );
    Ok(())
}

#[test]
fn close_check_blocks_open_and_expired_write_tickets() -> Result<(), Box<dyn Error>> {
    let mut open_harness = MethodHarness::new()?;
    open_harness.use_clock(ManualClock::at("2026-06-18T00:00:00Z"));
    let (open_task_id, open_change_unit_id) =
        create_task_with_change_unit(&open_harness, "open_ticket_close")?;
    open_harness.service.prepare_write(
        prepare_write_request(
            "req_open_ticket_prepare",
            "idem_open_ticket_prepare",
            Some(2),
            Some(&open_task_id),
            Some(&open_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let open = open_harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_open_ticket_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &open_task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(open.response_value["close_state"], "blocked");
    assert_close_blocker(&open.response_value, "open_write_ticket");

    let mut expired_harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    expired_harness.use_clock(clock.clone());
    let (expired_task_id, expired_change_unit_id) =
        create_task_with_change_unit(&expired_harness, "expired_ticket_close")?;
    expired_harness.service.prepare_write(
        prepare_write_request(
            "req_expired_ticket_prepare",
            "idem_expired_ticket_prepare",
            Some(2),
            Some(&expired_task_id),
            Some(&expired_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    clock.advance(Duration::minutes(15));

    let expired = expired_harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_expired_ticket_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &expired_task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(expired.response_value["close_state"], "blocked");
    assert_close_blocker(&expired.response_value, "expired_write_ticket");
    assert_no_close_blocker(&expired.response_value, "open_write_ticket");
    Ok(())
}

#[test]
fn guarded_pending_judgment_displays_user_answer_paths() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "guarded_pending", "detective", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "guarded_pending")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_pending",
        true,
    )?;
    let mut product_request = user_judgment_request(
        "req_guarded_pending_judgment",
        "idem_guarded_pending_judgment",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for = vec![volicord_types::JudgmentRequiredFor::CloseComplete];
    let requested = harness.service.request_user_judgment(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let requested_judgment_id = response_record_id(&requested.response_value, "user_judgment_ref");
    assert_eq!(
        requested.response_value["inbox_item"]["judgment_id"],
        requested_judgment_id.as_str()
    );
    assert_eq!(
        requested.response_value["inbox_item"]["choices"][0]["choice_id"],
        "accept"
    );
    assert_eq!(
        requested.response_value["inbox_item"]["preferred_capture_path"]["kind"],
        "prompt_capture"
    );
    assert!(requested.response_value["inbox_item"]["fallbacks"]
        .as_array()
        .expect("fallbacks should be an array")
        .iter()
        .any(|fallback| fallback["kind"] == "cli"));
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "guarded_pending",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_guarded_pending",
            idempotency_key: Some("idem_close_guarded_pending"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "pending_user_judgment");
    assert_eq!(
        response.response_value["pending_judgment_inbox_items"][0]["judgment_id"],
        requested_judgment_id.as_str()
    );
    assert_eq!(
        response.response_value["pending_judgment_inbox_items"][0]["requirement_status"],
        "required"
    );
    let close_item = &response.response_value["pending_judgment_inbox_items"][0];
    assert_eq!(
        close_item["preferred_capture_path"]["kind"],
        "prompt_capture"
    );
    assert!(close_item["fallbacks"]
        .as_array()
        .expect("close inbox fallbacks should be an array")
        .iter()
        .any(|fallback| fallback["kind"] == "cli"));
    let close_availability = &close_item["answer_path_availability"];
    assert_eq!(
        channel_path(close_availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(close_availability, "prompt_capture")["status"],
        "observed"
    );
    assert_eq!(
        channel_path(close_availability, "local_web_consent")["available"],
        false
    );
    assert_eq!(channel_path(close_availability, "cli")["available"], true);
    let pending = response.response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .find(|blocker| blocker["code"] == "pending_user_judgment")
        .expect("pending judgment blocker should be present");
    let guidance = pending["next_actions"][0]["blocking_question"]
        .as_str()
        .expect("pending blocker should include answer-path guidance");
    assert!(guidance.contains("chat command"), "{guidance}");
    assert!(!guidance.contains("host prompt"), "{guidance}");
    assert!(!guidance.contains("volicord user"), "{guidance}");
    assert_eq!(
        response.response_value["guard_health"]["prompt_capture_available"],
        true
    );

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_pending_paths",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let status_availability = &status.response_value["user_channel_availability"];
    assert_eq!(
        channel_path(status_availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(status_availability, "prompt_capture")["available"],
        true
    );
    assert_eq!(
        channel_path(status_availability, "local_web_consent")["available"],
        false
    );
    assert_eq!(channel_path(status_availability, "cli")["available"], true);
    assert_eq!(
        status.response_value["pending_judgment_inbox_items"][0]["preferred_capture_path"]["kind"],
        "prompt_capture"
    );

    let local_web_status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_pending_local_web_paths",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read).with_local_web_consent_available(true),
    )?;
    let local_web_availability = &local_web_status.response_value["user_channel_availability"];
    assert_eq!(
        channel_path(local_web_availability, "local_web_consent")["available"],
        true
    );
    assert!(
        local_web_status.response_value["pending_judgment_inbox_items"][0]["fallbacks"]
            .as_array()
            .expect("local web fallbacks should be an array")
            .iter()
            .any(|fallback| fallback["kind"] == "local_web_consent")
    );

    let host_elicitation_status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_pending_host_prompt_paths",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read).with_host_elicitation_available(true),
    )?;
    assert_eq!(
        channel_path(
            &host_elicitation_status.response_value["user_channel_availability"],
            "mcp_elicitation"
        )["available"],
        true
    );
    assert_eq!(
        host_elicitation_status.response_value["pending_judgment_inbox_items"][0]
            ["preferred_capture_path"]["kind"],
        "mcp_elicitation"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarded_pending_judgment_uses_prompt_capture_guidance_when_mcp_unhealthy(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    set_method_harness_connection_verification_status(&harness, VERIFIED_STATUS_FAILED)?;
    record_guard_installation(
        &harness,
        "guarded_pending_prompt_capture",
        "detective",
        "active",
        "{}",
    )?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "guarded_pending_prompt_capture")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "guarded_pending_prompt_capture",
        true,
    )?;
    let mut product_request = user_judgment_request(
        "req_guarded_pending_prompt_capture_judgment",
        "idem_guarded_pending_prompt_capture_judgment",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for = vec![volicord_types::JudgmentRequiredFor::CloseComplete];
    harness.service.request_user_judgment(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 1,
        "guarded_pending_prompt_capture",
    )?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_guarded_pending_prompt_capture",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["close_state"], "blocked");
    assert_pending_judgment_prompt_capture_guidance(&status.response_value);
    assert_eq!(
        status.response_value["pending_judgment_inbox_items"][0]["preferred_capture_path"]["kind"],
        "prompt_capture"
    );
    assert!(
        status.response_value["pending_judgment_inbox_items"][0]["fallbacks"]
            .as_array()
            .expect("status inbox fallbacks should be an array")
            .iter()
            .any(|fallback| fallback["kind"] == "cli")
    );

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_guarded_pending_prompt_capture",
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
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_pending_judgment_prompt_capture_guidance(&check.response_value);
    assert_eq!(
        check.response_value["pending_judgment_inbox_items"][0]["preferred_capture_path"]["kind"],
        "prompt_capture"
    );
    assert_eq!(
        check.response_value["guard_health"]["mcp_connection_healthy"],
        false
    );
    assert_eq!(
        check.response_value["guard_health"]["prompt_capture_available"],
        true
    );
    assert_eq!(
        check.response_value["guard_health"]["prompt_capture_status"],
        "observed"
    );
    Ok(())
}

#[test]
fn mcp_only_close_blocks_unresolved_unrecorded_change() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "mcp_only_unrecorded",
        "record",
        "configured",
        "{}",
    )?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "mcp_only_unrecorded")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "mcp_only_unrecorded",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "mcp_only_unrecorded",
    )?;
    insert_guarded_unrecorded_change(&harness, &task_id, "mcp_only_unrecorded")?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_mcp_only_unrecorded",
            idempotency_key: Some("idem_close_mcp_only_unrecorded"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_no_close_blocker(&response.response_value, "guard_not_observed");
    assert_eq!(
        response.response_value["guard_health"]["selected_profile"],
        "record"
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["bypass_detection_active"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_installation_status"],
        "configured"
    );
    assert_eq!(
        response.response_value["guard_health"]["guard_hook_observed"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    assert_eq!(
        response.response_value["coverage_summary"]["active_profile"],
        "record"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["host_hook_state"],
        "unsupported"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["session_watcher_state"],
        "unsupported"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["coverage_started_at"],
        Value::Null
    );
    assert_eq!(
        response.response_value["coverage_summary"]["unresolved_unrecorded_change_count"],
        1
    );
    assert_coverage_non_guarantees(&response.response_value["coverage_summary"]);
    assert_eq!(
        response.response_value["guard_health"]["prompt_capture_available"],
        false
    );
    Ok(())
}

#[test]
fn mcp_only_watcher_detects_bypass_file_changes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "watch_mcp_only", "record", "configured", "{}")?;
    let (task_id, _, after_final) = create_close_ready_task(&harness, "watch_mcp_only")?;
    let session_id = "session_watch_mcp_only";
    initialize_watch_baseline(&harness, &task_id, session_id, "mcp_only_seed")?;

    write_product_file(&harness, "src/watch.txt", "changed outside guard\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_mcp_only_detect",
            idempotency_key: Some("idem_watch_mcp_only_detect"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["selected_profile"],
        "record"
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_status"],
        "active"
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["session_watcher_active"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["unrecorded_changes_detectable"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(
        response.response_value["guard_health"]["bypass_detection_active"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_coverage_basis"],
        "method_boundary"
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_scan_summary"]
            ["not_full_filesystem_monitoring"],
        true
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_scan_summary"]["follows_symlinks"],
        false
    );
    assert_eq!(
        response.response_value["coverage_summary"]["active_profile"],
        "record"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["host_hook_state"],
        "unsupported"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["session_watcher_state"],
        "degraded"
    );
    assert_ne!(
        response.response_value["coverage_summary"]["coverage_started_at"],
        Value::Null
    );
    assert_eq!(
        response.response_value["coverage_summary"]["unresolved_unrecorded_change_count"],
        1
    );
    assert_eq!(
        response.response_value["coverage_summary"]["watcher_scan_summary"],
        response.response_value["guard_health"]["session_watch_scan_summary"]
    );
    assert_coverage_non_guarantees(&response.response_value["coverage_summary"]);
    assert!(
        response.response_value["guard_health"]["session_watch_partial_coverage_warning"]
            .as_str()
            .unwrap_or_default()
            .contains("method boundary")
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    let blocker = close_blocker_by_code(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(blocker["control_surface"]["session_watcher_active"], true);
    assert_eq!(
        blocker["control_surface"]["unrecorded_changes_detectable"],
        true
    );
    assert_eq!(blocker["control_surface"]["os_enforced"], false);
    let changes = unresolved_changes_for_connection(&harness)?;
    assert_eq!(changes.len(), 1);
    let detection: Value = serde_json::from_str(&changes[0].detection_json)?;
    assert_eq!(detection["source"], "volicord_session_watch");
    assert_eq!(detection["does_not_prevent_writes"], true);
    assert_eq!(detection["does_not_identify_actor"], true);
    assert!(!changes[0].detection_json.contains("changed outside guard"));
    Ok(())
}

#[test]
fn guarded_expected_write_does_not_create_duplicate_watcher_blocker() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let guard_installation_id =
        record_guard_installation(&harness, "watch_expected", "detective", "active", "{}")?;
    let (task_id, change_unit_id, after_final) =
        create_close_ready_task(&harness, "watch_expected")?;
    let session_id = "session_watch_expected";
    initialize_full_watch_baseline(
        &harness,
        session_id,
        &guard_installation_id,
        "expected_seed",
    )?;
    insert_expected_write_for_paths(
        &harness,
        &guard_installation_id,
        session_id,
        &task_id,
        &change_unit_id,
        "watch_expected",
        &["src/watch.txt"],
    )?;

    write_product_file(&harness, "src/watch.txt", "covered guarded write\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_expected_check",
            idempotency_key: Some("idem_watch_expected_check"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["session_watch_status"],
        "active"
    );
    assert!(unresolved_changes_for_connection(&harness)?.is_empty());
    Ok(())
}

#[test]
fn guarded_watcher_links_deterministic_active_write_ticket() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    harness.use_clock(ManualClock::at("2026-06-18T00:00:00Z"));
    let guard_installation_id =
        record_guard_installation(&harness, "watch_ticket", "detective", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "watch_ticket")?;
    let prepare = harness.service.prepare_write(
        prepare_write_request(
            "req_watch_ticket_prepare",
            "idem_watch_ticket_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_prepare = prepare.response_value["base"]["state_version"]
        .as_u64()
        .expect("prepare_write should report state version");
    let write_ticket_id = response_record_id(&prepare.response_value, "write_ticket_ref");
    let session_id = "session_watch_ticket";
    initialize_full_watch_baseline(&harness, session_id, &guard_installation_id, "ticket_seed")?;

    write_product_file(&harness, "src/export.rs", "ticket-backed watcher change\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_ticket_check",
            idempotency_key: Some("idem_watch_ticket_check"),
            dry_run: false,
            expected_state_version: Some(after_prepare),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "open_write_ticket");
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert!(unresolved_changes_for_connection(&harness)?.is_empty());
    let metadata: String = harness.conn()?.query_row(
        "SELECT metadata_json
           FROM session_watch_observations
          WHERE project_id = ?1
          ORDER BY observed_at DESC, watch_observation_id DESC
          LIMIT 1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    let metadata: Value = serde_json::from_str(&metadata)?;
    assert_eq!(metadata["correlation_status"], "write_ticket");
    assert_eq!(metadata["write_ticket_ids"], json!([write_ticket_id]));
    assert_eq!(metadata["does_not_prevent_writes"], true);
    assert_eq!(metadata["does_not_identify_actor"], true);
    Ok(())
}

#[test]
fn guarded_hook_missing_write_is_detected_by_watcher() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "watch_guarded_fallback",
        "detective",
        "active",
        "{}",
    )?;
    let (task_id, _, after_final) = create_close_ready_task(&harness, "watch_guarded_fallback")?;
    let session_id = "session_watch_guarded_fallback";
    initialize_watch_baseline(&harness, &task_id, session_id, "guarded_fallback_seed")?;

    write_product_file(&harness, "src/watch.txt", "guard hook skipped this write\n")?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_guarded_fallback",
            idempotency_key: Some("idem_watch_guarded_fallback"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "session_watch_unavailable");
    assert_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["active_profile"],
        "detective"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["host_hook_state"],
        "observed"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["session_watcher_state"],
        "degraded"
    );
    assert_eq!(
        response.response_value["coverage_summary"]["unresolved_unrecorded_change_count"],
        1
    );
    assert_eq!(unresolved_changes_for_connection(&harness)?.len(), 1);
    Ok(())
}

#[test]
fn watcher_reverted_change_auto_resolves() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "watch_revert", "record", "configured", "{}")?;
    let (task_id, _, after_final) = create_close_ready_task(&harness, "watch_revert")?;
    let session_id = "session_watch_revert";
    write_product_file(&harness, "src/watch.txt", "original\n")?;
    initialize_watch_baseline(&harness, &task_id, session_id, "revert_seed")?;
    write_product_file(&harness, "src/watch.txt", "changed\n")?;
    let blocked = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_revert_detect",
            idempotency_key: Some("idem_watch_revert_detect"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    assert_close_blocker(&blocked.response_value, "unresolved_unrecorded_changes");

    write_product_file(&harness, "src/watch.txt", "original\n")?;
    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_watch_revert_reconcile",
            "idem_watch_revert_reconcile",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "reverted"
    );
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    let changes = unresolved_changes_for_connection(&harness)?;
    assert!(changes.is_empty());
    Ok(())
}

#[test]
fn close_blocks_while_watcher_findings_remain_unresolved_and_unblocks_after_reconciliation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "watch_close_block", "record", "configured", "{}")?;
    let (task_id, _, after_final) = create_close_ready_task(&harness, "watch_close_block")?;
    let session_id = "session_watch_close_block";
    write_product_file(&harness, "src/watch.txt", "original\n")?;
    initialize_watch_baseline(&harness, &task_id, session_id, "close_block_seed")?;
    write_product_file(&harness, "src/watch.txt", "changed\n")?;

    let blocked = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_close_block",
            idempotency_key: Some("idem_watch_close_block"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    assert_eq!(blocked.response_value["close_state"], "blocked");
    assert_close_blocker(&blocked.response_value, "unresolved_unrecorded_changes");

    write_product_file(&harness, "src/watch.txt", "original\n")?;
    let reconciled = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_watch_close_block_reconcile",
            "idem_watch_close_block_reconcile",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    let after_reconcile = reconciled.response_value["base"]["state_version"]
        .as_u64()
        .expect("reconcile should report state version");
    assert_no_close_blocker(&reconciled.response_value, "unresolved_unrecorded_changes");

    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_watch_close_unblocked",
            idempotency_key: Some("idem_watch_close_unblocked"),
            dry_run: false,
            expected_state_version: Some(after_reconcile),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation_with_session(OperationCategory::AgentWorkflow, session_id),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    assert_no_close_blocker(&closed.response_value, "unresolved_unrecorded_changes");
    Ok(())
}

#[test]
fn close_task_cancel_success_despite_missing_completion_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_cancel")?;
    let (after_authority, _) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "close_cancel",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_cancel",
            idempotency_key: Some("idem_close_cancel"),
            dry_run: false,
            expected_state_version: Some(after_authority),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "cancelled");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(fields.lifecycle_phase, "cancelled");
    assert_eq!(fields.result.as_deref(), Some("cancelled"));
    assert_eq!(fields.close_summary["close_reason"], "cancelled");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_cancel_requires_current_user_cancellation_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "cancel_missing_authority")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_missing_authority",
            idempotency_key: Some("idem_cancel_missing_authority"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_cancellation_authority");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "ready"
    );
    Ok(())
}

#[test]
fn rejected_cancellation_authority_does_not_cancel_task() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cancel_rejected")?;
    let (after_rejection, judgment_id) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "cancel_rejected",
        false,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_rejected",
            idempotency_key: Some("idem_cancel_rejected"),
            dry_run: false,
            expected_state_version: Some(after_rejection),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        user_judgment_resolution_outcome(&harness, &judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "cancellation_rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_change_stales_cancellation_authority() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cancel_stale_scope")?;
    let (after_authority, judgment_id) =
        record_cancellation_authority(&harness, &task_id, &change_unit_id, 2, "stale_scope", true)?;
    let scope = harness.service.update_scope(
        update_scope_request(
            "req_cancel_stale_scope_update",
            "idem_cancel_stale_scope_update",
            false,
            Some(after_authority),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope after cancellation judgment.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_scope = scope.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "stale");
    assert_eq!(user_judgment_basis_status(&harness, &judgment_id)?, "stale");
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_stale_scope",
            idempotency_key: Some("idem_cancel_stale_scope"),
            dry_run: false,
            expected_state_version: Some(after_scope),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "cancellation_judgment_stale");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_supersede_success() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_supersede")?;
    let superseding_task_id = "task_close_superseding";
    insert_superseding_task(&harness, superseding_task_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_supersede",
            idempotency_key: Some("idem_close_supersede"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Supersede,
            close_reason: Some(CloseReason::Superseded),
            superseding_task_id: Some(superseding_task_id),
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "superseded");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(fields.lifecycle_phase, "superseded");
    assert_eq!(fields.result.as_deref(), Some("superseded"));
    assert_eq!(
        active_task_id(&harness)?.as_deref(),
        Some(superseding_task_id)
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_stale_state_rejected_without_blocker() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_stale")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_stale",
            idempotency_key: Some("idem_close_stale"),
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response.response_value.get("blockers").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_blocker_code_routing_uses_method_local_codes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_codes")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_codes",
            idempotency_key: Some("idem_close_codes"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
    assert!(response.response_value.get("errors").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_idempotency_replays_terminal_transition() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_replay")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "replay", true)?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "replay",
    )?;
    let request = close_task_request(CloseTaskFixture {
        request_id: "req_close_replay",
        idempotency_key: Some("idem_close_replay"),
        dry_run: false,
        expected_state_version: Some(after_final),
        task_id: &task_id,
        intent: CloseIntent::Complete,
        close_reason: Some(CloseReason::CompletedSelfChecked),
        superseding_task_id: None,
    });

    let first = harness.service.close_task(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .close_task(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(first.response_value["close_state"], "closed");
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}
