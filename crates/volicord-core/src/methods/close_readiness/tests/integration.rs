//! Close-readiness service and policy matrix coverage.

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
    assert_typed_result_contract::<CloseTaskResult>(&response);

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn advisor_check_close_never_recommends_prepare_write() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_mode_and_change_unit(
        &harness,
        "advisor_check_close",
        RequestedMode::Advisor,
    )?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_advisor_check_close",
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

    let blockers = response.response_value["blockers"]
        .as_array()
        .expect("blockers should be an array");
    assert!(blockers
        .iter()
        .all(|blocker| blocker["category"] != "write_compatibility"));
    assert!(blockers.iter().any(|blocker| {
        blocker["next_actions"].as_array().is_some_and(|actions| {
            actions
                .iter()
                .any(|action| action["action_kind"] == "record_run")
        })
    }));
    for blocker_group in [
        &response.response_value["blockers"],
        &response.response_value["state"]["close_blockers"],
    ] {
        for blocker in blocker_group
            .as_array()
            .expect("close blockers should be an array")
        {
            assert!(blocker["next_actions"]
                .as_array()
                .expect("next_actions should be an array")
                .iter()
                .all(|action| {
                    action["action_kind"] != "prepare_write"
                        && action["owner_method"] != "volicord.prepare_write"
                }));
        }
    }
    assert_ne!(
        response.response_value["summary_card"]["next_action"]["action_kind"],
        "prepare_write"
    );
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
            continuity_page: None,
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
fn stored_resolution_json_null_value_is_owner_state_corruption() -> Result<(), Box<dyn Error>> {
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
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let resolution_id = set_user_action_resolution_json(&harness, &judgment_id, Some("null"))?;
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
        "user_action_resolutions",
        &resolution_id,
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
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let resolution_id =
        set_user_action_resolution_json(&harness, &judgment_id, Some(corrupt_owner_json()))?;
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

    assert_owner_state_value_rejection(
        &response,
        "user_action_resolutions",
        &resolution_id,
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
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let corrupt_request_json = r#"{"presentation":17,"question":"must not leak secret-request-path","required_for":["close_complete"],"expires_at":null}"#;
    set_user_action_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_record_bad_request_type",
            "idem_record_bad_request_type",
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
    assert_public_response_omits(&response, "secret-request-path");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_action_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn request_user_action_rejects_expiration_at_clock_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_request_exact")?;
    let before = harness.counts()?;
    let mut request = user_action_request(
        "req_judgment_expiry_request_exact",
        "idem_judgment_expiry_request_exact",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::values::UtcTimestamp::parse(
        "2026-06-18T00:00:00Z",
    )?)
    .into();

    let response = harness
        .service
        .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;

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
fn resolve_user_action_uses_semantic_expiry_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_before")?;
    let mut request = user_action_request(
        "req_judgment_expiry_before",
        "idem_judgment_expiry_before",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::values::UtcTimestamp::parse(
        "2026-06-18T09:00:01+09:00",
    )?)
    .into();
    let judgment = harness
        .service
        .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_resolve_user_action_expiry_before",
            "idem_resolve_user_action_expiry_before",
            None,
            &task_id,
            &judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(user_action_status(&harness, &judgment_id)?, "resolved");

    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "judgment_expiry_exact")?;
    let mut request = user_action_request(
        "req_judgment_expiry_exact",
        "idem_judgment_expiry_exact",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.expires_at = Some(volicord_types::values::UtcTimestamp::parse(
        "2026-06-18T00:00:01Z",
    )?)
    .into();
    let judgment = harness
        .service
        .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    clock.advance(Duration::seconds(1));
    let before = harness.counts()?;

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_resolve_user_action_expiry_exact",
            "idem_resolve_user_action_expiry_exact",
            None,
            &task_id,
            &judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_action_status(&harness, &judgment_id)?, "expired");
    Ok(())
}

#[test]
fn stored_judgment_request_invalid_expiration_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bad_request_expiration")?;
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let corrupt_request_json = r#"{"presentation":"short","question":"must not leak secret-expiry-path","required_for":["close_complete"],"expires_at":"tomorrow"}"#;
    set_user_action_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_record_bad_request_expiration",
            "idem_record_bad_request_expiration",
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
    assert_public_response_omits(&response, "secret-expiry-path");
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_action_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn stored_judgment_request_missing_required_field_rejects_record_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_request_missing")?;
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let corrupt_request_json =
        r#"{"presentation":"short","required_for":["close_complete"],"expires_at":null}"#;
    set_user_action_owner_json(
        &harness,
        &judgment_id,
        "request_json",
        Some(corrupt_request_json),
    )?;
    let before = harness.counts()?;

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_record_bad_request_missing",
            "idem_record_bad_request_missing",
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
    assert_public_response_omits(&response, corrupt_request_json);
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_action_status(&harness, &judgment_id)?, "pending");
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
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let resolution_id = set_user_action_resolution_json(
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

    assert_owner_state_value_rejection(
        &response,
        "user_action_resolutions",
        &resolution_id,
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
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    set_user_action_owner_json(
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

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_record_bad_basis_revision",
            "idem_record_bad_basis_revision",
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
        "basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_action_status(&harness, &judgment_id)?, "pending");
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
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let resolution_id = set_user_action_resolution_json(
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

    assert_owner_state_value_rejection(
        &response,
        "user_action_resolutions",
        &resolution_id,
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
        produced_at_state_version: Some(state_version).into(),
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
    basis_request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
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
            continuity_page: None,
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
            continuity_page: None,
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
    assert_eq!(
        response.response_value["authority_receipt"]["completion_claim_allowed"],
        false
    );
    assert_eq!(
        response.response_value["authority_receipt"]["close_blockers"],
        response.response_value["blockers"]
    );
    let blocker = close_blocker_by_code(&response.response_value, "missing_final_acceptance");
    let action = &blocker["next_actions"][0];
    assert_eq!(action["presentation_role"], "primary");
    assert_eq!(action["action_kind"], "request_user_action");
    assert_eq!(action["owner_method"], "volicord.request_user_action");
    assert_eq!(
        action["allowed_operation_categories"],
        json!(["agent_workflow"])
    );
    assert_eq!(
        action["blocking_question"],
        "Does the user accept the current Task result and close basis as complete?"
    );
    assert!(action["label"]
        .as_str()
        .expect("final-acceptance action label should be text")
        .contains("Agent Connection"));
    assert_eq!(
        response.response_value["state"]["pending_user_action_summaries"],
        json!([])
    );
    assert_eq!(
        response.response_value["pending_user_action_summaries"],
        json!([])
    );
    assert!(response
        .response_value
        .get("pending_user_action_inbox_items")
        .is_none());
    assert_eq!(
        response.response_value["summary_card"]["next_action"],
        action.clone()
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_complete_blocks_only_relevant_pending_judgments() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_pending_kind")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "pending_kind", true)?;
    let mut product_request = user_action_request(
        "req_close_product_pending",
        "idem_close_product_pending",
        false,
        Some(after_evidence),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    product_request.required_for =
        vec![volicord_types::values::UserActionRequiredFor::CloseComplete];
    let requested = harness.service.request_user_action(
        product_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let requested_judgment_id =
        response_record_id(&requested.response_value, "user_action_request_ref");
    assert_eq!(
        requested.response_value["user_action_request_summary"],
        pending_user_action_summary(&requested_judgment_id)
    );
    assert!(requested.response_value.get("inbox_item").is_none());
    let facts = local_pending_user_action_facts(&harness, &task_id)?;
    let pending = facts
        .actions
        .iter()
        .find(|action| {
            action.request.user_action_request_id.as_str() == requested_judgment_id.as_str()
        })
        .expect("Core pending facts should retain the full semantic request");
    let UserActionRequestBody::Choice(choice) = &pending.request.body else {
        panic!("product decision should retain typed choice semantics");
    };
    assert_eq!(choice.options[0].option_id.as_str(), "accept");
    assert!(pending.resolution_availability.is_available());
    let mut prepare_write_request = user_action_request(
        "req_close_prepare_write_pending",
        "idem_close_prepare_write_pending",
        false,
        Some(after_evidence + 1),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    );
    prepare_write_request.required_for =
        vec![volicord_types::values::UserActionRequiredFor::PrepareWrite];
    let prepare_write_requested = harness.service.request_user_action(
        prepare_write_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let prepare_write_judgment_id = response_record_id(
        &prepare_write_requested.response_value,
        "user_action_request_ref",
    );
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence + 2,
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
    assert_close_blocker(&response.response_value, "pending_user_action");
    assert_close_blocker_category(
        &response.response_value,
        "pending_user_action",
        "pending_user_action",
    );
    let close_summaries = response.response_value["pending_user_action_summaries"]
        .as_array()
        .expect("close pending summaries should be an array");
    assert_eq!(close_summaries.len(), 1);
    assert_eq!(
        close_summaries[0],
        pending_user_action_summary(&requested_judgment_id)
    );
    assert!(
        close_summaries
            .iter()
            .all(|item| item["user_action_request_id"] != prepare_write_judgment_id),
        "prepare-write-only judgments must not enter the close summaries: {close_summaries:?}"
    );
    assert_eq!(
        response.response_value["state"]["pending_user_action_summaries"]
            .as_array()
            .expect("state pending judgment summaries should be an array")
            .len(),
        2
    );
    assert!(response
        .response_value
        .get("pending_user_action_inbox_items")
        .is_none());
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
    harness.service.request_user_action(
        user_action_request(
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
    assert_no_close_blocker(&response.response_value, "pending_user_action");
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
    let mut update = supported_evidence_update_with_provenance(
        "Close claim supported.",
        EvidenceSourceKind::UnverifiedClaim,
        EvidenceAssuranceLevel::Unverified,
    );
    let provenance = update
        .provenance
        .as_mut()
        .expect("test update has provenance");
    provenance.source_refs = vec![volicord_types::schema::SourceRef::UserContext(
        volicord_types::schema::UserContextSource {
            context_id: "message_unverified_claim".to_owned(),
        },
    )];
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "unverified_claim",
        vec![update],
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
    let (after_criteria, _) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        2,
        "missing_and_weak_evidence",
        &[
            ("Close claim supported.", EvidenceRequirement::Required),
            ("Missing close claim.", EvidenceRequirement::Required),
        ],
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        after_criteria,
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
fn user_observation_provenance_supports_the_attached_close_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_user_observation")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_observation",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "user_observation",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_user_observation",
            idempotency_key: Some("idem_close_user_observation"),
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
        response.response_value["evidence_gate"]["state"]
    );
    assert_eq!(
        response.response_value["evidence_gate"]["state"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["state"]["evidence_gate"],
        response.response_value["evidence_gate"]
    );
    assert_eq!(
        response.response_value["evidence_summary"]["evidence_state"],
        "accepted_for_close"
    );
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    assert_no_close_blocker(&response.response_value, "evidence_agent_report_only");
    Ok(())
}

#[test]
fn unanchored_external_tool_claim_is_downgraded_and_does_not_support_close(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_external_tool_unanchored")?;
    let criterion_id = volicord_types::ids::AcceptanceCriterionId::new(
        active_acceptance_criterion_id(&harness, &task_id)?,
    );
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let mut run = record_run_request(
        "req_close_external_tool_unanchored_run",
        "idem_close_external_tool_unanchored_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.evidence_updates = vec![evidence_update_for_acceptance_criterion(
        supported_evidence_update("Unanchored external claim."),
        &criterion_id,
    )];
    run.close_assessment = Some(close_assessment_with_risks(
        "Unanchored external claim.",
        Vec::new(),
    ))
    .into();
    let run_response = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        run_response.response_value["evidence_observations"][0]["source_kind"],
        "agent_report"
    );
    assert_eq!(
        run_response.response_value["evidence_observations"][0]["assurance_level"],
        "cooperative_report"
    );
    let observation_id = run_response.response_value["evidence_observations"][0]["observation_id"]
        .as_str()
        .expect("observation ID should be present");
    harness.conn()?.execute(
        "UPDATE evidence_observations
            SET source_kind = 'external_tool',
                assurance_level = 'external_tool_result'
          WHERE project_id = ?1
            AND evidence_observation_id = ?2",
        rusqlite::params![PROJECT_ID, observation_id],
    )?;
    let after_evidence = run_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "external_tool_unanchored",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_external_tool_unanchored",
            idempotency_key: Some("idem_close_external_tool_unanchored"),
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
    assert_no_close_blocker(&response.response_value, "evidence_agent_report_only");
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
    let (after_criteria, criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        2,
        "external_tool_scope",
        &[
            ("Close claim supported.", EvidenceRequirement::Required),
            ("Other claim supported.", EvidenceRequirement::Required),
        ],
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        after_criteria,
        "external_tool_scope",
        vec![evidence_update_for_acceptance_criterion(
            supported_evidence_update("Other claim supported."),
            &criteria[1].acceptance_criterion_id,
        )],
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
fn optional_not_required_and_supplemental_evidence_do_not_block_close() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_non_authoritative_evidence")?;
    let (after_criteria, criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        2,
        "non_authoritative_evidence",
        &[
            ("Optional criterion.", EvidenceRequirement::Optional),
            (
                "Criterion that requires no evidence.",
                EvidenceRequirement::NotRequired,
            ),
        ],
    )?;
    let after_evidence = record_close_evidence_with_updates(
        &harness,
        &task_id,
        &change_unit_id,
        after_criteria,
        "non_authoritative_evidence",
        vec![
            evidence_update_for_acceptance_criterion(
                unsupported_evidence_update("Optional criterion."),
                &criteria[0].acceptance_criterion_id,
            ),
            evidence_update_for_acceptance_criterion(
                unsupported_evidence_update("Criterion that requires no evidence."),
                &criteria[1].acceptance_criterion_id,
            ),
            unsupported_evidence_update("Supplemental diagnostic claim."),
        ],
        "Only non-authoritative evidence targets are unsupported.",
    )?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_non_authoritative_evidence",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: Some(after_evidence),
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(OperationCategory::Read),
    )?;

    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_no_close_blocker(&response.response_value, "evidence_claim_missing");
    assert_no_close_blocker(&response.response_value, "evidence_claim_unsupported");
    assert_no_close_blocker(&response.response_value, "evidence_provenance_insufficient");
    Ok(())
}

#[test]
fn unanchored_user_observation_is_downgraded_and_does_not_support_close(
) -> Result<(), Box<dyn Error>> {
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
    assert_close_blocker(&response.response_value, "evidence_agent_report_only");
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
    coverage[0]["observation_refs"][0]["produced_at_state_version"] = json!(after_evidence - 1);
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
fn light_policy_dependent_acceptance_requires_final_only_when_risk_exists(
) -> Result<(), Box<dyn Error>> {
    let no_risk = MethodHarness::new()?;
    no_risk.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_policy_and_change_unit(
        &no_risk,
        "light_policy_dependent_no_risk",
        RequestedMode::Direct,
        Some(AcceptancePolicy::PolicyDependent),
    )?;
    let mut run = record_run_request(
        "req_light_policy_dependent_no_risk_run",
        "idem_light_policy_dependent_no_risk_run",
        false,
        Some(no_risk.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Direct;
    run.evidence_updates = vec![supported_evidence_update(
        "Policy-dependent advice evidence.",
    )];
    run.close_assessment = Some(close_assessment_with_risks(
        "Policy-dependent advice without residual risk.",
        Vec::new(),
    ))
    .into();
    no_risk
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    let check = no_risk.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_light_policy_dependent_no_risk_check",
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
    assert_eq!(check.response_value["close_state"], "ready");
    assert!(!close_blocker_codes(&check.response_value)
        .iter()
        .any(|code| code == "missing_final_acceptance"));

    let with_risk = MethodHarness::new()?;
    with_risk.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_policy_and_change_unit(
        &with_risk,
        "light_policy_dependent_with_risk",
        RequestedMode::Direct,
        Some(AcceptancePolicy::PolicyDependent),
    )?;
    let mut run = record_run_request(
        "req_light_policy_dependent_with_risk_run",
        "idem_light_policy_dependent_with_risk_run",
        false,
        Some(with_risk.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Direct;
    run.evidence_updates = vec![supported_evidence_update("Risk-bearing advice evidence.")];
    run.close_assessment = Some(close_assessment_with_risks(
        "Policy-dependent advice with residual risk.",
        vec![residual_risk_input("The advice retains a user-owned risk.")],
    ))
    .into();
    with_risk
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    let check = with_risk.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_light_policy_dependent_with_risk_check",
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
    assert_close_blocker(&check.response_value, "missing_final_acceptance");
    assert_close_blocker(&check.response_value, "missing_residual_risk_acceptance");
    Ok(())
}

#[test]
fn close_uses_preserved_sensitive_policy_raise_after_policy_relaxation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_policy_and_change_unit(
        &harness,
        "close_preserved_policy_raise",
        RequestedMode::Direct,
        Some(AcceptancePolicy::PolicyDependent),
    )?;
    let mut run = record_run_request(
        "req_close_preserved_policy_raise_run",
        "idem_close_preserved_policy_raise_run",
        false,
        Some(harness.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Direct;
    run.evidence_updates = vec![supported_evidence_update(
        "Evidence recorded before the policy strengthening.",
    )];
    run.close_assessment = Some(close_assessment_with_risks(
        "The Light task has no residual risk.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;

    let mut sensitive_policy = light_workflow_policy();
    sensitive_policy["default_direct_control"] = json!("sensitive");
    harness.set_workflow_policy_version(2, sensitive_policy)?;
    harness.set_workflow_policy_version(3, light_workflow_policy())?;

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_preserved_policy_raise_check",
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
        check.response_value["state"]["effective_control_level"],
        "sensitive"
    );
    assert_eq!(
        check.response_value["state"]["acceptance_policy"],
        "required"
    );
    assert_close_blocker(&check.response_value, "missing_sensitive_action_basis");
    assert_close_blocker(&check.response_value, "missing_final_acceptance");
    Ok(())
}

#[test]
fn light_not_required_acceptance_still_fails_closed_on_current_risk() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let mut policy = light_workflow_policy();
    policy["light"]["final_acceptance"] = json!("not_required");
    harness.set_workflow_policy(policy)?;
    let (task_id, change_unit_id) = create_task_with_policy_and_change_unit(
        &harness,
        "light_not_required_with_risk",
        RequestedMode::Direct,
        Some(AcceptancePolicy::NotRequired),
    )?;
    let mut run = record_run_request(
        "req_light_not_required_with_risk_run",
        "idem_light_not_required_with_risk_run",
        false,
        Some(harness.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Direct;
    run.evidence_updates = vec![supported_evidence_update("Light risk evidence.")];
    run.close_assessment = Some(close_assessment_with_risks(
        "Light result retains a risk.",
        vec![residual_risk_input("A user-owned risk remains.")],
    ))
    .into();
    harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_light_not_required_with_risk_check",
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
    assert_close_blocker(&check.response_value, "missing_final_acceptance");
    assert_close_blocker(&check.response_value, "missing_residual_risk_acceptance");
    Ok(())
}

#[test]
fn light_non_write_sensitive_run_still_requires_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_policy_and_change_unit(
        &harness,
        "light_sensitive_non_write",
        RequestedMode::Direct,
        Some(AcceptancePolicy::PolicyDependent),
    )?;
    let mut run = record_run_request(
        "req_light_sensitive_non_write_run",
        "idem_light_sensitive_non_write_run",
        false,
        Some(harness.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Direct;
    run.observed_changes.sensitive_categories = vec!["network".to_owned(), "credential".to_owned()];
    run.evidence_updates = vec![supported_evidence_update(
        "Sensitive non-write run evidence.",
    )];
    run.close_assessment = Some(close_assessment_with_risks(
        "Sensitive non-write run completed.",
        Vec::new(),
    ))
    .into();
    let recorded = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_light_sensitive_non_write_check",
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
    assert_close_blocker(&check.response_value, "missing_final_acceptance");
    Ok(())
}

#[test]
fn light_policy_narrowing_after_write_requires_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_policy_and_change_unit(
        &harness,
        "light_policy_narrowed_write",
        RequestedMode::Direct,
        Some(AcceptancePolicy::PolicyDependent),
    )?;
    let prepared = harness.service.prepare_write(
        prepare_write_request(
            "req_light_policy_narrowed_prepare",
            "idem_light_policy_narrowed_prepare",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");
    let mut run = product_write_record_run_request(
        "req_light_policy_narrowed_run",
        "idem_light_policy_narrowed_run",
        harness.counts()?.state_version,
        &task_id,
        &change_unit_id,
        &ticket_id,
        "run_light_policy_narrowed",
    );
    run.evidence_updates = vec![supported_evidence_update("Narrowed-policy write evidence.")];
    run.close_assessment = Some(close_assessment_with_risks(
        "Write completed before policy narrowing.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    let mut narrowed = light_workflow_policy();
    narrowed["light"]["denied_path_patterns"] = json!(["src/export.rs"]);
    harness.set_workflow_policy_version(2, narrowed)?;

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_light_policy_narrowed_check",
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
    assert_close_blocker(&check.response_value, "missing_final_acceptance");
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
        user_action_resolution_outcome(&harness, &judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "rejected_cancellation_authority");
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
    assert_eq!(user_action_status(&harness, &judgment_id)?, "stale");
    assert_eq!(user_action_basis_status(&harness, &judgment_id)?, "stale");
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
    assert_close_blocker(&response.response_value, "stale_cancellation_authority");
    assert_eq!(harness.counts()?, before);
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
