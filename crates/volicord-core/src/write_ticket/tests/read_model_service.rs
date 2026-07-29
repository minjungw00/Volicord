use super::*;
use crate::write_ticket::read_model::{
    load_sensitive_approval_facts, load_write_ticket_candidates, load_write_ticket_control_facts,
    load_write_ticket_evidence_facts,
};
use crate::write_ticket::service::load_current_write_ticket_summary;
use volicord_store::StoreError;

#[test]
fn read_model_acquires_typed_ticket_control_and_user_action_facts() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "write_ticket_read_model")?;
    let approval = harness.service.request_user_action(
        user_action_request(
            "req_write_ticket_read_model_approval",
            "idem_write_ticket_read_model_approval",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let approval_request_id =
        response_record_id(&approval.response_value, "user_action_request_ref");
    harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_write_ticket_read_model_resolve",
            "idem_write_ticket_read_model_resolve",
            None,
            &task_id,
            &approval_request_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "write_ticket_read_model",
        4,
        "2026-06-18T00:00:00Z",
        "2026-06-18T00:15:00Z",
    )?;
    let store = harness.store()?;
    let typed_task_id = TaskId::new(&task_id);

    let candidates = load_write_ticket_candidates(&store, &typed_task_id)?;
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].write_ticket_id.as_str(),
        "write_ticket_read_model"
    );
    assert_eq!(candidates[0].ticket.basis_state_version, 4);

    let (task, workflow) = load_write_ticket_control_facts(&store, &typed_task_id)?;
    assert_eq!(task.scope_revision, 1);
    assert!(!task.pending_policy_reevaluation);
    assert!(!workflow.write_authority_fingerprint.is_empty());

    let approvals = load_sensitive_approval_facts(
        &store,
        &typed_task_id,
        &UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?,
    )?;
    assert_eq!(approvals.len(), 1);
    assert!(approvals[0].user_action_resolution_id.is_some());
    Ok(())
}

#[test]
fn read_model_preserves_store_not_found_failure() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let store = harness.store()?;

    let error = load_write_ticket_control_facts(&store, &TaskId::new("task-missing"))
        .expect_err("missing task should remain a Store failure");
    assert!(matches!(
        error,
        CorePipelineError::Store(StoreError::NotFound { entity: "task", .. })
    ));
    Ok(())
}

#[test]
fn service_coordinates_fact_loading_evaluation_selection_and_summary() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "write_ticket_service")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "write_ticket_service",
        2,
        "2026-06-18T00:00:00Z",
        "2026-06-18T00:15:00Z",
    )?;
    let store = harness.store()?;

    let summary = load_current_write_ticket_summary(
        &store,
        &TaskId::new(&task_id),
        2,
        &UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?,
        None,
    )?
    .expect("stored active ticket should be selected");

    assert_eq!(summary.status, WriteTicketStatus::Active);
    assert_eq!(
        summary
            .write_ticket_ref
            .as_ref()
            .map(|reference| reference.record_id.as_str()),
        Some("write_ticket_service")
    );
    assert_eq!(summary.basis_state_version, Some(2));
    assert_eq!(summary.intended_paths, vec!["src/export.rs".to_owned()]);
    Ok(())
}

#[test]
fn service_projects_current_expiry_and_approval_invalidation() -> Result<(), Box<dyn Error>> {
    let expiry_harness = MethodHarness::new()?;
    let (expiry_task_id, expiry_change_unit_id) =
        create_task_with_change_unit(&expiry_harness, "write_ticket_expiry")?;
    insert_active_write_ticket_with_timestamps(
        &expiry_harness,
        &expiry_task_id,
        &expiry_change_unit_id,
        "write_ticket_expiry",
        2,
        "2026-06-18T00:00:00Z",
        "2026-06-18T00:15:00Z",
    )?;
    let expiry_store = expiry_harness.store()?;
    let expired = load_current_write_ticket_summary(
        &expiry_store,
        &TaskId::new(&expiry_task_id),
        2,
        &UtcTimestamp::parse("2026-06-18T00:15:00Z")?,
        None,
    )?
    .expect("expired ticket remains the selected historical summary");
    assert_eq!(expired.status, WriteTicketStatus::Invalidated);
    assert_eq!(
        expired.invalidation_reason,
        Some(WriteTicketInvalidationReason::IdleTimeout)
    );

    let approval_harness = MethodHarness::new()?;
    let (approval_task_id, approval_change_unit_id) =
        create_task_with_change_unit(&approval_harness, "write_ticket_approval")?;
    insert_active_write_ticket_with_scope(
        &approval_harness,
        WriteTicketScopeFixture {
            task_id: &approval_task_id,
            change_unit_id: &approval_change_unit_id,
            write_ticket_id: "write_ticket_approval",
            basis_state_version: 2,
            created_at: "2026-06-18T00:00:00Z",
            expires_at: "2026-06-18T00:15:00Z",
            intended_operation: "local_sensitive_step",
            intended_paths: &["src/export.rs"],
            sensitive_categories: &["network"],
        },
    )?;
    let approval_store = approval_harness.store()?;
    let approval_changed = load_current_write_ticket_summary(
        &approval_store,
        &TaskId::new(&approval_task_id),
        2,
        &UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?,
        None,
    )?
    .expect("approval-dependent ticket remains selected as invalidated");
    assert_eq!(approval_changed.status, WriteTicketStatus::Invalidated);
    assert_eq!(
        approval_changed.invalidation_reason,
        Some(WriteTicketInvalidationReason::ApprovalBasisChanged)
    );
    Ok(())
}

#[test]
fn evidence_fact_loader_returns_refs_for_the_selected_run_identity() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "write_ticket_evidence")?;
    insert_active_write_ticket_with_timestamps(
        &harness,
        &task_id,
        &change_unit_id,
        "write_ticket_evidence",
        2,
        "2026-06-18T00:00:00Z",
        "2026-06-18T00:15:00Z",
    )?;
    let mut request = product_write_record_run_request(
        "req_write_ticket_evidence",
        "idem_write_ticket_evidence",
        2,
        &task_id,
        &change_unit_id,
        "write_ticket_evidence",
        "run_write_ticket_evidence",
    );
    request.evidence_observations = vec![EvidenceObservationInput {
        target: supplemental_evidence_target("Write Ticket evidence loader fact."),
        source_kind: EvidenceSourceKind::AgentReport,
        assurance_level: EvidenceAssuranceLevel::CooperativeReport,
        observed_by_actor_source: None.into(),
        tool_name: None.into(),
        tool_invocation_id: None.into(),
        tool_metadata: JsonObject::new(),
        input_refs: Vec::new(),
        source_refs: Vec::new(),
        output_artifact_refs: Vec::new(),
        limitations: vec!["Focused read-model fixture.".to_owned()],
        observed_at: UtcTimestamp::parse(DEFAULT_METHOD_TEST_CLOCK)?,
    }];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("recorded state version");
    let store = harness.store()?;

    let evidence = load_write_ticket_evidence_facts(
        &store,
        &TaskId::new(&task_id),
        Some(&RunId::new("run_write_ticket_evidence")),
        state_version,
    )?;
    assert_eq!(evidence.observation_refs.len(), 1);
    assert_eq!(
        evidence.observation_refs[0].record_kind,
        StateRecordKind::EvidenceObservation
    );
    Ok(())
}
