use super::*;

#[test]
fn status_is_read_only_including_dry_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status", None, false, None, None),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_typed_result_contract::<StatusResult>(&response);
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], false);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(harness.counts()?, before);

    let dry_run = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_dry",
                Some("idem_status_dry"),
                true,
                Some(0),
                None,
            ),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_typed_result_contract::<StatusResult>(&dry_run);

    assert_eq!(dry_run.response_value["base"]["response_kind"], "result");
    assert_eq!(dry_run.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(dry_run.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_renders_idle_timeout_invalidation_without_mutating_row() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_auth_expired")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "wa_status_future",
        2,
        "2026-06-18T00:00:00.000Z",
        "2026-06-18T00:15:00Z",
    )?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:15:00Z");
    harness.use_generator_and_clock(id_generator, clock);
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_auth_expired", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["write_ticket_summary"]["status"],
        "invalidated"
    );
    assert_eq!(
        response.response_value["active_task"]["write_ticket_summary"]["status"],
        "invalidated"
    );
    assert_eq!(
        response.response_value["write_ticket_summary"]["invalidation_reason"],
        "idle_timeout"
    );
    assert_eq!(write_ticket_status(&harness, "wa_status_future")?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_rejects_unrepresentable_stored_write_ticket_expiry_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "status_write_ticket_clock_range")?;
    let write_ticket_id = "wa_status_clock_range";
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        write_ticket_id,
        2,
        "2026-06-18T00:00:00Z",
        "2026-06-18T00:15:00Z",
    )?;
    harness.conn()?.execute(
        "UPDATE write_tickets
            SET idle_expires_at = '9999-12-31T23:59:59-23:59'
          WHERE project_id = ?1
            AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, write_ticket_id],
    )?;
    let before = harness.counts()?;
    let before_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_write_ticket_clock_range",
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

    assert_owner_state_value_rejection(
        &response,
        "write_tickets",
        write_ticket_id,
        "idle_expires_at",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    let after_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_floor, before_floor);
    Ok(())
}

#[test]
fn status_selects_latest_write_ticket_by_basis_state_version_when_ids_disagree(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "status_write_ticket_authority_order")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_status_write_ticket_authority_order",
            "idem_status_write_ticket_authority_order",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(requested.response_value["base"]["state_version"], 3);
    for (write_ticket_id, basis_state_version) in [("wa_z_old", 1), ("wa_a_new", 2)] {
        insert_active_write_ticket_with_timestamps(
            &harness,
            &task_id,
            &change_unit_id,
            write_ticket_id,
            basis_state_version,
            "2026-06-18T00:00:00.000000500Z",
            "2999-01-01T00:00:00Z",
        )?;
    }
    harness.use_clock(ManualClock::at("2026-06-18T00:00:01Z"));
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_write_ticket_authority_order_read",
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
        response.response_value["write_ticket_summary"]["status"],
        "active"
    );
    assert_eq!(
        response.response_value["write_ticket_summary"]["basis_state_version"],
        2
    );
    assert_eq!(
        response.response_value["write_ticket_summary"]["write_ticket_ref"]["record_id"],
        "wa_a_new"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_projects_control_policy_ticket_basis_invalidation_and_completion_claim(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let policy_state_version = harness.counts()?.state_version;
    let mut intake = intake_request(
        "req_status_authority_contract_task",
        "idem_status_authority_contract_task",
        false,
        Some(policy_state_version),
        RequestedMode::Direct,
    );
    intake.requested_control_level = RequestedControlLevel::Light;
    let created = harness
        .service
        .intake(intake, invocation(OperationCategory::AgentWorkflow))?;
    let task_id = response_record_id(&created.response_value, "task_ref");
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_status_authority_contract_scope",
            "idem_status_authority_contract_scope",
            false,
            Some(policy_state_version + 1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Initial status contract scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let prepared = harness.service.prepare_write(
        prepare_write_request(
            "req_status_authority_contract_prepare",
            "idem_status_authority_contract_prepare",
            Some(policy_state_version + 2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let validity_basis = prepared.response_value["write_ticket"]["validity_basis"].clone();
    assert_eq!(validity_basis["task_id"], task_id);
    assert_eq!(validity_basis["change_unit_id"], change_unit_id);
    assert_eq!(validity_basis["baseline_ref"], "baseline_test");
    assert_eq!(validity_basis["approval_basis_refs"], json!([]));

    harness.service.update_scope(
        update_scope_request(
            "req_status_authority_contract_change",
            "idem_status_authority_contract_change",
            false,
            Some(policy_state_version + 3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Materially changed status contract scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_authority_contract_read",
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

    let active_task = &status.response_value["active_task"];
    assert_eq!(active_task["requested_control_level"], "light");
    assert_eq!(active_task["effective_control_level"], "light");
    assert_eq!(
        active_task["control_level_reason"],
        "Core selected effective control `light` from the caller request and project workflow policy."
    );
    assert_eq!(active_task["project_policy"]["policy_version"], 1);
    assert_eq!(active_task["project_policy"]["source"], "test_fixture");
    assert_eq!(
        status.response_value["write_ticket_summary"]["validity_basis"],
        validity_basis
    );
    assert_eq!(
        status.response_value["write_ticket_summary"]["invalidation_reason"],
        "scope_revision_changed"
    );
    assert_eq!(
        active_task["write_ticket_summary"],
        status.response_value["write_ticket_summary"]
    );
    assert_eq!(
        status.response_value["authority_receipt"]["completion_claim_allowed"],
        false
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_include_evidence_returns_current_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_evidence")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_evidence",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_evidence", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: StatusInclude {
                task: true,
                pending_user_actions: false,
                write_ticket: false,
                evidence: true,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_status_evidence_check",
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
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["evidence_state"],
        "accepted_for_close"
    );
    assert_eq!(
        response.response_value["summary_card"]["evidence"],
        response.response_value["evidence_gate"]["state"]
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["target"]["target_kind"],
        "acceptance_criterion"
    );
    assert_eq!(
        response.response_value["active_task"]["evidence_summary"],
        response.response_value["evidence_summary"]
    );
    assert_eq!(
        response.response_value["active_task"]["evidence_gate"],
        response.response_value["evidence_gate"]
    );
    assert_eq!(
        response.response_value["evidence_gate"],
        check.response_value["evidence_gate"]
    );
    assert_eq!(
        response.response_value["summary_card"]["evidence"],
        check.response_value["summary_card"]["evidence"]
    );
    assert_eq!(
        check.response_value["state"]["evidence_gate"],
        check.response_value["evidence_gate"]
    );
    assert_field_absent(&response.response_value, "current_close_basis");
    assert_field_absent(&response.response_value, "close_state");
    assert_field_absent(&response.response_value, "close_blockers");
    assert_field_absent(&response.response_value, "risk_acceptance_coverage");
    assert_field_absent(&response.response_value["active_task"], "close_state");
    assert_field_absent(&response.response_value["active_task"], "close_blockers");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_receipt_uses_authority_commit_order_for_latest_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_latest_run")?;
    let first_run_id = RunId::new("run_zzzz_status_order_first");
    let second_run_id = RunId::new("run_aaaa_status_order_second");
    let mut first = record_run_request(
        "req_status_latest_run_first",
        "idem_status_latest_run_first",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    first.run_id = RequiredNullable::some(first_run_id.clone());
    let first_response = harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    let first_state_version = first_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("first Run state version");
    let mut second = record_run_request(
        "req_status_latest_run_second",
        "idem_status_latest_run_second",
        false,
        Some(first_state_version),
        &task_id,
        &change_unit_id,
    );
    second.run_id = RequiredNullable::some(second_run_id.clone());
    harness
        .service
        .record_run(second, invocation(OperationCategory::AgentWorkflow))?;
    harness.conn()?.execute(
        "UPDATE runs
            SET created_at = '2026-07-12T00:00:00.000Z'
          WHERE project_id = ?1
            AND run_id IN (?2, ?3)",
        rusqlite::params![PROJECT_ID, first_run_id.as_str(), second_run_id.as_str()],
    )?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_latest_run_receipt",
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
        status.response_value["authority_receipt"]["latest_run_ref"]["record_id"],
        second_run_id.as_str()
    );
    Ok(())
}

#[test]
fn record_run_evidence_without_close_basis_appears_attached() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "attached_evidence")?;
    let mut request = record_run_request(
        "req_attached_evidence",
        "idem_attached_evidence",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = vec![supported_evidence_update(
        "Attached evidence supports a recorded claim.",
    )];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["evidence_summary"]["evidence_state"],
        "attached"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert!(response.response_value["current_close_basis"].is_null());
    assert_eq!(
        response.response_value["state"]["evidence_summary"]["evidence_state"],
        "attached"
    );
    Ok(())
}

#[test]
fn next_action_dedup_ignores_presentation_role_and_selection_uses_primary_role() {
    for owner_method in [
        MethodName::UpdateScope,
        MethodName::PrepareWrite,
        MethodName::StageArtifact,
        MethodName::RecordRun,
        MethodName::RequestUserAction,
        MethodName::CloseTask,
    ] {
        assert_eq!(
            allowed_operation_categories(Some(owner_method)),
            vec![OperationCategory::AgentWorkflow]
        );
    }
    assert_eq!(
        allowed_operation_categories(Some(MethodName::ResolveUserAction)),
        vec![OperationCategory::UserOnly]
    );
    assert_eq!(
        allowed_operation_categories(Some(MethodName::ReconcileChanges)),
        vec![
            OperationCategory::AgentWorkflow,
            OperationCategory::LocalRecovery
        ]
    );
    assert!(allowed_operation_categories(None).is_empty());

    let primary = NextActionSummary {
        presentation_role: NextActionPresentationRole::Primary,
        action_kind: NextActionKind::RecordRun,
        owner_method: Some(MethodName::RecordRun),
        allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
        label: "Record the current result.".to_owned(),
        blocking_question: None,
        expected_state_version: RequiredNullable::null(),
        required_refs: Vec::new(),
    };
    let mut additional_duplicate = primary.clone();
    additional_duplicate.presentation_role = NextActionPresentationRole::Additional;
    additional_duplicate.expected_state_version = RequiredNullable::some(41);

    let deduplicated = super::super::status::unique_next_actions(vec![
        additional_duplicate.clone(),
        primary.clone(),
    ]);
    assert_eq!(deduplicated.len(), 1);

    let distinct_additional = NextActionSummary {
        label: "Additional action.".to_owned(),
        ..additional_duplicate
    };
    let reordered = [distinct_additional, primary.clone()];
    let selected =
        primary_next_action(&reordered, &[]).expect("primary action should be selected by role");
    assert_eq!(selected, &primary);

    let older_ref = StateRecordRef {
        record_kind: StateRecordKind::Task,
        record_id: RecordId::new("task_same_identity"),
        project_id: ProjectId::new(PROJECT_ID),
        task_id: Some(TaskId::new("task_context_old")).into(),
        produced_at_state_version: Some(3).into(),
    };
    let newer_ref = StateRecordRef {
        task_id: Some(TaskId::new("task_context_new")).into(),
        produced_at_state_version: Some(8).into(),
        ..older_ref.clone()
    };
    let deduplicated_refs = super::super::status::unique_next_actions(vec![NextActionSummary {
        required_refs: vec![newer_ref.clone(), older_ref],
        ..primary.clone()
    }]);
    assert_eq!(deduplicated_refs[0].required_refs, vec![newer_ref]);

    let mut user_only_action = NextActionSummary {
        owner_method: Some(MethodName::ResolveUserAction),
        expected_state_version: RequiredNullable::some(99),
        ..primary.clone()
    };
    normalize_next_action_collection(std::slice::from_mut(&mut user_only_action), 8);
    assert_eq!(
        user_only_action.allowed_operation_categories,
        vec![OperationCategory::UserOnly]
    );
    assert!(user_only_action.expected_state_version.is_none());

    let mut read_action = NextActionSummary {
        owner_method: Some(MethodName::Status),
        expected_state_version: RequiredNullable::some(99),
        ..primary
    };
    normalize_next_action_collection(std::slice::from_mut(&mut read_action), 8);
    assert!(read_action.allowed_operation_categories.is_empty());
    assert!(read_action.expected_state_version.is_none());
}

#[test]
fn status_ready_close_uses_empty_blockers_only_after_computation() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_ready_empty")?;
    let after_run = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_ready_empty",
        true,
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_run,
        "status_ready_empty",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_ready_empty", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(status.response_value["close_state"], "ready");
    assert_eq!(status.response_value["close_blockers"], json!([]));
    assert_eq!(status.response_value["active_task"]["close_state"], "ready");
    assert_eq!(
        status.response_value["active_task"]["close_blockers"],
        json!([])
    );
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(
        status.response_value["authority_receipt"]["close_state"],
        "ready"
    );
    assert_eq!(
        status.response_value["authority_receipt"]["next_actor"],
        "agent"
    );
    assert_eq!(
        status.response_value["authority_receipt"]["next_action"]["action_kind"],
        "close_task"
    );
    assert_eq!(
        status.response_value["authority_receipt"]["next_action"]["owner_method"],
        "volicord.close_task"
    );
    assert_eq!(
        status.response_value["authority_receipt"]["completion_claim_allowed"],
        true
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_include_false_omits_optional_sections_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_flags")?;
    record_close_evidence(&harness, &task_id, &change_unit_id, 2, "status_flags", true)?;
    let before = harness.counts()?;

    let none = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_flags_none", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert!(none.response_value["active_task"].is_null());
    assert!(none.response_value["write_ticket_summary"].is_null());
    assert_field_absent(&none.response_value, "evidence_summary");
    assert_field_absent(&none.response_value, "evidence_gate");
    assert_field_absent(&none.response_value, "close_state");
    assert_field_absent(&none.response_value, "current_close_basis");
    assert_field_absent(&none.response_value, "risk_acceptance_coverage");
    assert_field_absent(&none.response_value, "close_blockers");
    assert_field_absent(&none.response_value, "guarantee_display");
    assert_no_close_next_actions(&none.response_value);
    assert_eq!(
        none.response_value["authority_receipt"]["task_ref"]["record_id"],
        task_id
    );
    assert_eq!(
        none.response_value["authority_receipt"]["state_version"],
        before.state_version
    );
    assert!(none.response_value["authority_receipt"]["evidence_gate"].is_object());
    assert!(none.response_value["authority_receipt"]["close_blockers"].is_array());

    let evidence_only = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_flags_evidence",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: true,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(evidence_only.response_value["active_task"].is_null());
    assert_eq!(
        evidence_only.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert!(evidence_only.response_value["evidence_gate"].is_object());
    assert_field_absent(&evidence_only.response_value, "close_state");
    assert_field_absent(&evidence_only.response_value, "close_blockers");
    assert_field_absent(&evidence_only.response_value, "guarantee_display");
    assert_no_close_next_actions(&evidence_only.response_value);

    let close_only = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_flags_close", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(close_only.response_value["active_task"].is_null());
    assert_field_absent(&close_only.response_value, "evidence_summary");
    assert!(close_only.response_value["evidence_gate"].is_object());
    assert_field_absent(&close_only.response_value, "guarantee_display");
    assert_close_blocker(&close_only.response_value, "missing_final_acceptance");

    let guarantees_only = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_flags_guarantee",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(guarantees_only.response_value["active_task"].is_null());
    assert_field_absent(&guarantees_only.response_value, "evidence_summary");
    assert_field_absent(&guarantees_only.response_value, "close_state");
    assert_field_absent(&guarantees_only.response_value, "close_blockers");
    assert_eq!(
        guarantees_only.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_receipt_fails_closed_on_corrupt_close_basis_for_every_include_shape(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_close_not_read")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let excluded = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_close_not_read_excluded",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &excluded,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);

    let selected = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_close_not_read_selected",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &selected,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn fresh_project_registration_creates_baseline_enforcement_profile() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let profile_json = harness.project_enforcement_profile_json()?;
    assert_eq!(profile_json, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON);

    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let record = store.project_enforcement_profile()?;
    assert_eq!(record.project_id, PROJECT_ID);
    assert_eq!(record.profile.profile_id, "baseline_cooperative");
    assert_eq!(record.profile.enabled_mechanisms.len(), 0);
    Ok(())
}

#[test]
fn status_guarantee_include_false_does_not_read_corrupt_profile() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_profile_skip")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_profile_skip",
        true,
    )?;
    harness.set_project_enforcement_profile_json(corrupt_owner_json())?;
    let before = harness.counts()?;

    let excluded = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_profile_skip_excluded",
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
                evidence: false,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(excluded.response_value["base"]["response_kind"], "result");
    assert_field_absent(&excluded.response_value, "guarantee_display");
    assert_field_absent(&excluded.response_value["active_task"], "guarantee_display");
    assert_eq!(harness.counts()?, before);

    let selected = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_profile_skip_selected",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &selected,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_guarantee_include_true_rejects_unsupported_profile_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_project_enforcement_profile_json(
        &json!({
            "profile_id": "baseline_cooperative",
            "guarantee_level": "unsupported",
            "enabled_mechanisms": [],
            "source": "baseline_scope",
            "status": "active"
        })
        .to_string(),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_profile_unsupported", None, false, None, None),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_guarantee_include_true_rejects_missing_profile_fields() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    harness.set_project_enforcement_profile_json(
        &json!({
            "profile_id": "baseline_cooperative",
            "enabled_mechanisms": [],
            "source": "baseline_scope",
            "status": "active"
        })
        .to_string(),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_profile_missing", None, false, None, None),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_owner_state_rejection(
        &response,
        "project_state",
        PROJECT_ID,
        "enforcement_profile_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn guarantee_display_uses_verified_invocation_without_profile_elevation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_guarantee_invocation", None, false, None, None),
            continuity_page: None,
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert!(status.response_value["guarantee_display"]["basis"]
        .as_str()
        .is_some_and(|basis| {
            basis.contains(AGENT_ACTOR_SOURCE)
                && basis.contains("baseline_cooperative")
                && basis.contains("read")
                && basis.contains("enabled mechanisms: none")
                && basis.contains("no stronger enforcement")
        }));
    assert_eq!(
        status.response_value["guarantee_display"]["capability_refs"][0]["record_kind"],
        "agent_connection"
    );
    assert_eq!(
        status.response_value["guarantee_display"]["capability_refs"][0]["record_id"],
        CONNECTION_ID
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_reports_exact_missing_residual_risk_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_risk")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_risk",
        vec![
            residual_risk_input("First status risk."),
            residual_risk_input("Second status risk."),
        ],
    )?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "status_risk",
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_risk", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let coverage = response.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    let projected_ids = coverage
        .iter()
        .map(|item| item["risk_id"].as_str().expect("risk_id").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(projected_ids, risk_ids);
    assert!(coverage.iter().all(|item| item["accepted"] == false));
    assert!(coverage
        .iter()
        .all(|item| item["missing_reason"] == "acceptance_required"));
    assert_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_close_shows_stale_final_acceptance_blocker_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_stale_final")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_stale_final_old",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "status_stale_final",
    )?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "status_stale_final_new",
        true,
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_stale_final", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(user_action_status(&harness, &final_judgment_id)?, "stale");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    let final_blocker = response.response_value["close_blockers"]
        .as_array()
        .expect("close blockers")
        .iter()
        .find(|blocker| blocker["code"] == "stale_final_acceptance")
        .expect("final acceptance blocker");
    assert!(final_blocker["related_refs"]
        .as_array()
        .expect("related refs")
        .iter()
        .any(|record_ref| record_ref["record_id"] == final_judgment_id));
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_continuity_page_defaults_bounds_and_traversal_are_exact() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "status_continuity_pages")?;
    let record_ids = (0..65)
        .map(|index| format!("continuity_{index:03}"))
        .collect::<Vec<_>>();
    insert_active_continuity_records(
        &harness,
        &task_id,
        &change_unit_id,
        &record_ids,
        "2026-06-18T00:00:01Z",
    )?;
    let before = harness.counts()?;

    let omitted = continuity_status(
        &harness,
        "req_status_continuity_default_omitted",
        &task_id,
        None,
    )?;
    let null = continuity_status(
        &harness,
        "req_status_continuity_default_null",
        &task_id,
        Some(RequiredNullable::null()),
    )?;
    for response in [&omitted, &null] {
        let page = &response.response_value["continuity_summary"];
        assert_eq!(page["page_info"]["total_count"], 65);
        assert_eq!(page["page_info"]["returned_count"], 8);
        assert_eq!(page["page_info"]["truncated"], true);
        assert_eq!(continuity_page_ids(page).len(), 8);
    }
    assert_eq!(
        continuity_page_ids(&omitted.response_value["continuity_summary"]),
        continuity_page_ids(&null.response_value["continuity_summary"])
    );

    for page_size in [1, 8, 64] {
        let response = continuity_status(
            &harness,
            &format!("req_status_continuity_size_{page_size}"),
            &task_id,
            Some(RequiredNullable::some(ContinuityPageRequest {
                page_size,
                cursor: RequiredNullable::null(),
            })),
        )?;
        let page = &response.response_value["continuity_summary"];
        assert_eq!(page["page_info"]["total_count"], 65);
        assert_eq!(page["page_info"]["returned_count"], page_size);
        assert_eq!(page["page_info"]["truncated"], true);
    }

    let first_one = continuity_status(
        &harness,
        "req_status_continuity_exact_first",
        &task_id,
        Some(RequiredNullable::some(ContinuityPageRequest {
            page_size: 1,
            cursor: RequiredNullable::null(),
        })),
    )?;
    let exact_cursor: ContinuityCursor = serde_json::from_value(
        first_one.response_value["continuity_summary"]["page_info"]["next_cursor"].clone(),
    )?;
    let exact_remainder = continuity_status(
        &harness,
        "req_status_continuity_exact_remainder",
        &task_id,
        Some(RequiredNullable::some(ContinuityPageRequest {
            page_size: 64,
            cursor: RequiredNullable::some(exact_cursor),
        })),
    )?;
    let exact_page = &exact_remainder.response_value["continuity_summary"];
    assert_eq!(exact_page["page_info"]["returned_count"], 64);
    assert_eq!(exact_page["page_info"]["truncated"], false);
    assert_eq!(exact_page["page_info"]["next_cursor"], Value::Null);

    let expected_ids = record_ids.iter().rev().cloned().collect::<Vec<_>>();
    let mut traversed_ids = Vec::new();
    let mut cursor = RequiredNullable::null();
    let mut page_number = 0;
    loop {
        let response = continuity_status(
            &harness,
            &format!("req_status_continuity_traversal_{page_number}"),
            &task_id,
            Some(RequiredNullable::some(ContinuityPageRequest {
                page_size: 8,
                cursor: cursor.clone(),
            })),
        )?;
        let page = &response.response_value["continuity_summary"];
        assert_eq!(page["page_info"]["total_count"], 65);
        traversed_ids.extend(continuity_page_ids(page));
        if page["page_info"]["truncated"] == false {
            assert_eq!(page["page_info"]["next_cursor"], Value::Null);
            break;
        }
        cursor = RequiredNullable::some(serde_json::from_value(
            page["page_info"]["next_cursor"].clone(),
        )?);
        page_number += 1;
    }
    assert_eq!(traversed_ids, expected_ids);
    assert_eq!(
        traversed_ids.iter().collect::<BTreeSet<_>>().len(),
        traversed_ids.len()
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_continuity_empty_page_and_invalid_controls_fail_closed() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_continuity_empty")?;
    let before = harness.counts()?;

    let empty = continuity_status(&harness, "req_status_continuity_empty", &task_id, None)?;
    let page = &empty.response_value["continuity_summary"];
    assert_eq!(page["items"], json!([]));
    assert_eq!(page["page_info"]["total_count"], 0);
    assert_eq!(page["page_info"]["returned_count"], 0);
    assert_eq!(page["page_info"]["truncated"], false);
    assert_eq!(page["page_info"]["next_cursor"], Value::Null);

    for (request_id, page, expected_field) in [
        (
            "req_status_continuity_zero",
            ContinuityPageRequest {
                page_size: 0,
                cursor: RequiredNullable::null(),
            },
            "continuity_page.page_size",
        ),
        (
            "req_status_continuity_over_max",
            ContinuityPageRequest {
                page_size: 65,
                cursor: RequiredNullable::null(),
            },
            "continuity_page.page_size",
        ),
        (
            "req_status_continuity_empty_cursor_id",
            ContinuityPageRequest {
                page_size: 8,
                cursor: RequiredNullable::some(ContinuityCursor {
                    updated_at: UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
                    continuity_record_id: ProjectContinuityRecordId::new(" "),
                }),
            },
            "continuity_page.cursor.continuity_record_id",
        ),
    ] {
        let response = continuity_status(
            &harness,
            request_id,
            &task_id,
            Some(RequiredNullable::some(page)),
        )?;
        assert_status_continuity_validation_rejection(&response, expected_field);
    }

    let ambiguous = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_continuity_ambiguous", None, false, None, None),
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
            continuity_page: Some(RequiredNullable::some(ContinuityPageRequest {
                page_size: 8,
                cursor: RequiredNullable::null(),
            })),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_status_continuity_validation_rejection(&ambiguous, "continuity_page");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

fn continuity_status(
    harness: &MethodHarness,
    request_id: &str,
    task_id: &str,
    continuity_page: Option<RequiredNullable<ContinuityPageRequest>>,
) -> Result<PipelineResponse, Box<dyn Error>> {
    Ok(harness.service.status(
        StatusRequest {
            envelope: envelope(request_id, None, false, None, Some(task_id)),
            include: StatusInclude {
                task: false,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: true,
            },
            continuity_page,
        },
        invocation(OperationCategory::Read),
    )?)
}

fn continuity_page_ids(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .expect("continuity items")
        .iter()
        .map(|item| {
            item["continuity_record_ref"]["record_id"]
                .as_str()
                .expect("continuity record id")
                .to_owned()
        })
        .collect()
}

fn insert_active_continuity_records(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    record_ids: &[String],
    updated_at: &str,
) -> Result<(), Box<dyn Error>> {
    let mut conn = harness.conn()?;
    let transaction = conn.transaction()?;
    for record_id in record_ids {
        transaction.execute(
            "INSERT INTO project_continuity_records (
                project_id, continuity_record_id, source_task_id, source_change_unit_id,
                kind, title, summary, rationale, applies_to_paths_json,
                applies_to_refs_json, source_refs_json, artifact_refs_json, status,
                supersedes_refs_json, review_triggers_json, created_at, updated_at,
                metadata_json
             ) VALUES (
                ?1, ?2, ?3, ?4, 'decision', ?2, ?2, NULL, '[]', '[]', '[]', '[]',
                'active', '[]', '[]', ?5, ?5, '{}'
             )",
            rusqlite::params![PROJECT_ID, record_id, task_id, change_unit_id, updated_at],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn assert_status_continuity_validation_rejection(
    response: &PipelineResponse,
    expected_field: &str,
) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        expected_field
    );
}
