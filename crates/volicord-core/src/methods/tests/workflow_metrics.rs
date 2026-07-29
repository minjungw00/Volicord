use super::*;
use volicord_store::diagnostics::{
    read_workflow_metric_aggregates, start_diagnostic_session, DiagnosticSessionStart,
    DiagnosticTransport, WorkflowMetricAggregateRow,
};

fn start_metrics_session(
    harness: &MethodHarness,
    native_session_id: &str,
) -> Result<String, Box<dyn Error>> {
    let session_id = format!("core_workflow_{native_session_id}");
    let context = harness.service.context();
    start_diagnostic_session(
        &context,
        DiagnosticSessionStart {
            session_id: &session_id,
            connection_id: None,
            project_id: Some(PROJECT_ID),
            transport: DiagnosticTransport::CliInbox,
            host_kind: None,
            package_version: "test",
            build_id: "core-workflow-metrics-test",
        },
    )?;
    Ok(session_id)
}

fn metric_total(rows: &[WorkflowMetricAggregateRow], kind: &str) -> u64 {
    rows.iter()
        .filter(|row| row.metric_kind == kind)
        .map(|row| row.value_total)
        .sum()
}

fn metric_samples(rows: &[WorkflowMetricAggregateRow], kind: &str) -> u64 {
    rows.iter()
        .filter(|row| row.metric_kind == kind)
        .map(|row| row.sample_count)
        .sum()
}

#[test]
fn write_ticket_and_first_write_metrics_are_fresh_effect_only() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at(DEFAULT_METHOD_TEST_CLOCK);
    harness.use_clock(clock.clone());
    let session_id = start_metrics_session(&harness, "session_core_write_metrics")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "core_write_metrics")?;

    let issued = harness.service.prepare_write(
        prepare_write_request(
            "req_core_metrics_issue",
            "idem_core_metrics_issue",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(issued.response_value["write_ticket_effect"], "issued");
    let ticket_id = response_record_id(&issued.response_value, "write_ticket_ref");

    let reused = harness.service.prepare_write(
        prepare_write_request(
            "req_core_metrics_reuse",
            "idem_core_metrics_reuse",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(reused.response_value["write_ticket_effect"], "reused");

    let context = harness.service.context();
    insert_unrecorded_change(
        &context,
        PROJECT_ID,
        UnrecordedChangeInsert {
            unrecorded_change_id: "unrecorded_core_metrics_first_write".to_owned(),
            correlation: None,
            connection_internal_id: CONNECTION_ID.to_owned(),
            task_id: Some(task_id.clone()),
            confidence: UnrecordedChangeConfidence::Confirmed,
            summary: "Confirmed product write observation for metric timing.".to_owned(),
            observed_paths: vec!["src/export.rs".parse()?],
            detection: JsonObject::new(),
            detected_at: UtcTimestamp::parse("2026-06-18T00:00:01Z")?,
            metadata: JsonObject::new(),
        },
    )?;
    clock.advance(Duration::seconds(2));
    let run_request = product_write_record_run_request(
        "req_core_metrics_first_write",
        "idem_core_metrics_first_write",
        4,
        &task_id,
        &change_unit_id,
        &ticket_id,
        "run_core_metrics_first_write",
    );
    let first_run = harness.service.record_run(
        run_request.clone(),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(
        first_run.response_value["base"]["effect_kind"],
        "core_committed"
    );
    let replayed_run = harness.service.record_run(
        run_request,
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert!(replayed_run.replayed);

    let reissue_request = prepare_write_request(
        "req_core_metrics_reissue",
        "idem_core_metrics_reissue",
        Some(5),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let reissued = harness.service.prepare_write(
        reissue_request.clone(),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(reissued.response_value["write_ticket_effect"], "issued");
    let replayed_reissue = harness.service.prepare_write(
        reissue_request,
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert!(replayed_reissue.replayed);

    let reconciled = harness.service.reconcile_changes(
        reconcile_changes_request(
            "req_core_metrics_reconcile_write",
            "idem_core_metrics_reconcile_write",
            Some(6),
            &task_id,
            Vec::new(),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(
        reconciled.response_value["resolved_changes"][0]["resolution_basis"],
        "recorded_as_expected_write"
    );

    let rows = read_workflow_metric_aggregates(&harness.runtime_home_path, PROJECT_ID)?;
    assert_eq!(metric_total(&rows, "write_ticket_issued"), 1);
    assert_eq!(metric_total(&rows, "write_ticket_reused"), 1);
    assert_eq!(metric_total(&rows, "write_ticket_reissued"), 1);
    assert_eq!(
        metric_total(&rows, "first_product_write_duration_micros"),
        1_000_000
    );
    assert_eq!(
        metric_samples(&rows, "confirmed_unrecorded_false_positive"),
        1
    );
    assert_eq!(
        metric_total(&rows, "confirmed_unrecorded_false_positive"),
        0
    );
    Ok(())
}

#[test]
fn first_write_metric_stays_pending_without_an_actual_write_timestamp() -> Result<(), Box<dyn Error>>
{
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at(DEFAULT_METHOD_TEST_CLOCK);
    harness.use_clock(clock.clone());
    let session_id = start_metrics_session(&harness, "session_core_write_metric_pending")?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "core_write_metric_pending")?;
    let issued = harness.service.prepare_write(
        prepare_write_request(
            "req_core_write_metric_pending_prepare",
            "idem_core_write_metric_pending_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    let ticket_id = response_record_id(&issued.response_value, "write_ticket_ref");
    clock.advance(Duration::seconds(30));

    let recorded = harness.service.record_run(
        product_write_record_run_request(
            "req_core_write_metric_pending_run",
            "idem_core_write_metric_pending_run",
            3,
            &task_id,
            &change_unit_id,
            &ticket_id,
            "run_core_write_metric_pending",
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(
        recorded.response_value["base"]["effect_kind"],
        "core_committed"
    );

    let rows = read_workflow_metric_aggregates(&harness.runtime_home_path, PROJECT_ID)?;
    assert_eq!(
        metric_samples(&rows, "first_product_write_duration_micros"),
        0
    );
    Ok(())
}

#[test]
fn terminal_task_duration_metric_is_not_replayed() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at(DEFAULT_METHOD_TEST_CLOCK);
    harness.use_clock(clock.clone());
    let session_id = start_metrics_session(&harness, "session_core_close_metrics")?;
    let (task_id, _change_unit_id, state_version) =
        create_close_ready_task(&harness, "core_close_metrics")?;
    clock.advance(Duration::seconds(5));
    let request = close_task_request(CloseTaskFixture {
        request_id: "req_core_close_metrics",
        idempotency_key: Some("idem_core_close_metrics"),
        dry_run: false,
        expected_state_version: Some(state_version),
        task_id: &task_id,
        intent: CloseIntent::Complete,
        close_reason: Some(CloseReason::CompletedSelfChecked),
        superseding_task_id: None,
    });

    let closed = harness.service.close_task(
        request.clone(),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    let replayed = harness.service.close_task(
        request,
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert!(replayed.replayed);

    let rows = read_workflow_metric_aggregates(&harness.runtime_home_path, PROJECT_ID)?;
    assert_eq!(metric_total(&rows, "task_duration_micros"), 5_000_000);
    Ok(())
}

#[test]
fn user_roundtrip_metric_counts_one_committed_resolution() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let session_id = start_metrics_session(&harness, "session_core_user_roundtrip")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "core_user_roundtrip")?;
    let pending = harness.service.request_user_action(
        user_action_request(
            "req_core_user_roundtrip_pending",
            "idem_core_user_roundtrip_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let request_id = response_record_id(&pending.response_value, "user_action_request_ref");
    let resolution = resolve_user_action_request(
        "req_core_user_roundtrip_resolve",
        "submission_core_user_roundtrip",
        None,
        &task_id,
        &request_id,
        "accept",
    );

    let resolved = harness.service.resolve_user_action(
        resolution.clone(),
        invocation_with_session(OperationCategory::UserOnly, &session_id),
    )?;
    assert!(!resolved.replayed);
    assert_eq!(
        resolved.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(
        resolved
            .verified_invocation
            .as_ref()
            .and_then(|invocation| invocation.session_id.as_deref()),
        Some(session_id.as_str())
    );
    let replayed = harness.service.resolve_user_action(
        resolution,
        invocation_with_session(OperationCategory::UserOnly, &session_id),
    )?;
    assert!(replayed.replayed);

    let rows = read_workflow_metric_aggregates(&harness.runtime_home_path, PROJECT_ID)?;
    assert_eq!(metric_total(&rows, "user_roundtrip"), 1);
    Ok(())
}

#[test]
fn corrupt_confirmed_observation_records_no_false_positive_metric() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let session_id = start_metrics_session(&harness, "session_core_reconcile_metrics")?;
    record_guard_installation(&harness, "core_reconcile_metrics")?;
    let (task_id, _) = create_task_with_change_unit(&harness, "core_reconcile_metrics")?;
    let unrecorded_change_id = insert_guarded_unrecorded_change_with_paths(
        &harness,
        &task_id,
        "core_reconcile_metrics",
        r#"["src/export.rs"]"#,
    )?;
    harness.conn()?.execute(
        "UPDATE unrecorded_changes
            SET observed_paths_json = '[123]'
          WHERE project_id = ?1
            AND unrecorded_change_id = ?2",
        rusqlite::params![PROJECT_ID, unrecorded_change_id],
    )?;
    let request = reconcile_changes_request(
        "req_core_reconcile_metrics",
        "idem_core_reconcile_metrics",
        Some(2),
        &task_id,
        Vec::new(),
    );

    let response = harness.service.reconcile_changes(
        request,
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_owner_state_rejection(
        &response,
        "unrecorded_changes",
        &unrecorded_change_id,
        "observed_paths_json",
        &harness.runtime_home_path,
    );

    let rows = read_workflow_metric_aggregates(&harness.runtime_home_path, PROJECT_ID)?;
    assert_eq!(
        metric_total(&rows, "confirmed_unrecorded_false_positive"),
        0
    );
    Ok(())
}

#[test]
fn explicit_sensitive_approval_block_is_counted_once() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let session_id = start_metrics_session(&harness, "session_core_sensitive_block")?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "core_sensitive_block")?;
    let mut request = prepare_write_request(
        "req_core_sensitive_block",
        "idem_core_sensitive_block",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];

    let blocked = harness.service.prepare_write(
        request.clone(),
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert_eq!(blocked.response_value["decision"], "approval_required");
    let replayed = harness.service.prepare_write(
        request,
        invocation_with_session(OperationCategory::AgentWorkflow, &session_id),
    )?;
    assert!(replayed.replayed);

    let rows = read_workflow_metric_aggregates(&harness.runtime_home_path, PROJECT_ID)?;
    assert_eq!(metric_total(&rows, "sensitive_approval_missing_block"), 1);
    assert_eq!(metric_total(&rows, "confirmed_structured_write_deny"), 0);
    Ok(())
}
