//! Representative behavior that crosses close-readiness owner boundaries.

use super::*;

#[test]
fn check_close_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check")?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_check",
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

    assert_typed_result_contract::<CheckCloseResult>(&response);
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_dry_run_has_no_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_dry_run")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_dry_run",
            idempotency_key: Some("idem_close_dry_run"),
            dry_run: true,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn complete_close_crosses_evidence_acceptance_and_terminal_storage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_complete")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "complete", true)?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "complete",
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_complete",
            idempotency_key: Some("idem_close_complete"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "completed"
    );
    Ok(())
}

#[test]
fn missing_final_acceptance_is_projected_end_to_end_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_blocked")?;
    let state_version =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "blocked", true)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_blocked",
            idempotency_key: Some("idem_close_blocked"),
            dry_run: false,
            expected_state_version: Some(state_version),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(
        response.response_value["summary_card"]["next_action"],
        response.response_value["blockers"][0]["next_actions"][0]
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn persisted_close_basis_corruption_propagates_through_core() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_corruption")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_corruption",
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
        &response,
        "tasks",
        &task_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn accepted_cancellation_authority_allows_terminal_transition() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cancel_accepted")?;
    let (after_authority, _) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "cancel_accepted",
        true,
    )?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_accepted",
            idempotency_key: Some("idem_cancel_accepted"),
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
    assert_eq!(
        task_terminal_fields(&harness, &task_id)?.lifecycle_phase,
        "cancelled"
    );
    Ok(())
}

#[test]
fn rejected_cancellation_authority_keeps_task_open() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cancel_rejected")?;
    let (after_rejection, _) = record_cancellation_authority(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "cancel_rejected",
        false,
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_cancel_rejected",
            idempotency_key: Some("idem_cancel_rejected"),
            dry_run: false,
            expected_state_version: Some(after_rejection),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "rejected_cancellation_authority");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stale_state_rejects_before_close_blocker_projection() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_stale")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_stale",
            idempotency_key: Some("idem_close_stale"),
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response.response_value.get("blockers").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_and_check_close_share_one_readiness_projection() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "status_consistency")?;
    record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "status_consistency",
        true,
    )?;
    let before = harness.counts()?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_consistency", None, false, None, Some(&task_id)),
            continuity_page: None,
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_consistency",
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
        status.response_value["close_blockers"],
        close.response_value["blockers"]
    );
    assert_eq!(
        status.response_value["evidence_gate"],
        close.response_value["evidence_gate"]
    );
    assert_eq!(
        status.response_value["current_close_basis"],
        close.response_value["current_close_basis"]
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}
