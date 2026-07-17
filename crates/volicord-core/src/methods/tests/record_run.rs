use super::*;

#[test]
fn task_mode_run_kind_matrix_is_enforced_before_commit() -> Result<(), Box<dyn Error>> {
    for (requested_mode, task_mode, run_kind, run_kind_value, allowed, suffix) in [
        (
            RequestedMode::Advisor,
            "advisor",
            RunKind::ShapingUpdate,
            "shaping_update",
            true,
            "advisor_shaping",
        ),
        (
            RequestedMode::Advisor,
            "advisor",
            RunKind::Implementation,
            "implementation",
            false,
            "advisor_implementation",
        ),
        (
            RequestedMode::Advisor,
            "advisor",
            RunKind::Direct,
            "direct",
            false,
            "advisor_direct",
        ),
        (
            RequestedMode::Direct,
            "direct",
            RunKind::ShapingUpdate,
            "shaping_update",
            false,
            "direct_shaping",
        ),
        (
            RequestedMode::Direct,
            "direct",
            RunKind::Implementation,
            "implementation",
            false,
            "direct_implementation",
        ),
        (
            RequestedMode::Direct,
            "direct",
            RunKind::Direct,
            "direct",
            true,
            "direct_direct",
        ),
        (
            RequestedMode::Work,
            "work",
            RunKind::ShapingUpdate,
            "shaping_update",
            false,
            "work_shaping",
        ),
        (
            RequestedMode::Work,
            "work",
            RunKind::Implementation,
            "implementation",
            true,
            "work_implementation",
        ),
        (
            RequestedMode::Work,
            "work",
            RunKind::Direct,
            "direct",
            false,
            "work_direct",
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_mode_and_change_unit(&harness, suffix, requested_mode)?;
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_mode_kind_{suffix}"),
            &format!("idem_mode_kind_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.kind = run_kind;

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        let after = harness.counts()?;
        if allowed {
            assert_eq!(
                response.response_value["base"]["response_kind"], "result",
                "{suffix}: {:?}",
                response.response_value
            );
            assert_eq!(response.response_value["state"]["mode"], task_mode);
            assert_eq!(
                response.response_value["run_summary"]["kind"],
                run_kind_value
            );
            let run_id = run_id_from_record_run(&response.response_value);
            assert_eq!(stored_run_kind(&harness, &run_id)?, run_kind_value);
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "VALIDATION_FAILED"
            );
            assert_eq!(after, before, "{suffix} must have no storage effect");
        }
    }
    Ok(())
}

#[test]
fn advisor_run_rejects_write_and_sensitive_effects_without_effect() -> Result<(), Box<dyn Error>> {
    for (suffix, product_write_observed, changed_paths, write_ticket_id, sensitive_categories) in [
        (
            "advisor_observed_write",
            true,
            vec!["src/export.rs".to_owned()],
            None,
            Vec::new(),
        ),
        (
            "advisor_changed_paths",
            false,
            vec!["src/export.rs".to_owned()],
            None,
            Vec::new(),
        ),
        (
            "advisor_write_ticket",
            false,
            Vec::new(),
            Some(WriteTicketId::new("wt_advisor_forbidden")),
            Vec::new(),
        ),
        (
            "advisor_sensitive_effect",
            false,
            Vec::new(),
            None,
            vec!["network".to_owned()],
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_mode_and_change_unit(&harness, suffix, RequestedMode::Advisor)?;
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_{suffix}"),
            &format!("idem_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.kind = RunKind::ShapingUpdate;
        request.observed_changes.product_file_write_observed = product_write_observed;
        request.observed_changes.changed_paths = changed_paths;
        request.observed_changes.sensitive_categories = sensitive_categories;
        request.write_ticket_id = write_ticket_id.into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(
            harness.counts()?,
            before,
            "{suffix} must have no storage effect"
        );
    }
    Ok(())
}

#[test]
fn record_run_without_product_write_commits_run_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_no_write")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let response = harness.service.record_run(
        record_run_request(
            "req_run_no_write",
            "idem_run_no_write",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["run_summary"]["observed_changes"]["product_file_write_observed"],
        false
    );
    let run_id = run_id_from_record_run(&response.response_value);
    assert_eq!(run_scope_revision(&harness, &run_id)?, 1);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let after_revision = task_revision(&harness, &task_id)?;
    assert_eq!(
        after_revision.close_basis_revision,
        before_revision.close_basis_revision + 1
    );
    assert!(after_revision.current_close_basis.is_none());
    assert!(response.response_value["current_close_basis"].is_null());
    Ok(())
}

#[test]
fn record_run_rejects_old_ticket_while_policy_control_reevaluation_is_pending(
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
    let metadata_json = volicord_types::canonical_json_string(&json!({
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
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
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
    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let stored_task = store
        .task_record(&TaskId::new(&task_id))?
        .expect("recorded Run Task remains current");
    assert_eq!(stored_task.effective_control_level, "light");
    assert_eq!(stored_task.acceptance_policy, "required");
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
    let volicord_types::UserActionDraft::Choice(choice) = &mut approval_request.action else {
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
    run.close_assessment = Some(volicord_types::CloseAssessmentInput {
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
fn record_run_blocks_only_matching_pending_observation_actions_without_effect(
) -> Result<(), Box<dyn Error>> {
    for (required_for, should_block, suffix) in [
        (
            volicord_types::UserActionRequiredFor::RecordRun,
            true,
            "matching",
        ),
        (
            volicord_types::UserActionRequiredFor::Informational,
            false,
            "informational",
        ),
        (
            volicord_types::UserActionRequiredFor::CloseComplete,
            false,
            "nonmatching_close",
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_pending_observation_{suffix}"))?;
        let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            &format!("pending_observation_{suffix}"),
        )?;
        let mut pending = observation_action_request(
            &format!("req_run_pending_observation_{suffix}"),
            &format!("idem_run_pending_observation_{suffix}"),
            after_artifact,
            &task_id,
            &change_unit_id,
            supplemental_evidence_target("Artifact registered for corruption coverage."),
            vec![artifact_ref.artifact_id],
        );
        pending.required_for = vec![required_for];
        let requested = harness
            .service
            .request_user_action(pending, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(requested.response_value["base"]["response_kind"], "result");
        let before = harness.counts()?;

        let response = harness.service.record_run(
            record_run_request(
                &format!("req_run_pending_observation_record_{suffix}"),
                &format!("idem_run_pending_observation_record_{suffix}"),
                false,
                Some(before.state_version),
                &task_id,
                &change_unit_id,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;

        if should_block {
            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "DECISION_UNRESOLVED"
            );
            assert_eq!(harness.counts()?, before);

            let dry_run = harness.service.record_run(
                record_run_request(
                    &format!("req_run_pending_observation_dry_{suffix}"),
                    &format!("idem_run_pending_observation_dry_{suffix}"),
                    true,
                    Some(before.state_version),
                    &task_id,
                    &change_unit_id,
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(dry_run.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                dry_run.response_value["errors"][0]["code"],
                "DECISION_UNRESOLVED"
            );
            assert_eq!(harness.counts()?, before);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "result");
            let after = harness.counts()?;
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
        }
    }
    Ok(())
}

#[test]
fn sensitive_pending_action_blocks_record_run_only_on_validated_matching_scope(
) -> Result<(), Box<dyn Error>> {
    for (
        suffix,
        action_operation,
        action_paths,
        action_categories,
        run_operation,
        run_paths,
        run_categories,
        mismatched_baseline,
        should_block,
    ) in [
        (
            "matching",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            false,
            true,
        ),
        (
            "no_sensitive_categories",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &[][..],
            false,
            false,
        ),
        (
            "operation_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "other_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            false,
            false,
        ),
        (
            "path_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["tests/export.rs"][..],
            &["network"][..],
            false,
            false,
        ),
        (
            "category_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["credential"][..],
            false,
            false,
        ),
        (
            "baseline_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            true,
            false,
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_pending_sensitive_{suffix}"))?;

        if !run_categories.is_empty() {
            let mut ticket_approval = user_action_request(
                &format!("req_run_ticket_approval_{suffix}"),
                &format!("idem_run_ticket_approval_{suffix}"),
                false,
                Some(harness.counts()?.state_version),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::SensitiveApproval,
            );
            let volicord_types::UserActionDraft::Choice(choice) = &mut ticket_approval.action
            else {
                unreachable!("sensitive approval fixture is choice-shaped")
            };
            choice.sensitive_action_scope = Some(sensitive_scope(
                run_operation,
                run_paths.to_vec(),
                run_categories.to_vec(),
            ))
            .into();
            let requested = harness.service.request_user_action(
                ticket_approval,
                invocation(OperationCategory::AgentWorkflow),
            )?;
            let approval_id =
                response_record_id(&requested.response_value, "user_action_request_ref");
            harness.service.resolve_user_action(
                resolve_user_action_request(
                    &format!("req_run_ticket_approval_resolve_{suffix}"),
                    &format!("submission_run_ticket_approval_{suffix}"),
                    None,
                    &task_id,
                    &approval_id,
                    "accept",
                ),
                invocation(OperationCategory::UserOnly),
            )?;
        }
        let mut prepare = prepare_write_request(
            &format!("req_run_ticket_prepare_{suffix}"),
            &format!("idem_run_ticket_prepare_{suffix}"),
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        );
        prepare.intended_operation = run_operation.to_owned();
        prepare.intended_paths = run_paths.iter().map(|path| (*path).to_owned()).collect();
        prepare.sensitive_categories = run_categories
            .iter()
            .map(|category| (*category).to_owned())
            .collect();
        let prepared = harness
            .service
            .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(prepared.response_value["decision"], "allowed");
        let write_ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");

        let mut pending = user_action_request(
            &format!("req_run_pending_sensitive_{suffix}"),
            &format!("idem_run_pending_sensitive_{suffix}"),
            false,
            Some(harness.counts()?.state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        );
        pending.required_for = vec![volicord_types::UserActionRequiredFor::RecordRun];
        let volicord_types::UserActionDraft::Choice(choice) = &mut pending.action else {
            unreachable!("sensitive approval fixture is choice-shaped")
        };
        choice.sensitive_action_scope = Some(sensitive_scope(
            action_operation,
            action_paths.to_vec(),
            action_categories.to_vec(),
        ))
        .into();
        let requested = harness
            .service
            .request_user_action(pending, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(requested.response_value["base"]["response_kind"], "result");
        let user_action_request_id =
            response_record_id(&requested.response_value, "user_action_request_ref");
        if mismatched_baseline {
            mutate_user_action_basis_json(&harness, &user_action_request_id, |basis| {
                basis["coordinates"]["baseline_ref"] = json!("baseline_other");
            })?;
        }

        let before = harness.counts()?;
        let mut request = product_write_record_run_request(
            &format!("req_run_pending_sensitive_record_{suffix}"),
            &format!("idem_run_pending_sensitive_record_{suffix}"),
            before.state_version,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            &format!("run_pending_sensitive_{suffix}"),
        );
        request.observed_changes.changed_paths =
            run_paths.iter().map(|path| (*path).to_owned()).collect();
        request.observed_changes.sensitive_categories = run_categories
            .iter()
            .map(|category| (*category).to_owned())
            .collect();
        request.performed_operation = Some(run_operation.to_owned()).into();
        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

        if should_block {
            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "DECISION_UNRESOLVED"
            );
            assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
            assert_eq!(harness.counts()?, before);

            let mut dry_run = product_write_record_run_request(
                &format!("req_run_pending_sensitive_dry_{suffix}"),
                &format!("idem_run_pending_sensitive_dry_{suffix}"),
                before.state_version,
                &task_id,
                &change_unit_id,
                &write_ticket_id,
                &format!("run_pending_sensitive_dry_{suffix}"),
            );
            dry_run.envelope.dry_run = true;
            dry_run.observed_changes.changed_paths =
                run_paths.iter().map(|path| (*path).to_owned()).collect();
            dry_run.observed_changes.sensitive_categories = run_categories
                .iter()
                .map(|category| (*category).to_owned())
                .collect();
            dry_run.performed_operation = Some(run_operation.to_owned()).into();
            let dry_run = harness
                .service
                .record_run(dry_run, invocation(OperationCategory::AgentWorkflow))?;
            assert_eq!(dry_run.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                dry_run.response_value["errors"][0]["code"],
                "DECISION_UNRESOLVED"
            );
            assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
            assert_eq!(harness.counts()?, before);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "result");
            let after = harness.counts()?;
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
            assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
        }
    }
    Ok(())
}

#[test]
fn record_run_non_null_close_assessment_creates_current_basis() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_basis")?;
    let generator = CountingDurableIdGenerator::new(["run_basis", "event_basis"]);
    let clock = ManualClock::at("2026-06-18T12:00:00Z");
    harness.use_generator_and_clock(generator, clock);

    let mut request = record_run_request(
        "req_run_basis",
        "idem_run_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(close_assessment_with_risks(
        "Recorded close basis.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;
    let basis = revision
        .current_close_basis
        .expect("current close basis should be stored");

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(basis.task_id.as_str(), task_id);
    assert_eq!(basis.change_unit_id.as_str(), change_unit_id);
    assert_eq!(basis.scope_revision, 1);
    assert_eq!(basis.close_basis_revision, revision.close_basis_revision);
    assert_eq!(basis.result_summary, "Recorded close basis.");
    assert!(basis.residual_risks.is_empty());
    assert_eq!(basis.updated_at.to_string(), "2026-06-18T12:00:00Z");
    assert_eq!(
        response.response_value["current_close_basis"]["residual_risks"],
        json!([])
    );
    assert!(
        response.response_value["current_close_basis"]["result_refs"]
            .as_array()
            .expect("result_refs should be present")
            .iter()
            .filter_map(|record_ref| record_ref["record_kind"].as_str())
            .any(|kind| kind == "run")
    );
    Ok(())
}

#[test]
fn current_compatible_run_ref_can_enter_close_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "current_run_ref")?;

    let mut first = record_run_request(
        "req_current_run_ref_first",
        "idem_current_run_ref_first",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    first.run_id = Some(RunId::new("run_current_ref_first")).into();
    let first_response = harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(first_response.response_value["base"]["state_version"], 3);

    let mut second = record_run_request(
        "req_current_run_ref_second",
        "idem_current_run_ref_second",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    second.run_id = Some(RunId::new("run_current_ref_second")).into();
    second.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Current prior Run can support this close basis.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_current_ref_first",
            PROJECT_ID,
            &task_id,
            Some(999),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(second, invocation(OperationCategory::AgentWorkflow))?;
    let basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current basis should be stored");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_current_ref_first"
            && record_ref.produced_at_state_version.as_ref() == Some(&4)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_current_ref_second"
            && record_ref.produced_at_state_version.as_ref() == Some(&4)
    }));
    Ok(())
}

#[test]
fn record_run_rejects_superseded_change_unit_run_ref_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "old_unit_run_ref")?;

    let mut old = record_run_request(
        "req_old_unit_run",
        "idem_old_unit_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    old.run_id = Some(RunId::new("run_old_unit")).into();
    harness
        .service
        .record_run(old, invocation(OperationCategory::AgentWorkflow))?;

    let replace = harness.service.update_scope(
        update_scope_request(
            "req_old_unit_replace",
            "idem_old_unit_replace",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_old_unit_rejected",
        "idem_old_unit_rejected",
        false,
        Some(4),
        &task_id,
        &replacement_change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Old unit Run must not become current.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_old_unit",
            PROJECT_ID,
            &task_id,
            Some(3),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_scope_revision_is_required_by_storage_constraint() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_scope_required")?;

    let mut request = record_run_request(
        "req_scope_required_run",
        "idem_scope_required_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_scope_required")).into();
    harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;

    let error = harness
        .conn()?
        .execute(
            "UPDATE runs
                SET scope_revision = NULL
              WHERE project_id = ?1
                AND run_id = 'run_scope_required'",
            rusqlite::params![PROJECT_ID],
        )
        .expect_err("runs.scope_revision is required");
    assert_constraint_error(error);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_baseline_incompatible_run_ref_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "baseline_run_ref")?;

    let mut baseline = record_run_request(
        "req_baseline_run",
        "idem_baseline_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    baseline.run_id = Some(RunId::new("run_baseline_mismatch")).into();
    harness
        .service
        .record_run(baseline, invocation(OperationCategory::AgentWorkflow))?;
    set_run_observed_baseline(&harness, "run_baseline_mismatch", "baseline_other")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_baseline_ref_rejected",
        "idem_baseline_ref_rejected",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Baseline-mismatched Run must not become current.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_baseline_mismatch",
            PROJECT_ID,
            &task_id,
            Some(3),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn historical_verified_artifact_reuse_requires_new_current_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "artifact_reuse")?;
    let (artifact_state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "artifact_reuse")?;
    let old_run_id = latest_run_id(&harness, &task_id)?;

    let replace = harness.service.update_scope(
        update_scope_request(
            "req_artifact_reuse_replace",
            "idem_artifact_reuse_replace",
            false,
            Some(artifact_state_version),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope for artifact reuse.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");

    let mut direct_old_run = record_run_request(
        "req_artifact_reuse_old_run",
        "idem_artifact_reuse_old_run",
        false,
        Some(artifact_state_version + 1),
        &task_id,
        &replacement_change_unit_id,
    );
    direct_old_run.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Old Run must not be reused directly.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            &old_run_id,
            PROJECT_ID,
            &task_id,
            Some(artifact_state_version),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_reject = harness.counts()?;
    let rejected = harness
        .service
        .record_run(direct_old_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before_reject);

    let mut current_reuse = record_run_request(
        "req_artifact_reuse_current",
        "idem_artifact_reuse_current",
        false,
        Some(artifact_state_version + 1),
        &task_id,
        &replacement_change_unit_id,
    );
    current_reuse.run_id = Some(RunId::new("run_artifact_reuse_current")).into();
    current_reuse.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_reuse_current",
        artifact_ref.clone(),
    )];
    current_reuse.evidence_updates = vec![supported_evidence_update(
        "Historical verified artifact reused by a current Run.",
    )];
    current_reuse.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Artifact reuse is recorded by a current Run.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Artifact,
            artifact_ref.artifact_id.as_str(),
            PROJECT_ID,
            &task_id,
            Some(artifact_state_version),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(current_reuse, invocation(OperationCategory::AgentWorkflow))?;
    let basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current basis should be stored");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        run_scope_revision(&harness, "run_artifact_reuse_current")?,
        2
    );
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_artifact_reuse_current"
    }));
    assert!(basis.result_refs.iter().all(|record_ref| {
        record_ref.record_kind != StateRecordKind::Run
            || record_ref.record_id.as_str() != old_run_id
    }));
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
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
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
    evidence_run.close_assessment = Some(volicord_types::CloseAssessmentInput {
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
    no_update_run.close_assessment = Some(volicord_types::CloseAssessmentInput {
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
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
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

fn assert_record_run_close_projection_matches_status(response: &Value, status: &Value) {
    assert_eq!(
        response["current_close_basis"],
        status["current_close_basis"]
    );
    assert_eq!(response["state"]["close_state"], status["close_state"]);
    assert_eq!(
        response["state"]["close_blockers"],
        status["close_blockers"]
    );
    assert!(!response["state"]["close_blockers"]
        .as_array()
        .expect("record_run close blockers should be an array")
        .iter()
        .any(|blocker| blocker["code"] == "stale_current_close_basis"));
    let primary_next_actions = response["state"]["close_blockers"]
        .as_array()
        .expect("record_run close blockers should be an array")
        .iter()
        .flat_map(|blocker| {
            blocker["next_actions"]
                .as_array()
                .expect("close blocker next_actions should be an array")
        })
        .filter(|action| action["presentation_role"] == "primary")
        .collect::<Vec<_>>();
    assert_eq!(primary_next_actions.len(), 1);
    assert_eq!(
        primary_next_actions[0],
        &status["summary_card"]["next_action"]
    );
}

#[test]
fn record_run_generates_opaque_residual_risk_ids_on_commit() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_risks")?;
    let generator = CountingDurableIdGenerator::new(["risk_alpha", "risk_beta", "event_risks"]);
    let clock = ManualClock::at("2026-06-18T12:30:00Z");
    harness.use_generator_and_clock(generator.clone(), clock);

    let mut request = record_run_request(
        "req_run_risks",
        "idem_run_risks",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_risks_supplied")).into();
    request.close_assessment = Some(close_assessment_with_risks(
        "Recorded close basis with risks.",
        vec![
            residual_risk_input("First residual risk."),
            residual_risk_input("Second residual risk."),
        ],
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let risk_ids = response.response_value["current_close_basis"]["residual_risks"]
        .as_array()
        .expect("residual risks should be an array")
        .iter()
        .map(|risk| {
            risk["risk_id"]
                .as_str()
                .expect("risk id should be present")
                .to_owned()
        })
        .collect::<Vec<_>>();
    let (_, event_payload, _) = latest_authority_event(&harness)?;

    assert_eq!(risk_ids, vec!["risk_risk_alpha", "risk_risk_beta"]);
    assert_eq!(generator.count(DurableIdKind::Risk), 2);
    assert_eq!(event_payload["residual_risk_ids"], json!(risk_ids));
    assert_eq!(
        event_payload["source_run_ref"]["record_id"],
        "run_risks_supplied"
    );
    assert_eq!(event_payload["scope_revision"], 1);
    assert_eq!(event_payload["close_basis_revision"], 2);
    Ok(())
}

#[test]
fn record_run_rejects_unsupported_close_basis_ref_kinds_without_effect(
) -> Result<(), Box<dyn Error>> {
    let unsupported = [
        (StateRecordKind::WriteTicket, "wa_fabricated"),
        (StateRecordKind::UserActionRequest, "uj_fabricated"),
        (StateRecordKind::Blocker, "blocker_fabricated"),
        (StateRecordKind::TaskEvent, "evt_fabricated"),
        (StateRecordKind::ProjectState, "project_state_fabricated"),
        (StateRecordKind::Task, "task_fabricated"),
        (StateRecordKind::AgentConnection, "connection_fabricated"),
    ];

    for (index, (record_kind, record_id)) in unsupported.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("unsupported_ref_{index}"))?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_unsupported_ref_{index}"),
            &format!("idem_unsupported_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(volicord_types::CloseAssessmentInput {
            result_summary: "Unsupported refs must not enter close authority.".to_owned(),
            result_refs: vec![test_state_record_ref(
                record_kind,
                record_id,
                PROJECT_ID,
                &task_id,
                Some(999),
            )],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn record_run_rejects_nonexistent_allowed_close_basis_refs_without_effect(
) -> Result<(), Box<dyn Error>> {
    let allowed_but_missing = [
        (StateRecordKind::Run, "run_missing"),
        (StateRecordKind::Artifact, "artifact_missing"),
        (StateRecordKind::EvidenceSummary, "evidence_missing"),
        (StateRecordKind::ChangeUnit, "cu_missing"),
    ];

    for (index, (record_kind, record_id)) in allowed_but_missing.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("missing_ref_{index}"))?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_missing_ref_{index}"),
            &format!("idem_missing_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(volicord_types::CloseAssessmentInput {
            result_summary: "Missing allowed refs still need stored records.".to_owned(),
            result_refs: vec![test_state_record_ref(
                record_kind,
                record_id,
                PROJECT_ID,
                &task_id,
                Some(2),
            )],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn record_run_rejects_cross_project_artifact_and_cross_task_run_refs_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cross_refs")?;

    for (index, record_ref) in [
        test_state_record_ref(
            StateRecordKind::Artifact,
            "artifact_cross_project",
            "project_other",
            &task_id,
            Some(2),
        ),
        test_state_record_ref(
            StateRecordKind::Run,
            "run_cross_task",
            PROJECT_ID,
            "task_other",
            Some(2),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_cross_ref_{index}"),
            &format!("idem_cross_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.run_id = Some(RunId::new(format!("run_cross_ref_{index}"))).into();
        request.close_assessment = Some(volicord_types::CloseAssessmentInput {
            result_summary: "Cross-owner refs must not enter close authority.".to_owned(),
            result_refs: vec![record_ref],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn record_run_rejects_corrupt_artifact_close_basis_ref_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "corrupt_basis_artifact")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "corrupt_basis_artifact",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    set_artifact_integrity(
        &harness,
        &artifact_id,
        "corrupt",
        artifact_ref.content_type.as_deref(),
        artifact_ref.sha256.as_deref(),
        artifact_ref.size_bytes.as_ref().copied(),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_unverified_artifact_basis",
        "idem_unverified_artifact_basis",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Unverified artifact must not enter close authority.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Artifact,
            &artifact_id,
            PROJECT_ID,
            &task_id,
            Some(999),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_noncurrent_evidence_summary_close_basis_ref_without_effect(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2999-07-13T12:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "noncurrent_evidence")?;
    let first_state =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "old_evidence", true)?;
    let old_evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    clock.advance(Duration::milliseconds(1));
    let current_state = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        first_state,
        "new_evidence",
        true,
    )?;
    let current_evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let summaries = {
        let conn = harness.conn()?;
        let mut stmt = conn.prepare(
            "SELECT evidence_summary_id, produced_at_state_version
               FROM evidence_summaries
              WHERE project_id = ?1 AND task_id = ?2
              ORDER BY produced_at_state_version DESC",
        )?;
        let summaries = stmt
            .query_map(rusqlite::params![PROJECT_ID, task_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        summaries
    };
    assert_ne!(
        old_evidence_summary_id, current_evidence_summary_id,
        "second evidence commit must establish a distinct latest summary: {summaries:?}"
    );
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_noncurrent_evidence_basis",
        "idem_noncurrent_evidence_basis",
        false,
        Some(current_state),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Old evidence summary must not enter current close authority.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::EvidenceSummary,
            &old_evidence_summary_id,
            PROJECT_ID,
            &task_id,
            Some(first_state),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_canonicalizes_deduplicates_and_adds_current_close_basis_refs(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_refs")?;
    let mut request = record_run_request(
        "req_canonical_refs",
        "idem_canonical_refs",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_canonical_refs")).into();
    request.evidence_updates = vec![supported_evidence_update("Canonical close basis claim.")];
    let future_run_ref = test_state_record_ref(
        StateRecordKind::Run,
        "run_canonical_refs",
        PROJECT_ID,
        &task_id,
        Some(999),
    );
    let past_run_ref = test_state_record_ref(
        StateRecordKind::Run,
        "run_canonical_refs",
        PROJECT_ID,
        &task_id,
        Some(1),
    );
    let mut risk = residual_risk_input("Caller-versioned risk source.");
    risk.acceptance_required = false;
    risk.source_refs = vec![future_run_ref.clone(), past_run_ref.clone()];
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Canonical refs are stored.".to_owned(),
        result_refs: vec![future_run_ref, past_run_ref],
        residual_risks: vec![risk],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;
    let basis = revision
        .current_close_basis
        .expect("current close basis should be stored");

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(basis.result_refs.len(), 3);
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_canonical_refs"
            && record_ref.produced_at_state_version.as_ref() == Some(&3)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::ChangeUnit
            && record_ref.record_id.as_str() == change_unit_id
            && record_ref.produced_at_state_version.as_ref() == Some(&3)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::EvidenceSummary
            && record_ref.produced_at_state_version.as_ref() == Some(&3)
    }));
    assert_eq!(
        basis
            .evidence_summary_ref
            .as_ref()
            .and_then(|record_ref| record_ref.produced_at_state_version.as_ref().copied()),
        Some(3)
    );
    assert_eq!(basis.residual_risks[0].source_refs.len(), 1);
    assert_eq!(
        basis.residual_risks[0].source_refs[0]
            .produced_at_state_version
            .as_ref(),
        Some(&3)
    );
    Ok(())
}

#[test]
fn final_acceptance_judgment_basis_uses_canonical_close_basis_refs() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_final")?;
    let state_version = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "canonical_final",
        true,
    )?;
    let close_basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current close basis should be stored");

    let response = harness.service.request_user_action(
        user_action_request(
            "req_canonical_final",
            "idem_canonical_final",
            false,
            Some(state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let request_id = response_record_id(&response.response_value, "user_action_request_ref");
    assert_eq!(
        response.response_value["user_action_request_summary"],
        pending_user_action_summary(&request_id)
    );
    let projection = cli_user_channel_projection(&harness, &task_id)?;
    let projected = projection
        .items
        .iter()
        .find(|item| item.request.user_action_request_id.as_str() == request_id)
        .expect("trusted User Channel projection should retain the close-basis request");
    assert_eq!(
        projected.request.basis.result_refs(),
        close_basis.result_refs.as_slice()
    );
    assert!(close_basis.result_refs.iter().all(|record_ref| {
        record_ref.produced_at_state_version.as_ref() == Some(&state_version)
    }));
    Ok(())
}

#[test]
fn record_run_null_close_assessment_invalidates_existing_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_clear_basis")?;

    let mut establish = record_run_request(
        "req_run_establish_basis",
        "idem_run_establish_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    establish.close_assessment = Some(close_assessment_with_risks(
        "Established basis.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(establish, invocation(OperationCategory::AgentWorkflow))?;
    assert!(task_revision(&harness, &task_id)?
        .current_close_basis
        .is_some());

    let clear = record_run_request(
        "req_run_clear_basis",
        "idem_run_clear_basis",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    let response = harness
        .service
        .record_run(clear, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;

    assert!(response.response_value["current_close_basis"].is_null());
    assert_eq!(revision.close_basis_revision, 3);
    assert!(revision.current_close_basis.is_none());
    Ok(())
}

#[test]
fn record_run_dry_run_allocates_no_residual_risk_ids() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_dry_risk")?;
    let generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T13:00:00Z");
    harness.use_generator_and_clock(generator.clone(), clock);
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_dry_risk",
        "idem_run_dry_risk",
        true,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_dry_risk_supplied")).into();
    request.close_assessment = Some(close_assessment_with_risks(
        "Dry-run close basis.",
        vec![residual_risk_input("Dry-run residual risk.")],
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(generator.count(DurableIdKind::Risk), 0);
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
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
fn record_run_rejects_branch_change_after_write_ticket_issue_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let intake = harness.service.intake(
        intake_request(
            "req_run_workspace_task",
            "idem_run_workspace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let original = crate::pipeline::GitWorkspaceContext {
        git_common_dir: "/tmp/volicord-run-workspace/.git".to_owned(),
        worktree_id: format!("sha256:{}", "5".repeat(64)),
        branch_ref: Some("refs/heads/original".to_owned()),
        head_sha: Some("5".repeat(40)),
        workspace_fingerprint: format!("sha256:{}", "6".repeat(64)),
    };
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_run_workspace_scope",
            "idem_run_workspace_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bind the ticket to the original branch.",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let ticket = harness.service.prepare_write(
        prepare_write_request(
            "req_run_workspace_ticket",
            "idem_run_workspace_ticket",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    assert_eq!(ticket.response_value["decision"], "allowed");
    let write_ticket_id = response_record_id(&ticket.response_value, "write_ticket_ref");
    let before = harness.counts()?;

    let mut changed = original;
    changed.branch_ref = Some("refs/heads/other".to_owned());
    changed.head_sha = Some("7".repeat(40));
    changed.workspace_fingerprint = format!("sha256:{}", "8".repeat(64));
    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_workspace_changed",
            "idem_run_workspace_changed",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_workspace_changed",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(changed),
    )?;

    assert_write_ticket_invalid_reason(&response, "workspace_context_mismatch");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
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
    touch.acceptance_criteria = Some(vec![volicord_types::AcceptanceCriterionReplacement {
        acceptance_criterion_id: Some(volicord_types::AcceptanceCriterionId::new(
            active_acceptance_criterion_id(&harness, &task_id)?,
        ))
        .into(),
        statement: "The scoped behavior is represented.".to_owned(),
        evidence_requirement: EvidenceRequirement::NotRequired,
    }])
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
fn record_run_missing_write_ticket_rejects_product_write_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_missing_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_auth",
        "idem_run_missing_auth",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_TICKET_REQUIRED"
    );
    assert_eq!(harness.counts()?, before);
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
    touch.acceptance_criteria = Some(vec![volicord_types::AcceptanceCriterionReplacement {
        acceptance_criterion_id: Some(volicord_types::AcceptanceCriterionId::new(
            active_acceptance_criterion_id(&harness, &task_id)?,
        ))
        .into(),
        statement: "The scoped behavior is represented.".to_owned(),
        evidence_requirement: EvidenceRequirement::NotRequired,
    }])
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

    assert_write_ticket_invalid_reason(&response, "task_mismatch");
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

    assert_write_ticket_invalid_reason(&response, "change_unit_mismatch");
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
    let volicord_types::UserActionDraft::Choice(choice) = &mut approval_request.action else {
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
fn record_run_promotes_staged_artifact_and_updates_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_artifact", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let expected_content_type = handle.content_type.clone();
    let expected_sha256 = handle.sha256.clone();
    let expected_size_bytes = handle.size_bytes;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_artifact",
        "idem_run_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_report",
        handle,
        Some("validation_report"),
        Some("Search-result count validation passed."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Search-result count validation passed.",
    )];
    request.evidence_observations = vec![EvidenceObservationInput {
        target: supplemental_evidence_target("Search-result count validation passed."),
        source_kind: EvidenceSourceKind::ExternalTool,
        assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
        observed_by_actor_source: None.into(),
        tool_name: Some("search-count-validator".to_owned()).into(),
        tool_invocation_id: None.into(),
        tool_metadata: Map::from_iter([("validator".to_owned(), json!("search-count"))]),
        input_refs: Vec::new(),
        source_refs: vec![
            volicord_types::SourceRef::ExternalUri(volicord_types::ExternalUriSource {
                uri: "https://example.invalid/search-spec".to_owned(),
                retrieved_at: volicord_types::UtcTimestamp::parse("2026-06-17T23:59:00Z")?,
                content_sha256: "d".repeat(64),
            }),
            volicord_types::SourceRef::UserContext(volicord_types::UserContextSource {
                context_id: "message_search_requirement".to_owned(),
            }),
        ],
        output_artifact_refs: Vec::new(),
        limitations: vec!["External tool output is not product correctness proof.".to_owned()],
        observed_at: volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
    }];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();
    let observation = &response.response_value["evidence_observations"][0];
    let observation_id = observation["observation_id"]
        .as_str()
        .expect("observation id should be present")
        .to_owned();
    let artifact_row = persistent_artifact_row(&harness, &artifact_id)?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["content_type"],
        expected_content_type
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["sha256"],
        expected_sha256
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["size_bytes"],
        expected_size_bytes
    );
    assert_eq!(artifact_row.integrity_status, "verified");
    assert_eq!(
        artifact_row.content_type.as_deref(),
        Some(expected_content_type.as_str())
    );
    let body_path = artifact_row
        .body_path
        .as_deref()
        .expect("promoted artifact should store a body path");
    let staging_row = staged_artifact_row(&harness, &handle_id)?;
    assert!(
        body_path.starts_with("tmp/"),
        "persistent body_path should be artifact-store-relative: {body_path}"
    );
    assert!(
        !body_path.starts_with("artifacts/"),
        "persistent body_path must not include the project-home artifact prefix"
    );
    assert_eq!(staging_row.tmp_path, format!("artifacts/{body_path}"));
    assert_eq!(
        artifact_row.sha256.as_deref(),
        Some(expected_sha256.as_str())
    );
    assert_eq!(artifact_row.size_bytes, Some(expected_size_bytes));
    assert_eq!(artifact_row.status, "available");
    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["supporting_run_refs"][0]
            ["record_kind"],
        "run"
    );
    assert_eq!(observation["source_kind"], "agent_report");
    assert_eq!(observation["assurance_level"], "cooperative_report");
    assert_eq!(
        observation["producer_anchor"]["producer_kind"],
        "unverified_caller"
    );
    assert_eq!(observation["relevance_assessment"]["status"], "unassessed");
    assert_eq!(observation["observed_by_actor_source"], AGENT_ACTOR_SOURCE);
    assert_eq!(observation["tool_metadata"]["validator"], "search-count");
    assert_eq!(observation["source_refs"][0]["source_kind"], "external_uri");
    assert_eq!(observation["source_refs"][1]["source_kind"], "user_context");
    assert_eq!(
        observation["output_artifact_refs"][0]["artifact_id"],
        artifact_id
    );
    assert!(
        observation_id.starts_with("evidence_observation_"),
        "generated observation id should use the durable prefix: {observation_id}"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            ["record_kind"],
        "evidence_observation"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            ["record_id"],
        observation_id
    );
    assert_eq!(
        response.response_value["evidence_summary"]["observation_refs"][0]["record_id"],
        observation_id
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.artifacts, before.artifacts + 1);
    assert_eq!(after.artifact_links, before.artifact_links + 3);
    assert_eq!(after.evidence_summaries, before.evidence_summaries + 1);
    assert_eq!(
        after.evidence_observations,
        before.evidence_observations + 1
    );
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "consumed");
    let stored_source_refs: String = harness.conn()?.query_row(
        "SELECT source_refs_json FROM evidence_observations WHERE project_id = ?1 AND evidence_observation_id = ?2",
        rusqlite::params![PROJECT_ID, observation_id],
        |row| row.get(0),
    )?;
    let stored_source_refs: serde_json::Value = serde_json::from_str(&stored_source_refs)?;
    assert_eq!(stored_source_refs, observation["source_refs"]);
    assert!(artifact_owner_link_exists(&harness, &artifact_id, "run")?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_summary"
    )?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_observation"
    )?);
    Ok(())
}

#[test]
fn record_run_observations_derive_provenance_and_actor_fail_closed() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_observation_classes")?;
    let classes = [
        (
            "Agent cooperative report.",
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
            "agent_report",
            "cooperative_report",
        ),
        (
            "External tool result.",
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
            "agent_report",
            "cooperative_report",
        ),
        (
            "User observation.",
            EvidenceSourceKind::UserObservation,
            EvidenceAssuranceLevel::UserObserved,
            "agent_report",
            "cooperative_report",
        ),
        (
            "Caller-declared reused evidence.",
            EvidenceSourceKind::ReusedEvidence,
            EvidenceAssuranceLevel::ExternalToolResult,
            "agent_report",
            "cooperative_report",
        ),
        (
            "Unverified claim.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
            "unverified_claim",
            "unverified",
        ),
    ];
    let mut request = record_run_request(
        "req_run_observation_classes",
        "idem_run_observation_classes",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = classes
        .iter()
        .map(|(claim, source_kind, assurance_level, _, _)| {
            supported_evidence_update_with_provenance(claim, *source_kind, *assurance_level)
        })
        .collect();
    request.evidence_observations = classes
        .iter()
        .map(
            |(claim, source_kind, assurance_level, _, _)| EvidenceObservationInput {
                target: supplemental_evidence_target(claim),
                source_kind: *source_kind,
                assurance_level: *assurance_level,
                observed_by_actor_source: Some(ActorSource::LocalUser).into(),
                tool_name: Some("fixture-evidence-check".to_owned()).into(),
                tool_invocation_id: None.into(),
                tool_metadata: JsonObject::new(),
                input_refs: Vec::new(),
                source_refs: Vec::new(),
                output_artifact_refs: Vec::new(),
                limitations: Vec::new(),
                observed_at: volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")
                    .expect("fixture timestamp should parse"),
            },
        )
        .collect();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let observations = response.response_value["evidence_observations"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "evidence observations should be present: {}",
                response.response_value
            )
        });

    assert_eq!(observations.len(), classes.len());
    for (observation, (_, _, _, source_value, assurance_value)) in observations.iter().zip(classes)
    {
        assert_eq!(observation["source_kind"], source_value);
        assert_eq!(observation["assurance_level"], assurance_value);
        assert_eq!(observation["observed_by_actor_source"], AGENT_ACTOR_SOURCE);
        assert!(observation.get("guarantee_display").is_none());
    }
    Ok(())
}

#[test]
fn user_channel_observation_is_strong_and_reuse_revalidates_its_authority_chain(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_evidence_authority")?;
    let criterion_id = volicord_types::AcceptanceCriterionId::new(active_acceptance_criterion_id(
        &harness, &task_id,
    )?);
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_evidence_authority",
    )?;
    let target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: criterion_id,
    };
    let (after_user, user_action_resolution_ref) = request_and_resolve_user_observation(
        &harness,
        UserObservationFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: after_artifact,
            suffix: "user_evidence_authority",
            target: target.clone(),
            artifact_ref: &artifact_ref,
            relevance_status: EvidenceRelevanceStatus::Supported,
        },
    )?;

    let mut record = record_run_request(
        "req_record_user_evidence",
        "idem_record_user_evidence",
        false,
        Some(after_user),
        &task_id,
        &change_unit_id,
    );
    record.evidence_updates = vec![EvidenceCoverageUpdate {
        target: target.clone(),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: vec![artifact_ref.clone()],
        gap_refs: Vec::new(),
    }];
    record.evidence_observations = vec![EvidenceObservationInput {
        target: target.clone(),
        source_kind: EvidenceSourceKind::UserObservation,
        assurance_level: EvidenceAssuranceLevel::UserObserved,
        observed_by_actor_source: None.into(),
        tool_name: None.into(),
        tool_invocation_id: None.into(),
        tool_metadata: JsonObject::new(),
        input_refs: vec![user_action_resolution_ref.clone()],
        source_refs: Vec::new(),
        output_artifact_refs: vec![artifact_ref.clone()],
        limitations: Vec::new(),
        observed_at: volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
    }];
    record.close_assessment = Some(close_assessment_with_risks(
        "User-observed evidence is current.",
        Vec::new(),
    ))
    .into();
    let recorded = harness
        .service
        .record_run(record, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");
    assert_eq!(
        recorded.response_value["evidence_observations"][0]["source_kind"],
        "user_observation"
    );
    assert_eq!(
        recorded.response_value["evidence_observations"][0]["producer_anchor"]["producer_kind"],
        "user_channel_observation"
    );
    assert_no_close_blocker(
        &recorded.response_value["state"],
        "evidence_provenance_insufficient",
    );
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("record state version");
    let original_observation_ref: StateRecordRef = serde_json::from_value(
        recorded.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            .clone(),
    )?;

    let mut reuse = record_run_request(
        "req_reuse_user_evidence",
        "idem_reuse_user_evidence",
        false,
        Some(after_record),
        &task_id,
        &change_unit_id,
    );
    reuse.evidence_updates = vec![EvidenceCoverageUpdate {
        target: target.clone(),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: vec![original_observation_ref],
        supporting_artifact_refs: vec![artifact_ref],
        gap_refs: Vec::new(),
    }];
    reuse.close_assessment = Some(close_assessment_with_risks(
        "Reused user-observed evidence is current.",
        Vec::new(),
    ))
    .into();
    let reused = harness
        .service
        .record_run(reuse, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(reused.response_value["base"]["response_kind"], "result");
    assert_eq!(
        reused.response_value["evidence_observations"][0]["source_kind"],
        "reused_evidence"
    );
    assert_no_close_blocker(
        &reused.response_value["state"],
        "evidence_provenance_insufficient",
    );

    let conn = harness.conn()?;
    let resolution_json: String = conn.query_row(
        "SELECT resolution_json
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_resolution_ref.record_id.as_str()],
        |row| row.get(0),
    )?;
    let mut resolution_json: serde_json::Value = serde_json::from_str(&resolution_json)?;
    resolution_json["observation"]["relevance_status"] =
        serde_json::Value::String("contradicted".to_owned());
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolution_json = ?3
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            user_action_resolution_ref.record_id.as_str(),
            serde_json::to_string(&resolution_json)?
        ],
    )?;
    let close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_tampered_user_evidence",
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
    assert_close_blocker(&close.response_value, "evidence_provenance_insufficient");
    Ok(())
}

#[test]
fn user_channel_observation_preserves_relevance_resolution_time_and_supported_only_reuse(
) -> Result<(), Box<dyn Error>> {
    for (suffix, relevance_status, coverage_state, relevance_value, caller_observed_at) in [
        (
            "supported",
            EvidenceRelevanceStatus::Supported,
            EvidenceCoverageUpdateState::Supported,
            "supported",
            "2000-01-01T00:00:00Z",
        ),
        (
            "contradicted",
            EvidenceRelevanceStatus::Contradicted,
            EvidenceCoverageUpdateState::Contradicted,
            "contradicted",
            "2999-01-01T00:00:00Z",
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("user_relevance_{suffix}"))?;
        let criterion_id = volicord_types::AcceptanceCriterionId::new(
            active_acceptance_criterion_id(&harness, &task_id)?,
        );
        set_active_acceptance_criterion_requirement(
            &harness,
            &task_id,
            EvidenceRequirement::Required,
        )?;
        let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            &format!("user_relevance_{suffix}"),
        )?;
        let target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: criterion_id,
        };
        let (after_resolution, resolution_ref) = request_and_resolve_user_observation(
            &harness,
            UserObservationFixture {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                expected_state_version: after_artifact,
                suffix: &format!("user_relevance_{suffix}"),
                target: target.clone(),
                artifact_ref: &artifact_ref,
                relevance_status,
            },
        )?;
        let resolved_at: String = harness.conn()?.query_row(
            "SELECT resolved_at
               FROM user_action_resolutions
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
            rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
            |row| row.get(0),
        )?;
        assert_ne!(resolved_at, caller_observed_at);

        let mut record = record_run_request(
            &format!("req_user_relevance_{suffix}"),
            &format!("idem_user_relevance_{suffix}"),
            false,
            Some(after_resolution),
            &task_id,
            &change_unit_id,
        );
        record.evidence_updates = vec![EvidenceCoverageUpdate {
            target: target.clone(),
            coverage_state,
            provenance: None,
            supporting_run_refs: Vec::new(),
            observation_refs: Vec::new(),
            supporting_artifact_refs: vec![artifact_ref.clone()],
            gap_refs: Vec::new(),
        }];
        record.evidence_observations = vec![EvidenceObservationInput {
            target: target.clone(),
            source_kind: EvidenceSourceKind::UserObservation,
            assurance_level: EvidenceAssuranceLevel::UserObserved,
            observed_by_actor_source: None.into(),
            tool_name: None.into(),
            tool_invocation_id: None.into(),
            tool_metadata: JsonObject::new(),
            input_refs: vec![resolution_ref.clone()],
            source_refs: Vec::new(),
            output_artifact_refs: vec![artifact_ref.clone()],
            limitations: Vec::new(),
            observed_at: volicord_types::UtcTimestamp::parse(caller_observed_at)?,
        }];
        record.close_assessment = Some(close_assessment_with_risks(
            &format!("User-observed {suffix} evidence is preserved."),
            Vec::new(),
        ))
        .into();
        let replay_request = record.clone();
        let before_record = harness.counts()?;
        let recorded = harness
            .service
            .record_run(record, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(recorded.response_value["base"]["response_kind"], "result");
        let observation = &recorded.response_value["evidence_observations"][0];
        assert_eq!(observation["source_kind"], "user_observation");
        assert_eq!(observation["assurance_level"], "user_observed");
        assert_eq!(observation["observed_by_actor_source"], "local_user");
        assert_eq!(
            observation["producer_anchor"]["producer_kind"],
            "user_channel_observation"
        );
        assert_eq!(
            observation["relevance_assessment"]["status"],
            relevance_value
        );
        assert_eq!(
            observation["relevance_assessment"]["assessed_by_actor_source"],
            "local_user"
        );
        assert_eq!(observation["observed_at"], resolved_at);
        assert_eq!(
            recorded.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
            relevance_value
        );
        assert_no_close_blocker(
            &recorded.response_value["state"],
            "evidence_provenance_insufficient",
        );
        let after_record = harness.counts()?;
        assert_eq!(after_record.runs, before_record.runs + 1);
        assert_eq!(
            after_record.evidence_observations,
            before_record.evidence_observations + 1
        );
        let observation_id = observation["observation_id"]
            .as_str()
            .expect("committed observation id");
        let (stored_observed_at, stored_metadata): (String, String) = harness.conn()?.query_row(
            "SELECT observed_at, metadata_json
                   FROM evidence_observations
                  WHERE project_id = ?1
                    AND evidence_observation_id = ?2",
            rusqlite::params![PROJECT_ID, observation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(stored_observed_at, resolved_at);
        let stored_metadata: Value = serde_json::from_str(&stored_metadata)?;
        assert_eq!(
            stored_metadata["relevance_assessment"]["status"],
            relevance_value
        );
        let store =
            CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
        let observation_record = store
            .evidence_observation_record(observation_id)?
            .expect("committed user observation should be readable");
        assert_eq!(
            super::super::record_run::stored_evidence_observation_provenance_class(
                &store,
                &observation_record,
                &super::super::record_run::StoredEvidenceProvenanceBasis {
                    project_id: &ProjectId::new(PROJECT_ID),
                    task_id: &TaskId::new(&task_id),
                    change_unit_id: &change_unit_id,
                    scope_revision: 1,
                    baseline_ref: Some("baseline_test"),
                    target: &target,
                    now: &volicord_types::UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?,
                },
            )?,
            EvidenceProvenanceClass::Strong,
            "{suffix} must retain strong user-channel producer provenance"
        );
        drop(store);

        let before_replay = harness.counts()?;
        let before_replay_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        let replayed = harness
            .service
            .record_run(replay_request, invocation(OperationCategory::AgentWorkflow))?;
        assert!(replayed.replayed);
        assert_eq!(replayed.response_json, recorded.response_json);
        assert_eq!(harness.counts()?, before_replay);
        let after_replay_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_replay_floor, before_replay_floor);

        let observation_ref: StateRecordRef = serde_json::from_value(
            recorded.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
                .clone(),
        )?;
        if relevance_status == EvidenceRelevanceStatus::Supported {
            let original_resolution_json: String = harness.conn()?.query_row(
                "SELECT resolution_json
                   FROM user_action_resolutions
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
                rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
                |row| row.get(0),
            )?;
            let mut mismatched_resolution: Value = serde_json::from_str(&original_resolution_json)?;
            mismatched_resolution["observation"]["relevance_status"] =
                Value::String("contradicted".to_owned());
            harness.conn()?.execute(
                "UPDATE user_action_resolutions
                    SET resolution_json = ?3
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
                rusqlite::params![
                    PROJECT_ID,
                    resolution_ref.record_id.as_str(),
                    serde_json::to_string(&mismatched_resolution)?
                ],
            )?;
        }

        let committed_state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .expect("committed record state version");
        for dry_run in [true, false] {
            let branch = if dry_run { "dry" } else { "commit" };
            let mut reuse = record_run_request(
                &format!("req_user_relevance_reuse_{suffix}_{branch}"),
                &format!("idem_user_relevance_reuse_{suffix}_{branch}"),
                dry_run,
                Some(committed_state_version),
                &task_id,
                &change_unit_id,
            );
            reuse.evidence_updates = vec![EvidenceCoverageUpdate {
                target: target.clone(),
                coverage_state: EvidenceCoverageUpdateState::Supported,
                provenance: None,
                supporting_run_refs: Vec::new(),
                observation_refs: vec![observation_ref.clone()],
                supporting_artifact_refs: vec![artifact_ref.clone()],
                gap_refs: Vec::new(),
            }];
            let before_rejection = harness.counts()?;
            let before_rejection_floor: String = harness.conn()?.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            let rejected = harness
                .service
                .record_run(reuse, invocation(OperationCategory::AgentWorkflow))?;
            assert_eq!(
                rejected.response_value["base"]["response_kind"], "rejected",
                "case {suffix}, dry_run={dry_run}"
            );
            assert_eq!(
                rejected.response_value["errors"][0]["details"]["field"],
                "evidence_updates[].observation_refs"
            );
            assert_eq!(harness.counts()?, before_rejection);
            let after_rejection_floor: String = harness.conn()?.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            assert_eq!(after_rejection_floor, before_rejection_floor);
        }
    }
    Ok(())
}

#[test]
fn user_channel_observation_rejects_tampered_exact_artifact_binding_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_evidence_exact_artifacts")?;
    let criterion_id = volicord_types::AcceptanceCriterionId::new(active_acceptance_criterion_id(
        &harness, &task_id,
    )?);
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let (after_first_artifact, first_artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_evidence_exact_first",
    )?;
    let (after_second_artifact, second_artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        after_first_artifact,
        "user_evidence_exact_second",
    )?;
    let target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: criterion_id,
    };
    let artifact_refs = vec![first_artifact_ref, second_artifact_ref];
    let requested = harness.service.request_user_action(
        volicord_types::RequestUserActionRequest {
            envelope: envelope(
                "req_user_action_observation_exact_artifacts",
                Some("idem_user_action_observation_exact_artifacts"),
                false,
                Some(after_second_artifact),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            change_unit_id: Some(ChangeUnitId::new(&change_unit_id)).into(),
            action: volicord_types::UserActionDraft::EvidenceObservation(
                volicord_types::UserActionEvidenceObservationDraft {
                    question: "Do these exact artifacts support the selected target?".to_owned(),
                    context_summary: "The user must inspect both exact candidate artifacts."
                        .to_owned(),
                    target_candidates: vec![target.clone()],
                    artifact_candidate_ids: artifact_refs
                        .iter()
                        .map(|artifact| artifact.artifact_id.clone())
                        .collect(),
                },
            ),
            required_for: vec![volicord_types::UserActionRequiredFor::RecordRun],
            expires_at: None.into(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id =
        response_record_id(&requested.response_value, "user_action_request_ref");
    let resolved = harness.service.resolve_user_action(
        volicord_types::ResolveUserActionRequest {
            envelope: envelope(
                "req_user_action_observation_exact_artifacts_resolve",
                Some("submission_user_action_observation_exact_artifacts"),
                false,
                None,
                Some(&task_id),
            ),
            user_action_request_id: volicord_types::UserActionRequestId::new(
                user_action_request_id,
            ),
            channel_submission_id: "submission_user_action_observation_exact_artifacts".to_owned(),
            resolution: volicord_types::UserActionResolutionInput::EvidenceObservation {
                target: target.clone(),
                artifact_ids: artifact_refs
                    .iter()
                    .map(|artifact| artifact.artifact_id.clone())
                    .collect(),
                relevance_status: EvidenceRelevanceStatus::Supported,
                summary: "The user assessed both exact candidate artifacts.".to_owned(),
            },
        },
        invocation(OperationCategory::UserOnly),
    )?;
    let after_user_action = resolved.response_value["base"]["state_version"]
        .as_u64()
        .expect("user-action resolution state version");
    let resolution_ref: StateRecordRef =
        serde_json::from_value(resolved.response_value["user_action_resolution_ref"].clone())?;

    let record_request = |suffix: &str, dry_run: bool| {
        let request_id = format!("req_user_evidence_exact_{suffix}");
        let idempotency_key = format!("idem_user_evidence_exact_{suffix}");
        let mut record = record_run_request(
            &request_id,
            &idempotency_key,
            dry_run,
            Some(after_user_action),
            &task_id,
            &change_unit_id,
        );
        record.evidence_updates = vec![EvidenceCoverageUpdate {
            target: target.clone(),
            coverage_state: EvidenceCoverageUpdateState::Supported,
            provenance: None,
            supporting_run_refs: Vec::new(),
            observation_refs: Vec::new(),
            supporting_artifact_refs: artifact_refs.clone(),
            gap_refs: Vec::new(),
        }];
        record.evidence_observations = vec![EvidenceObservationInput {
            target: target.clone(),
            source_kind: EvidenceSourceKind::UserObservation,
            assurance_level: EvidenceAssuranceLevel::UserObserved,
            observed_by_actor_source: None.into(),
            tool_name: None.into(),
            tool_invocation_id: None.into(),
            tool_metadata: JsonObject::new(),
            input_refs: vec![resolution_ref.clone()],
            source_refs: Vec::new(),
            output_artifact_refs: artifact_refs.clone(),
            limitations: Vec::new(),
            observed_at: volicord_types::UtcTimestamp::parse("2026-06-18T00:00:00Z")
                .expect("fixture timestamp"),
        }];
        record
    };

    let before_control = harness.counts()?;
    let before_control_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    let control = harness.service.record_run(
        record_request("control", true),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(control.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(harness.counts()?, before_control);
    let after_control_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_control_floor, before_control_floor);

    let conn = harness.conn()?;
    let original_resolution_json: String = conn.query_row(
        "SELECT resolution_json
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
        |row| row.get(0),
    )?;
    let original_resolution: Value = serde_json::from_str(&original_resolution_json)?;
    let mut tampered_resolutions = Vec::new();
    for (suffix, pointer, replacement) in [
        (
            "display_name",
            "/observation/output_artifact_refs/0/display_name",
            json!("tampered-display-name.bin"),
        ),
        (
            "content_type",
            "/observation/output_artifact_refs/0/content_type",
            json!("application/tampered"),
        ),
        (
            "redaction_state",
            "/observation/output_artifact_refs/0/redaction_state",
            json!("redacted"),
        ),
        (
            "producer_identity",
            "/observation/output_artifact_refs/0/created_by_run_ref/record_id",
            json!("run_tampered_exact_output"),
        ),
        (
            "producer_presence",
            "/observation/output_artifact_refs/0/created_by_run_ref",
            Value::Null,
        ),
        (
            "producer_actor",
            "/observation/output_artifact_refs/0/created_by_actor_source",
            json!("local_user"),
        ),
        (
            "storage_ref",
            "/observation/output_artifact_refs/0/storage_ref",
            json!("artifact://tampered-storage-ref"),
        ),
    ] {
        let mut tampered = original_resolution.clone();
        *tampered
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("fixture pointer should exist: {pointer}")) = replacement;
        tampered_resolutions.push((suffix, tampered));
    }
    let mut duplicate = original_resolution.clone();
    let duplicated_ref = duplicate["observation"]["output_artifact_refs"][0].clone();
    duplicate["observation"]["output_artifact_refs"][1] = duplicated_ref;
    tampered_resolutions.push(("duplicate_artifact_id", duplicate));

    for (suffix, tampered) in tampered_resolutions {
        conn.execute(
            "UPDATE user_action_resolutions
                SET resolution_json = ?3
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
            rusqlite::params![
                PROJECT_ID,
                resolution_ref.record_id.as_str(),
                serde_json::to_string(&tampered)?
            ],
        )?;
        for dry_run in [true, false] {
            let branch = if dry_run { "dry" } else { "commit" };
            let before = harness.counts()?;
            let before_floor: String = conn.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            let response = harness.service.record_run(
                record_request(&format!("{suffix}_{branch}"), dry_run),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                response.response_value["base"]["response_kind"], "rejected",
                "case {suffix}, dry_run={dry_run}"
            );
            assert_eq!(
                harness.counts()?,
                before,
                "case {suffix}, dry_run={dry_run}"
            );
            let after_floor: String = conn.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            assert_eq!(
                after_floor, before_floor,
                "case {suffix}, dry_run={dry_run}"
            );
        }
    }
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolution_json = ?3
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            resolution_ref.record_id.as_str(),
            original_resolution_json
        ],
    )?;
    Ok(())
}

#[test]
fn not_applicable_rejects_required_criterion_but_commits_for_optional_criterion(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_not_applicable_requirement")?;
    let (after_required, required_criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        2,
        "run_not_applicable_required",
        &[(
            "Criterion requiring evidence.",
            EvidenceRequirement::Required,
        )],
    )?;
    let required_id = required_criteria[0].acceptance_criterion_id.clone();
    let required_update = EvidenceCoverageUpdate {
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: required_id.clone(),
        },
        coverage_state: EvidenceCoverageUpdateState::NotApplicable,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    };
    let mut required_request = record_run_request(
        "req_run_not_applicable_required",
        "idem_run_not_applicable_required",
        false,
        Some(after_required),
        &task_id,
        &change_unit_id,
    );
    required_request.evidence_updates = vec![required_update];
    let before_rejection = harness.counts()?;
    let rejected = harness.service.record_run(
        required_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        rejected.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].coverage_state"
    );
    assert_eq!(harness.counts()?, before_rejection);

    let (after_optional, optional_criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        after_required,
        "run_not_applicable_optional",
        &[(
            "Criterion where evidence is optional.",
            EvidenceRequirement::Optional,
        )],
    )?;
    assert_eq!(optional_criteria[0].acceptance_criterion_id, required_id);
    let mut optional_request = record_run_request(
        "req_run_not_applicable_optional",
        "idem_run_not_applicable_optional",
        false,
        Some(after_optional),
        &task_id,
        &change_unit_id,
    );
    optional_request.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: required_id,
        },
        coverage_state: EvidenceCoverageUpdateState::NotApplicable,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let committed = harness.service.record_run(
        optional_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(
        committed.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "not_applicable"
    );
    Ok(())
}

#[test]
fn supplemental_evidence_claim_identity_is_task_scoped_and_statement_is_immutable(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (first_task_id, first_change_unit_id) =
        create_task_with_change_unit(&harness, "task_scoped_evidence_claim")?;
    let shared_claim_id = volicord_types::EvidenceClaimId::new("claim_shared_between_tasks");

    let mut first_run = record_run_request(
        "req_task_scoped_claim_first",
        "idem_task_scoped_claim_first",
        false,
        Some(2),
        &first_task_id,
        &first_change_unit_id,
    );
    first_run.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: shared_claim_id.clone(),
            statement: "Statement owned by the first Task.".to_owned(),
        },
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let first_response = harness
        .service
        .record_run(first_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(first_response.response_value["base"]["state_version"], 3);

    let second_intake = harness.service.intake(
        intake_request(
            "req_task_scoped_claim_second_task",
            "idem_task_scoped_claim_second_task",
            false,
            Some(3),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_task_id = response_record_id(&second_intake.response_value, "task_ref");
    let second_scope = harness.service.update_scope(
        update_scope_request(
            "req_task_scoped_claim_second_scope",
            "idem_task_scoped_claim_second_scope",
            false,
            Some(4),
            &second_task_id,
            ChangeUnitOperation::CreateCurrent,
            "Second Task claim scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_change_unit_id = response_record_id(&second_scope.response_value, "change_unit_ref");

    let mut second_run = record_run_request(
        "req_task_scoped_claim_second",
        "idem_task_scoped_claim_second",
        false,
        Some(5),
        &second_task_id,
        &second_change_unit_id,
    );
    second_run.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: shared_claim_id.clone(),
            statement: "Independent statement owned by the second Task.".to_owned(),
        },
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let second_response = harness
        .service
        .record_run(second_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(second_response.response_value["base"]["state_version"], 6);

    let claims = {
        let conn = harness.conn()?;
        let mut statement = conn.prepare(
            "SELECT task_id, statement
               FROM evidence_claims
              WHERE project_id = ?1
                AND evidence_claim_id = ?2
              ORDER BY task_id ASC",
        )?;
        let rows = statement
            .query_map(
                rusqlite::params![PROJECT_ID, shared_claim_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    assert_eq!(claims.len(), 2);
    assert!(claims.iter().any(|(task_id, statement)| {
        task_id == &first_task_id && statement == "Statement owned by the first Task."
    }));
    assert!(claims.iter().any(|(task_id, statement)| {
        task_id == &second_task_id && statement == "Independent statement owned by the second Task."
    }));

    let before_mutation = harness.counts()?;
    let mut mutated = record_run_request(
        "req_task_scoped_claim_mutation",
        "idem_task_scoped_claim_mutation",
        false,
        Some(6),
        &second_task_id,
        &second_change_unit_id,
    );
    mutated.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: shared_claim_id,
            statement: "Attempted mutation within the second Task.".to_owned(),
        },
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let rejected = harness
        .service
        .record_run(mutated, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before_mutation);
    Ok(())
}

#[test]
fn cooperative_observation_cannot_be_promoted_by_reuse_or_stale_artifact_ref(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "reused_observation")?;
    let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "reused_observation",
    )?;
    let target = EvidenceTarget::SupplementalClaim {
        evidence_claim_id: volicord_types::EvidenceClaimId::new("claim_reused_observation"),
        statement: "Strong observation can be reused in the current scope.".to_owned(),
    };

    let mut first = record_run_request(
        "req_reused_observation_source",
        "idem_reused_observation_source",
        false,
        Some(after_artifact),
        &task_id,
        &change_unit_id,
    );
    let mut first_artifact_input = existing_artifact_input(
        "artifact_input_reused_observation_source",
        artifact_ref.clone(),
    );
    first_artifact_input.evidence_target = Some(target.clone()).into();
    first.artifact_inputs = vec![first_artifact_input];
    let mut first_update =
        supported_evidence_update("Strong observation can be reused in the current scope.");
    first_update.target = target.clone();
    first.evidence_updates = vec![first_update];
    let source_response = harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    let source_state_version = source_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("source state version should be present");
    let source_observation_id = source_response.response_value["evidence_observations"][0]
        ["observation_id"]
        .as_str()
        .expect("source observation ID should be present")
        .to_owned();
    assert_eq!(
        source_response.response_value["evidence_observations"][0]["source_kind"],
        "agent_report"
    );

    let source_observation_ref = state_ref(
        StateRecordKind::EvidenceObservation,
        &source_observation_id,
        &ProjectId::new(PROJECT_ID),
        Some(&TaskId::new(&task_id)),
        Some(source_state_version),
    );
    let mut caller_artifact_ref = artifact_ref.clone();
    caller_artifact_ref.display_name = "caller-supplied-stale-name.bin".to_owned();
    caller_artifact_ref.content_type = None.into();
    caller_artifact_ref.sha256 = None.into();
    caller_artifact_ref.size_bytes = None.into();
    caller_artifact_ref.integrity_status = ArtifactIntegrityStatus::Corrupt;
    caller_artifact_ref.availability = ArtifactAvailability::Missing;
    caller_artifact_ref.created_by_run_ref = None.into();
    caller_artifact_ref.created_by_actor_source = None.into();
    caller_artifact_ref.storage_ref = None.into();

    let mut reuse = record_run_request(
        "req_reused_observation_current",
        "idem_reused_observation_current",
        false,
        Some(source_state_version),
        &task_id,
        &change_unit_id,
    );
    reuse.evidence_updates = vec![EvidenceCoverageUpdate {
        target: target.clone(),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: vec![source_observation_ref],
        supporting_artifact_refs: vec![caller_artifact_ref],
        gap_refs: Vec::new(),
    }];
    let before_reuse = harness.counts()?;
    let reused_response = harness
        .service
        .record_run(reuse, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        reused_response.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        reused_response.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].observation_refs"
    );
    assert_eq!(harness.counts()?, before_reuse);
    Ok(())
}

#[test]
fn record_run_rejects_supported_evidence_without_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_missing_provenance")?;
    let mut request = record_run_request(
        "req_run_missing_provenance",
        "idem_run_missing_provenance",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let mut evidence_update = supported_evidence_update("Claim without provenance.");
    evidence_update.provenance = None;
    request.evidence_updates = vec![evidence_update];
    let before = harness.counts()?;

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].provenance"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_promotes_zero_byte_artifact_with_real_empty_sha256() -> Result<(), Box<dyn Error>> {
    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_zero_artifact")?;
    let mut stage_request = stage_artifact_request(
        "req_stage_zero_artifact",
        Some("idem_stage_zero_artifact"),
        false,
        Some(2),
        &task_id,
    );
    stage_request.safe_bytes_or_notice = String::new();
    stage_request.expected_sha256 = Some(EMPTY_SHA256.to_owned()).into();
    stage_request.expected_size_bytes = Some(0).into();
    let stage_response = harness
        .service
        .stage_artifact(stage_request, invocation(OperationCategory::AgentWorkflow))?;
    let handle: StagedArtifactHandle =
        serde_json::from_value(stage_response.response_value["staged_artifact_handle"].clone())?;

    let mut request = record_run_request(
        "req_run_zero_artifact",
        "idem_run_zero_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_zero",
        handle,
        Some("empty_report"),
        Some("Zero-byte artifact was registered."),
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present");
    let artifact_row = persistent_artifact_row(&harness, artifact_id)?;

    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["sha256"],
        EMPTY_SHA256
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["size_bytes"],
        0
    );
    assert_eq!(artifact_row.integrity_status, "verified");
    assert_eq!(artifact_row.sha256.as_deref(), Some(EMPTY_SHA256));
    assert_eq!(artifact_row.size_bytes, Some(0));
    Ok(())
}

#[test]
fn corrupt_artifact_blocks_evidence_and_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "corrupt_evidence_artifact")?;
    let acceptance_criterion_id = active_acceptance_criterion_id(&harness, &task_id)?;
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "corrupt_evidence_artifact", 2)?;

    let mut request = record_run_request(
        "req_run_corrupt_evidence_artifact",
        "idem_run_corrupt_evidence_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let mut artifact_input = artifact_input_for_handle(
        "artifact_input_corrupt",
        handle,
        Some("validation_report"),
        Some("Corrupt integrity evidence."),
    );
    artifact_input.evidence_target = Some(EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: volicord_types::AcceptanceCriterionId::new(
            &acceptance_criterion_id,
        ),
    })
    .into();
    request.artifact_inputs = vec![artifact_input];
    request.evidence_updates = vec![evidence_update_for_acceptance_criterion(
        supported_evidence_update("Corrupt integrity evidence."),
        &volicord_types::AcceptanceCriterionId::new(acceptance_criterion_id),
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Corrupt integrity evidence.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();

    set_artifact_integrity(&harness, &artifact_id, "corrupt", None, None, None)?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_corrupt_evidence_artifact",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: true,
                pending_user_actions: false,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let artifact_ref = &status.response_value["evidence_summary"]["coverage_items"][0]
        ["supporting_artifact_refs"][0];

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "stale"
    );
    assert_eq!(artifact_ref["availability"], "integrity_failed");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert!(artifact_ref["content_type"].is_null());
    assert!(artifact_ref["sha256"].is_null());
    assert!(artifact_ref["size_bytes"].is_null());
    assert_close_blocker(&status.response_value, "artifact_unavailable");

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_corrupt_evidence_artifact",
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
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn corrupt_artifact_is_not_linkable_as_existing_artifact() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "corrupt_artifact")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "corrupt")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let before = harness.counts()?;
    set_artifact_integrity(
        &harness,
        &artifact_id,
        "corrupt",
        artifact_ref.content_type.as_ref().map(String::as_str),
        artifact_ref.sha256.as_ref().map(String::as_str),
        artifact_ref.size_bytes.as_ref().copied(),
    )?;

    let mut request = record_run_request(
        "req_run_corrupt_existing",
        "idem_run_corrupt_existing",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_corrupt_existing",
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
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn verified_existing_artifact_ref_missing_integrity_fact_is_rejected() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "missing_ref_fact")?;
    let (state_version, mut artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "missing_ref")?;
    artifact_ref.sha256 = RequiredNullable::null();
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_existing_ref_fact",
        "idem_run_missing_existing_ref_fact",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_missing_existing_ref_fact",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn missing_persistent_artifact_body_blocks_evidence_and_close_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "missing_body")?;
    let before_counts = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, fixture.artifact_id())?;

    fs::remove_file(&fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "stale"
    );
    assert_eq!(artifact_ref["availability"], "missing");
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&status, &harness.runtime_home_path);

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&check, &harness.runtime_home_path);
    assert_eq!(harness.counts()?, before_counts);
    assert_eq!(
        persistent_artifact_row(&harness, fixture.artifact_id())?,
        before_row
    );
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

#[test]
fn changed_persistent_artifact_body_blocks_evidence_and_close_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "changed_body")?;
    let before_counts = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, fixture.artifact_id())?;

    fs::write(&fixture.body_path, b"{\"fixture\":\"changed\"}")?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "stale"
    );
    assert_eq!(artifact_ref["availability"], "integrity_failed");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert_close_blocker(&status.response_value, "artifact_unavailable");

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_eq!(harness.counts()?, before_counts);
    assert_eq!(
        persistent_artifact_row(&harness, fixture.artifact_id())?,
        before_row
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_escape_persistent_artifact_body_is_unusable_without_path_leak(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "symlink_escape")?;
    let before_counts = harness.counts()?;
    let outside_path = harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join("outside-artifact-store.json");
    fs::write(&outside_path, b"{\"fixture\":\"symlink_escape\"}")?;
    fs::remove_file(&fixture.body_path)?;
    std::os::unix::fs::symlink(&outside_path, &fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(artifact_ref["availability"], "unusable");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&status, &harness.runtime_home_path);

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&check, &harness.runtime_home_path);
    assert_eq!(harness.counts()?, before_counts);
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_within_artifact_store_keeps_persistent_artifact_usable() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "symlink_inside")?;
    let original_bytes = fs::read(&fixture.body_path)?;
    let inside_target = fixture
        .body_path
        .parent()
        .expect("artifact body has parent")
        .join("symlink-inside-target.json");
    fs::write(&inside_target, original_bytes)?;
    fs::remove_file(&fixture.body_path)?;
    std::os::unix::fs::symlink(&inside_target, &fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(artifact_ref["availability"], "available");
    assert_eq!(artifact_ref["integrity_status"], "verified");
    assert_no_close_blocker(&status.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn record_run_corrupt_staged_artifact_metadata_rejects_without_effect() -> Result<(), Box<dyn Error>>
{
    for (suffix, artifact_json) in [("malformed", corrupt_owner_json()), ("non_object", "[]")] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_stage_metadata_{suffix}"))?;
        let handle =
            stage_artifact_for_record_run(&harness, &task_id, &format!("metadata_{suffix}"), 2)?;
        let handle_id = handle.handle_id.as_str().to_owned();
        set_artifact_staging_artifact_json(&harness, &handle_id, artifact_json)?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_run_stage_metadata_{suffix}"),
            &format!("idem_run_stage_metadata_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            &format!("artifact_input_metadata_{suffix}"),
            handle,
            None,
            None,
        )];
        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_owner_state_rejection(
            &response,
            "artifact_staging",
            &handle_id,
            "artifact_json",
            &harness.runtime_home_path,
        );
        assert_eq!(harness.counts()?, before, "case {suffix}");
        assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    }
    Ok(())
}

#[test]
fn record_run_staged_artifact_without_display_name_uses_handle_id() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_default_display")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "default_display", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    set_artifact_staging_artifact_json(&harness, &handle_id, "{}")?;

    let mut request = record_run_request(
        "req_run_stage_default_display",
        "idem_run_stage_default_display",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_default_display",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["registered_artifacts"][0]["display_name"],
        handle_id
    );
    Ok(())
}

#[test]
fn record_run_staged_artifact_actor_source_mismatch_rejects_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_source")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_source", 2)?;
    handle.created_by_actor_source = ActorSource::agent_connection("forged_connection");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_source",
        "idem_run_stage_source",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_source",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_actor_source_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_expired_staged_artifact_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_expired")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_expired", 2)?;
    expire_staged_artifact(&harness, handle.handle_id.as_str())?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_expired",
        "idem_run_stage_expired",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_expired",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_staged_artifact_uses_semantic_expiry_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_boundary")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_boundary", 2)?;
    clock.advance(Duration::seconds(24 * 60 * 60 - 1));

    let mut request = record_run_request(
        "req_run_stage_boundary_before",
        "idem_run_stage_boundary_before",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_boundary_before",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");

    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_boundary_exact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_boundary_exact", 2)?;
    clock.advance(Duration::hours(24));
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_boundary_exact",
        "idem_run_stage_boundary_exact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_boundary_exact",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_staged_artifact_accepts_equivalent_offset_expiration() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_offset")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_offset", 2)?;
    handle.expires_at = volicord_types::UtcTimestamp::parse("2026-06-19T09:00:00+09:00")?;

    let mut request = record_run_request(
        "req_run_stage_offset",
        "idem_run_stage_offset",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_offset",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    Ok(())
}

#[test]
fn record_run_invalid_stored_staged_artifact_expiration_is_corrupt_state(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_bad_expires")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_bad_expires", 2)?;
    set_staged_artifact_expires_at(&harness, handle.handle_id.as_str(), "tomorrow")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_bad_expires",
        "idem_run_stage_bad_expires",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_bad_expires",
        handle.clone(),
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_value_rejection(
        &response,
        "artifact_staging",
        handle.handle_id.as_str(),
        "expires_at",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        artifact_staging_status(&harness, handle.handle_id.as_str())?,
        "staged"
    );
    Ok(())
}

#[test]
fn record_run_rejects_future_reversed_and_out_of_range_staging_windows_without_effect(
) -> Result<(), Box<dyn Error>> {
    for (suffix, created_at, expires_at) in [
        (
            "future_created",
            "2026-06-18T00:00:01Z",
            "2026-06-19T00:00:00Z",
        ),
        (
            "reversed_window",
            "2026-06-19T00:00:00Z",
            "2026-06-18T00:00:00Z",
        ),
        (
            "out_of_range_created",
            "9999-12-31T23:59:59-23:59",
            "9999-12-31T23:59:59Z",
        ),
    ] {
        let mut harness = MethodHarness::new()?;
        harness.use_clock(ManualClock::at("2026-06-18T00:00:00Z"));
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_stage_{suffix}"))?;
        let handle = stage_artifact_for_record_run(&harness, &task_id, suffix, 2)?;
        set_staged_artifact_window(&harness, handle.handle_id.as_str(), created_at, expires_at)?;
        let before = harness.counts()?;
        let before_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;

        let mut request = record_run_request(
            &format!("req_run_stage_{suffix}"),
            &format!("idem_run_stage_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            &format!("artifact_input_{suffix}"),
            handle.clone(),
            None,
            None,
        )];
        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, before, "case {suffix}");
        assert_eq!(
            artifact_staging_status(&harness, handle.handle_id.as_str())?,
            "staged",
            "case {suffix}"
        );
        let after_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor, "case {suffix}");
    }
    Ok(())
}

#[test]
fn record_run_checksum_mismatch_rejects_and_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut input = artifact_input_for_handle("artifact_input_sha", handle, None, None);
    input.expected_sha256 =
        Some("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).into();
    let mut request = record_run_request(
        "req_run_stage_sha",
        "idem_run_stage_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![input];
    request.close_assessment = Some(close_assessment_with_risks(
        "Rejected close basis.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_checksum_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_body_checksum_mismatch_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_body_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    fs::write(
        staged_artifact_body_path(&harness, &handle_id)?,
        vec![b'x'; handle.size_bytes as usize],
    )?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_sha",
        "idem_run_body_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_sha",
        handle,
        Some("validation_report"),
        Some("Tampered body should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Tampered body should not promote.",
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Tampered body should not promote.",
        Vec::new(),
    ))
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_body_size_mismatch_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_body_size")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_size", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    fs::write(
        staged_artifact_body_path(&harness, &handle_id)?,
        vec![b'x'; handle.size_bytes as usize + 1],
    )?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_size",
        "idem_run_body_size",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_size",
        handle,
        Some("validation_report"),
        Some("Resized body should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Resized body should not promote.",
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Resized body should not promote.",
        Vec::new(),
    ))
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_staging_path_outside_artifact_store_rolls_back_all_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_body_path_outside")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_path_outside", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    set_artifact_staging_tmp_path(&harness, &handle_id, "tmp/not-under-artifacts.txt")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_path_outside",
        "idem_run_body_path_outside",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_path_outside",
        handle,
        Some("validation_report"),
        Some("Invalid staging path should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Invalid staging path should not promote.",
    )];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_dry_run_and_idempotency_replay_have_no_extra_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_replay")?;
    let before_dry = harness.counts()?;
    let dry_run = harness.service.record_run(
        record_run_request(
            "req_run_dry",
            "idem_run_dry",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(harness.counts()?, before_dry);

    let request = record_run_request(
        "req_run_replay",
        "idem_run_replay",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let first = harness.service.record_run(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}
