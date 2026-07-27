//! Close-operation request orchestration and mutation coverage.

use super::*;

#[test]
fn advisor_check_close_uses_non_write_semantic_guidance() -> Result<(), Box<dyn Error>> {
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

    for blocker in response.response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
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
    assert_ne!(
        response.response_value["summary_card"]["next_action"]["action_kind"],
        "prepare_write"
    );
    Ok(())
}

#[test]
fn complete_orchestration_commits_the_terminal_task_transition() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_orchestration_complete")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "orchestration",
        true,
    )?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "orchestration",
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_orchestration_complete",
            idempotency_key: Some("idem_close_orchestration_complete"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    let stored = task_terminal_fields(&harness, &task_id)?;

    assert_typed_result_contract::<CloseTaskResult>(&response);
    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(stored.lifecycle_phase, "completed");
    assert_eq!(stored.result.as_deref(), Some("completed"));
    assert_eq!(
        stored.close_summary["close_reason"],
        "completed_self_checked"
    );
    assert_eq!(
        response.response_value["authority_receipt"]["completion_claim_allowed"],
        true
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.authority_events, before.authority_events + 1);
    Ok(())
}

#[test]
fn advisor_completion_orchestration_persists_advice_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_mode_and_change_unit(
        &harness,
        "close_orchestration_advisor",
        RequestedMode::Advisor,
    )?;
    let mut run = record_run_request(
        "req_close_orchestration_advisor_run",
        "idem_close_orchestration_advisor_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.kind = RunKind::ShapingUpdate;
    run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    run.close_assessment = Some(close_assessment_with_risks(
        "Close claim supported.",
        Vec::new(),
    ))
    .into();
    let run_response = harness
        .service
        .record_run(run, invocation(OperationCategory::AgentWorkflow))?;
    let after_evidence = run_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_orchestration_advisor",
            idempotency_key: Some("idem_close_orchestration_advisor"),
            dry_run: false,
            expected_state_version: Some(after_evidence),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let stored = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["state"]["mode"], "advisor");
    assert_eq!(
        response.response_value["state"]["lifecycle"]["result"],
        "advice_only"
    );
    assert_eq!(stored.lifecycle_phase, "completed");
    assert_eq!(stored.result.as_deref(), Some("advice_only"));
    Ok(())
}

#[test]
fn cancel_orchestration_invalidates_an_active_write_ticket_atomically() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_orchestration_cancel")?;
    let prepared = harness.service.prepare_write(
        prepare_write_request(
            "req_close_orchestration_cancel_prepare",
            "idem_close_orchestration_cancel_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");
    let (after_authority, _) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        3,
        "close_orchestration_cancel",
        true,
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_orchestration_cancel",
            idempotency_key: Some("idem_close_orchestration_cancel"),
            dry_run: false,
            expected_state_version: Some(after_authority),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "cancelled");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(write_ticket_status(&harness, &ticket_id)?, "invalidated");
    let reason: String = harness.conn()?.query_row(
        "SELECT invalidation_reason FROM write_tickets
          WHERE project_id = ?1 AND write_ticket_id = ?2",
        rusqlite::params![PROJECT_ID, ticket_id],
        |row| row.get(0),
    )?;
    assert_eq!(reason, "task_closed");
    let stored = task_terminal_fields(&harness, &task_id)?;
    assert_eq!(stored.lifecycle_phase, "cancelled");
    assert_eq!(stored.result.as_deref(), Some("cancelled"));
    Ok(())
}

#[test]
fn supersede_orchestration_activates_the_successor() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_orchestration_supersede")?;
    let superseding_task_id = "task_close_orchestration_successor";
    insert_superseding_task(&harness, superseding_task_id)?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_orchestration_supersede",
            idempotency_key: Some("idem_close_orchestration_supersede"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Supersede,
            close_reason: Some(CloseReason::Superseded),
            superseding_task_id: Some(superseding_task_id),
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "superseded");
    assert_eq!(
        active_task_id(&harness)?.as_deref(),
        Some(superseding_task_id)
    );
    let stored = task_terminal_fields(&harness, &task_id)?;
    assert_eq!(stored.lifecycle_phase, "superseded");
    assert_eq!(stored.result.as_deref(), Some("superseded"));
    Ok(())
}

#[test]
fn terminal_orchestration_replays_the_committed_result() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "close_orchestration_replay")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "replay", true)?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "replay",
    )?;
    let request = close_task_request(CloseTaskFixture {
        request_id: "req_close_orchestration_replay",
        idempotency_key: Some("idem_close_orchestration_replay"),
        dry_run: false,
        expected_state_version: Some(after_final),
        task_id: &task_id,
        intent: CloseIntent::Complete,
        close_reason: Some(CloseReason::CompletedSelfChecked),
        superseding_task_id: None,
    });

    let first = harness.service.close_task(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let replay = harness
        .service
        .close_task(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(first.response_value["close_state"], "closed");
    assert!(replay.replayed);
    assert_eq!(replay.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}
