use super::*;

#[test]
fn request_user_judgment_creates_pending_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "pending")?;
    let before = harness.counts()?;

    let response = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_pending",
            "idem_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let judgment_id = response_record_id(&response.response_value, "user_judgment_ref");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["user_judgment"]["status"],
        "pending"
    );
    assert_eq!(
        response.response_value["user_judgment"]["judgment_kind"],
        "product_decision"
    );
    assert_eq!(
        response.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments + 1);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn authority_request_waits_and_last_resolution_restores_ready() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "lifecycle_ready")?;
    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_lifecycle_ready",
            "idem_judgment_lifecycle_ready",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending.response_value, "user_judgment_ref");

    assert_eq!(
        pending.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "waiting_user"
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "waiting_user"
    );

    let resolved = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_lifecycle_ready",
            "idem_record_lifecycle_ready",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(
        resolved.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "ready"
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "ready"
    );
    Ok(())
}

#[test]
fn last_authority_resolution_without_change_unit_restores_shaping() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_judgment_lifecycle_shaping_task",
            "idem_judgment_lifecycle_shaping_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_lifecycle_shaping",
            "idem_judgment_lifecycle_shaping",
            false,
            Some(1),
            &task_id,
            None,
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending.response_value, "user_judgment_ref");
    assert_eq!(
        pending.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "waiting_user"
    );

    let resolved = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_lifecycle_shaping",
            "idem_record_lifecycle_shaping",
            Some(2),
            &task_id,
            &judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(
        resolved.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "shaping"
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "shaping"
    );
    Ok(())
}

#[test]
fn resolving_one_of_multiple_authority_judgments_keeps_waiting() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "multiple_waiting")?;
    let first = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_multiple_first",
            "idem_judgment_multiple_first",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_id = response_record_id(&first.response_value, "user_judgment_ref");
    harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_multiple_second",
            "idem_judgment_multiple_second",
            false,
            Some(3),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let resolved = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_multiple_first",
            "idem_record_multiple_first",
            Some(4),
            &task_id,
            &first_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(
        resolved.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "waiting_user"
    );
    assert_eq!(
        resolved.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        1
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "waiting_user"
    );
    Ok(())
}

#[test]
fn informational_and_deferred_judgments_do_not_keep_waiting() -> Result<(), Box<dyn Error>> {
    let informational_harness = MethodHarness::new()?;
    let (informational_task_id, informational_change_unit_id) =
        create_task_with_change_unit(&informational_harness, "informational_lifecycle")?;
    let mut informational_request = user_judgment_request(
        "req_judgment_informational_lifecycle",
        "idem_judgment_informational_lifecycle",
        false,
        Some(2),
        &informational_task_id,
        Some(&informational_change_unit_id),
        JudgmentKind::ScopeDecision,
    );
    informational_request.required_for = vec![JudgmentRequiredFor::Informational];
    let informational = informational_harness.service.request_user_judgment(
        informational_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        informational.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "ready"
    );
    assert_eq!(
        task_terminal_fields(&informational_harness, &informational_task_id)?.lifecycle_phase,
        "ready"
    );

    let deferred_harness = MethodHarness::new()?;
    let (deferred_task_id, deferred_change_unit_id) =
        create_task_with_change_unit(&deferred_harness, "deferred_lifecycle")?;
    let pending = deferred_harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_deferred_lifecycle",
            "idem_judgment_deferred_lifecycle",
            false,
            Some(2),
            &deferred_task_id,
            Some(&deferred_change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_id = response_record_id(&pending.response_value, "user_judgment_ref");
    let mut deferred_request = record_judgment_request(
        "req_record_deferred_lifecycle",
        "idem_record_deferred_lifecycle",
        Some(3),
        &deferred_task_id,
        &pending_id,
        JudgmentKind::ScopeDecision,
        scope_decision_payload("deferred"),
    );
    deferred_request.selected_option_id = UserJudgmentOptionId::new("defer");
    let deferred = deferred_harness
        .service
        .record_user_judgment(deferred_request, invocation(OperationCategory::UserOnly))?;
    assert_eq!(
        deferred.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "deferred"
    );
    assert_eq!(
        deferred.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "ready"
    );
    assert_eq!(
        task_terminal_fields(&deferred_harness, &deferred_task_id)?.lifecycle_phase,
        "ready"
    );
    Ok(())
}

#[test]
fn judgment_commits_do_not_reopen_terminal_task() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "terminal_lifecycle")?;
    harness.conn()?.execute(
        "UPDATE tasks
            SET lifecycle_phase = 'completed',
                result = 'completed',
                closed_at = '2026-07-12T00:00:00Z'
          WHERE project_id = ?1
            AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
    )?;

    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_terminal_lifecycle",
            "idem_judgment_terminal_lifecycle",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_id = response_record_id(&pending.response_value, "user_judgment_ref");
    assert_eq!(
        pending.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "completed"
    );

    let resolved = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_terminal_lifecycle",
            "idem_record_terminal_lifecycle",
            Some(3),
            &task_id,
            &pending_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(
        resolved.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "completed"
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "completed"
    );
    Ok(())
}

#[test]
fn authority_bearing_judgment_generates_canonical_options() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_options")?;

    let response = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_canonical_options",
            "idem_judgment_canonical_options",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let options = response.response_value["user_judgment"]["options"]
        .as_array()
        .expect("options should be an array");
    assert_eq!(options.len(), 3);
    assert_eq!(options[0]["option_id"], "accept");
    assert_eq!(options[0]["machine_action"], "accept");
    assert_eq!(options[0]["resolution_outcome"], "accepted");
    assert_eq!(options[1]["option_id"], "reject");
    assert_eq!(options[1]["machine_action"], "reject");
    assert_eq!(options[1]["resolution_outcome"], "rejected");
    assert_eq!(options[2]["option_id"], "defer");
    assert_eq!(options[2]["machine_action"], "defer");
    assert_eq!(options[2]["resolution_outcome"], "deferred");
    Ok(())
}

#[test]
fn authority_option_locale_changes_display_only() -> Result<(), Box<dyn Error>> {
    let english_harness = MethodHarness::new()?;
    let (english_task_id, english_change_unit_id) =
        create_task_with_change_unit(&english_harness, "locale_en")?;
    let english = english_harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_locale_en",
            "idem_judgment_locale_en",
            false,
            Some(2),
            &english_task_id,
            Some(&english_change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let korean_harness = MethodHarness::new()?;
    let (korean_task_id, korean_change_unit_id) =
        create_task_with_change_unit(&korean_harness, "locale_ko")?;
    let mut korean_request = user_judgment_request(
        "req_judgment_locale_ko",
        "idem_judgment_locale_ko",
        false,
        Some(2),
        &korean_task_id,
        Some(&korean_change_unit_id),
        JudgmentKind::Cancellation,
    );
    korean_request.envelope.locale = Some("ko-KR".to_owned()).into();
    let korean = korean_harness
        .service
        .request_user_judgment(korean_request, invocation(OperationCategory::AgentWorkflow))?;

    let english_accept = &english.response_value["user_judgment"]["options"][0];
    let korean_accept = &korean.response_value["user_judgment"]["options"][0];
    assert_ne!(english_accept["label"], korean_accept["label"]);
    assert_eq!(english_accept["option_id"], korean_accept["option_id"]);
    assert_eq!(
        english_accept["machine_action"],
        korean_accept["machine_action"]
    );
    assert_eq!(
        english_accept["resolution_outcome"],
        korean_accept["resolution_outcome"]
    );

    let fallback_harness = MethodHarness::new()?;
    let (fallback_task_id, fallback_change_unit_id) =
        create_task_with_change_unit(&fallback_harness, "locale_fallback")?;
    let mut fallback_request = user_judgment_request(
        "req_judgment_locale_fallback",
        "idem_judgment_locale_fallback",
        false,
        Some(2),
        &fallback_task_id,
        Some(&fallback_change_unit_id),
        JudgmentKind::Cancellation,
    );
    fallback_request.envelope.locale = Some("zz-ZZ".to_owned()).into();
    let fallback = fallback_harness.service.request_user_judgment(
        fallback_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        english_accept["label"],
        fallback.response_value["user_judgment"]["options"][0]["label"]
    );
    Ok(())
}

#[test]
fn authority_bearing_judgment_request_rejects_caller_options() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "authority_options")?;
    let mut request = user_judgment_request(
        "req_judgment_authority_options",
        "idem_judgment_authority_options",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::Cancellation,
    );
    request.options = Some(vec![volicord_types::UserJudgmentOptionInput {
        option_id: volicord_types::UserJudgmentOptionId::new("reject_visible_accept"),
        label: "Reject".to_owned(),
        description: "Caller-authored authority options are not accepted.".to_owned(),
        consequence: "Core must generate the authority option set.".to_owned(),
        is_default: false,
    }])
    .into();
    let before = harness.counts()?;

    let response = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_user_judgment_resolves_pending_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "resolve")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_resolve",
            "idem_judgment_resolve",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    assert_eq!(
        pending_judgment.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "waiting_user"
    );
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_resolve",
            "idem_record_resolve",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(
        response.response_value["user_judgment"]["status"],
        "resolved"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolved_by_actor_source"],
        LOCAL_USER_ACTOR_SOURCE
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["rationale"]["summary"],
        "The user selected the focused judgment option."
    );
    assert_eq!(
        response.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        0
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    assert!(
        resolution_json(&harness, &pending_judgment_id)?["answer"]["product_decision"].is_object()
    );
    assert_eq!(
        resolution_json(&harness, &pending_judgment_id)?["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        resolution_rationale_json(&harness, &pending_judgment_id)?["summary"],
        response.response_value["user_judgment"]["resolution"]["rationale"]["summary"]
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    assert_eq!(
        user_judgment_actor_provenance(&harness, &pending_judgment_id)?,
        UserJudgmentActorProvenance {
            resolved_by_actor_source: Some(LOCAL_USER_ACTOR_SOURCE.to_owned()),
            resolved_verification_basis: Some(VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned()),
            resolved_assurance_level: Some("local_user_channel".to_owned()),
        }
    );
    let (event_kind, event_payload, _) = latest_task_event(&harness)?;
    assert_eq!(event_kind, "user_judgment_recorded");
    assert_eq!(event_payload["resolution_outcome"], "accepted");
    Ok(())
}

#[test]
fn record_user_judgment_persists_authority_accept_action() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "accept_action")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_accept_action",
            "idem_judgment_accept_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_accept_action",
            "idem_record_accept_action",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::Cancellation,
            answer_payload(JudgmentKind::Cancellation),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["machine_action"],
        "accept"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        resolution_json(&harness, &pending_judgment_id)?["machine_action"],
        "accept"
    );
    assert_eq!(
        user_judgment_resolution_machine_action(&harness, &pending_judgment_id)?,
        Some("accept".to_owned())
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn accepted_decision_judgments_create_project_continuity_records() -> Result<(), Box<dyn Error>> {
    for (suffix, judgment_kind, title_prefix) in [
        (
            "product_continuity",
            JudgmentKind::ProductDecision,
            "Product decision:",
        ),
        (
            "technical_continuity",
            JudgmentKind::TechnicalDecision,
            "Technical decision:",
        ),
        (
            "scope_continuity",
            JudgmentKind::ScopeDecision,
            "Scope decision:",
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                &format!("req_judgment_{suffix}"),
                &format!("idem_judgment_{suffix}"),
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                judgment_kind,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                &format!("req_record_{suffix}"),
                &format!("idem_record_{suffix}"),
                Some(3),
                &task_id,
                &pending_judgment_id,
                judgment_kind,
                answer_payload(judgment_kind),
            ),
            invocation(OperationCategory::UserOnly),
        )?;

        let after = harness.counts()?;
        let rows = harness.continuity_records()?;
        assert_eq!(
            after.project_continuity_records,
            before.project_continuity_records + 1
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "decision");
        assert_eq!(rows[0].status, "active");
        assert_eq!(rows[0].source_task_id, task_id);
        assert_eq!(
            rows[0].source_change_unit_id.as_deref(),
            Some(change_unit_id.as_str())
        );
        assert!(rows[0].title.starts_with(title_prefix));
        assert!(rows[0].source_refs_json.contains(&pending_judgment_id));
        assert!(response.response_value["updated_refs"]
            .as_array()
            .expect("updated_refs should be an array")
            .iter()
            .any(|record_ref| record_ref["record_kind"] == "project_continuity_record"));
    }
    Ok(())
}

#[test]
fn accepted_residual_risk_creates_project_continuity_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_continuity")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "continuity",
        vec![residual_risk_input(
            "Visible residual risk needing acceptance.",
        )],
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_continuity",
            "idem_risk_continuity",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_continuity_record",
            "idem_risk_continuity_record",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let after = harness.counts()?;
    let rows = harness.continuity_records()?;
    assert_eq!(
        after.project_continuity_records,
        before.project_continuity_records + 1
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "accepted_risk");
    assert_eq!(rows[0].summary, "Visible residual risk needing acceptance.");
    assert!(rows[0].title.starts_with("Accepted residual risk:"));
    assert!(rows[0].source_refs_json.contains(&pending_judgment_id));
    assert!(response.response_value["updated_refs"]
        .as_array()
        .expect("updated_refs should be an array")
        .iter()
        .any(|record_ref| record_ref["record_kind"] == "project_continuity_record"));
    Ok(())
}

#[test]
fn status_continuity_summary_is_include_gated() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_continuity")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_status_continuity_judgment",
            "idem_status_continuity_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_status_continuity_record",
            "idem_status_continuity_record",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::TechnicalDecision,
            answer_payload(JudgmentKind::TechnicalDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    let hidden = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_continuity_hidden",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                continuity: false,
                ..status_include()
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_field_absent(&hidden.response_value, "continuity_summary");

    let shown = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_continuity_shown",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                continuity: true,
                ..status_include()
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let summary = shown.response_value["continuity_summary"]
        .as_array()
        .expect("continuity_summary should be an array");
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0]["kind"], "decision");
    assert_eq!(summary[0]["status"], "active");
    assert!(summary[0]["continuity_record_ref"].is_object());
    Ok(())
}

#[test]
fn stale_judgment_does_not_create_project_continuity_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "stale_continuity")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_stale_continuity_judgment",
            "idem_stale_continuity_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    harness.service.update_scope(
        update_scope_request(
            "req_stale_continuity_scope",
            "idem_stale_continuity_scope",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "stale_continuity_scope",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_stale_continuity_record",
            "idem_stale_continuity_record",
            Some(4),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert!(harness.continuity_records()?.is_empty());
    Ok(())
}

#[test]
fn close_completion_creates_known_limit_continuity_and_preserves_records(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_continuity")?;
    let (after_basis, _) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "known_limit",
        vec![volicord_types::ResidualRiskInput {
            summary: "Known limitation that does not require acceptance.".to_owned(),
            consequence: "Future related work should remember this limitation.".to_owned(),
            acceptance_required: false,
            source_refs: Vec::new(),
        }],
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "known_limit",
    )?;
    let before_close = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_continuity_complete",
            idempotency_key: Some("idem_close_continuity_complete"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    let after_close = harness.counts()?;
    let rows = harness.continuity_records()?;
    assert_eq!(response.response_value["close_state"], "closed");
    let response_continuity = response.response_value["continuity_summary"]
        .as_array()
        .expect("continuity_summary should be an array");
    assert_eq!(response_continuity.len(), 1);
    assert_eq!(response_continuity[0]["kind"], "known_limit");
    assert_eq!(response_continuity[0]["status"], "active");
    assert_eq!(
        response_continuity[0]["source_task_ref"]["record_id"],
        task_id
    );
    assert_eq!(
        after_close.project_continuity_records,
        before_close.project_continuity_records + 1
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "known_limit");
    assert_eq!(rows[0].status, "active");
    assert_eq!(rows[0].source_task_id, task_id);
    assert_eq!(
        rows[0].source_change_unit_id.as_deref(),
        Some(change_unit_id.as_str())
    );
    assert_eq!(
        rows[0].summary,
        "Known limitation that does not require acceptance."
    );
    Ok(())
}

#[test]
fn accepted_authority_judgment_requires_structured_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "accepted_rationale_required")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_rationale_required",
            "idem_judgment_rationale_required",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_rationale_required",
        "idem_record_rationale_required",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::Cancellation,
        answer_payload(JudgmentKind::Cancellation),
    );
    request.rationale.selected_reason = None.into();
    request.rationale.tradeoffs.clear();
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "rationale.selected_reason"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn record_user_judgment_persists_rejected_option_outcome() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "reject_outcome")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_reject_outcome",
            "idem_judgment_reject_outcome",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_reject_outcome",
        "idem_record_reject_outcome",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::Cancellation,
        cancellation_payload_with_decision("rejected"),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_ne!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(
        user_judgment_resolution_machine_action(&harness, &pending_judgment_id)?,
        Some("reject".to_owned())
    );
    assert_eq!(
        resolution_json(&harness, &pending_judgment_id)?["resolution_outcome"],
        "rejected"
    );
    assert_eq!(response.response_value["state"]["close_state"], "blocked");
    assert_close_blocker(
        &response.response_value["state"],
        "missing_current_close_basis",
    );
    let (event_kind, event_payload, _) = latest_task_event(&harness)?;
    assert_eq!(event_kind, "user_judgment_recorded");
    assert_eq!(event_payload["resolution_outcome"], "rejected");
    Ok(())
}

#[test]
fn rejected_authority_judgment_accepts_concise_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "rejected_concise_rationale")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_concise_rationale",
            "idem_judgment_concise_rationale",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_concise_rationale",
        "idem_record_concise_rationale",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::Cancellation,
        cancellation_payload_with_decision("rejected"),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    request.rationale = JudgmentRationale {
        summary: "The user declined cancellation for now.".to_owned(),
        selected_reason: None.into(),
        considered_alternatives: Vec::new(),
        rejected_alternatives: Vec::new(),
        assumptions: Vec::new(),
        tradeoffs: Vec::new(),
        uncertainties: Vec::new(),
        review_triggers: Vec::new(),
        related_refs: Vec::new(),
        artifact_refs: Vec::new(),
    };

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "rejected"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["rationale"]["summary"],
        "The user declined cancellation for now."
    );
    assert_eq!(
        resolution_rationale_json(&harness, &pending_judgment_id)?["summary"],
        "The user declined cancellation for now."
    );
    Ok(())
}

#[test]
fn resolved_judgment_without_machine_action_is_owner_state_corruption() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "missing_action")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_action",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_missing_action_judgment",
            "idem_missing_action_judgment",
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
            &json!({
                "selected_option_id": "accept",
                "answer": {
                    "product_decision": null,
                    "technical_decision": null,
                    "scope_decision": null,
                    "sensitive_action_scope": null,
                    "final_acceptance": { "judgment": { "decision": "accepted" } },
                    "residual_risk_acceptance": null,
                    "cancellation": null
                },
                "note": null,
                "accepted_risks": [],
                "resolved_by_actor_source": "user"
            })
            .to_string(),
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_missing_action_close",
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
        "resolution_machine_action",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_judgment_null_action_column_with_json_action_is_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("null_action_column", |harness, judgment_id| {
        set_user_judgment_resolution_machine_action(harness, judgment_id, None)
    })
}

#[test]
fn stored_judgment_action_column_with_missing_json_action_is_corrupt() -> Result<(), Box<dyn Error>>
{
    assert_final_acceptance_action_corruption_with(
        "missing_json_action",
        "resolution_json",
        "corrupt_stored_json",
        |harness, judgment_id| {
            let mut resolution = resolution_json(harness, judgment_id)?;
            resolution
                .as_object_mut()
                .expect("resolution JSON should be an object")
                .remove("machine_action");
            set_user_judgment_resolution_json_only_value(harness, judgment_id, &resolution)
        },
    )
}

#[test]
fn stored_judgment_differing_action_values_are_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("differing_action", |harness, judgment_id| {
        let mut resolution = resolution_json(harness, judgment_id)?;
        resolution["machine_action"] = json!("reject");
        set_user_judgment_resolution_json_only_value(harness, judgment_id, &resolution)
    })
}

#[test]
fn stored_judgment_action_outcome_mismatch_is_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("action_outcome_mismatch", |harness, judgment_id| {
        set_user_judgment_resolution_machine_action(harness, judgment_id, Some("reject"))?;
        let mut resolution = resolution_json(harness, judgment_id)?;
        resolution["machine_action"] = json!("reject");
        set_user_judgment_resolution_json_value(harness, judgment_id, &resolution)
    })
}

#[test]
fn stored_judgment_unsupported_action_string_is_corrupt() -> Result<(), Box<dyn Error>> {
    assert_final_acceptance_action_corruption("unsupported_action", |harness, judgment_id| {
        set_user_judgment_resolution_machine_action_raw(harness, judgment_id, Some("approve"))
    })
}

#[test]
fn non_authority_custom_options_remain_usable_without_outcome_input() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "custom_option")?;
    let mut request = user_judgment_request(
        "req_judgment_custom_option",
        "idem_judgment_custom_option",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    request.options = Some(vec![volicord_types::UserJudgmentOptionInput {
        option_id: volicord_types::UserJudgmentOptionId::new("reject_like_custom_id"),
        label: "Use the alternate copy".to_owned(),
        description: "Record the user's product choice without caller-defined authority."
            .to_owned(),
        consequence: "The selected custom option is recorded for this product decision.".to_owned(),
        is_default: true,
    }])
    .into();
    let pending_judgment = harness
        .service
        .request_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    assert_eq!(
        pending_judgment.response_value["user_judgment"]["options"][0]["option_id"],
        "reject_like_custom_id"
    );
    assert_eq!(
        pending_judgment.response_value["user_judgment"]["options"][0]["machine_action"],
        "accept"
    );
    assert_eq!(
        pending_judgment.response_value["user_judgment"]["options"][0]["resolution_outcome"],
        "accepted"
    );

    let mut record_request = record_judgment_request(
        "req_record_custom_option",
        "idem_record_custom_option",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_request.selected_option_id =
        volicord_types::UserJudgmentOptionId::new("reject_like_custom_id");

    let response = harness
        .service
        .record_user_judgment(record_request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["machine_action"],
        "accept"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        "accepted"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn record_user_judgment_rejects_answer_outcome_contradicting_option() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "outcome_conflict")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_outcome_conflict",
            "idem_judgment_outcome_conflict",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_outcome_conflict",
        "idem_record_outcome_conflict",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ScopeDecision,
        answer_payload(JudgmentKind::ScopeDecision),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        None
    );
    Ok(())
}

#[test]
fn record_user_judgment_rejects_blocked_answer_outcome() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "blocked_outcome")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_blocked_outcome",
            "idem_judgment_blocked_outcome",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_blocked_outcome",
        "idem_record_blocked_outcome",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ScopeDecision,
        scope_decision_payload("blocked"),
    );
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        None
    );
    Ok(())
}

#[test]
fn non_user_actor_cannot_resolve_authority_bearing_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "authority_actor")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "authority_actor",
        true,
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_authority_actor",
            "idem_judgment_authority_actor",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_authority_actor",
        "idem_record_authority_actor",
        Some(after_basis + 1),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::FinalAcceptance,
        answer_payload(JudgmentKind::FinalAcceptance),
    );
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn local_user_can_resolve_authority_bearing_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "authority_role")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_authority_role",
            "idem_judgment_authority_role",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_authority_role",
            "idem_record_authority_role",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(harness.counts()?.state_version, before.state_version + 1);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    Ok(())
}

#[test]
fn local_user_can_resolve_non_authority_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "agent_non_authority")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_agent_non_authority",
            "idem_judgment_agent_non_authority",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_agent_non_authority",
            "idem_record_agent_non_authority",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        user_judgment_actor_provenance(&harness, &pending_judgment_id)?.resolved_by_actor_source,
        Some(LOCAL_USER_ACTOR_SOURCE.to_owned())
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn agent_actor_cannot_resolve_non_authority_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "agent_non_authority_reject")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_agent_non_authority_reject",
            "idem_judgment_agent_non_authority_reject",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_agent_non_authority_reject",
            "idem_record_agent_non_authority_reject",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation_with_actor(
            ActorSource::agent_connection("connection_agent_user_only"),
            OperationCategory::UserOnly,
        ),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "invocation.actor_source"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn stored_final_acceptance_without_actor_provenance_does_not_authorize_close(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "final_missing_provenance")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_missing_provenance",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "final_missing_provenance",
    )?;
    clear_user_judgment_actor_provenance(&harness, &final_judgment_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_final_missing_provenance",
            idempotency_key: Some("idem_close_final_missing_provenance"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &final_judgment_id,
        "resolved_by_actor_source",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn rejected_final_acceptance_does_not_authorize_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let suffix = "final_negative_rejected";
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
    let after_basis = record_close_evidence(&harness, &task_id, &change_unit_id, 2, suffix, true)?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_final_negative_rejected",
            "idem_final_negative_rejected",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let final_judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let mut record = record_judgment_request(
        "req_final_negative_rejected_record",
        "idem_final_negative_rejected_record",
        Some(after_basis + 1),
        &task_id,
        &final_judgment_id,
        JudgmentKind::FinalAcceptance,
        rejected_final_acceptance_payload(),
    );
    record.selected_option_id = volicord_types::UserJudgmentOptionId::new("reject");
    let recorded = harness
        .service
        .record_user_judgment(record, invocation(OperationCategory::UserOnly))?;
    let after_final = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_final_negative_rejected",
            idempotency_key: Some("idem_close_final_negative_rejected"),
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
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &final_judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(
        user_judgment_status(&harness, &final_judgment_id)?,
        "resolved"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_final_acceptance_non_user_actor_does_not_authorize_close_or_status(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "final_non_user")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_non_user",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "final_non_user",
    )?;
    set_user_judgment_resolution_actor(&harness, &final_judgment_id, AGENT_ACTOR_SOURCE)?;
    let before = harness.counts()?;

    let close = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_final_non_user",
            idempotency_key: Some("idem_close_final_non_user"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_final_non_user",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(close.response_value["close_state"], "blocked");
    assert_close_blocker(&close.response_value, "missing_final_acceptance");
    assert_eq!(status.response_value["close_state"], "blocked");
    assert_close_blocker(&status.response_value, "missing_final_acceptance");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &final_judgment_id)?,
        Some("accepted".to_owned())
    );
    assert_eq!(
        user_judgment_status(&harness, &final_judgment_id)?,
        "resolved"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_residual_risk_acceptance_non_user_actor_covers_no_risks() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_non_user")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "risk_non_user",
        vec![residual_risk_input("Risk needing user acceptance.")],
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_non_user",
            "idem_risk_non_user",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let accepted = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_non_user_record",
            "idem_risk_non_user_record",
            Some(after_basis + 1),
            &task_id,
            &judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_risk = accepted.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    set_user_judgment_resolution_actor(&harness, &judgment_id, AGENT_ACTOR_SOURCE)?;
    record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_risk,
        "risk_non_user",
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_risk_non_user",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    let coverage = status.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["risk_id"], risk_ids[0]);
    assert_eq!(coverage[0]["accepted"], false);
    assert_eq!(coverage[0]["accepted_by_judgment_refs"], json!([]));
    assert_close_blocker(&status.response_value, "missing_residual_risk_acceptance");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &judgment_id)?,
        Some("accepted".to_owned())
    );
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stored_sensitive_approval_non_user_actor_does_not_authorize_write() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive_non_user")?;
    let (after_approval, judgment_id) =
        record_sensitive_approval(&harness, &task_id, &change_unit_id, 2, "sensitive_non_user")?;
    set_user_judgment_resolution_actor(&harness, &judgment_id, AGENT_ACTOR_SOURCE)?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_sensitive_non_user",
        "idem_prepare_sensitive_non_user",
        Some(after_approval),
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
    assert_eq!(
        response.response_value["active_user_judgment_refs"],
        json!([])
    );
    assert!(response.response_value["write_ticket"].is_null());
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "resolved");
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &judgment_id)?,
        Some("accepted".to_owned())
    );
    Ok(())
}

#[test]
fn incompatible_judgment_kind_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "kind")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_kind",
            "idem_judgment_kind",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_wrong_kind",
            "idem_record_wrong_kind",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::TechnicalDecision,
            answer_payload(JudgmentKind::TechnicalDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn final_acceptance_does_not_substitute_for_residual_risk_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk")?;
    enable_record_run_capabilities(&harness)?;
    let mut basis_request = record_run_request(
        "req_judgment_risk_basis",
        "idem_judgment_risk_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    basis_request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    basis_request.close_assessment = Some(close_assessment_with_risks(
        "Close claim supported with a residual risk.",
        vec![residual_risk_input(
            "Risk that still needs user acceptance.",
        )],
    ))
    .into();
    let basis_response = harness
        .service
        .record_run(basis_request, invocation(OperationCategory::AgentWorkflow))?;
    let after_basis = basis_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_risk",
            "idem_judgment_risk",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_final_for_risk",
            "idem_record_final_for_risk",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            answer_payload(JudgmentKind::FinalAcceptance),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn stale_scope_final_acceptance_blocks_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "final_old_scope")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_old_scope_initial",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "old_scope",
    )?;

    let scope_response = harness.service.update_scope(
        update_scope_request(
            "req_final_old_scope_change",
            "idem_final_old_scope_change",
            false,
            Some(after_final),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Materially changed scope after final acceptance.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_scope = scope_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    assert_eq!(user_judgment_status(&harness, &final_judgment_id)?, "stale");
    assert_eq!(
        user_judgment_basis_status(&harness, &final_judgment_id)?,
        "stale"
    );

    let after_new_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_scope,
        "final_old_scope_new_basis",
        true,
    )?;
    let before_close = harness.counts()?;
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_final_old_scope_close",
            idempotency_key: Some("idem_final_old_scope_close"),
            dry_run: false,
            expected_state_version: Some(after_new_basis),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    assert_eq!(harness.counts()?, before_close);
    Ok(())
}

#[test]
fn stale_close_basis_final_acceptance_blocks_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "final_old_basis")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "final_old_basis_initial",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "old_basis",
    )?;
    let after_new_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "final_old_basis_new_run",
        true,
    )?;

    assert_eq!(user_judgment_status(&harness, &final_judgment_id)?, "stale");
    assert_eq!(
        user_judgment_basis_status(&harness, &final_judgment_id)?,
        "stale"
    );
    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_final_old_basis_close",
            idempotency_key: Some("idem_final_old_basis_close"),
            dry_run: false,
            expected_state_version: Some(after_new_basis),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "stale_final_acceptance");
    Ok(())
}

#[test]
fn resolved_judgment_without_outcome_is_owner_state_corruption() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "missing_outcome")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "missing_outcome",
        true,
    )?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_missing_outcome_judgment",
            "idem_missing_outcome_judgment",
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
                "answer":{
                    "product_decision":null,
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
    let before_close = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_missing_outcome_close",
            idempotency_key: Some("idem_missing_outcome_close"),
            dry_run: false,
            expected_state_version: Some(after_basis + 1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_owner_state_value_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_machine_action",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before_close);
    Ok(())
}

#[test]
fn partial_residual_risk_acceptance_leaves_current_risk_blocker() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_partial")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "partial",
        vec![
            residual_risk_input("First risk needing acceptance."),
            residual_risk_input("Second risk needing acceptance."),
        ],
    )?;
    let accepted_risk_ids = vec![risk_ids[0].clone()];
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_partial",
            "idem_risk_partial",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let accepted = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_partial_record",
            "idem_risk_partial_record",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&accepted_risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_partial = accepted.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_partial,
        "risk_partial",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_risk_partial_close",
            idempotency_key: Some("idem_risk_partial_close"),
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
    assert_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    Ok(())
}

#[test]
fn stale_residual_risk_acceptance_is_distinct_from_missing_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_stale")?;
    let (after_old_basis, old_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "stale_old",
        vec![residual_risk_input(
            "Risk accepted against the old close basis.",
        )],
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_stale",
            "idem_risk_stale",
            false,
            Some(after_old_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let accepted = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_stale_record",
            "idem_risk_stale_record",
            Some(after_old_basis + 1),
            &task_id,
            &judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&old_risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_old_acceptance = accepted.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let (after_current_basis, current_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        after_old_acceptance,
        "stale_current",
        vec![residual_risk_input(
            "Risk accepted against the old close basis.",
        )],
    )?;
    assert_ne!(old_risk_ids[0], current_risk_ids[0]);
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_current_basis,
        "risk_stale",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_risk_stale_close",
            idempotency_key: Some("idem_risk_stale_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "stale");
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "stale_residual_risk_acceptance");
    assert_no_close_blocker(&response.response_value, "missing_residual_risk_acceptance");
    let coverage = response.response_value["risk_acceptance_coverage"]
        .as_array()
        .expect("risk coverage should be an array");
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0]["risk_id"], current_risk_ids[0]);
    assert_eq!(coverage[0]["accepted"], false);
    assert_eq!(coverage[0]["missing_reason"], "stale_acceptance");
    let risk_blocker = response.response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .find(|blocker| blocker["code"] == "stale_residual_risk_acceptance")
        .expect("stale residual-risk blocker");
    assert!(risk_blocker["related_refs"]
        .as_array()
        .expect("related refs should be an array")
        .iter()
        .any(|record_ref| record_ref["record_id"] == judgment_id));
    Ok(())
}

#[test]
fn residual_risk_answer_rejects_identical_text_with_different_risk_id() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk_identity")?;
    let (after_old_basis, old_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "old_identity",
        vec![residual_risk_input("Same visible risk text.")],
    )?;
    let (after_current_basis, current_risk_ids) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        after_old_basis,
        "current_identity",
        vec![residual_risk_input("Same visible risk text.")],
    )?;
    assert_ne!(old_risk_ids[0], current_risk_ids[0]);
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_wrong_id",
            "idem_risk_wrong_id",
            false,
            Some(after_current_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_wrong_id_record",
            "idem_risk_wrong_id_record",
            Some(after_current_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&old_risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn sensitive_approval_requires_exact_path_category_and_change_unit() -> Result<(), Box<dyn Error>> {
    let path_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&path_harness, "sensitive_path")?;
    let (after_approval, _) =
        record_sensitive_approval(&path_harness, &task_id, &change_unit_id, 2, "path")?;
    let mut request = prepare_write_request(
        "req_sensitive_path_prepare",
        "idem_sensitive_path_prepare",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["tests/export.rs".to_owned()];
    request.sensitive_categories = vec!["network".to_owned()];
    let response = path_harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_ticket"].is_null());

    let category_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&category_harness, "sensitive_category")?;
    let (after_approval, _) =
        record_sensitive_approval(&category_harness, &task_id, &change_unit_id, 2, "category")?;
    let mut request = prepare_write_request(
        "req_sensitive_category_prepare",
        "idem_sensitive_category_prepare",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["credential".to_owned()];
    let response = category_harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_ticket"].is_null());

    let cu_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&cu_harness, "sensitive_change_unit")?;
    let (after_approval, _) =
        record_sensitive_approval(&cu_harness, &task_id, &change_unit_id, 2, "change_unit")?;
    let replace = cu_harness.service.update_scope(
        update_scope_request(
            "req_sensitive_cu_replace",
            "idem_sensitive_cu_replace",
            false,
            Some(after_approval),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope for sensitive approval.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");
    let after_replace = replace.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let mut request = prepare_write_request(
        "req_sensitive_cu_prepare",
        "idem_sensitive_cu_prepare",
        Some(after_replace),
        Some(&task_id),
        Some(&replacement_change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = cu_harness
        .service
        .prepare_write(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_ticket"].is_null());
    Ok(())
}

#[test]
fn public_sensitive_lifecycle_derives_full_requirement_and_closes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "sensitive_public_lifecycle")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_sensitive_public_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["base"]["response_kind"], "result");

    let (after_sensitive, _) =
        record_sensitive_approval(&harness, &task_id, &change_unit_id, 2, "public_lifecycle")?;
    let mut prepare = prepare_write_request(
        "req_sensitive_public_prepare",
        "idem_sensitive_public_prepare",
        Some(after_sensitive),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.sensitive_categories = vec!["network".to_owned()];
    let prepared = harness
        .service
        .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(prepared.response_value["decision"], "allowed");
    let write_ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");
    let after_prepare = prepared.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    enable_record_run_capabilities(&harness)?;
    let staged = stage_artifact_for_record_run(
        &harness,
        &task_id,
        "sensitive_public_lifecycle",
        after_prepare,
    )?;
    let mut run = product_write_record_run_request(
        "req_sensitive_public_run",
        "idem_sensitive_public_run",
        after_prepare,
        &task_id,
        &change_unit_id,
        &write_ticket_id,
        "run_sensitive_public",
    );
    run.observed_changes.sensitive_categories = vec!["network".to_owned()];
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_sensitive_public",
        staged,
        Some("validation_report"),
        Some("Close claim supported."),
    )];
    run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    run.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Sensitive product write is ready for close.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: vec!["network".to_owned()],
        recovery_constraints: Vec::new(),
    })
    .into();
    let recorded = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");
    let requirement =
        &recorded.response_value["current_close_basis"]["sensitive_action_requirements"][0];
    assert_eq!(requirement["action_kind"], "local_sensitive_step");
    assert_eq!(requirement["normalized_paths"], json!(["src/export.rs"]));
    assert_eq!(requirement["sensitive_categories"], json!(["network"]));
    assert_eq!(requirement["change_unit_id"], change_unit_id);
    assert_eq!(
        requirement["source_write_ticket_ref"]["record_id"],
        write_ticket_id
    );
    assert!(requirement["action_kind"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(!requirement["normalized_paths"]
        .as_array()
        .expect("paths should be an array")
        .is_empty());
    let after_run = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_sensitive_public_status_after_run",
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
    assert_eq!(
        status.response_value["current_close_basis"]["sensitive_action_requirements"][0]
            ["normalized_paths"],
        json!(["src/export.rs"])
    );

    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_run,
        "sensitive_public_lifecycle",
    )?;
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_public_close",
            idempotency_key: Some("idem_sensitive_public_close"),
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
    assert_no_close_blocker(&closed.response_value, "missing_sensitive_approval");
    Ok(())
}

#[test]
fn close_sensitive_approval_coverage_rejects_mismatched_approvals() -> Result<(), Box<dyn Error>> {
    fn assert_mismatch(
        suffix: &str,
        requirement_categories: &[&str],
        approval_scope: volicord_types::SensitiveActionScope,
        mutate_basis: Option<fn(&mut Value)>,
        accepted: bool,
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
        let write_ticket_id = format!("wa_sensitive_{suffix}");
        let recorded = record_sensitive_product_write_close_basis(
            &harness,
            SensitiveProductWriteBasisFixture {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                expected_state_version: 2,
                suffix,
                write_ticket_id: &write_ticket_id,
                intended_operation: "local_sensitive_step",
                intended_paths: &["src/export.rs"],
                observed_categories: requirement_categories,
                assessment_categories: requirement_categories,
            },
        )?;
        assert_eq!(recorded.response_value["base"]["response_kind"], "result");
        let after_basis = recorded.response_value["base"]["state_version"]
            .as_u64()
            .expect("state_version should be present");
        let (after_approval, judgment_id) = record_sensitive_approval_with_scope(
            &harness,
            &task_id,
            &change_unit_id,
            after_basis,
            suffix,
            approval_scope,
            accepted,
        )?;
        if let Some(mutate_basis) = mutate_basis {
            mutate_user_judgment_basis_json(&harness, &judgment_id, mutate_basis)?;
        }
        let after_final =
            record_final_acceptance(&harness, &task_id, &change_unit_id, after_approval, suffix)?;
        let close_request_id = format!("req_close_sensitive_{suffix}");
        let close_idempotency_key = format!("idem_close_sensitive_{suffix}");
        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: &close_request_id,
                idempotency_key: Some(&close_idempotency_key),
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
        assert_close_blocker(&response.response_value, "missing_sensitive_approval");
        Ok(())
    }

    assert_mismatch(
        "sensitive_wrong_operation",
        &["network"],
        sensitive_scope(
            "other_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        None,
        true,
    )?;
    assert_mismatch(
        "sensitive_wrong_path",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["tests/export.rs"],
            vec!["network"],
        ),
        None,
        true,
    )?;
    assert_mismatch(
        "sensitive_partial_category",
        &["network", "credential"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        None,
        true,
    )?;
    assert_mismatch(
        "sensitive_wrong_baseline",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        Some(|basis| basis["baseline_ref"] = json!("other_baseline")),
        true,
    )?;
    assert_mismatch(
        "sensitive_wrong_change_unit",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        Some(|basis| basis["change_unit_id"] = json!("other_change_unit")),
        true,
    )?;
    assert_mismatch(
        "sensitive_rejected",
        &["network"],
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        None,
        false,
    )?;
    Ok(())
}

#[test]
fn multiple_sensitive_requirements_require_complete_coverage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive_multiple")?;
    let first = record_sensitive_product_write_close_basis(
        &harness,
        SensitiveProductWriteBasisFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: 2,
            suffix: "multiple_network",
            write_ticket_id: "wa_sensitive_multiple_network",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            observed_categories: &["network"],
            assessment_categories: &["network"],
        },
    )?;
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let second = record_sensitive_product_write_close_basis(
        &harness,
        SensitiveProductWriteBasisFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: after_first,
            suffix: "multiple_credential",
            write_ticket_id: "wa_sensitive_multiple_credential",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            observed_categories: &["credential"],
            assessment_categories: &["network", "credential"],
        },
    )?;
    let requirements = second.response_value["current_close_basis"]
        ["sensitive_action_requirements"]
        .as_array()
        .expect("requirements should be an array");
    assert_eq!(requirements.len(), 2);
    let after_second = second.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let (after_network, _) = record_sensitive_approval_with_scope(
        &harness,
        &task_id,
        &change_unit_id,
        after_second,
        "multiple_network_only",
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["network"],
        ),
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_network,
        "multiple_network_only",
    )?;
    let blocked = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_multiple_blocked",
            idempotency_key: Some("idem_sensitive_multiple_blocked"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(blocked.response_value["close_state"], "blocked");
    assert_close_blocker(&blocked.response_value, "missing_sensitive_approval");

    let (after_credential, _) = record_sensitive_approval_with_scope(
        &harness,
        &task_id,
        &change_unit_id,
        after_final,
        "multiple_credential",
        sensitive_scope(
            "local_sensitive_step",
            vec!["src/export.rs"],
            vec!["credential"],
        ),
        true,
    )?;
    let closed = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_multiple_closed",
            idempotency_key: Some("idem_sensitive_multiple_closed"),
            dry_run: false,
            expected_state_version: Some(after_credential),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    Ok(())
}

#[test]
fn close_assessment_cannot_invent_or_erase_sensitive_requirements() -> Result<(), Box<dyn Error>> {
    let invent_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&invent_harness, "sensitive_invent")?;
    enable_record_run_capabilities(&invent_harness)?;
    let mut invent = record_run_request(
        "req_sensitive_invent",
        "idem_sensitive_invent",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    invent.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    invent.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Caller tries to invent a sensitive category.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: vec!["network".to_owned()],
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_invent = invent_harness.counts()?;
    let invented = invent_harness
        .service
        .record_run(invent, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(invented.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        invented.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(invent_harness.counts()?, before_invent);

    let erase_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&erase_harness, "sensitive_erase")?;
    let first = record_sensitive_product_write_close_basis(
        &erase_harness,
        SensitiveProductWriteBasisFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: 2,
            suffix: "erase_initial",
            write_ticket_id: "wa_sensitive_erase_initial",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            observed_categories: &["network"],
            assessment_categories: &["network"],
        },
    )?;
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    enable_record_run_capabilities(&erase_harness)?;
    let mut erase = record_run_request(
        "req_sensitive_erase",
        "idem_sensitive_erase",
        false,
        Some(after_first),
        &task_id,
        &change_unit_id,
    );
    erase.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    erase.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Caller tries to erase the sensitive requirement.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_erase = erase_harness.counts()?;
    let erased = erase_harness
        .service
        .record_run(erase, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(erased.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        erased.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(erase_harness.counts()?, before_erase);
    Ok(())
}

#[test]
fn category_only_close_basis_is_corrupt_owner_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "category_only_basis")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "category_only_basis",
        true,
    )?;
    let revision = task_revision(&harness, &task_id)?;
    let mut category_only_basis = serde_json::to_value(
        revision
            .current_close_basis
            .expect("close basis should exist"),
    )?;
    category_only_basis["sensitive_categories"] = json!(["network"]);
    category_only_basis
        .as_object_mut()
        .expect("close basis should be an object")
        .remove("sensitive_action_requirements");
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(&category_only_basis.to_string()),
    )?;
    let before = harness.counts()?;

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_category_only_basis_check",
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
        &check,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_change_supersedes_pending_judgment_and_stale_pending_answer_has_no_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "pending_superseded")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_pending_superseded",
            "idem_pending_superseded",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    assert_eq!(
        pending_judgment.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "waiting_user"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &pending_judgment_id)?,
        "current"
    );
    let scope_response = harness.service.update_scope(
        update_scope_request(
            "req_pending_superseded_material_scope",
            "idem_pending_superseded_material_scope",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Material scope change after pending judgment.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        scope_response.response_value["base"]["response_kind"], "result",
        "{:?}",
        scope_response.response_value
    );
    assert_eq!(scope_response.response_value["base"]["state_version"], 4);
    assert_eq!(
        scope_response.response_value["state"]["pending_user_judgment_refs"],
        json!([])
    );
    assert_eq!(
        scope_response.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "ready"
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "ready"
    );
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "superseded"
    );
    assert_eq!(
        user_judgment_basis_status(&harness, &pending_judgment_id)?,
        "superseded"
    );
    let before = harness.counts()?;
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_pending_superseded_answer",
            "idem_pending_superseded_answer",
            Some(4),
            &task_id,
            &pending_judgment_id,
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
    Ok(())
}

#[test]
fn new_run_invalidates_final_acceptance_wait_and_restores_ready() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_invalidates_wait")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "run_invalidates_wait",
        true,
    )?;
    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_final_wait_before_new_run",
            "idem_final_wait_before_new_run",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending.response_value, "user_judgment_ref");
    assert_eq!(
        pending.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "waiting_user"
    );

    let recorded = harness.service.record_run(
        record_run_request(
            "req_run_invalidates_final_wait",
            "idem_run_invalidates_final_wait",
            false,
            Some(after_basis + 1),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        recorded.response_value["state"]["pending_user_judgment_refs"],
        json!([])
    );
    assert_eq!(
        recorded.response_value["state"]["lifecycle"]["lifecycle_phase"],
        "ready"
    );
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "ready"
    );
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "superseded");
    Ok(())
}

#[test]
fn basisless_resolved_judgment_is_rejected_by_storage_constraint() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "basis_required")?;
    let after_basis = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "basis_required",
        true,
    )?;
    let (after_final, final_judgment_id) = record_final_acceptance_with_id(
        &harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "basis_required",
    )?;
    let before = harness.counts()?;

    let error = harness
        .conn()?
        .execute(
            "UPDATE user_judgments
                SET basis_json = NULL
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![PROJECT_ID, final_judgment_id],
        )
        .expect_err("basis_json is required for stored judgments");
    assert_constraint_error(error);
    assert_eq!(harness.counts()?, before);
    assert_eq!(after_final, before.state_version);
    Ok(())
}

#[test]
fn bare_array_authority_options_are_owner_state_corruption() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "bare_authority_options")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bare_authority_options",
            "idem_bare_authority_options",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::Cancellation,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    set_user_judgment_owner_json(
        &harness,
        &pending_judgment_id,
        "options_json",
        Some(
            r#"[{
                "option_id":"accept",
                "label":"Accept",
                "description":"Bare array option without machine action.",
                "consequence":"Ambiguity must not become current authority.",
                "is_default":true
            }]"#,
        ),
    )?;
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_bare_authority_options",
            "idem_record_bare_authority_options",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::Cancellation,
            answer_payload(JudgmentKind::Cancellation),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &pending_judgment_id,
        "options_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        None
    );
    Ok(())
}

#[test]
fn record_user_judgment_rejects_selected_option_outside_original_request(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "judgment_option")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_option",
            "idem_judgment_option",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_judgment_option_record",
        "idem_judgment_option_record",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    request.selected_option_id = volicord_types::UserJudgmentOptionId::new("not_an_option");
    let before = harness.counts()?;
    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn sensitive_action_scope_does_not_create_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_sensitive",
            "idem_judgment_sensitive",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_sensitive",
            "idem_record_sensitive",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(
        response.response_value["state"]["write_ticket_summary"],
        Value::Null
    );
    Ok(())
}

#[test]
fn recorded_scope_decision_does_not_change_scope_or_current_change_unit(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_judgment")?;
    let original_scope = current_change_unit_scope(&harness, &task_id)?;
    let original_current = current_change_unit_id(&harness, &task_id)?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_scope",
            "idem_judgment_scope",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_scope",
            "idem_record_scope",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["state"]["scope_summary"],
        "Initial current scope."
    );
    assert_eq!(
        current_change_unit_scope(&harness, &task_id)?,
        original_scope
    );
    assert_eq!(
        current_change_unit_id(&harness, &task_id)?,
        original_current
    );
    assert_eq!(after.change_units, before.change_units);
    Ok(())
}

#[test]
fn local_user_channel_records_authority_judgment_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "judgment_provenance")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_provenance",
            "idem_judgment_provenance",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_judgment_provenance",
            "idem_record_judgment_provenance",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    assert_eq!(
        user_judgment_resolution_machine_action(&harness, &pending_judgment_id)?,
        Some("accept".to_owned())
    );
    assert_eq!(
        user_judgment_resolution_outcome(&harness, &pending_judgment_id)?,
        Some("accepted".to_owned())
    );
    let provenance = user_judgment_actor_provenance(&harness, &pending_judgment_id)?;
    assert_eq!(
        provenance.resolved_by_actor_source,
        Some(LOCAL_USER_ACTOR_SOURCE.to_owned())
    );
    let verification_basis = provenance
        .resolved_verification_basis
        .expect("resolved verification basis should be present");
    assert_eq!(verification_basis, VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    assert_eq!(
        provenance.resolved_assurance_level,
        Some("local_user_channel".to_owned())
    );
    assert_eq!(harness.counts()?.user_judgments, before.user_judgments);
    Ok(())
}

#[test]
fn agent_connection_cannot_record_authority_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "agent_connection")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_agent_connection_judgment",
            "idem_agent_connection_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_agent_connection_record",
            "idem_agent_connection_record",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "invocation.operation_category"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn agent_actor_cannot_record_user_only_judgment_answer() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "agent_user_only_actor")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_agent_actor_user_only_judgment",
            "idem_agent_actor_user_only_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_agent_actor_user_only_record",
            "idem_agent_actor_user_only_record",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation_with_actor(
            ActorSource::agent_connection("connection_agent_user_only_answer"),
            OperationCategory::UserOnly,
        ),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "invocation.actor_source"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn local_web_consent_rejects_token_bound_to_different_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "local_web_wrong_judgment")?;
    let first = harness.service.request_user_judgment(
        user_judgment_request(
            "req_local_web_wrong_judgment_first",
            "idem_local_web_wrong_judgment_first",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_judgment_id = response_record_id(&first.response_value, "user_judgment_ref");
    let second = harness.service.request_user_judgment(
        user_judgment_request(
            "req_local_web_wrong_judgment_second",
            "idem_local_web_wrong_judgment_second",
            false,
            Some(3),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_judgment_id = response_record_id(&second.response_value, "user_judgment_ref");
    let token = "6666666666666666666666666666666666666666666666666666666666666666";
    let token_hash = create_local_web_token_for_judgment(&harness, token, &first_judgment_id)?;
    let before = harness.counts()?;

    let response = harness.service.record_local_web_consent_judgment(
        crate::LocalWebConsentJudgmentRequest {
            request: record_judgment_request(
                "req_local_web_wrong_judgment_record",
                "idem_local_web_wrong_judgment_record",
                Some(4),
                &task_id,
                &second_judgment_id,
                JudgmentKind::ProductDecision,
                answer_payload(JudgmentKind::ProductDecision),
            ),
            token: token.to_owned(),
            expected_connection_internal_id: CONNECTION_ID.to_owned(),
            completion_metadata_json: "{}".to_owned(),
        },
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &first_judgment_id)?,
        "pending"
    );
    assert_eq!(
        user_judgment_status(&harness, &second_judgment_id)?,
        "pending"
    );
    assert_eq!(
        local_web_token_status(&harness, &token_hash)?,
        ("pending".to_owned(), None, None)
    );
    Ok(())
}

#[test]
fn local_web_consent_rejects_agent_origin_without_consuming_token() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "local_web_agent_origin")?;
    let pending = harness.service.request_user_judgment(
        user_judgment_request(
            "req_local_web_agent_origin",
            "idem_local_web_agent_origin",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = response_record_id(&pending.response_value, "user_judgment_ref");
    let token = "7777777777777777777777777777777777777777777777777777777777777777";
    let token_hash = create_local_web_token_for_judgment(&harness, token, &judgment_id)?;
    let before = harness.counts()?;

    let response = harness.service.record_local_web_consent_judgment(
        crate::LocalWebConsentJudgmentRequest {
            request: record_judgment_request(
                "req_local_web_agent_origin_record",
                "idem_local_web_agent_origin_record",
                Some(3),
                &task_id,
                &judgment_id,
                JudgmentKind::ProductDecision,
                answer_payload(JudgmentKind::ProductDecision),
            ),
            token: token.to_owned(),
            expected_connection_internal_id: CONNECTION_ID.to_owned(),
            completion_metadata_json: "{}".to_owned(),
        },
        local_web_invocation(
            ActorSource::agent_connection(CONNECTION_ID),
            OperationCategory::UserOnly,
        ),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "invocation.actor_source"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    assert_eq!(
        local_web_token_status(&harness, &token_hash)?,
        ("pending".to_owned(), None, None)
    );
    Ok(())
}

#[test]
fn accepted_authority_judgments_require_structured_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "rationale_required")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_rationale_required",
            "idem_rationale_required",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut request = record_judgment_request(
        "req_record_rationale_required",
        "idem_record_rationale_required",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ScopeDecision,
        answer_payload(JudgmentKind::ScopeDecision),
    );
    request.rationale.selected_reason = None.into();
    let before = harness.counts()?;

    let response = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "rationale.selected_reason"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn project_continuity_and_rationale_do_not_replace_final_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "continuity_not_auth")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "continuity_not_auth",
        true,
    )?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_continuity_product",
            "idem_continuity_product",
            false,
            Some(after_evidence),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let recorded = harness.service.record_user_judgment(
        record_judgment_request(
            "req_continuity_product_record",
            "idem_continuity_product_record",
            Some(after_evidence + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_judgment = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let continuity = harness.continuity_records()?;
    assert_eq!(continuity.len(), 1);
    assert_eq!(continuity[0].kind, "decision");
    let before_close = harness.counts()?;

    let close = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_continuity_not_auth_close",
            idempotency_key: Some("idem_continuity_not_auth_close"),
            dry_run: false,
            expected_state_version: Some(after_judgment),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(close.response_value["close_state"], "blocked");
    assert_close_blocker(&close.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before_close);
    Ok(())
}

#[test]
fn final_and_residual_risk_acceptance_are_non_substitutable() -> Result<(), Box<dyn Error>> {
    let final_only_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&final_only_harness, "final_only_risk")?;
    let (after_basis, _) = record_close_basis_with_risks(
        &final_only_harness,
        &task_id,
        &change_unit_id,
        2,
        "final_only_risk",
        vec![residual_risk_input("Risk still needs separate acceptance.")],
    )?;
    let after_final = record_final_acceptance(
        &final_only_harness,
        &task_id,
        &change_unit_id,
        after_basis,
        "final_only_risk",
    )?;
    let final_only = final_only_harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_final_only_risk_close",
            idempotency_key: Some("idem_final_only_risk_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_close_blocker(
        &final_only.response_value,
        "missing_residual_risk_acceptance",
    );
    assert_no_close_blocker(&final_only.response_value, "missing_final_acceptance");

    let risk_only_harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&risk_only_harness, "risk_only_final")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &risk_only_harness,
        &task_id,
        &change_unit_id,
        2,
        "risk_only_final",
        vec![residual_risk_input(
            "Risk is accepted but final acceptance is absent.",
        )],
    )?;
    let pending_judgment = risk_only_harness.service.request_user_judgment(
        user_judgment_request(
            "req_risk_only_judgment",
            "idem_risk_only_judgment",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let recorded = risk_only_harness.service.record_user_judgment(
        record_judgment_request(
            "req_risk_only_record",
            "idem_risk_only_record",
            Some(after_basis + 1),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            residual_risk_acceptance_payload(&risk_ids),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_risk = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let risk_only = risk_only_harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_risk_only_final_close",
            idempotency_key: Some("idem_risk_only_final_close"),
            dry_run: false,
            expected_state_version: Some(after_risk),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_close_blocker(&risk_only.response_value, "missing_final_acceptance");
    assert_no_close_blocker(
        &risk_only.response_value,
        "missing_residual_risk_acceptance",
    );
    Ok(())
}

#[test]
fn judgment_dry_runs_have_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_judgment")?;
    let before_request = harness.counts()?;

    let request_preview = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_dry",
            "idem_judgment_dry",
            true,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        request_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(harness.counts()?, before_request);

    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_dry_record",
            "idem_judgment_dry_record",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before_record = harness.counts()?;

    let mut record_preview_request = record_judgment_request(
        "req_record_dry",
        "idem_record_dry",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_preview_request.envelope.dry_run = true;
    let record_preview = harness.service.record_user_judgment(
        record_preview_request,
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(
        record_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(harness.counts()?, before_record);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn stale_state_rejects_record_user_judgment_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "stale_judgment")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_stale",
            "idem_judgment_stale",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_stale",
            "idem_record_stale",
            Some(2),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn record_user_judgment_idempotency_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "replay_judgment")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_replay",
            "idem_judgment_replay",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_replay",
        "idem_record_replay",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );

    let first = harness
        .service
        .record_user_judgment(request.clone(), invocation(OperationCategory::UserOnly))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_user_judgment(request, invocation(OperationCategory::UserOnly))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}
