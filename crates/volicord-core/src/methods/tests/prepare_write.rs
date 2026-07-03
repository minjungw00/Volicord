use super::*;

#[test]
fn prepare_write_allowed_issues_one_write_ticket_with_post_commit_basis(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_allowed")?;
    let sensitive_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_prepare_allowed_sensitive",
            "idem_prepare_allowed_sensitive",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let sensitive_judgment_id =
        response_record_id(&sensitive_judgment.response_value, "user_judgment_ref");
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_prepare_allowed_record",
            "idem_prepare_allowed_record",
            Some(3),
            &task_id,
            &sensitive_judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let id_generator =
        CountingDurableIdGenerator::new(["prepare_allowed_auth", "prepare_allowed_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_allowed",
        "idem_prepare_allowed",
        Some(4),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_authority_disclosure(&response.response_value);
    assert_eq!(response.response_value["write_ticket_effect"], "issued");
    let write_ticket_id = response.response_value["write_ticket_id"]
        .as_str()
        .expect("prepare_write should return a write ticket id")
        .to_owned();
    assert_eq!(
        response.response_value["write_ticket_ref"]["record_kind"],
        "write_ticket"
    );
    assert_eq!(
        response.response_value["write_ticket_ref"]["record_id"],
        write_ticket_id
    );
    assert_eq!(
        response.response_value["write_ticket"]["write_ticket_id"],
        write_ticket_id
    );
    assert_eq!(
        response.response_value["write_ticket"]["write_ticket_ref"],
        response.response_value["write_ticket_ref"]
    );
    assert_eq!(response.response_value["write_ticket"]["state"], "open");
    assert_eq!(
        response.response_value["write_ticket"]["scope"]["task_id"],
        task_id
    );
    assert_eq!(
        response.response_value["write_ticket"]["scope"]["change_unit_id"],
        change_unit_id
    );
    assert_eq!(
        response.response_value["write_ticket"]["path_patterns"]["allowed"],
        json!(["src/export.rs"])
    );
    assert_eq!(
        response.response_value["write_ticket"]["path_patterns"]["denied"],
        json!([])
    );
    assert_eq!(
        response.response_value["allowed_path_patterns"],
        json!(["src/export.rs"])
    );
    assert_eq!(response.response_value["denied_path_patterns"], json!([]));
    assert_eq!(
        response.response_value["write_ticket"]["observed_paths"],
        json!([])
    );
    assert_eq!(
        response.response_value["write_ticket"]["basis_state_version"],
        5
    );
    assert_eq!(
        response.response_value["write_ticket"]["control_surface"],
        response.response_value["control_surface"]
    );
    assert_eq!(
        response.response_value["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(response.response_value["base"]["state_version"], 5);
    assert_eq!(response.response_value["write_ticket_effect"], "issued");
    assert_eq!(
        response.response_value["write_ticket"]["basis_state_version"],
        5
    );
    assert_eq!(
        response.response_value["write_ticket"]["path_patterns"]["allowed"],
        json!(["src/export.rs"])
    );
    assert_eq!(
        response.response_value["active_user_judgment_refs"]
            .as_array()
            .expect("active judgment refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_tickets, before.write_tickets + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let ref_write_ticket_id = response_record_id(&response.response_value, "write_ticket_ref");
    assert_eq!(write_ticket_id, ref_write_ticket_id);
    assert_eq!(write_ticket_basis(&harness, &ref_write_ticket_id)?, 5);
    let (created_at, expires_at) = write_ticket_timestamps(&harness, &ref_write_ticket_id)?;
    assert_eq!(created_at, "2026-06-18T00:00:00Z");
    assert_eq!(expires_at, "2026-06-18T00:15:00Z");
    assert_eq!(
        response.response_value["write_ticket"]["expires_at"],
        expires_at
    );
    assert_eq!(
        response.response_value["write_ticket"]["expires_at"],
        expires_at
    );
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_prepare_allowed_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["base"]["state_version"], 5);
    let mut response_state = response.response_value["state"].clone();
    let status_state = status.response_value["active_task"].clone();
    response_state["guarantee_display"] = status_state["guarantee_display"].clone();
    response_state["write_ticket_summary"]["guarantee_display"] =
        status_state["write_ticket_summary"]["guarantee_display"].clone();
    assert_eq!(response_state, status_state);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 1);
    Ok(())
}

#[test]
fn change_unit_effect_contract_is_stored_and_returned() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_effect_contract_task",
            "idem_effect_contract_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let contract = ChangeUnitEffectContract {
        allowed_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
        forbidden_effects: vec![ChangeUnitEffectKind::ExternalNetwork],
        allowed_paths: vec!["src/export.rs".to_owned()],
        expected_outputs: vec!["Updated export behavior.".to_owned()],
        invariants: vec!["Do not alter unrelated exports.".to_owned()],
        evidence_expectations: vec!["Record a focused test run.".to_owned()],
        sensitive_action_expectations: vec!["No secret access is expected.".to_owned()],
    };
    let mut request = update_scope_request(
        "req_effect_contract_scope",
        "idem_effect_contract_scope",
        false,
        Some(1),
        &task_id,
        ChangeUnitOperation::CreateCurrent,
        "Effect-contract current scope.",
    );
    request.change_unit.effect_contract = Some(contract.clone());

    let response = harness
        .service
        .update_scope(request, invocation(OperationCategory::AgentWorkflow))?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_effect_contract_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let expected = serde_json::to_value(contract)?;
    assert_eq!(
        response.response_value["state"]["effect_contract"],
        expected
    );
    assert_eq!(
        status.response_value["active_task"]["effect_contract"],
        expected
    );
    Ok(())
}

#[test]
fn state_summary_reports_absent_effect_contract_as_null() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "no_effect_contract")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_no_effect_contract_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_no_effect_contract_prepare",
            "idem_no_effect_contract_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert!(status.response_value["active_task"]["effect_contract"].is_null());
    assert_eq!(response.response_value["decision"], "allowed");
    assert!(response.response_value["state"]["effect_contract"].is_null());
    Ok(())
}

#[test]
fn prepare_write_rejects_product_write_forbidden_by_effect_contract() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_forbid_write",
        ChangeUnitEffectContract {
            forbidden_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_contract_forbid_write",
            "idem_contract_forbid_write",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(
        &response.response_value,
        "effect_contract_forbids_product_file_write",
    );
    assert_eq!(
        response.response_value["write_decision_reasons"][0]["category"],
        "effect_contract"
    );
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn prepare_write_rejects_paths_outside_effect_contract_allowed_paths() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_path",
        ChangeUnitEffectContract {
            allowed_effects: vec![ChangeUnitEffectKind::ProductFileWrite],
            allowed_paths: vec!["tests".to_owned()],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_contract_path",
            "idem_contract_path",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "effect_contract_path_not_allowed");
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn effect_contract_does_not_create_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_effect_contract(
        &harness,
        "contract_no_final",
        ChangeUnitEffectContract {
            expected_outputs: vec!["Implementation output is expected.".to_owned()],
            evidence_expectations: vec!["Evidence is expected before close.".to_owned()],
            ..ChangeUnitEffectContract::default()
        },
    )?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_contract_no_final_close",
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

    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    Ok(())
}

#[test]
fn effect_contract_does_not_replace_sensitive_approval() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_sensitive",
        ChangeUnitEffectContract {
            allowed_effects: vec![
                ChangeUnitEffectKind::ProductFileWrite,
                ChangeUnitEffectKind::SensitiveAction,
            ],
            sensitive_action_expectations: vec!["Network-sensitive step may be needed.".to_owned()],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_contract_sensitive",
        "idem_contract_sensitive",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn prepare_write_blocked_path_issues_no_write_ticket() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_path")?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_blocked_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_path",
        "idem_prepare_path",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "path_out_of_scope");
    assert!(response.response_value["write_ticket_id"].is_null());
    assert!(response.response_value["write_ticket"].is_null());
    assert!(response.response_value["write_ticket_ref"].is_null());
    assert_eq!(response.response_value["write_ticket_effect"], "none");
    assert_eq!(response.response_value["allowed_path_patterns"], json!([]));
    assert_eq!(
        response.response_value["denied_path_patterns"],
        json!(["src/other.rs"])
    );
    assert!(response.response_value["write_ticket"].is_null());
    assert!(response.response_value["write_ticket_ref"].is_null());
    assert_eq!(response.response_value["write_ticket_effect"], "none");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(after.artifact_staging, before.artifact_staging);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.artifact_links, before.artifact_links);
    assert_eq!(after.evidence_summaries, before.evidence_summaries);
    assert_eq!(after.blockers, before.blockers);
    assert_eq!(after.runs, before.runs);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "blocked",
        "path_out_of_scope",
    )?;
    assert_eq!(event_payload["task_id"], task_id);
    assert_eq!(event_payload["change_unit_id"], change_unit_id);
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "scope");
    assert_eq!(reason["code"], "path_out_of_scope");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("outside the current Change Unit path scope"));
    assert!(!reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_missing_change_unit_returns_decision_reason() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_prepare_no_cu_task",
            "idem_prepare_no_cu_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_no_cu",
        "idem_prepare_no_cu",
        Some(1),
        Some(&task_id),
        None,
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "no_current_change_unit");
    assert_eq!(after.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn prepare_write_unresolved_user_judgment_requires_decision() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_judgment")?;
    let mut judgment_request = user_judgment_request(
        "req_prepare_judgment_pending",
        "idem_prepare_judgment_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    judgment_request.required_for = vec![volicord_types::JudgmentRequiredFor::PrepareWrite];
    harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_decision_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_judgment",
        "idem_prepare_judgment",
        Some(3),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "decision_required");
    assert_prepare_reason(&response.response_value, "user_judgment_unresolved");
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "decision_required",
        "user_judgment_unresolved",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "user_judgment");
    assert_eq!(reason["code"], "user_judgment_unresolved");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("user-owned judgment"));
    assert!(!reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_ignores_pending_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_ignore_final")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "prepare_ignore_final",
        true,
    )?;
    harness.service.request_user_judgment(
        user_judgment_request(
            "req_prepare_ignore_final_pending",
            "idem_prepare_ignore_final_pending",
            false,
            Some(after_evidence),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_ignore_final",
            "idem_prepare_ignore_final",
            Some(after_evidence + 1),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert!(response.response_value["write_decision_reasons"]
        .as_array()
        .expect("write_decision_reasons should be an array")
        .is_empty());
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets + 1);
    Ok(())
}

#[test]
fn informational_judgment_does_not_block_prepare_write_or_close_check() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "informational_judgment")?;
    let mut judgment_request = user_judgment_request(
        "req_info_pending",
        "idem_info_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    );
    judgment_request.required_for = vec![volicord_types::JudgmentRequiredFor::Informational];
    harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let prepare = harness.service.prepare_write(
        prepare_write_request(
            "req_info_prepare",
            "idem_info_prepare",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(prepare.response_value["decision"], "allowed");

    let close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_info_close_check",
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
    assert_no_close_blocker(&close.response_value, "pending_user_judgment");
    Ok(())
}

#[test]
fn prepare_write_ignores_another_change_unit_pending_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_other_cu")?;
    let mut judgment_request = user_judgment_request(
        "req_prepare_other_cu_pending",
        "idem_prepare_other_cu_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    judgment_request.required_for = vec![volicord_types::JudgmentRequiredFor::PrepareWrite];
    let judgment = harness.service.request_user_judgment(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    mutate_user_judgment_basis_json(&harness, &judgment_id, |basis| {
        basis["change_unit_id"] = json!("cu_unrelated");
    })?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_other_cu",
            "idem_prepare_other_cu",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_no_prepare_reason(&response.response_value, "user_judgment_unresolved");
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets + 1);
    Ok(())
}

#[test]
fn malformed_stored_required_for_rejects_prepare_write_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_required_for")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_required_for_pending",
            "idem_bad_required_for_pending",
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
        "request_json",
        Some(
            r#"{"presentation":"short","question":"Bad required_for","required_for":["not_a_target"],"expires_at":null}"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_required_for_prepare",
            "idem_bad_required_for_prepare",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "request_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_missing_sensitive_approval_requires_approval() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_sensitive")?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_approval_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_sensitive",
        "idem_prepare_sensitive",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "approval_required",
        "sensitive_approval_missing",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "sensitive_approval");
    assert_eq!(reason["code"], "sensitive_approval_missing");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("sensitive-action approval"));
    assert!(reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_baseline_mismatch_blocks_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_baseline")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_baseline",
        "idem_prepare_baseline",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.baseline_ref = BaselineRef::new("baseline_other");
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "baseline_mismatch");
    assert!(response.response_value["write_ticket_id"].is_null());
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(response.response_value["write_ticket_effect"], "none");
    assert_eq!(after.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn prepare_write_user_only_category_is_invocation_context_rejection() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_binding")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_invocation_context",
        "idem_prepare_invocation_context",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::UserOnly))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(after.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn prepare_write_uses_agent_workflow_invocation_without_extra_binding() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_binding_ok")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_binding_ok",
        "idem_prepare_binding_ok",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets + 1);
    Ok(())
}

#[test]
fn prepare_write_uses_agent_workflow_invocation_without_extra_profile() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_cap")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_capability",
        "idem_prepare_capability",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(after.write_tickets, before.write_tickets + 1);
    Ok(())
}

#[test]
fn prepare_write_product_write_flag_mismatch_blocks_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_flag")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_flag",
        "idem_prepare_flag",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.product_file_write_intended = false;
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "product_write_flag_mismatch");
    assert!(response.response_value["write_ticket_id"].is_null());
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(response.response_value["write_ticket_effect"], "none");
    assert_eq!(after.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn prepare_write_dry_run_has_no_write_ticket_effect() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_dry")?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let mut request = prepare_write_request(
        "req_prepare_dry",
        "idem_prepare_dry",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = true;
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert!(
        response.response_value.get("write_ticket_id").is_none(),
        "dry-run response must not include a committed write ticket id"
    );
    assert!(
        !response.response_json.contains("write_ticket_id"),
        "dry-run response must not serialize committed write ticket fields"
    );
    assert_eq!(
        response.response_value["dry_run_summary"]["planned_effects"][0]["action"],
        "would_issue"
    );
    assert_eq!(
        response.response_value["dry_run_summary"]["planned_effects"][0]["target_kind"],
        "write_ticket"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);

    let mut blocked_preview = prepare_write_request(
        "req_prepare_dry_blocked",
        "idem_prepare_dry_blocked",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    blocked_preview.envelope.dry_run = true;
    blocked_preview.intended_paths = vec!["src/other.rs".to_owned()];
    let blocked_preview = harness.service.prepare_write(
        blocked_preview,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        blocked_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(
        blocked_preview.response_value["dry_run_summary"]["would_blockers"][0]["code"],
        "path_out_of_scope"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    Ok(())
}

#[test]
fn prepare_write_rejects_escaping_product_path_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_escape")?;
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let mut request = prepare_write_request(
        "req_prepare_escape",
        "idem_prepare_escape",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["../outside.rs".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    Ok(())
}

#[test]
fn prepare_write_stale_state_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_stale")?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock);
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let request = prepare_write_request(
        "req_prepare_stale",
        "idem_prepare_stale",
        Some(1),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert!(!response.response_json.contains("write_decision_reasons"));
    assert!(!response
        .response_json
        .contains("STATE_VERSION_CONFLICT\",\"category"));
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    Ok(())
}

#[test]
fn prepare_write_idempotency_replays_without_second_write_ticket() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_replay")?;
    let id_generator =
        CountingDurableIdGenerator::new(["prepare_replay_auth", "prepare_replay_event"]);
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_generator_and_clock(id_generator.clone(), clock.clone());
    let request = prepare_write_request(
        "req_prepare_replay",
        "idem_prepare_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );

    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    clock.advance(Duration::minutes(5));
    let second = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(first.response_value["decision"], "allowed");
    assert_eq!(first.response_value["write_ticket_effect"], "issued");
    assert_eq!(
        second.response_value["write_ticket_id"],
        first.response_value["write_ticket_id"]
    );
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    assert_eq!(write_ticket_count(&harness)?, 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 1);
    assert_eq!(
        second.response_value["write_ticket"]["expires_at"],
        first.response_value["write_ticket"]["expires_at"]
    );
    Ok(())
}

#[test]
fn prepare_write_non_allow_replay_returns_original_response_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_non_allow_replay")?;
    let mut request = prepare_write_request(
        "req_prepare_non_allow_replay",
        "idem_prepare_non_allow_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];
    let before = harness.counts()?;

    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let same_context = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let context_mismatch = harness.service.prepare_write(
        request,
        invocation_with_actor(
            ActorSource::agent_connection("connection_other"),
            OperationCategory::AgentWorkflow,
        ),
    )?;

    assert_eq!(first.response_value["decision"], "blocked");
    assert_prepare_reason(&first.response_value, "path_out_of_scope");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);
    assert_eq!(after_first.write_tickets, before.write_tickets);
    assert_latest_prepare_write_event(
        &harness,
        &first.response_value,
        "blocked",
        "path_out_of_scope",
    )?;
    assert!(same_context.replayed);
    assert_eq!(same_context.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    assert!(!context_mismatch.replayed);
    assert_eq!(
        context_mismatch.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        context_mismatch.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert!(!context_mismatch.response_json.contains("path_out_of_scope"));
    assert!(context_mismatch
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn prepare_write_replay_requires_current_invocation_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_replay_verify")?;
    let request = prepare_write_request(
        "req_prepare_replay_verify",
        "idem_prepare_replay_verify",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness.service.prepare_write(
        request,
        invocation_with_actor(
            ActorSource::agent_connection("connection_other"),
            OperationCategory::AgentWorkflow,
        ),
    )?;

    assert_eq!(first.response_value["decision"], "allowed");
    assert!(!second.replayed);
    assert_eq!(second.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        second.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_ne!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}
