use std::error::Error;

use harness_core::{rejected_response, tool_error, CoreService, InvocationContext};
use harness_test_support::core_fixtures::{
    answer_payload, artifact_input_for_handle, supported_evidence_update,
    unsupported_evidence_update, CloseTaskFixture, CoreFixture, RecordJudgmentFixture,
    UpdateScopeFixture, UserJudgmentFixture, DEFAULT_PRODUCT_PATH,
};
use harness_types::{
    AccessClass, ChangeUnitOperation, CloseIntent, CloseReason, EffectKind, ErrorCode,
    JudgmentKind, ResponseKind, StagedArtifactHandle, WriteAuthorizationId,
};
use serde_json::{json, Value};

#[test]
fn no_effect_branches_state_version_and_idempotency_are_stable() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("effects")?;
    let service = core(&fixture);
    let initial_counts = fixture.counts()?;

    let stale = service.intake(
        fixture.intake_request("req_stale_intake", "idem_stale_intake", false, Some(99)),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&stale.response_value, "STATE_VERSION_CONFLICT");
    assert_eq!(fixture.counts()?, initial_counts);

    let dry_run = service.intake(
        fixture.intake_request("req_dry_intake", "idem_dry_intake", true, Some(0)),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(fixture.counts()?, initial_counts);

    let intake_request =
        fixture.intake_request("req_commit_intake", "idem_commit_intake", false, Some(0));
    let committed = service.intake(
        intake_request.clone(),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let after_commit = fixture.counts()?;
    let task_id = committed.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();

    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(
        committed.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(committed.response_value["base"]["state_version"], 1);
    assert_eq!(after_commit.state_version, initial_counts.state_version + 1);
    assert_eq!(after_commit.tasks, initial_counts.tasks + 1);
    assert_eq!(after_commit.task_events, initial_counts.task_events + 1);
    assert_eq!(
        after_commit.tool_invocations,
        initial_counts.tool_invocations + 1
    );

    let replay = service.intake(
        intake_request,
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert!(replay.replayed);
    assert_eq!(replay.response_json, committed.response_json);
    assert_eq!(fixture.counts()?, after_commit);

    let mut conflicting =
        fixture.intake_request("req_conflict_intake", "idem_commit_intake", false, Some(0));
    conflicting.plain_language_request = "A different request with the same key.".to_owned();
    let conflict = service.intake(conflicting, invocation(&fixture, AccessClass::CoreMutation))?;
    assert_rejected_code(&conflict.response_value, "STATE_VERSION_CONFLICT");
    assert_eq!(fixture.counts()?, after_commit);

    let status = service.status(
        fixture.status_request("req_status_read", Some(&task_id)),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(status.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(fixture.counts()?, after_commit);

    let check = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(check.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(fixture.counts()?, after_commit);

    let surface_mismatch = service.status(
        fixture.status_request("req_status_wrong_surface", Some(&task_id)),
        InvocationContext {
            surface_instance_id: Some(harness_types::SurfaceInstanceId::new(
                "missing_surface_instance",
            )),
            access_class: AccessClass::ReadStatus,
            verification_basis: "conformance_wrong_surface".to_owned(),
        },
    )?;
    assert_rejected_code(&surface_mismatch.response_value, "LOCAL_ACCESS_MISMATCH");
    assert_eq!(fixture.counts()?, after_commit);

    let envelope_value =
        serde_json::to_value(fixture.envelope("req_shape", None, false, None, Some(&task_id)))?;
    assert!(envelope_value.get("access_class").is_none());
    assert!(envelope_value.get("surface_instance_id").is_none());

    let stage_value = serde_json::to_value(fixture.stage_artifact_request(
        "req_stage_shape",
        None,
        false,
        None,
        &task_id,
    ))?;
    assert!(stage_value.get("access_class").is_none());
    assert!(stage_value.get("surface_instance_id").is_none());
    Ok(())
}

#[test]
fn write_authorization_lifecycle_is_single_use_and_state_bound() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("write_lifecycle")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(&fixture, &service, "write")?;

    let before_blocked = fixture.counts()?;
    let mut path_block = fixture.prepare_write_request(
        "req_prepare_path_block",
        "idem_prepare_path_block",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    path_block.intended_paths = vec!["src/other.rs".to_owned()];
    let path_blocked = service.prepare_write(
        path_block,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(path_blocked.response_value["decision"], "blocked");
    assert_prepare_reason(&path_blocked.response_value, "path_out_of_scope");
    assert_eq!(
        fixture.counts()?.write_authorizations,
        before_blocked.write_authorizations
    );

    let before_approval = fixture.counts()?;
    let mut approval_required = fixture.prepare_write_request(
        "req_prepare_sensitive_block",
        "idem_prepare_sensitive_block",
        Some(3),
        Some(&task_id),
        Some(&change_unit_id),
    );
    approval_required.sensitive_categories = vec!["network".to_owned()];
    let approval_blocked = service.prepare_write(
        approval_required,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(
        approval_blocked.response_value["decision"],
        "approval_required"
    );
    assert_prepare_reason(
        &approval_blocked.response_value,
        "sensitive_approval_missing",
    );
    assert_eq!(
        fixture.counts()?.write_authorizations,
        before_approval.write_authorizations
    );

    let allowed = service.prepare_write(
        fixture.prepare_write_request(
            "req_prepare_allowed",
            "idem_prepare_allowed",
            Some(4),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    let write_authorization_id = allowed.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("write authorization ref should be present")
        .to_owned();
    assert_eq!(allowed.response_value["decision"], "allowed");
    assert_eq!(allowed.response_value["authorization_effect"], "created");
    assert_eq!(
        fixture.write_authorization_status(&write_authorization_id)?,
        "active"
    );
    assert_eq!(
        fixture.write_authorization_basis(&write_authorization_id)?,
        allowed.response_value["base"]["state_version"]
            .as_u64()
            .unwrap()
    );

    let before_consume = fixture.counts()?;
    let mut run = fixture.record_run_request(
        "req_run_consumes_write",
        "idem_run_consumes_write",
        false,
        Some(5),
        &task_id,
        &change_unit_id,
    );
    run.observed_changes.product_file_write_observed = true;
    run.observed_changes.changed_paths = vec![DEFAULT_PRODUCT_PATH.to_owned()];
    run.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    let consumed = service.record_run(run, invocation(&fixture, AccessClass::RunRecording))?;
    assert_eq!(consumed.response_value["base"]["state_version"], 6);
    assert_eq!(
        fixture.write_authorization_status(&write_authorization_id)?,
        "consumed"
    );
    let after_consume = fixture.counts()?;
    assert_eq!(
        after_consume.state_version,
        before_consume.state_version + 1
    );
    assert_eq!(after_consume.runs, before_consume.runs + 1);

    let before_reuse = fixture.counts()?;
    let mut reuse = fixture.record_run_request(
        "req_run_reuses_write",
        "idem_run_reuses_write",
        false,
        Some(6),
        &task_id,
        &change_unit_id,
    );
    reuse.observed_changes.product_file_write_observed = true;
    reuse.observed_changes.changed_paths = vec![DEFAULT_PRODUCT_PATH.to_owned()];
    reuse.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    let rejected = service.record_run(reuse, invocation(&fixture, AccessClass::RunRecording))?;
    assert_rejected_code(&rejected.response_value, "WRITE_AUTHORIZATION_INVALID");
    assert_eq!(fixture.counts()?, before_reuse);

    let stale_fixture = CoreFixture::new("write_stale")?;
    let stale_service = core(&stale_fixture);
    let (stale_task_id, stale_change_unit_id) =
        create_task_with_change_unit(&stale_fixture, &stale_service, "stale")?;
    let stale_auth = prepare_write_authorization(
        &stale_fixture,
        &stale_service,
        &stale_task_id,
        &stale_change_unit_id,
        2,
        "stale",
    )?;
    stale_service.update_scope(
        stale_fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_scope_marks_stale",
            idempotency_key: "idem_scope_marks_stale",
            dry_run: false,
            expected_state_version: Some(3),
            task_id: &stale_task_id,
            operation: ChangeUnitOperation::ReplaceCurrent,
            scope_summary: "Replacement current scope.",
        }),
        invocation(&stale_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(
        stale_fixture.write_authorization_status(&stale_auth)?,
        "stale"
    );
    Ok(())
}

#[test]
fn artifact_lifecycle_promotes_valid_handles_and_rolls_back_invalid_ones(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("artifacts")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(&fixture, &service, "artifact")?;

    let before_stage = fixture.counts()?;
    let mut stage_request = fixture.stage_artifact_request(
        "req_stage_report",
        Some("idem_stage_report"),
        false,
        Some(2),
        &task_id,
    );
    stage_request.display_name = "validation.json".to_owned();
    stage_request.content_type = "application/json".to_owned();
    stage_request.safe_bytes_or_notice = "{\"fixture\":\"artifact\"}".to_owned();
    let staged = service.stage_artifact(
        stage_request,
        invocation(&fixture, AccessClass::ArtifactRegistration),
    )?;
    let after_stage = fixture.counts()?;
    let handle: StagedArtifactHandle =
        serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;

    assert_eq!(
        staged.response_value["base"]["effect_kind"],
        "staging_created"
    );
    assert_eq!(staged.response_value["base"]["state_version"], 2);
    assert_eq!(after_stage.state_version, before_stage.state_version);
    assert_eq!(
        after_stage.artifact_staging,
        before_stage.artifact_staging + 1
    );
    assert_eq!(after_stage.artifacts, before_stage.artifacts);
    assert_eq!(after_stage.tool_invocations, before_stage.tool_invocations);

    let before_invalid = fixture.counts()?;
    let mut invalid_input = artifact_input_for_handle(
        "artifact_input_invalid",
        handle.clone(),
        Some("validation_report"),
        Some("Validation passed."),
    );
    invalid_input.expected_sha256 = Some("sha256:0000".to_owned());
    let mut invalid_run = fixture.record_run_request(
        "req_run_invalid_artifact",
        "idem_run_invalid_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    invalid_run.artifact_inputs = vec![invalid_input];
    invalid_run.evidence_updates = vec![supported_evidence_update("Validation passed.")];
    let invalid =
        service.record_run(invalid_run, invocation(&fixture, AccessClass::RunRecording))?;
    assert_rejected_code(&invalid.response_value, "VALIDATION_FAILED");
    assert_eq!(
        invalid.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_checksum_mismatch"
    );
    assert_eq!(fixture.counts()?, before_invalid);
    assert_eq!(
        fixture.artifact_staging_status(handle.handle_id.as_str())?,
        "staged"
    );

    let before_valid = fixture.counts()?;
    let mut valid_run = fixture.record_run_request(
        "req_run_valid_artifact",
        "idem_run_valid_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    valid_run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_valid",
        handle.clone(),
        Some("validation_report"),
        Some("Validation passed."),
    )];
    valid_run.evidence_updates = vec![supported_evidence_update("Validation passed.")];
    let valid = service.record_run(valid_run, invocation(&fixture, AccessClass::RunRecording))?;
    let after_valid = fixture.counts()?;
    let artifact_id = valid.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present");

    assert_eq!(
        valid.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(
        valid.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(after_valid.state_version, before_valid.state_version + 1);
    assert_eq!(after_valid.runs, before_valid.runs + 1);
    assert_eq!(after_valid.artifacts, before_valid.artifacts + 1);
    assert_eq!(after_valid.artifact_links, before_valid.artifact_links + 2);
    assert_eq!(
        fixture.artifact_staging_status(handle.handle_id.as_str())?,
        "consumed"
    );
    assert!(fixture.artifact_owner_link_exists(artifact_id, "run")?);
    assert!(fixture.artifact_owner_link_exists(artifact_id, "evidence_summary")?);
    Ok(())
}

#[test]
fn user_judgment_kinds_remain_separate_from_scope_and_write_authority() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("judgment")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(&fixture, &service, "judgment")?;
    let original_scope = fixture.current_change_unit_scope(&task_id)?;
    let original_current = fixture.current_change_unit_id(&task_id)?;

    service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_scope_decision",
            idempotency_key: "idem_scope_decision",
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ScopeDecision,
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let before_scope_record = fixture.counts()?;
    let scope_recorded = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_scope_decision_record",
            idempotency_key: "idem_scope_decision_record",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: "uj_req_scope_decision",
            judgment_kind: JudgmentKind::ScopeDecision,
            answer: answer_payload(JudgmentKind::ScopeDecision),
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(
        scope_recorded.response_value["base"]["response_kind"],
        "result"
    );
    assert_eq!(fixture.current_change_unit_scope(&task_id)?, original_scope);
    assert_eq!(fixture.current_change_unit_id(&task_id)?, original_current);
    assert_eq!(
        fixture.counts()?.change_units,
        before_scope_record.change_units
    );

    service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_sensitive_only",
            idempotency_key: "idem_sensitive_only",
            dry_run: false,
            expected_state_version: Some(4),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::SensitiveApproval,
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let before_sensitive = fixture.counts()?;
    let sensitive = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_sensitive_only_record",
            idempotency_key: "idem_sensitive_only_record",
            expected_state_version: Some(5),
            task_id: &task_id,
            user_judgment_id: "uj_req_sensitive_only",
            judgment_kind: JudgmentKind::SensitiveApproval,
            answer: answer_payload(JudgmentKind::SensitiveApproval),
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(sensitive.response_value["base"]["response_kind"], "result");
    assert_eq!(
        fixture.counts()?.write_authorizations,
        before_sensitive.write_authorizations
    );

    service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_risk_judgment",
            idempotency_key: "idem_risk_judgment",
            dry_run: false,
            expected_state_version: Some(6),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let before_wrong_kind = fixture.counts()?;
    let wrong_kind = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_risk_wrong_answer",
            idempotency_key: "idem_risk_wrong_answer",
            expected_state_version: Some(7),
            task_id: &task_id,
            user_judgment_id: "uj_req_risk_judgment",
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
            answer: answer_payload(JudgmentKind::FinalAcceptance),
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&wrong_kind.response_value, "VALIDATION_FAILED");
    assert_eq!(fixture.counts()?, before_wrong_kind);
    assert_eq!(
        fixture.user_judgment_status("uj_req_risk_judgment")?,
        "pending"
    );
    Ok(())
}

#[test]
fn close_readiness_reports_distinct_blockers_without_substitution() -> Result<(), Box<dyn Error>> {
    let final_fixture = CoreFixture::new("close_final")?;
    let final_service = core(&final_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&final_fixture, &final_service, "final")?;
    let after_evidence = record_close_evidence(
        &final_fixture,
        &final_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let before_close = final_fixture.counts()?;
    let final_blocked = final_service.close_task(
        final_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_final_blocker",
            idempotency_key: Some("idem_close_final_blocker"),
            dry_run: false,
            expected_state_version: Some(after_evidence),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&final_fixture, AccessClass::CoreMutation),
    )?;
    assert_close_blocker(&final_blocked.response_value, "missing_final_acceptance");
    assert_eq!(final_fixture.counts()?, before_close);

    let evidence_fixture = CoreFixture::new("close_evidence")?;
    let evidence_service = core(&evidence_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&evidence_fixture, &evidence_service, "evidence")?;
    let after_bad_evidence = record_close_evidence(
        &evidence_fixture,
        &evidence_service,
        &task_id,
        &change_unit_id,
        2,
        false,
    )?;
    let after_final = record_final_acceptance(
        &evidence_fixture,
        &evidence_service,
        &task_id,
        &change_unit_id,
        after_bad_evidence,
        "evidence",
    )?;
    let evidence_blocked = evidence_service.close_task(
        evidence_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_evidence_blocker",
            idempotency_key: Some("idem_close_evidence_blocker"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&evidence_fixture, AccessClass::CoreMutation),
    )?;
    assert_close_blocker(
        &evidence_blocked.response_value,
        "evidence_claim_unsupported",
    );

    let artifact_fixture = CoreFixture::new("close_artifact")?;
    let artifact_service = core(&artifact_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&artifact_fixture, &artifact_service, "artifact_close")?;
    let staged = stage_artifact_for_record_run(&artifact_fixture, &artifact_service, &task_id)?;
    let mut run = artifact_fixture.record_run_request(
        "req_close_artifact_evidence",
        "idem_close_artifact_evidence",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_close",
        staged,
        Some("validation_report"),
        Some("Close claim supported."),
    )];
    run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    let artifact_run = artifact_service.record_run(
        run,
        invocation(&artifact_fixture, AccessClass::RunRecording),
    )?;
    let artifact_id = artifact_run.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();
    artifact_fixture.set_artifact_status(&artifact_id, "missing")?;
    let after_final = record_final_acceptance(
        &artifact_fixture,
        &artifact_service,
        &task_id,
        &change_unit_id,
        3,
        "artifact",
    )?;
    let artifact_blocked = artifact_service.close_task(
        artifact_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_artifact_blocker",
            idempotency_key: Some("idem_close_artifact_blocker"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&artifact_fixture, AccessClass::CoreMutation),
    )?;
    assert_close_blocker(&artifact_blocked.response_value, "artifact_unavailable");

    let risk_fixture = CoreFixture::new("close_risk")?;
    let risk_service = core(&risk_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&risk_fixture, &risk_service, "risk")?;
    let after_evidence = record_close_evidence(
        &risk_fixture,
        &risk_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let after_final = record_final_acceptance(
        &risk_fixture,
        &risk_service,
        &task_id,
        &change_unit_id,
        after_evidence,
        "risk",
    )?;
    risk_fixture.set_task_close_summary(
        &task_id,
        json!({
            "close_reason": "none",
            "visible_risks": [
                {
                    "risk_id": "risk_visible_001",
                    "summary": "Manual verification remains partial."
                }
            ]
        }),
    )?;
    let risk_blocked = risk_service.close_task(
        risk_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_risk_blocker",
            idempotency_key: Some("idem_close_risk_blocker"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedWithRiskAccepted),
            superseding_task_id: None,
        }),
        invocation(&risk_fixture, AccessClass::CoreMutation),
    )?;
    assert_close_blocker(
        &risk_blocked.response_value,
        "missing_residual_risk_acceptance",
    );
    Ok(())
}

#[test]
fn cancel_and_supersede_terminal_paths_commit_once() -> Result<(), Box<dyn Error>> {
    let cancel_fixture = CoreFixture::new("cancel")?;
    let cancel_service = core(&cancel_fixture);
    let (task_id, _) = create_task_with_change_unit(&cancel_fixture, &cancel_service, "cancel")?;
    let before_cancel = cancel_fixture.counts()?;
    let cancel = cancel_service.close_task(
        cancel_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_cancel",
            idempotency_key: Some("idem_cancel"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(&cancel_fixture, AccessClass::CoreMutation),
    )?;
    let after_cancel = cancel_fixture.counts()?;
    let cancel_fields = cancel_fixture.task_terminal_fields(&task_id)?;
    assert_eq!(cancel.response_value["close_state"], "cancelled");
    assert_eq!(after_cancel.state_version, before_cancel.state_version + 1);
    assert_eq!(after_cancel.task_events, before_cancel.task_events + 1);
    assert_eq!(cancel_fields.lifecycle_phase, "cancelled");
    assert_eq!(cancel_fields.result.as_deref(), Some("cancelled"));

    let supersede_fixture = CoreFixture::new("supersede")?;
    let supersede_service = core(&supersede_fixture);
    let (task_id, _) =
        create_task_with_change_unit(&supersede_fixture, &supersede_service, "supersede")?;
    let replacement_task_id = "task_superseding";
    supersede_fixture.insert_superseding_task(replacement_task_id)?;
    let before_supersede = supersede_fixture.counts()?;
    let supersede = supersede_service.close_task(
        supersede_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_supersede",
            idempotency_key: Some("idem_supersede"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Supersede,
            close_reason: Some(CloseReason::Superseded),
            superseding_task_id: Some(replacement_task_id),
        }),
        invocation(&supersede_fixture, AccessClass::CoreMutation),
    )?;
    let after_supersede = supersede_fixture.counts()?;
    let supersede_fields = supersede_fixture.task_terminal_fields(&task_id)?;
    assert_eq!(supersede.response_value["close_state"], "superseded");
    assert_eq!(
        after_supersede.state_version,
        before_supersede.state_version + 1
    );
    assert_eq!(
        after_supersede.task_events,
        before_supersede.task_events + 1
    );
    assert_eq!(supersede_fields.lifecycle_phase, "superseded");
    assert_eq!(supersede_fields.result.as_deref(), Some("superseded"));
    assert_eq!(
        supersede_fixture.active_task_id()?.as_deref(),
        Some(replacement_task_id)
    );
    Ok(())
}

#[test]
fn public_error_precedence_keeps_validation_primary() {
    let response = rejected_response(
        false,
        Some(12),
        vec![
            tool_error(ErrorCode::AcceptanceRequired, "acceptance", false, None),
            tool_error(ErrorCode::StateVersionConflict, "stale state", true, None),
            tool_error(ErrorCode::ValidationFailed, "invalid request", false, None),
        ],
    );

    assert_eq!(response.base.response_kind, ResponseKind::Rejected);
    assert_eq!(response.base.effect_kind, EffectKind::NoEffect);
    assert_eq!(response.errors[0].code, ErrorCode::ValidationFailed);
    assert_eq!(response.errors[1].code, ErrorCode::StateVersionConflict);
}

fn core(fixture: &CoreFixture) -> CoreService {
    CoreService::new(fixture.runtime_home_path())
}

fn invocation(fixture: &CoreFixture, access_class: AccessClass) -> InvocationContext {
    InvocationContext {
        surface_instance_id: Some(harness_types::SurfaceInstanceId::new(
            fixture.surface_instance_id(),
        )),
        access_class,
        verification_basis: "conformance_fixture".to_owned(),
    }
}

fn create_task_with_change_unit(
    fixture: &CoreFixture,
    service: &CoreService,
    suffix: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let intake = service.intake(
        fixture.intake_request(
            &format!("req_{suffix}_task"),
            &format!("idem_{suffix}_task"),
            false,
            Some(0),
        ),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let scope = service.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: &format!("req_{suffix}_scope"),
            idempotency_key: &format!("idem_{suffix}_scope"),
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Initial current scope.",
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    Ok((task_id, change_unit_id))
}

fn prepare_write_authorization(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<String, Box<dyn Error>> {
    let response = service.prepare_write(
        fixture.prepare_write_request(
            &format!("req_prepare_{suffix}"),
            &format!("idem_prepare_{suffix}"),
            Some(expected_state_version),
            Some(task_id),
            Some(change_unit_id),
        ),
        invocation(fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(response.response_value["decision"], "allowed");
    Ok(
        response.response_value["write_authorization_ref"]["record_id"]
            .as_str()
            .expect("write authorization ref should be present")
            .to_owned(),
    )
}

fn stage_artifact_for_record_run(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
) -> Result<StagedArtifactHandle, Box<dyn Error>> {
    let mut request =
        fixture.stage_artifact_request("req_stage_close_artifact", None, false, Some(2), task_id);
    request.display_name = "close-evidence.json".to_owned();
    request.content_type = "application/json".to_owned();
    request.safe_bytes_or_notice = "{\"fixture\":\"close\"}".to_owned();
    let response = service.stage_artifact(
        request,
        invocation(fixture, AccessClass::ArtifactRegistration),
    )?;
    Ok(serde_json::from_value(
        response.response_value["staged_artifact_handle"].clone(),
    )?)
}

fn record_close_evidence(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    supported: bool,
) -> Result<u64, Box<dyn Error>> {
    let mut request = fixture.record_run_request(
        &format!("req_close_evidence_{supported}"),
        &format!("idem_close_evidence_{supported}"),
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.evidence_updates = vec![if supported {
        supported_evidence_update("Close claim supported.")
    } else {
        unsupported_evidence_update("Close claim supported.")
    }];
    let response = service.record_run(request, invocation(fixture, AccessClass::RunRecording))?;
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present"))
}

fn record_final_acceptance(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<u64, Box<dyn Error>> {
    service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: &format!("req_final_{suffix}"),
            idempotency_key: &format!("idem_final_{suffix}"),
            dry_run: false,
            expected_state_version: Some(expected_state_version),
            task_id,
            change_unit_id: Some(change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    let response = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: &format!("req_final_record_{suffix}"),
            idempotency_key: &format!("idem_final_record_{suffix}"),
            expected_state_version: Some(expected_state_version + 1),
            task_id,
            user_judgment_id: &format!("uj_req_final_{suffix}"),
            judgment_kind: JudgmentKind::FinalAcceptance,
            answer: answer_payload(JudgmentKind::FinalAcceptance),
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present"))
}

fn assert_prepare_reason(response_value: &Value, code: &str) {
    let reasons = response_value["write_decision_reasons"]
        .as_array()
        .expect("write_decision_reasons should be an array");
    assert!(
        reasons.iter().any(|reason| reason["code"] == code),
        "expected prepare_write reason code {code}, got {reasons:?}"
    );
}

fn assert_close_blocker(response_value: &Value, code: &str) {
    let codes = response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .filter_map(|blocker| blocker["code"].as_str())
        .collect::<Vec<_>>();
    assert!(
        codes.contains(&code),
        "expected close blocker code {code}, got {codes:?}"
    );
}

fn assert_rejected_code(response_value: &Value, code: &str) {
    assert_eq!(response_value["base"]["response_kind"], "rejected");
    assert_eq!(response_value["errors"][0]["code"], code);
}
