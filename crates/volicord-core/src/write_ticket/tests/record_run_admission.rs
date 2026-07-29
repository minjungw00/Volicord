use super::*;

#[test]
fn record_run_rejects_noncurrent_ticket_while_policy_control_reevaluation_is_pending(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_pending_policy_raise")?;
    let prepared = harness.service.prepare_write(
        prepare_write_request(
            "req_run_pending_policy_raise_prepare",
            "idem_run_pending_policy_raise_prepare",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");
    let marker_fingerprint = format!("sha256:{}", "c".repeat(64));
    let metadata_json = volicord_types::canonical::canonical_json_string(&json!({
        "policy_control_reevaluation": {
            "policy_version": 2,
            "policy_fingerprint": marker_fingerprint,
            "required_effective_control_level": "sensitive",
            "required_acceptance_policy": "required",
            "marked_at": "2026-06-18T00:00:00Z"
        }
    }))?;
    harness.conn()?.execute(
        "UPDATE tasks SET metadata_json = ?3 WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id, metadata_json],
    )?;
    let before = harness.counts()?;
    let request = product_write_record_run_request(
        "req_run_pending_policy_raise_record",
        "idem_run_pending_policy_raise_record",
        before.state_version,
        &task_id,
        &change_unit_id,
        &ticket_id,
        "run_pending_policy_raise",
    );

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_TICKET_INVALID"
    );
    assert_write_ticket_invalid_reason(&response, "approval_basis_changed");
    assert_eq!(harness.counts()?, before);
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "active");
    Ok(())
}

#[test]
fn non_product_category_signal_requires_final_without_manufacturing_sensitive_control(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "run_non_product_sensitive",
        RequestedMode::Direct,
    )?;
    let before = harness.counts()?;
    let mut request = record_run_request(
        "req_run_non_product_sensitive",
        "idem_run_non_product_sensitive",
        false,
        Some(before.state_version),
        &task_id,
        &change_unit_id,
    );
    request.kind = RunKind::Direct;
    request.observed_changes.sensitive_categories =
        vec!["network".to_owned(), "secret_access".to_owned()];
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "The non-product sensitive effects were recorded.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["run_summary"]["observed_changes"]["product_file_write_observed"],
        false
    );
    assert_eq!(
        response.response_value["run_summary"]["observed_changes"]["sensitive_categories"],
        json!(["network", "secret_access"])
    );
    assert_eq!(
        response.response_value["state"]["effective_control_level"],
        "light"
    );
    assert_eq!(
        response.response_value["state"]["acceptance_policy"],
        "required"
    );
    assert_close_blocker(
        &response.response_value["state"],
        "missing_final_acceptance",
    );
    assert_eq!(
        response.response_value["current_close_basis"]["sensitive_action_requirements"],
        json!([])
    );

    let after = harness.counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(
        after.user_action_requests, before.user_action_requests,
        "observed categories must not manufacture sensitive-approval authority"
    );
    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let stored_task = store
        .task_record(&TaskId::new(&task_id))?
        .expect("recorded Run Task remains current");
    assert_eq!(stored_task.effective_control_level, TaskControlLevel::Light);
    assert_eq!(stored_task.acceptance_policy, AcceptancePolicy::Required);
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_run_non_product_sensitive_status",
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
    assert_record_run_close_projection_matches_status(
        &response.response_value,
        &status.response_value,
    );
    Ok(())
}

#[test]
fn explicit_sensitive_non_product_run_requires_and_preserves_exact_approval_basis(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-07-16T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;

    let mut intake = intake_request(
        "req_sensitive_non_product_task",
        "idem_sensitive_non_product_task",
        false,
        Some(0),
        RequestedMode::Direct,
    );
    intake.requested_control_level = RequestedControlLevel::Sensitive;
    let intake = harness
        .service
        .intake(intake, invocation(OperationCategory::AgentWorkflow))?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let scope = harness.service.update_scope(
        update_scope_request(
            "req_sensitive_non_product_scope",
            "idem_sensitive_non_product_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bound one non-product sensitive action.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = response_record_id(&scope.response_value, "change_unit_ref");

    let before_unauthorized = harness.counts()?;
    let mut unauthorized = record_run_request(
        "req_sensitive_non_product_without_ticket",
        "idem_sensitive_non_product_without_ticket",
        false,
        Some(before_unauthorized.state_version),
        &task_id,
        &change_unit_id,
    );
    unauthorized.kind = RunKind::Direct;
    let rejected = harness
        .service
        .record_run(unauthorized, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "WRITE_TICKET_REQUIRED"
    );
    assert_eq!(harness.counts()?, before_unauthorized);
    let basisless_close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_sensitive_non_product_basisless_close",
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
    assert_close_blocker(
        &basisless_close.response_value,
        "missing_sensitive_action_basis",
    );

    let expires_at = UtcTimestamp::parse("2026-07-16T00:05:00Z")?;
    let mut approval_request = user_action_request(
        "req_sensitive_non_product_approval",
        "idem_sensitive_non_product_approval",
        false,
        Some(before_unauthorized.state_version),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::SensitiveApproval,
    );
    approval_request.expires_at = Some(expires_at.clone()).into();
    let volicord_types::schema::UserActionDraft::Choice(choice) = &mut approval_request.action
    else {
        unreachable!("sensitive approval fixture is choice-shaped")
    };
    choice.sensitive_action_scope = Some(sensitive_scope(
        "local_sensitive_step",
        Vec::new(),
        Vec::new(),
    ))
    .into();
    choice
        .sensitive_action_scope
        .as_mut()
        .expect("sensitive approval scope")
        .expires_at = Some(expires_at).into();
    let approval = harness.service.request_user_action(
        approval_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let approval_id = response_record_id(&approval.response_value, "user_action_request_ref");
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_sensitive_non_product_approval_resolve",
            "idem_sensitive_non_product_approval_resolve",
            None,
            &task_id,
            &approval_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let mut prepare = prepare_write_request(
        "req_sensitive_non_product_prepare",
        "idem_sensitive_non_product_prepare",
        Some(harness.counts()?.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.product_file_write_intended = false;
    prepare.intended_paths.clear();
    let prepared = harness
        .service
        .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(prepared.response_value["decision"], "allowed");
    let write_ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");

    for (suffix, performed_operation) in [
        ("missing", None),
        ("different", Some("another_sensitive_step".to_owned())),
    ] {
        let before_mismatch = harness.counts()?;
        let mut mismatch = record_run_request(
            &format!("req_sensitive_non_product_{suffix}_operation"),
            &format!("idem_sensitive_non_product_{suffix}_operation"),
            false,
            Some(before_mismatch.state_version),
            &task_id,
            &change_unit_id,
        );
        mismatch.kind = RunKind::Direct;
        mismatch.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
        mismatch.performed_operation = performed_operation.into();
        let rejected = harness
            .service
            .record_run(mismatch, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            rejected.response_value["errors"][0]["code"],
            "WRITE_TICKET_INVALID"
        );
        assert_eq!(
            rejected.response_value["errors"][0]["details"]["write_ticket_reason"],
            "operation_mismatch"
        );
        assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
        assert_eq!(harness.counts()?, before_mismatch);
    }

    let mut run = record_run_request(
        "req_sensitive_non_product_record",
        "idem_sensitive_non_product_record",
        false,
        Some(harness.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Direct;
    run.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
    run.performed_operation = Some("  local_sensitive_step  ".to_owned()).into();
    run.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "The exact non-product sensitive action was recorded.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let recorded = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        recorded.response_value["base"]["response_kind"], "result",
        "{:#}",
        recorded.response_value
    );
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
    let requirements = recorded.response_value["current_close_basis"]
        ["sensitive_action_requirements"]
        .as_array()
        .expect("sensitive action requirements");
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0]["action_kind"], "local_sensitive_step");
    assert_eq!(requirements[0]["normalized_paths"], json!([]));
    assert_eq!(requirements[0]["sensitive_categories"], json!([]));
    assert_no_close_blocker(
        &recorded.response_value["state"],
        "missing_sensitive_action_basis",
    );
    assert_no_close_blocker(
        &recorded.response_value["state"],
        "missing_sensitive_approval",
    );
    assert_close_blocker(
        &recorded.response_value["state"],
        "missing_final_acceptance",
    );

    clock.advance(Duration::minutes(6));
    let close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_sensitive_non_product_close_after_expiry",
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
    assert_no_close_blocker(&close.response_value, "missing_sensitive_action_basis");
    assert_close_blocker(&close.response_value, "missing_sensitive_approval");
    Ok(())
}

#[test]
fn record_run_product_write_omits_optional_operation_and_consumes_ticket_once(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_write")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_write")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_write",
        "idem_run_write",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
    request.performed_operation = None.into();
    request.evidence_updates = vec![supported_evidence_update(
        "Product write was reported with external tool output.",
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let run_id = run_id_from_record_run(&response.response_value);
    let observation_id = response.response_value["evidence_observations"][0]["observation_id"]
        .as_str()
        .expect("observation id should be present")
        .to_owned();
    let write_summary = &response.response_value["state"]["write_ticket_summary"];

    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
    assert_eq!(write_summary["status"], "consumed");
    assert_eq!(write_summary["consumed_by_run_ref"]["record_id"], run_id);
    assert_eq!(
        write_summary["observation_refs"][0]["record_kind"],
        "evidence_observation"
    );
    assert_eq!(
        write_summary["observation_refs"][0]["record_id"],
        observation_id
    );
    assert_eq!(
        write_summary["guarantee_display"]["capability_refs"][0]["record_kind"],
        "agent_connection"
    );
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_run_write_status", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let mut response_write_summary =
        response.response_value["state"]["write_ticket_summary"].clone();
    let status_write_summary = status.response_value["write_ticket_summary"].clone();
    response_write_summary["guarantee_display"] = status_write_summary["guarantee_display"].clone();
    assert_eq!(status_write_summary, response_write_summary);

    mutate_write_ticket_validity_basis_json(&harness, &write_ticket_id, |basis| {
        basis
            .as_object_mut()
            .expect("validity basis should be an object")
            .remove("write_authority_fingerprint");
    })?;
    let corrupt_status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_run_write_corrupt_consumed_status",
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
    assert_store_rejection(
        &corrupt_status,
        "PERSISTED_DATA_CORRUPT",
        "corrupt_stored_json",
    );
    assert_eq!(
        corrupt_status.response_value["base"]["effect_kind"],
        "no_effect"
    );
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn record_run_rejects_non_write_authority_after_workspace_change() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let intake = harness.service.intake(
        intake_request(
            "req_run_non_write_workspace_task",
            "idem_run_non_write_workspace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let original = crate::pipeline::GitWorkspaceContext {
        git_common_dir: "/tmp/volicord-run-non-write-workspace/.git".to_owned(),
        worktree_id: format!("sha256:{}", "9".repeat(64)),
        branch_ref: Some("refs/heads/original".to_owned()),
        head_sha: Some("9".repeat(40)),
        workspace_fingerprint: format!("sha256:{}", "a".repeat(64)),
    };
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_run_non_write_workspace_scope",
            "idem_run_non_write_workspace_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bind all Run authority to the original workspace.",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let before = harness.counts()?;
    let mut changed = original;
    changed.branch_ref = Some("refs/heads/other".to_owned());
    changed.head_sha = Some("b".repeat(40));
    changed.workspace_fingerprint = format!("sha256:{}", "c".repeat(64));
    let response = harness.service.record_run(
        record_run_request(
            "req_run_non_write_workspace_changed",
            "idem_run_non_write_workspace_changed",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(changed),
    )?;

    assert_eq!(
        response.response_value["errors"][0]["code"],
        "BASELINE_STALE"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["workspace_reason"],
        "workspace_context_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_consumes_write_ticket_at_fourteen_minutes_fifty_nine_seconds(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_1459")?;
    let id_generator =
        CountingDurableIdGenerator::new(["auth_1459", "prepare_event_1459", "record_event_1459"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator, clock.clone());
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_auth_1459")?;
    clock.advance(Duration::seconds(14 * 60 + 59));
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_1459",
            "idem_run_auth_1459",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_auth_1459",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.authority_events, before.authority_events + 1);
    Ok(())
}

#[test]
fn record_run_accepts_default_write_ticket_after_fifteen_minutes() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_1500")?;
    let id_generator =
        CountingDurableIdGenerator::new(["auth_1500", "prepare_event_1500", "record_event_1500"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator, clock.clone());
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_auth_1500")?;
    clock.advance(Duration::minutes(15));
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_1500",
            "idem_run_auth_1500",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_auth_1500",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
    assert_eq!(harness.counts()?.runs, before.runs + 1);
    Ok(())
}

#[test]
fn record_run_honors_configured_far_future_idle_timeout_without_fixed_cap(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_far_future")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_far_future_expiration",
        2,
        "2026-06-18T00:00:00.000Z",
        "2999-01-01T00:00:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(["record_event_far_future"]);
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_far_future",
            "idem_run_auth_far_future",
            2,
            &task_id,
            &change_unit_id,
            "wa_far_future_expiration",
            "run_auth_far_future",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?.runs, before.runs + 1);
    Ok(())
}

#[test]
fn record_run_honors_configured_idle_timeout_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_early_exp")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_early_expiration",
        2,
        "2026-06-18T00:00:00.000Z",
        "2026-06-18T00:05:00.000Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:05:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_early_exp",
            "idem_run_auth_early_exp",
            2,
            &task_id,
            &change_unit_id,
            "wa_early_expiration",
            "run_auth_early_exp",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_ticket_reason"],
        "idle_timeout"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_treats_invalid_write_ticket_timestamp_as_corrupt_state() -> Result<(), Box<dyn Error>>
{
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_bad_time")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_bad_timestamp",
        2,
        "2026-06-18T00:00:00.000Z",
        "not-a-timestamp",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_bad_time",
            "idem_run_auth_bad_time",
            2,
            &task_id,
            &change_unit_id,
            "wa_bad_timestamp",
            "run_auth_bad_time",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_store_rejection(&response, "PERSISTED_DATA_CORRUPT", "corrupt_stored_value");
    let details = &response.response_value["errors"][0]["details"]["owner_state_error"];
    assert_eq!(details["table"], "write_tickets");
    assert_eq!(details["record_ref"], "wa_bad_timestamp");
    assert_eq!(details["logical_column"], "idle_expires_at");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_ignores_unrelated_state_version_increment() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_auth_stale_exp")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_stale_and_expired",
        2,
        "2026-06-18T00:00:00.000Z",
        "2999-01-01T00:00:00.000Z",
    )?;
    let mut touch = update_scope_request(
        "req_run_auth_stale_exp_touch",
        "idem_run_auth_stale_exp_touch",
        false,
        Some(2),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Initial current scope.",
    );
    touch.acceptance_criteria = Some(vec![
        volicord_types::schema::AcceptanceCriterionReplacement {
            acceptance_criterion_id: Some(volicord_types::ids::AcceptanceCriterionId::new(
                active_acceptance_criterion_id(&harness, &task_id)?,
            ))
            .into(),
            statement: "The scoped behavior is represented.".to_owned(),
            evidence_requirement: EvidenceRequirement::NotRequired,
        },
    ])
    .into();
    harness
        .service
        .update_scope(touch, invocation(OperationCategory::AgentWorkflow))?;
    let id_generator = CountingDurableIdGenerator::new(["record_event_unrelated_state"]);
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_auth_stale_exp",
            "idem_run_auth_stale_exp",
            3,
            &task_id,
            &change_unit_id,
            "wa_stale_and_expired",
            "run_auth_stale_exp",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?.runs, before.runs + 1);
    Ok(())
}

#[test]
fn record_run_ticket_survives_unrelated_committed_scope_noop() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stale_auth")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_stale_auth")?;
    let mut touch = update_scope_request(
        "req_run_stale_auth_touch",
        "idem_run_stale_auth_touch",
        false,
        Some(3),
        &task_id,
        ChangeUnitOperation::KeepCurrent,
        "Initial current scope.",
    );
    touch.acceptance_criteria = Some(vec![
        volicord_types::schema::AcceptanceCriterionReplacement {
            acceptance_criterion_id: Some(volicord_types::ids::AcceptanceCriterionId::new(
                active_acceptance_criterion_id(&harness, &task_id)?,
            ))
            .into(),
            statement: "The scoped behavior is represented.".to_owned(),
            evidence_requirement: EvidenceRequirement::NotRequired,
        },
    ])
    .into();
    harness
        .service
        .update_scope(touch, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stale_auth",
        "idem_run_stale_auth",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
    assert_eq!(harness.counts()?.runs, before.runs + 1);
    Ok(())
}

#[test]
fn record_run_consumed_write_ticket_reuse_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_reuse_auth")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_reuse_auth")?;

    let mut first = record_run_request(
        "req_run_reuse_first",
        "idem_run_reuse_first",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    first.observed_changes.product_file_write_observed = true;
    first.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    first.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
    harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;

    let mut second = record_run_request(
        "req_run_reuse_second",
        "idem_run_reuse_second",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    second.observed_changes.product_file_write_observed = true;
    second.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    second.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
    let response = harness
        .service
        .record_run(second, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_TICKET_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_ticket_reason"],
        "consumed"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_path_mismatch_rejects_without_consuming_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_path_auth")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_path_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_path_auth",
        "idem_run_path_auth",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["tests/export.rs".to_owned()];
    request.write_ticket_id = Some(WriteTicketId::new(&write_ticket_id)).into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_TICKET_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["write_ticket_reason"],
        "path_mismatch"
    );
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_ticket_baseline_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_baseline_auth")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_baseline_auth")?;
    mutate_write_ticket_scope_json(&harness, &write_ticket_id, |scope| {
        scope["baseline_ref"] = json!("baseline_other");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_baseline_auth",
            "idem_run_baseline_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_baseline_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_ticket_invalid_reason(&response, "baseline_mismatch");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_missing_write_authority_binding_as_corrupt_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_missing_policy_binding")?;
    let write_ticket_id = prepare_write_ticket(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "run_missing_policy_binding",
    )?;
    mutate_write_ticket_validity_basis_json(&harness, &write_ticket_id, |basis| {
        basis
            .as_object_mut()
            .expect("validity basis should be an object")
            .remove("write_authority_fingerprint");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_missing_policy_binding",
            "idem_run_missing_policy_binding",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_missing_policy_binding",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_store_rejection(&response, "PERSISTED_DATA_CORRUPT", "corrupt_stored_json");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_missing_policy_binding",
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
    assert_store_rejection(&status, "PERSISTED_DATA_CORRUPT", "corrupt_stored_json");
    assert_eq!(status.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    Ok(())
}

#[test]
fn record_run_rejects_mismatched_write_authority_binding_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_mismatched_policy_binding")?;
    let write_ticket_id = prepare_write_ticket(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "run_mismatched_policy_binding",
    )?;
    mutate_write_ticket_validity_basis_json(&harness, &write_ticket_id, |basis| {
        basis["write_authority_fingerprint"] =
            json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_mismatched_policy_binding",
            "idem_run_mismatched_policy_binding",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_mismatched_policy_binding",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_ticket_invalid_reason(&response, "policy_authority_mismatch");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_ticket_task_mismatch_without_consumption() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_task_auth")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_task_auth")?;
    mutate_write_ticket_scope_json(&harness, &write_ticket_id, |scope| {
        scope["task_id"] = json!("task_other");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_task_auth",
            "idem_run_task_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_task_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "write_tickets",
        &write_ticket_id,
        "validity_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_ticket_change_unit_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_change_unit_auth")?;
    let write_ticket_id = prepare_write_ticket(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "run_change_unit_auth",
    )?;
    mutate_write_ticket_scope_json(&harness, &write_ticket_id, |scope| {
        scope["change_unit_id"] = json!("cu_other");
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_change_unit_auth",
            "idem_run_change_unit_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_change_unit_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "write_tickets",
        &write_ticket_id,
        "validity_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_ticket_product_write_flag_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_flag_auth")?;
    let write_ticket_id =
        prepare_write_ticket(&harness, &task_id, &change_unit_id, 2, "run_flag_auth")?;
    mutate_write_ticket_scope_json(&harness, &write_ticket_id, |scope| {
        scope["product_file_write_intended"] = json!(false);
    })?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_flag_auth",
            "idem_run_flag_auth",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_flag_auth",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_write_ticket_invalid_reason(&response, "product_write_flag_mismatch");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_write_ticket_sensitive_category_mismatch_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_sensitive_auth")?;
    insert_active_write_ticket_with_scope(
        &harness,
        WriteTicketScopeFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            write_ticket_id: "wa_sensitive_mismatch",
            basis_state_version: 2,
            created_at: "2999-01-01T00:00:00.000Z",
            expires_at: "2999-01-01T00:15:00.000Z",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            sensitive_categories: &["network"],
        },
    )?;
    enable_record_run_capabilities(&harness)?;
    let before = harness.counts()?;
    let mut request = product_write_record_run_request(
        "req_run_sensitive_auth",
        "idem_run_sensitive_auth",
        2,
        &task_id,
        &change_unit_id,
        "wa_sensitive_mismatch",
        "run_sensitive_auth",
    );
    request.observed_changes.sensitive_categories = vec!["credential".to_owned()];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_write_ticket_invalid_reason(&response, "sensitive_category_mismatch");
    assert_eq!(
        write_ticket_status(&harness, "wa_sensitive_mismatch")?,
        "active"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_expired_sensitive_approval_basis_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_expired_sensitive_basis")?;
    let mut approval_request = user_action_request(
        "req_run_expired_sensitive_basis_approval",
        "idem_run_expired_sensitive_basis_approval",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::SensitiveApproval,
    );
    let expires_at = UtcTimestamp::parse("2026-06-18T00:05:00Z")?;
    approval_request.expires_at = Some(expires_at.clone()).into();
    let volicord_types::schema::UserActionDraft::Choice(choice) = &mut approval_request.action
    else {
        unreachable!("sensitive approval fixture is choice-shaped")
    };
    choice
        .sensitive_action_scope
        .as_mut()
        .expect("sensitive approval scope")
        .expires_at = Some(expires_at).into();
    let approval = harness.service.request_user_action(
        approval_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let approval_id = response_record_id(&approval.response_value, "user_action_request_ref");
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_run_expired_sensitive_basis_resolve",
            "idem_run_expired_sensitive_basis_resolve",
            None,
            &task_id,
            &approval_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let mut prepare = prepare_write_request(
        "req_run_expired_sensitive_basis_prepare",
        "idem_run_expired_sensitive_basis_prepare",
        Some(4),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.sensitive_categories = vec!["network".to_owned()];
    let prepared = harness
        .service
        .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(prepared.response_value["decision"], "allowed");
    let write_ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");
    clock.advance(Duration::minutes(6));
    let before = harness.counts()?;

    let mut request = product_write_record_run_request(
        "req_run_expired_sensitive_basis_record",
        "idem_run_expired_sensitive_basis_record",
        before.state_version,
        &task_id,
        &change_unit_id,
        &write_ticket_id,
        "run_expired_sensitive_basis",
    );
    request.observed_changes.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_write_ticket_invalid_reason(&response, "approval_basis_changed");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_expired_sensitive_basis",
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
        status.response_value["write_ticket_summary"]["status"],
        "invalidated"
    );
    assert_eq!(
        status.response_value["write_ticket_summary"]["invalidation_reason"],
        "approval_basis_changed"
    );
    assert_no_close_blocker(&status.response_value, "open_write_ticket");
    Ok(())
}

#[test]
fn modified_persistent_artifact_body_blocks_existing_link_before_write_ticket(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "modified_existing")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "modified_existing",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let write_ticket_id = prepare_write_ticket(
        &harness,
        &task_id,
        &change_unit_id,
        state_version,
        "modified_existing",
    )?;
    let before = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, &artifact_id)?;
    let body_path = persistent_artifact_body_path(&harness, &artifact_id)?;
    fs::write(&body_path, b"{\"fixture\":\"changed_bytes\"}")?;

    let mut request = product_write_record_run_request(
        "req_run_modified_existing",
        "idem_run_modified_existing",
        state_version + 1,
        &task_id,
        &change_unit_id,
        &write_ticket_id,
        "run_modified_existing",
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_modified_existing",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "ARTIFACT_MISSING"
    );
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    assert_eq!(persistent_artifact_row(&harness, &artifact_id)?, before_row);
    assert_public_response_has_no_internal_leak(&response, &harness.runtime_home_path);
    Ok(())
}
