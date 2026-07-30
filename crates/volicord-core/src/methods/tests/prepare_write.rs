use super::*;

#[test]
fn advisor_prepare_write_rejects_without_ticket_or_state_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "advisor_prepare_write",
        RequestedMode::Advisor,
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_advisor_prepare_write",
            "idem_advisor_prepare_write",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_projects_semantic_validation_with_method_metadata() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_semantic_validation")?;
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    harness.use_generator_and_clock(
        id_generator.clone(),
        ManualClock::at(DEFAULT_METHOD_TEST_CLOCK),
    );
    let before = harness.counts()?;
    let mut request = prepare_write_request(
        "req_prepare_semantic_validation",
        "idem_prepare_semantic_validation",
        Some(before.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;
    request.intended_operation = "   ".to_owned();

    let response = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_eq!(
        response.response_value["base"]["state_version"],
        before.state_version
    );
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "intended_operation"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    Ok(())
}

#[test]
fn prepare_write_allowed_issues_one_write_ticket_with_post_commit_basis(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_allowed")?;
    let sensitive_judgment = harness.service.request_user_action(
        user_action_request(
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
    let sensitive_judgment_id = response_record_id(
        &sensitive_judgment.response_value,
        "user_action_request_ref",
    );
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_prepare_allowed_record",
            "idem_prepare_allowed_record",
            None,
            &task_id,
            &sensitive_judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let id_generator =
        CountingDurableIdGenerator::new(["prepare_allowed_auth", "prepare_allowed_event"]);
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
    assert_typed_result_contract::<PrepareWriteResult>(&response);
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
    assert_prepare_write_ticket_matches_stored_source(
        &harness,
        &response.response_value,
        &write_ticket_id,
    )?;
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
        response.response_value["write_ticket"]["validity_basis"]["write_authority_fingerprint"],
        project_write_authority_fingerprint(None)?
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
        response.response_value["active_user_action_refs"]
            .as_array()
            .expect("active judgment refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_tickets, before.write_tickets + 1);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let ref_write_ticket_id = response_record_id(&response.response_value, "write_ticket_ref");
    assert_eq!(write_ticket_id, ref_write_ticket_id);
    assert_eq!(write_ticket_basis(&harness, &ref_write_ticket_id)?, 5);
    let (created_at, idle_expires_at) = write_ticket_timestamps(&harness, &ref_write_ticket_id)?;
    assert_eq!(created_at, "2026-06-18T00:00:00Z");
    assert_eq!(idle_expires_at, None);
    assert_eq!(
        response.response_value["write_ticket"]["idle_expires_at"],
        Value::Null
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
            continuity_page: None,
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
fn prepare_write_reuses_compatible_ticket_across_unrelated_state_increment(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_ticket_reuse")?;

    let first = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_ticket_reuse_first",
            "idem_prepare_ticket_reuse_first",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_ticket_id = response_record_id(&first.response_value, "write_ticket_ref");

    let mut blocked_request = prepare_write_request(
        "req_prepare_ticket_reuse_blocked",
        "idem_prepare_ticket_reuse_blocked",
        Some(3),
        Some(&task_id),
        Some(&change_unit_id),
    );
    blocked_request.intended_paths = vec!["src/other.rs".to_owned()];
    let blocked = harness.service.prepare_write(
        blocked_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(blocked.response_value["decision"], "blocked");

    let reused = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_ticket_reuse_again",
            "idem_prepare_ticket_reuse_again",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(reused.response_value["decision"], "allowed");
    assert_eq!(reused.response_value["write_ticket_effect"], "reused");
    assert_eq!(
        response_record_id(&reused.response_value, "write_ticket_ref"),
        first_ticket_id
    );
    assert_prepare_write_ticket_matches_stored_source(
        &harness,
        &reused.response_value,
        &first_ticket_id,
    )?;
    assert_eq!(write_ticket_count(&harness)?, 1);
    assert_eq!(write_ticket_status(&harness, &first_ticket_id)?, "active");
    Ok(())
}

#[test]
fn prepare_write_replaces_active_ticket_with_mismatched_write_authority_binding(
) -> Result<(), Box<dyn Error>> {
    assert_prepare_write_replaces_policy_stale_ticket(
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "mismatched",
    )
}

fn assert_prepare_write_replaces_policy_stale_ticket(
    stale_fingerprint: &str,
    fixture_name: &str,
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, &format!("prepare_policy_stale_{fixture_name}"))?;

    let first = harness.service.prepare_write(
        prepare_write_request(
            &format!("req_prepare_policy_stale_{fixture_name}_first"),
            &format!("idem_prepare_policy_stale_{fixture_name}_first"),
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let stale_ticket_id = response_record_id(&first.response_value, "write_ticket_ref");
    mutate_write_ticket_validity_basis_json(&harness, &stale_ticket_id, |basis| {
        let object = basis
            .as_object_mut()
            .expect("write-ticket validity basis is object-shaped");
        object.insert(
            "write_authority_fingerprint".to_owned(),
            Value::String(stale_fingerprint.to_owned()),
        );
    })?;
    let replacement = harness.service.prepare_write(
        prepare_write_request(
            &format!("req_prepare_policy_stale_{fixture_name}_replacement"),
            &format!("idem_prepare_policy_stale_{fixture_name}_replacement"),
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(replacement.response_value["decision"], "allowed");
    assert_eq!(replacement.response_value["write_ticket_effect"], "issued");
    let replacement_ticket_id = response_record_id(&replacement.response_value, "write_ticket_ref");
    assert_ne!(replacement_ticket_id, stale_ticket_id);
    assert_eq!(write_ticket_count(&harness)?, 2);
    assert_eq!(
        write_ticket_status(&harness, &stale_ticket_id)?,
        "invalidated"
    );
    assert_eq!(
        write_ticket_status(&harness, &replacement_ticket_id)?,
        "active"
    );
    assert_eq!(
        write_ticket_invalidation_reason(&harness, &stale_ticket_id)?,
        Some("explicit_revoke".to_owned())
    );
    assert!(replacement.response_value["write_ticket"]["validity_basis"]
        ["write_authority_fingerprint"]
        .as_str()
        .is_some_and(|fingerprint| fingerprint.starts_with("sha256:")));
    Ok(())
}

#[test]
fn prepare_write_reuses_ticket_after_status_user_action_evidence_and_non_product_run(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_reuse_unrelated_sequence")?;
    let first = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_reuse_sequence_first",
            "idem_prepare_reuse_sequence_first",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let ticket_id = response_record_id(&first.response_value, "write_ticket_ref");

    harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_prepare_reuse_sequence_status",
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
    let action = harness.service.request_user_action(
        user_action_request(
            "req_prepare_reuse_sequence_action",
            "idem_prepare_reuse_sequence_action",
            false,
            Some(3),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let action_id = response_record_id(&action.response_value, "user_action_request_ref");
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_prepare_reuse_sequence_resolve",
            "idem_prepare_reuse_sequence_resolve",
            Some(4),
            &task_id,
            &action_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let mut run = record_run_request(
        "req_prepare_reuse_sequence_run",
        "idem_prepare_reuse_sequence_run",
        false,
        Some(harness.counts()?.state_version),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::Implementation;
    run.evidence_updates = vec![supported_evidence_update(
        "Unrelated evidence recording preserves the active write ticket.",
    )];
    let before_run = harness.counts()?;
    let recorded = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        recorded.response_value["base"]["response_kind"], "result",
        "{:#}",
        recorded.response_value
    );
    assert!(
        recorded.response_value["evidence_summary"]["coverage_items"]
            .as_array()
            .is_some_and(|items| items
                .iter()
                .any(|item| item["coverage_state"] == "supported"))
    );
    let after_run = harness.counts()?;
    assert_eq!(after_run.runs, before_run.runs + 1);
    assert_eq!(
        after_run.evidence_summaries,
        before_run.evidence_summaries + 1
    );
    assert_eq!(after_run.write_tickets, before_run.write_tickets);
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "active");

    let reused = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_reuse_sequence_again",
            "idem_prepare_reuse_sequence_again",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(reused.response_value["decision"], "allowed");
    assert_eq!(reused.response_value["write_ticket_effect"], "reused");
    assert_eq!(
        response_record_id(&reused.response_value, "write_ticket_ref"),
        ticket_id
    );
    assert_eq!(write_ticket_count(&harness)?, 1);
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "active");
    Ok(())
}

#[test]
fn prepare_write_reuses_sensitive_ticket_only_for_exact_operation_and_approval_basis(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_sensitive_ticket_identity")?;
    let approval = harness.service.request_user_action(
        user_action_request(
            "req_prepare_sensitive_ticket_identity_approval",
            "idem_prepare_sensitive_ticket_identity_approval",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let approval_id = response_record_id(&approval.response_value, "user_action_request_ref");
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_prepare_sensitive_ticket_identity_resolve",
            "idem_prepare_sensitive_ticket_identity_resolve",
            None,
            &task_id,
            &approval_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let mut first_request = prepare_write_request(
        "req_prepare_sensitive_ticket_identity_first",
        "idem_prepare_sensitive_ticket_identity_first",
        Some(4),
        Some(&task_id),
        Some(&change_unit_id),
    );
    first_request.sensitive_categories = vec!["network".to_owned()];
    let first = harness
        .service
        .prepare_write(first_request, invocation(OperationCategory::AgentWorkflow))?;
    let first_ticket_id = response_record_id(&first.response_value, "write_ticket_ref");

    let mut reworded = prepare_write_request(
        "req_prepare_sensitive_ticket_identity_reworded",
        "idem_prepare_sensitive_ticket_identity_reworded",
        Some(5),
        Some(&task_id),
        Some(&change_unit_id),
    );
    reworded.intended_operation = "Reworded description of the already authorized step".to_owned();
    reworded.sensitive_categories = vec!["network".to_owned()];
    let rejected = harness
        .service
        .prepare_write(reworded, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(rejected.response_value["decision"], "approval_required");
    assert_eq!(rejected.response_value["write_ticket_effect"], "none");
    assert!(rejected.response_value["write_decision_reasons"]
        .as_array()
        .is_some_and(|reasons| reasons
            .iter()
            .any(|reason| { reason["code"] == "sensitive_approval_missing" })));
    assert_eq!(write_ticket_count(&harness)?, 1);
    assert_eq!(write_ticket_status(&harness, &first_ticket_id)?, "active");

    let mut exact = prepare_write_request(
        "req_prepare_sensitive_ticket_identity_exact",
        "idem_prepare_sensitive_ticket_identity_exact",
        Some(harness.counts()?.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    exact.sensitive_categories = vec!["network".to_owned()];
    let reused = harness
        .service
        .prepare_write(exact, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(reused.response_value["decision"], "allowed");
    assert_eq!(reused.response_value["write_ticket_effect"], "reused");
    assert_eq!(
        response_record_id(&reused.response_value, "write_ticket_ref"),
        first_ticket_id
    );
    assert_eq!(write_ticket_count(&harness)?, 1);
    assert_eq!(write_ticket_status(&harness, &first_ticket_id)?, "active");
    Ok(())
}

#[test]
fn denied_sensitive_prepare_for_different_scope_preserves_current_ticket(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_sensitive_unrelated_scope")?;
    let approval = harness.service.request_user_action(
        user_action_request(
            "req_prepare_sensitive_unrelated_scope_approval",
            "idem_prepare_sensitive_unrelated_scope_approval",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let approval_id = response_record_id(&approval.response_value, "user_action_request_ref");
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_prepare_sensitive_unrelated_scope_resolve",
            "idem_prepare_sensitive_unrelated_scope_resolve",
            None,
            &task_id,
            &approval_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let mut initial = prepare_write_request(
        "req_prepare_sensitive_unrelated_scope_initial",
        "idem_prepare_sensitive_unrelated_scope_initial",
        Some(harness.counts()?.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    initial.sensitive_categories = vec!["network".to_owned()];
    let initial = harness
        .service
        .prepare_write(initial, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(initial.response_value["decision"], "allowed");
    let ticket_id = response_record_id(&initial.response_value, "write_ticket_ref");

    let mut different_scope = prepare_write_request(
        "req_prepare_sensitive_unrelated_scope_denied",
        "idem_prepare_sensitive_unrelated_scope_denied",
        Some(harness.counts()?.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    different_scope.intended_paths = vec!["tests/export.rs".to_owned()];
    different_scope.sensitive_categories = vec!["network".to_owned()];
    let denied = harness.service.prepare_write(
        different_scope,
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(denied.response_value["decision"], "approval_required");
    assert_prepare_reason(&denied.response_value, "sensitive_approval_missing");
    assert_eq!(denied.response_value["write_ticket_effect"], "none");
    assert_eq!(write_ticket_count(&harness)?, 1);
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "active");
    Ok(())
}

#[test]
fn prepare_write_invalidates_expired_approval_basis_even_when_new_write_is_not_allowed(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_expired_approval_basis")?;
    let mut approval_request = user_action_request(
        "req_prepare_expired_approval_basis_approval",
        "idem_prepare_expired_approval_basis_approval",
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
            "req_prepare_expired_approval_basis_resolve",
            "idem_prepare_expired_approval_basis_resolve",
            None,
            &task_id,
            &approval_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let mut initial_request = prepare_write_request(
        "req_prepare_expired_approval_basis_initial",
        "idem_prepare_expired_approval_basis_initial",
        Some(4),
        Some(&task_id),
        Some(&change_unit_id),
    );
    initial_request.sensitive_categories = vec!["network".to_owned()];
    let initial = harness.service.prepare_write(
        initial_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let write_ticket_id = response_record_id(&initial.response_value, "write_ticket_ref");
    clock.advance(Duration::minutes(6));
    let before = harness.counts()?;

    let mut retry = prepare_write_request(
        "req_prepare_expired_approval_basis_retry",
        "idem_prepare_expired_approval_basis_retry",
        Some(before.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    retry.sensitive_categories = vec!["network".to_owned()];
    let blocked = harness
        .service
        .prepare_write(retry, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(blocked.response_value["decision"], "approval_required");
    assert_prepare_reason(&blocked.response_value, "sensitive_approval_missing");
    assert_eq!(blocked.response_value["write_ticket_effect"], "none");
    assert_eq!(
        write_ticket_status(&harness, &write_ticket_id)?,
        "invalidated"
    );
    assert_eq!(
        write_ticket_invalidation_reason(&harness, &write_ticket_id)?,
        Some("approval_basis_changed".to_owned())
    );
    let after = harness.counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_tickets, before.write_tickets);
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
            continuity_page: None,
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
            continuity_page: None,
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
fn effective_sensitive_control_requires_approval_even_without_declared_categories(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_effect_contract(
        &harness,
        "contract_sensitive_implicit",
        ChangeUnitEffectContract {
            allowed_effects: vec![
                ChangeUnitEffectKind::ProductFileWrite,
                ChangeUnitEffectKind::ExternalNetwork,
            ],
            ..ChangeUnitEffectContract::default()
        },
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_contract_sensitive_implicit",
            "idem_contract_sensitive_implicit",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets);
    Ok(())
}

#[test]
fn policy_denied_path_raises_light_task_to_sensitive() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let mut policy = light_workflow_policy();
    policy["light"]["denied_path_patterns"] = json!(["src/export.rs"]);
    harness.set_workflow_policy(policy)?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "prepare_policy_denied_path",
        RequestedMode::Direct,
    )?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_policy_denied_path",
            "idem_prepare_policy_denied_path",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_ticket"].is_null());

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_prepare_policy_denied_path",
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
        status.response_value["active_task"]["effective_control_level"],
        "sensitive"
    );
    assert!(status.response_value["active_task"]["control_level_reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("denied project-policy prefix")));
    Ok(())
}

#[test]
fn prepare_write_honors_and_clears_durable_policy_reevaluation_mark() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    harness.set_workflow_policy(light_workflow_policy())?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "prepare_policy_reevaluation_mark",
        RequestedMode::Direct,
    )?;
    let marker_fingerprint = format!("sha256:{}", "a".repeat(64));
    let metadata_json = volicord_types::canonical::canonical_json_string(&json!({
        "policy_control_reevaluation": {
            "policy_version": 2,
            "policy_fingerprint": marker_fingerprint,
            "required_effective_control_level": "tracked",
            "marked_at": "2026-06-18T00:00:00Z"
        }
    }))?;
    harness.conn()?.execute(
        "UPDATE tasks SET metadata_json = ?3 WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id, metadata_json],
    )?;
    let before = harness.counts()?.state_version;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_policy_reevaluation_mark",
            "idem_prepare_policy_reevaluation_mark",
            Some(before),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    let task = harness
        .service
        .status(
            StatusRequest {
                envelope: envelope(
                    "req_status_prepare_policy_reevaluation_mark",
                    None,
                    false,
                    None,
                    Some(&task_id),
                ),
                continuity_page: None,
                include: status_include(),
            },
            invocation(OperationCategory::Read),
        )?
        .response_value;
    assert_eq!(task["active_task"]["effective_control_level"], "tracked");
    assert!(task["active_task"]["control_level_reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("pending project-policy reevaluation")));
    let persisted_metadata: String = harness.conn()?.query_row(
        "SELECT metadata_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    assert!(serde_json::from_str::<Value>(&persisted_metadata)?
        .get("policy_control_reevaluation")
        .is_none());
    Ok(())
}

#[test]
fn prepare_write_clears_same_level_policy_reevaluation_mark() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_same_level_policy_reevaluation")?;
    harness.set_workflow_policy(light_workflow_policy())?;

    let marked_metadata: String = harness.conn()?.query_row(
        "SELECT metadata_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    assert!(serde_json::from_str::<Value>(&marked_metadata)?
        .get("policy_control_reevaluation")
        .is_some());

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_same_level_policy_reevaluation",
            "idem_prepare_same_level_policy_reevaluation",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(response.response_value["write_ticket_effect"], "issued");
    let persisted_metadata: String = harness.conn()?.query_row(
        "SELECT metadata_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    assert!(serde_json::from_str::<Value>(&persisted_metadata)?
        .get("policy_control_reevaluation")
        .is_none());
    Ok(())
}

#[test]
fn policy_strengthening_invalidates_nonsensitive_ticket_and_requires_approval(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_policy_strengthening")?;
    let initial = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_policy_strengthening_initial",
            "idem_prepare_policy_strengthening_initial",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let initial_ticket_id = response_record_id(&initial.response_value, "write_ticket_ref");
    let marker_fingerprint = format!("sha256:{}", "b".repeat(64));
    let metadata_json = volicord_types::canonical::canonical_json_string(&json!({
        "policy_control_reevaluation": {
            "policy_version": 2,
            "policy_fingerprint": marker_fingerprint,
            "required_effective_control_level": "sensitive",
            "marked_at": "2026-06-18T00:00:00Z"
        }
    }))?;
    harness.conn()?.execute(
        "UPDATE tasks SET metadata_json = ?3 WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id, metadata_json],
    )?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_policy_strengthening_retry",
            "idem_prepare_policy_strengthening_retry",
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert_eq!(response.response_value["write_ticket_effect"], "none");
    assert_eq!(
        write_ticket_status(&harness, &initial_ticket_id)?,
        "invalidated"
    );
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
    assert_eq!(after.authority_events, before.authority_events + 1);
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
fn prepare_write_without_current_change_unit_rejects_before_policy_or_effects(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let diagnostic_time = "2099-07-17T01:02:03Z";
    harness.use_clock(ManualClock::at(diagnostic_time));
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
    let response = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "NO_ACTIVE_CHANGE_UNIT"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["reason"],
        "current_change_unit_required"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["method"],
        "volicord.prepare_write"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["project_id"],
        PROJECT_ID
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["task_id"],
        task_id
    );
    assert!(response.response_value.get("decision").is_none());
    assert_eq!(after, before);

    let repeated = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(repeated.response_value, response.response_value);
    assert_eq!(harness.counts()?, before);
    let diagnostics = read_core_rejection_diagnostics(&harness.runtime_home_path)?;
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].project_id, PROJECT_ID);
    assert_eq!(diagnostics[0].task_id, task_id);
    assert_eq!(diagnostics[0].method_name, "volicord.prepare_write");
    assert_eq!(diagnostics[0].reason, "current_change_unit_required");
    assert_eq!(diagnostics[0].occurred_at, diagnostic_time);
    Ok(())
}

#[test]
fn prepare_write_dry_run_without_current_change_unit_has_no_effects() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_prepare_no_cu_dry_task",
            "idem_prepare_no_cu_dry_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let before = harness.counts()?;
    let mut request = prepare_write_request(
        "req_prepare_no_cu_dry",
        "idem_prepare_no_cu_dry",
        Some(1),
        Some(&task_id),
        None,
    );
    request.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;

    let response = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let repeated = harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "NO_ACTIVE_CHANGE_UNIT"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["reason"],
        "current_change_unit_required"
    );
    assert_eq!(repeated.response_value, response.response_value);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_unresolved_user_action_requires_decision() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_judgment")?;
    let mut judgment_request = user_action_request(
        "req_prepare_judgment_pending",
        "idem_prepare_judgment_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    judgment_request.required_for =
        vec![volicord_types::values::UserActionRequiredFor::PrepareWrite];
    harness.service.request_user_action(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let id_generator = CountingDurableIdGenerator::new(["prepare_decision_event"]);
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
    assert_prepare_reason(&response.response_value, "user_action_unresolved");
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(id_generator.count(DurableIdKind::WriteTicket), 0);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "decision_required",
        "user_action_unresolved",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "user_action");
    assert_eq!(reason["code"], "user_action_unresolved");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("user action"));
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
    harness.service.request_user_action(
        user_action_request(
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
    let mut judgment_request = user_action_request(
        "req_info_pending",
        "idem_info_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    );
    judgment_request.required_for =
        vec![volicord_types::values::UserActionRequiredFor::Informational];
    harness.service.request_user_action(
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
    assert_no_close_blocker(&close.response_value, "pending_user_action");
    assert!(close.response_value["pending_user_action_summaries"]
        .as_array()
        .expect("close pending judgment summaries should be an array")
        .is_empty());
    assert_eq!(
        close.response_value["state"]["pending_user_action_summaries"]
            .as_array()
            .expect("state pending judgment summaries should be an array")
            .len(),
        1
    );
    assert!(close
        .response_value
        .get("pending_user_action_inbox_items")
        .is_none());
    Ok(())
}

#[test]
fn prepare_write_ignores_another_change_unit_pending_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_other_cu")?;
    let mut judgment_request = user_action_request(
        "req_prepare_other_cu_pending",
        "idem_prepare_other_cu_pending",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    judgment_request.required_for =
        vec![volicord_types::values::UserActionRequiredFor::PrepareWrite];
    let judgment = harness.service.request_user_action(
        judgment_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    let replaced = harness.service.update_scope(
        update_scope_request(
            "req_prepare_other_cu_replace",
            "idem_prepare_other_cu_replace",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replace the current Change Unit before preparing the next write.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let current_change_unit_id = replaced.response_value["state"]["active_change_unit_ref"]
        ["record_id"]
        .as_str()
        .expect("replacement Change Unit")
        .to_owned();
    let current_state_version = replaced.response_value["base"]["state_version"]
        .as_u64()
        .expect("replacement state version");
    assert_eq!(user_action_status(&harness, &judgment_id)?, "superseded");
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_other_cu",
            "idem_prepare_other_cu",
            Some(current_state_version),
            Some(&task_id),
            Some(&current_change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_no_prepare_reason(&response.response_value, "user_action_unresolved");
    assert_eq!(harness.counts()?.write_tickets, before.write_tickets + 1);
    Ok(())
}

#[test]
fn malformed_stored_required_for_rejects_prepare_write_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_required_for")?;
    let judgment = harness.service.request_user_action(
        user_action_request(
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
    let judgment_id = response_record_id(&judgment.response_value, "user_action_request_ref");
    set_user_action_owner_json(
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
    assert_eq!(after.authority_events, before.authority_events + 1);
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
fn prepare_write_rejects_divergent_task_and_change_unit_baselines() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_divergent_baseline")?;
    set_task_baseline_owner_state(&harness, &task_id, "baseline_other")?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_divergent_baseline",
            "idem_prepare_divergent_baseline",
            Some(before.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "baseline_mismatch");
    assert!(response.response_value["write_ticket"].is_null());
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
    request.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;
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
    blocked_preview.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;
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
fn prepare_write_dry_run_projects_reuse_without_a_persisted_plan() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_dry_reuse")?;
    let issued = harness.service.prepare_write(
        prepare_write_request(
            "req_prepare_dry_reuse_issued",
            "idem_prepare_dry_reuse_issued",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let ticket_id = response_record_id(&issued.response_value, "write_ticket_ref");
    let id_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    harness.use_generator_and_clock(
        id_generator.clone(),
        ManualClock::at("2026-06-18T00:00:00Z"),
    );
    let before = harness.counts()?;

    let mut preview = prepare_write_request(
        "req_prepare_dry_reuse_preview",
        "idem_prepare_dry_reuse_preview",
        Some(before.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    preview.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;
    let preview = harness
        .service
        .prepare_write(preview, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(preview.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(
        preview.response_value["dry_run_summary"]["planned_effects"][0]["action"],
        "would_reuse"
    );
    assert!(!preview.response_json.contains("write_ticket_id"));
    assert_eq!(harness.counts()?, before);
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "active");
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
fn prepare_write_rejects_portable_drive_prefixed_paths_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_drive_prefix")?;
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    for (request_id, idempotency_key, path) in [
        (
            "req_prepare_drive_absolute",
            "idem_prepare_drive_absolute",
            "C:/outside.rs",
        ),
        (
            "req_prepare_drive_relative",
            "idem_prepare_drive_relative",
            "c:relative",
        ),
    ] {
        let mut request = prepare_write_request(
            request_id,
            idempotency_key,
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.intended_paths = vec![path.to_owned()];
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
    }

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
        second.response_value["write_ticket"]["idle_expires_at"],
        first.response_value["write_ticket"]["idle_expires_at"]
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
    assert_eq!(after_first.authority_events, before.authority_events + 1);
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

#[test]
fn prepare_write_replay_rejects_changed_git_workspace_without_exposing_ticket(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_workspace_replay_task",
            "idem_workspace_replay_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    let original = crate::pipeline::GitWorkspaceContext {
        git_common_dir: "/tmp/volicord-workspace-replay/.git".to_owned(),
        worktree_id: format!("sha256:{}", "1".repeat(64)),
        branch_ref: Some("refs/heads/original".to_owned()),
        head_sha: Some("1".repeat(40)),
        workspace_fingerprint: format!("sha256:{}", "2".repeat(64)),
    };
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_workspace_replay_scope",
            "idem_workspace_replay_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bind replay to the original workspace.",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let change_unit_id = scoped.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit id")
        .to_owned();
    let request = prepare_write_request(
        "req_workspace_replay_write",
        "idem_workspace_replay_write",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );

    let first = harness.service.prepare_write(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let after_first = harness.counts()?;
    assert_eq!(first.response_value["decision"], "allowed");
    assert!(first.response_value["write_ticket"].is_object());

    let mut changed = original;
    changed.branch_ref = Some("refs/heads/other".to_owned());
    changed.head_sha = Some("3".repeat(40));
    changed.workspace_fingerprint = format!("sha256:{}", "4".repeat(64));
    let replay = harness.service.prepare_write(
        request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(changed),
    )?;

    assert!(!replay.replayed);
    assert_eq!(replay.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        replay.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_ne!(replay.response_json, first.response_json);
    assert!(!replay.response_json.contains(
        first.response_value["write_ticket_id"]
            .as_str()
            .expect("write ticket id")
    ));
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn prepare_write_blocks_changed_git_workspace_until_explicit_retarget() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_workspace_task",
            "idem_workspace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    let original = crate::pipeline::GitWorkspaceContext {
        git_common_dir: "/tmp/volicord-workspace/.git".to_owned(),
        worktree_id: format!("sha256:{}", "1".repeat(64)),
        branch_ref: Some("refs/heads/original".to_owned()),
        head_sha: Some("1".repeat(40)),
        workspace_fingerprint: format!("sha256:{}", "2".repeat(64)),
    };
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_workspace_scope",
            "idem_workspace_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bind the original branch.",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let change_unit_id = scoped.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit id")
        .to_owned();
    assert_eq!(
        scoped.response_value["state"]["workspace_context"]["branch_ref"],
        "refs/heads/original"
    );
    let prepared = harness.service.prepare_write(
        prepare_write_request(
            "req_workspace_original_write",
            "idem_workspace_original_write",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let original_ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");

    let mut changed = original;
    changed.branch_ref = Some("refs/heads/other".to_owned());
    changed.head_sha = Some("3".repeat(40));
    changed.workspace_fingerprint = format!("sha256:{}", "4".repeat(64));
    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_workspace_write",
            "idem_workspace_write",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(changed),
    )?;
    assert_eq!(response.response_value["decision"], "blocked");
    assert!(response.response_value["write_decision_reasons"]
        .as_array()
        .expect("reasons")
        .iter()
        .any(|reason| {
            reason["category"] == "workspace" && reason["code"] == "workspace_context_mismatch"
        }));
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(
        write_ticket_status(&harness, &original_ticket_id)?,
        "invalidated"
    );
    assert_eq!(
        write_ticket_invalidation_reason(&harness, &original_ticket_id)?,
        Some("workspace_changed".to_owned())
    );
    Ok(())
}

fn assert_prepare_write_ticket_matches_stored_source(
    harness: &MethodHarness,
    response: &Value,
    write_ticket_id: &str,
) -> Result<(), Box<dyn Error>> {
    let store = harness.store()?;
    let stored = store
        .write_ticket_record(write_ticket_id)?
        .ok_or_else(|| format!("missing Write Ticket {write_ticket_id}"))?;
    let allowed = stored
        .path_scope()
        .allowed()
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>();
    let denied = stored
        .path_scope()
        .denied()
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>();
    let attempt_scope = stored.attempt_scope();

    assert_eq!(response["write_ticket_id"], write_ticket_id);
    assert_eq!(response["write_ticket_ref"]["record_id"], write_ticket_id);
    assert_eq!(
        response["write_ticket"]["write_ticket_id"],
        response["write_ticket_id"]
    );
    assert_eq!(
        response["write_ticket"]["write_ticket_ref"],
        response["write_ticket_ref"]
    );
    assert_eq!(
        response["write_ticket"]["path_patterns"]["allowed"],
        response["allowed_path_patterns"]
    );
    assert_eq!(
        response["write_ticket"]["path_patterns"]["denied"],
        response["denied_path_patterns"]
    );
    assert_eq!(
        response["allowed_path_patterns"],
        serde_json::to_value(allowed)?
    );
    assert_eq!(
        response["denied_path_patterns"],
        serde_json::to_value(denied)?
    );
    assert_eq!(
        response["write_ticket"]["basis_state_version"],
        stored.basis_state_version()
    );
    assert_eq!(
        response["write_ticket"]["validity_basis"],
        serde_json::to_value(stored.validity_basis())?
    );
    assert_eq!(
        response["write_ticket"]["idle_expires_at"],
        serde_json::to_value(stored.idle_expires_at())?
    );
    assert_eq!(
        response["write_ticket"]["scope"]["task_id"],
        attempt_scope.task_id.as_str()
    );
    assert_eq!(
        response["write_ticket"]["scope"]["change_unit_id"],
        attempt_scope.change_unit_id.as_str()
    );
    assert_eq!(
        response["write_ticket"]["scope"]["intended_operation"],
        attempt_scope.intended_operation
    );
    assert_eq!(
        response["write_ticket"]["scope"]["product_file_write_intended"],
        attempt_scope.product_file_write_intended
    );
    assert_eq!(
        response["write_ticket"]["scope"]["sensitive_categories"],
        serde_json::to_value(&attempt_scope.sensitive_categories)?
    );
    assert_eq!(
        response["write_ticket"]["scope"]["baseline_ref"],
        serde_json::to_value(attempt_scope.baseline_ref.as_ref())?
    );
    Ok(())
}
