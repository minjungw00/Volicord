use super::*;

#[test]
fn reconcile_changes_resolves_not_product_change_and_updates_close_blocker(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "reconcile_not_product",
        "detective",
        "active",
        "{}",
    )?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "reconcile_not_product")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "reconcile_not_product",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "reconcile_not_product",
    )?;
    let unrecorded_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_not_product",
        "[]",
    )?;

    let before = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_reconcile_not_product_check_before",
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
    assert_close_blocker(&before.response_value, "unresolved_unrecorded_changes");
    assert_close_blocker_resolution(
        &before.response_value,
        "unresolved_unrecorded_changes",
        true,
        false,
    );
    let blocker = close_blocker_by_code(&before.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        blocker["next_actions"][0]["owner_method"],
        "volicord.reconcile_changes"
    );
    assert_eq!(
        blocker["next_actions"][0]["action_kind"],
        "reconcile_changes"
    );
    let before_dry_run_counts = harness.counts()?;
    let mut dry_run_request = reconcile_changes_request(
        "req_reconcile_not_product_dry",
        "idem_reconcile_not_product_dry",
        Some(after_final),
        &task_id,
        Vec::new(),
    );
    dry_run_request.envelope.dry_run = true;
    dry_run_request.envelope.idempotency_key = RequiredNullable::null();

    let dry_run = harness.service.reconcile_changes(
        dry_run_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(dry_run.response_value["base"]["effect_kind"], "no_effect");
    assert_authority_disclosure(&dry_run.response_value);
    assert_eq!(
        dry_run.response_value["dry_run_summary"]["planned_effects"][0]["action"],
        "classify"
    );
    assert_eq!(
        dry_run.response_value["dry_run_summary"]["planned_effects"][1]["action"],
        "would_resolve"
    );
    assert!(dry_run.response_value["dry_run_summary"]["would_blockers"]
        .as_array()
        .expect("would_blockers should be an array")
        .iter()
        .all(|blocker| blocker["code"] != "unresolved_unrecorded_changes"));
    let diagnostics = dry_run.response_value["dry_run_summary"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");
    assert!(diagnostics
        .iter()
        .any(|value| value == "automatically_reconcilable_changes=1"));
    assert!(diagnostics
        .iter()
        .any(|value| value == "changes_needing_user_judgment=0"));
    assert!(diagnostics
        .iter()
        .any(|value| { value == "close_blockers=unresolved_unrecorded_changes would be reduced" }));
    assert!(diagnostics.iter().any(
        |value| value == "non_guarantees=no actor proof; no intent proof; no correctness proof"
    ));
    assert_eq!(harness.counts()?, before_dry_run_counts);
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_not_product",
            "idem_reconcile_not_product",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_authority_disclosure(&response.response_value);
    assert_eq!(
        response.response_value["summary_card"]["recording"],
        "core_committed"
    );
    assert_eq!(response.response_value["summary_card"]["changes"], "none");
    assert_eq!(
        response.response_value["summary_card"]["evidence"],
        response.response_value["state"]["evidence_gate"]["state"]
    );
    assert_eq!(
        response.response_value["summary_card"]["next"],
        response.response_value["close_blockers"][0]["next_actions"][0]["label"]
    );
    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "not_product_change"
    );
    assert_eq!(
        response.response_value["resolved_changes"][0]["resolved_by_actor_source"],
        "system"
    );
    assert!(
        response.response_value["next_actions"]
            .as_array()
            .expect("next_actions should be an array")
            .is_empty(),
        "deterministically resolved reconciliation should not leave stale next actions: {:?}",
        response.response_value["next_actions"]
    );
    assert_no_close_blocker(&response.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    let resolution = row_resolution(&row);
    assert_eq!(resolution["resolution_basis"], "not_product_change");
    assert_eq!(
        resolution["capture_basis"],
        "core_deterministic_not_product_change"
    );

    let after = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_reconcile_not_product_check_after",
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
    assert_no_close_blocker(&after.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        response.response_value["state"]["evidence_gate"],
        after.response_value["evidence_gate"]
    );
    Ok(())
}

#[test]
fn reconcile_changes_accepts_local_recovery_and_persists_replay_category(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_local", "detective", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_local")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change_with_paths(&harness, &task_id, "reconcile_local", "[]")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local",
            "idem_reconcile_local",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "not_product_change"
    );
    assert_eq!(
        response
            .verified_invocation
            .as_ref()
            .expect("local recovery should verify invocation")
            .operation_category,
        OperationCategory::LocalRecovery
    );
    assert_eq!(
        response
            .verified_invocation
            .as_ref()
            .expect("local recovery should verify invocation")
            .actor_source,
        ActorSource::LocalUser
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "resolved"
    );

    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let replay = store
        .tool_invocation(
            MethodName::ReconcileChanges,
            &IdempotencyKey::new("idem_reconcile_local"),
        )?
        .expect("local recovery commit should persist replay row");
    assert_eq!(replay.actor_source, LOCAL_USER_ACTOR_SOURCE);
    assert_eq!(replay.operation_category, "local_recovery");
    assert_eq!(
        replay.verification_basis.as_deref(),
        Some(VERIFICATION_BASIS_TEST_FIXTURE_BINDING)
    );
    Ok(())
}

#[test]
fn reconcile_changes_local_recovery_reports_no_unresolved_findings_read_only(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_none")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_none",
            "idem_reconcile_none",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert!(response.response_value["unresolved_changes"]
        .as_array()
        .expect("unresolved_changes should be an array")
        .is_empty());
    assert!(response.response_value["resolved_changes"]
        .as_array()
        .expect("resolved_changes should be an array")
        .is_empty());
    assert!(response.response_value["pending_user_judgment_refs"]
        .as_array()
        .expect("pending refs should be an array")
        .is_empty());
    assert_eq!(
        response
            .verified_invocation
            .as_ref()
            .expect("local recovery should verify invocation")
            .actor_source,
        ActorSource::LocalUser
    );
    Ok(())
}

#[test]
fn reconcile_changes_dry_run_classifies_user_judgment_without_state_effect(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "reconcile_judgment_dry",
        "detective",
        "active",
        "{}",
    )?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_judgment_dry")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_judgment_dry")?;
    let dry_run_generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-30T00:10:00Z");
    harness.use_generator_and_clock(dry_run_generator.clone(), clock.clone());
    let before = harness.counts()?;

    let mut dry_run_request = reconcile_changes_request(
        "req_reconcile_judgment_dry",
        "idem_reconcile_judgment_dry",
        Some(2),
        &task_id,
        Vec::new(),
    );
    dry_run_request.envelope.dry_run = true;
    dry_run_request.envelope.idempotency_key = RequiredNullable::null();
    let dry_run = harness.service.reconcile_changes(
        dry_run_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(
        dry_run.response_value["dry_run_summary"]["planned_effects"][0]["description"],
        "Classify 0 automatically reconcilable change(s) and 1 change(s) needing user judgment."
    );
    assert!(dry_run.response_value["dry_run_summary"]["planned_effects"]
        .as_array()
        .expect("planned_effects should be an array")
        .iter()
        .any(|effect| effect["action"] == "would_request"));
    let diagnostics = dry_run.response_value["dry_run_summary"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");
    assert!(diagnostics
        .iter()
        .any(|value| value == "changes_needing_user_judgment=1"));
    assert!(diagnostics
        .iter()
        .any(|value| value == "would_create_user_judgments=1"));
    assert!(diagnostics.iter().any(|value| value
        .as_str()
        .is_some_and(|text| text.contains(&unrecorded_change_id))));
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    assert_eq!(dry_run_generator.count(DurableIdKind::UserJudgment), 0);
    assert_eq!(dry_run_generator.count(DurableIdKind::Event), 0);

    let commit_generator =
        CountingDurableIdGenerator::new(["reconcile_judgment_preview", "reconcile_judgment_event"]);
    harness.use_generator_and_clock(commit_generator.clone(), clock);
    let committed = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_judgment_commit",
            "idem_reconcile_judgment_commit",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(
        committed.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(
        committed.response_value["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(commit_generator.count(DurableIdKind::UserJudgment), 1);
    assert_eq!(commit_generator.count(DurableIdKind::Event), 1);
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn reconcile_changes_creates_and_consumes_user_acceptance_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_accept", "detective", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_accept")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_accept")?;

    let first = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_accept_first",
            "idem_reconcile_accept_first",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        first.response_value["unresolved_changes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        first.response_value["pending_user_judgment_refs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    let judgment_id = first.response_value["pending_user_judgment_refs"][0]["record_id"]
        .as_str()
        .expect("pending judgment ref should be present")
        .to_owned();
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let recorded = harness.service.record_user_judgment(
        record_judgment_request(
            "req_reconcile_accept_record",
            "idem_reconcile_accept_record",
            Some(after_first),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let second = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_accept_second",
            "idem_reconcile_accept_second",
            Some(after_record),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        second.response_value["resolved_changes"][0]["resolution_basis"],
        "accepted_by_user"
    );
    assert_eq!(
        second.response_value["resolved_changes"][0]["resolved_by_actor_source"],
        "local_user"
    );
    assert!(second.response_value["pending_user_judgment_refs"]
        .as_array()
        .expect("pending refs should be an array")
        .is_empty());
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    let resolution = row_resolution(&row);
    assert_eq!(resolution["resolution_basis"], "accepted_by_user");
    assert_eq!(
        resolution["capture_basis"],
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING
    );
    assert_eq!(
        resolution["user_judgment_ref"]["record_id"],
        judgment_id.as_str()
    );
    assert_eq!(
        row.resolved_by_actor_source.as_deref(),
        Some(LOCAL_USER_ACTOR_SOURCE)
    );
    Ok(())
}

#[test]
fn reconcile_changes_local_recovery_consumes_user_acceptance_and_removes_close_blocker(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "reconcile_local_accept",
        "detective",
        "active",
        "{}",
    )?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "reconcile_local_accept")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "reconcile_local_accept",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "reconcile_local_accept",
    )?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_local_accept")?;

    let before = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_reconcile_local_accept_before",
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
    assert_close_blocker(&before.response_value, "unresolved_unrecorded_changes");

    let first = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local_accept_first",
            "idem_reconcile_local_accept_first",
            Some(after_final),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;
    let judgment_id = first.response_value["pending_user_judgment_refs"][0]["record_id"]
        .as_str()
        .expect("pending judgment ref should be present")
        .to_owned();
    let after_first = first.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let recorded = harness.service.record_user_judgment(
        record_judgment_request(
            "req_reconcile_local_accept_record",
            "idem_reconcile_local_accept_record",
            Some(after_first),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let second = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local_accept_second",
            "idem_reconcile_local_accept_second",
            Some(after_record),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::LocalRecovery),
    )?;

    assert_eq!(
        second.response_value["resolved_changes"][0]["resolution_basis"],
        "accepted_by_user"
    );
    assert_eq!(
        second.response_value["resolved_changes"][0]["resolved_by_actor_source"],
        LOCAL_USER_ACTOR_SOURCE
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    assert_eq!(
        row.resolved_by_actor_source.as_deref(),
        Some(LOCAL_USER_ACTOR_SOURCE)
    );
    let resolution = row_resolution(&row);
    assert_eq!(
        resolution["capture_basis"],
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING
    );

    let after = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_reconcile_local_accept_after",
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
    assert_no_close_blocker(&after.response_value, "unresolved_unrecorded_changes");
    Ok(())
}

#[test]
fn reconcile_changes_rejects_agent_supplied_system_resolution_basis() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_reject", "detective", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_reject")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_reject")?;

    let seed = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_reject_seed",
            "idem_reconcile_reject_seed",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_seed = seed.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_reject_basis",
            "idem_reconcile_reject_basis",
            Some(after_seed),
            &task_id,
            vec![UnrecordedChangeResolutionRequest {
                unrecorded_change_id: UnrecordedChangeId::new(unrecorded_change_id.clone()),
                basis: UnrecordedChangeResolutionBasis::InvalidObservation,
                user_judgment_id: RequiredNullable::null(),
            }],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["rejected_resolution_requests"][0]["code"],
        "system_resolution_basis_not_caller_owned"
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn reconcile_changes_rejects_agent_direct_accepted_by_user_without_judgment(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(
        &harness,
        "reconcile_agent_accept",
        "detective",
        "active",
        "{}",
    )?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_agent_accept")?;
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_agent_accept")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_agent_accept",
            "idem_reconcile_agent_accept",
            Some(2),
            &task_id,
            vec![UnrecordedChangeResolutionRequest {
                unrecorded_change_id: UnrecordedChangeId::new(unrecorded_change_id.clone()),
                basis: UnrecordedChangeResolutionBasis::AcceptedByUser,
                user_judgment_id: Some(UserJudgmentId::new("judgment_missing_accept")).into(),
            }],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["rejected_resolution_requests"][0]["code"],
        "user_judgment_not_accepted"
    );
    assert_eq!(
        response.response_value["unresolved_changes"][0]["unrecorded_change_ref"]["record_id"],
        unrecorded_change_id
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn reconcile_changes_rejects_mismatched_invocation_project() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_project_mismatch")?;
    let other_project_id = register_additional_project(&harness, "project_methods_mismatch")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_project_mismatch",
            "idem_reconcile_project_mismatch",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        InvocationContext::new(
            ProjectId::new(other_project_id),
            ActorSource::LocalUser,
            OperationCategory::LocalRecovery,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        ),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "envelope.project_id"
    );
    Ok(())
}

#[test]
fn reconcile_changes_resolves_invalid_observation_deterministically() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_invalid", "detective", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_invalid")?;
    let unrecorded_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_invalid",
        "[123]",
    )?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_invalid",
            "idem_reconcile_invalid",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "invalid_observation"
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    assert_eq!(
        row_resolution(&row)["capture_basis"],
        "core_deterministic_invalid_observation"
    );
    Ok(())
}

#[test]
fn reconcile_changes_isolates_other_projects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_cross", "detective", "active", "{}")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_cross")?;
    let main_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_cross_main",
        "[]",
    )?;
    let other_project_id = register_additional_project(&harness, "project_methods_other")?;
    let other_change_id = insert_project_unrecorded_change(
        &harness,
        &other_project_id,
        None,
        "reconcile_cross_other",
        "[]",
    )?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_cross",
            "idem_reconcile_cross",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["unrecorded_change_ref"]["record_id"],
        main_change_id
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &main_change_id)?.status,
        "resolved"
    );
    assert_eq!(
        unrecorded_change_row(&harness, &other_project_id, &other_change_id)?.status,
        "unresolved"
    );
    Ok(())
}

#[test]
fn reconcile_changes_resolves_deterministic_active_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_ticket", "detective", "active", "{}")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "reconcile_ticket")?;
    let prepare = harness.service.prepare_write(
        prepare_write_request(
            "req_reconcile_ticket_prepare",
            "idem_reconcile_ticket_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_prepare = prepare.response_value["base"]["state_version"]
        .as_u64()
        .expect("prepare_write should report state_version");
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_ticket")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_ticket",
            "idem_reconcile_ticket",
            Some(after_prepare),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["resolved_changes"][0]["resolution_basis"],
        "covered_by_write_ticket"
    );
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, "resolved");
    assert_eq!(
        row_resolution(&row)["capture_basis"],
        "core_deterministic_write_ticket"
    );
    Ok(())
}
