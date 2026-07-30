use super::*;

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
    assert!(response.response_value["pending_user_action_summaries"]
        .as_array()
        .expect("pending summaries should be an array")
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
fn reconcile_changes_dry_run_classifies_user_action_without_state_effect(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_judgment_dry")?;
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
    dry_run_request.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;
    dry_run_request.envelope.idempotency_key = RequiredNullable::null();
    let dry_run = harness.service.reconcile_changes(
        dry_run_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(
        dry_run.response_value["dry_run_summary"]["planned_effects"][0]["description"],
        "Classify 0 automatically reconcilable change(s) and 1 change(s) needing a user action."
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
        .any(|value| value == "changes_needing_user_action=1"));
    assert!(diagnostics
        .iter()
        .any(|value| value == "would_create_user_actions=1"));
    assert!(diagnostics.iter().any(|value| value
        .as_str()
        .is_some_and(|text| text.contains(&unrecorded_change_id))));
    assert!(diagnostics.iter().all(|value| value
        .as_str()
        .is_none_or(|text| !text.contains("question="))));
    assert!(dry_run.response_value["dry_run_summary"]["next_actions"]
        .as_array()
        .expect("dry-run next_actions should be an array")
        .iter()
        .all(|action| action["blocking_question"].is_null()));
    assert!(!dry_run
        .response_value
        .to_string()
        .contains("Does the user accept the observed Product Repository change as intentional?"));
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
    );
    assert_eq!(dry_run_generator.count(DurableIdKind::UserActionRequest), 0);
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
        committed.response_value["pending_user_action_summaries"]
            .as_array()
            .expect("pending summaries should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_action_requests, before.user_action_requests + 1);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(commit_generator.count(DurableIdKind::UserActionRequest), 1);
    assert_eq!(commit_generator.count(DurableIdKind::Event), 1);
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
    );
    Ok(())
}

#[test]
fn reconcile_changes_commits_multiple_user_actions_with_shared_source_idempotency(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_multi_origin")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_multi_origin")?;
    let first_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_multi_origin_a")?;
    let second_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_multi_origin_b")?;
    let before = harness.counts()?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_multi_origin",
            "idem_reconcile_multi_origin",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["pending_user_action_summaries"]
            .as_array()
            .expect("pending summaries should be an array")
            .len(),
        2
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_action_requests, before.user_action_requests + 2);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);

    let pending_request_ids = response.response_value["pending_user_action_summaries"]
        .as_array()
        .expect("pending summaries should be an array")
        .iter()
        .map(|summary| {
            summary["user_action_request_id"]
                .as_str()
                .expect("pending summary should identify its request")
                .to_owned()
        })
        .collect::<Vec<_>>();
    for request_id in pending_request_ids {
        let resumed = harness.service.resume_user_action_request(
            ProjectId::new(PROJECT_ID),
            volicord_types::ids::UserActionRequestId::new(request_id),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert!(
            resumed.is_none(),
            "reconcile-origin requests must not replay the compound reconcile result"
        );
        assert_eq!(
            harness.counts()?,
            after,
            "denied resume must not create storage effects"
        );
    }

    let source_rows = {
        let conn = harness.conn()?;
        let mut stmt = conn.prepare(
            "SELECT source_method, source_idempotency_key
               FROM user_action_requests
              WHERE project_id = ?1
              ORDER BY user_action_request_id",
        )?;
        let rows = stmt.query_map([PROJECT_ID], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<(String, String)>, _>>()?
    };
    assert_eq!(source_rows.len(), 2);
    assert!(source_rows.iter().all(|(source_method, source_key)| {
        source_method == MethodName::ReconcileChanges.as_str()
            && source_key == "idem_reconcile_multi_origin"
    }));
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &first_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &second_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
    );
    Ok(())
}

#[test]
fn reconcile_changes_creates_and_consumes_user_acceptance_judgment() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_accept")?;
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
        first.response_value["pending_user_action_summaries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
    );
    let judgment_id = first.response_value["pending_user_action_summaries"][0]
        ["user_action_request_id"]
        .as_str()
        .expect("pending judgment summary should identify its request")
        .to_owned();
    let recorded = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_reconcile_accept_record",
            "idem_reconcile_accept_record",
            None,
            &task_id,
            &judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let user_action_resolution_id =
        response_record_id(&recorded.response_value, "user_action_resolution_ref");

    let second = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_accept_second",
            "idem_reconcile_accept_second",
            Some(after_record),
            &task_id,
            vec![UnrecordedChangeResolutionRequest {
                unrecorded_change_id: UnrecordedChangeId::new(unrecorded_change_id.clone()),
                basis: UnrecordedChangeResolutionBasis::AcceptedByUser,
                user_action_resolution_id: Some(volicord_types::ids::UserActionResolutionId::new(
                    user_action_resolution_id.clone(),
                ))
                .into(),
            }],
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
    assert!(second.response_value["pending_user_action_summaries"]
        .as_array()
        .expect("pending summaries should be an array")
        .is_empty());
    let row = unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?;
    assert_eq!(row.status, UnrecordedChangeStatus::Resolved);
    let resolution = row_resolution(&row);
    assert_eq!(resolution["resolution_basis"], "accepted_by_user");
    assert_eq!(
        resolution["capture_basis"],
        UserActionVerificationBasis::CliDirectUserChannel.as_str()
    );
    assert_eq!(
        resolution["user_action_resolution_ref"]["record_id"],
        user_action_resolution_id.as_str()
    );
    assert_eq!(row.resolved_by_actor_source, Some(ActorSource::LocalUser));
    Ok(())
}

#[test]
fn reconcile_changes_local_recovery_consumes_user_acceptance_and_removes_close_blocker(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_local_accept")?;
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
    let judgment_id = first.response_value["pending_user_action_summaries"][0]
        ["user_action_request_id"]
        .as_str()
        .expect("pending judgment summary should identify its request")
        .to_owned();
    let recorded = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_reconcile_local_accept_record",
            "idem_reconcile_local_accept_record",
            None,
            &task_id,
            &judgment_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");
    let user_action_resolution_id =
        response_record_id(&recorded.response_value, "user_action_resolution_ref");

    let second = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_local_accept_second",
            "idem_reconcile_local_accept_second",
            Some(after_record),
            &task_id,
            vec![UnrecordedChangeResolutionRequest {
                unrecorded_change_id: UnrecordedChangeId::new(unrecorded_change_id.clone()),
                basis: UnrecordedChangeResolutionBasis::AcceptedByUser,
                user_action_resolution_id: Some(volicord_types::ids::UserActionResolutionId::new(
                    user_action_resolution_id,
                ))
                .into(),
            }],
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
    assert_eq!(row.status, UnrecordedChangeStatus::Resolved);
    assert_eq!(row.resolved_by_actor_source, Some(ActorSource::LocalUser));
    let resolution = row_resolution(&row);
    assert_eq!(
        resolution["capture_basis"],
        UserActionVerificationBasis::CliDirectUserChannel.as_str()
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
    record_guard_installation(&harness, "reconcile_reject")?;
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
                basis: UnrecordedChangeResolutionBasis::RecordedAsExpectedWrite,
                user_action_resolution_id: RequiredNullable::null(),
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
        UnrecordedChangeStatus::Unresolved
    );
    Ok(())
}

#[test]
fn reconcile_changes_rejects_agent_direct_accepted_by_user_without_judgment(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_agent_accept")?;
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
                user_action_resolution_id: Some(volicord_types::ids::UserActionResolutionId::new(
                    "uar_missing_accept",
                ))
                .into(),
            }],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["rejected_resolution_requests"][0]["code"],
        "user_action_resolution_not_accepted"
    );
    assert_eq!(
        response.response_value["unresolved_changes"][0]["unrecorded_change_ref"]["record_id"],
        unrecorded_change_id
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
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
        InvocationContext::local_user(
            ProjectId::new(other_project_id),
            OperationCategory::LocalRecovery,
            UserActionChannelKind::Cli,
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
fn reconcile_changes_rejects_invalid_persisted_observation_at_store_boundary(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_invalid")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "reconcile_invalid")?;
    let unrecorded_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "reconcile_invalid",
        r#"["src/export.rs"]"#,
    )?;
    harness.conn()?.execute(
        "UPDATE unrecorded_changes
            SET observed_paths_json = '[123]'
          WHERE project_id = ?1
            AND unrecorded_change_id = ?2",
        rusqlite::params![PROJECT_ID, unrecorded_change_id],
    )?;
    let before = harness.counts()?;

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
    assert_owner_state_rejection(
        &response,
        "unrecorded_changes",
        &unrecorded_change_id,
        "observed_paths_json",
        &harness.runtime_home_path,
    );
    let status: String = harness.conn()?.query_row(
        "SELECT status
           FROM unrecorded_changes
          WHERE project_id = ?1
            AND unrecorded_change_id = ?2",
        rusqlite::params![PROJECT_ID, unrecorded_change_id],
        |row| row.get(0),
    )?;
    assert_eq!(status, "unresolved");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn reconcile_changes_resolves_deterministic_active_write_ticket() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_ticket")?;
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
    assert_eq!(row.status, UnrecordedChangeStatus::Resolved);
    assert_eq!(
        row_resolution(&row)["capture_basis"],
        "core_deterministic_write_ticket"
    );
    Ok(())
}

#[test]
fn reconcile_changes_keeps_ambiguous_active_write_ticket_match_unresolved(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    record_guard_installation(&harness, "reconcile_ticket_ambiguous")?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "reconcile_ticket_ambiguous")?;
    let ticket_ids = ["write_ticket_z", "write_ticket_a"];
    for (index, write_ticket_id) in ticket_ids.iter().enumerate() {
        insert_active_write_ticket_with_scope(
            &harness,
            WriteTicketScopeFixture {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                write_ticket_id,
                basis_state_version: u64::try_from(index + 3)?,
                created_at: "2026-06-18T00:00:00.000Z",
                expires_at: "2026-06-18T00:15:00.000Z",
                intended_operation: "local_sensitive_step",
                intended_paths: &["src/export.rs"],
                sensitive_categories: &[],
                approval_basis_refs: Vec::new(),
            },
        )?;
    }
    let unrecorded_change_id =
        insert_guarded_unrecorded_change(&harness, &task_id, "reconcile_ticket_ambiguous")?;

    let response = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_reconcile_ticket_ambiguous",
            "idem_reconcile_ticket_ambiguous",
            Some(2),
            &task_id,
            Vec::new(),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(
        response.response_value["unresolved_changes"][0]["unrecorded_change_ref"]["record_id"],
        unrecorded_change_id
    );
    assert!(response.response_value["resolved_changes"]
        .as_array()
        .expect("resolved_changes should be an array")
        .is_empty());
    assert_eq!(
        response.response_value["pending_user_action_summaries"]
            .as_array()
            .expect("pending summaries should be an array")
            .len(),
        1
    );
    assert_eq!(
        unrecorded_change_row(&harness, PROJECT_ID, &unrecorded_change_id)?.status,
        UnrecordedChangeStatus::Unresolved
    );
    for write_ticket_id in ticket_ids {
        let record = harness
            .store()?
            .write_ticket_record(write_ticket_id)?
            .ok_or_else(|| format!("missing Write Ticket fixture {write_ticket_id}"))?;
        assert_eq!(record.status(), WriteTicketStatus::Active);
    }
    Ok(())
}
