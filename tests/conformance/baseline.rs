use std::{error::Error, fs, path::Path};

use chrono::{DateTime, Duration, Utc};
use harness_core::{
    rejected_response, tool_error, AdapterSessionBinding, Clock, CoreService, InvocationContext,
};
use harness_test_support::core_fixtures::{
    answer_payload, artifact_input_for_handle, supported_evidence_update,
    unsupported_evidence_update, ChangeUnitOwnerJsonColumn, CloseTaskFixture, CoreFixture,
    RecordJudgmentFixture, TaskOwnerJsonColumn, UpdateScopeFixture, UserJudgmentFixture,
    DEFAULT_PRODUCT_PATH,
};
use harness_types::{
    AccessClass, ChangeUnitOperation, CloseAssessmentInput, CloseIntent, CloseReason, EffectKind,
    ErrorCode, JudgmentKind, ResidualRiskInput, ResponseKind, StagedArtifactHandle, StatusRequest,
    WriteAuthorizationId, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
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
            binding: AdapterSessionBinding::new(
                harness_types::ProjectId::new(fixture.project_id()),
                harness_types::SurfaceId::new(fixture.surface_id()),
                harness_types::SurfaceInstanceId::new("missing_surface_instance"),
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
            requested_access_class: AccessClass::ReadStatus,
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
fn idempotency_replay_is_bound_to_verified_access_context() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("replay_context")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "replay_context")?;
    let request = fixture.prepare_write_request(
        "req_prepare_replay_context",
        "idem_prepare_replay_context",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );

    let first = service.prepare_write(
        request.clone(),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    let after_first = fixture.counts()?;
    let write_authorization_id = first.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("prepare_write should return an authorization id")
        .to_owned();

    let mismatch =
        service.prepare_write(request, invocation(&fixture, AccessClass::CoreMutation))?;

    assert!(!mismatch.replayed);
    assert_rejected_code(&mismatch.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(!mismatch.response_json.contains(&write_authorization_id));
    assert_eq!(fixture.counts()?, after_first);
    Ok(())
}

#[test]
fn direct_public_request_parsing_rejects_invocation_authority_fields() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("strict_direct_parse")?;
    let params = serde_json::to_value(fixture.status_request("req_strict_direct", None))?;

    for (field_path, forged_value) in [
        ("envelope.verified", json!(true)),
        (
            "envelope.surface_instance_id",
            json!("surface_instance_forged"),
        ),
        ("verified_surface_context", json!({ "verified": true })),
        ("access_class", json!("core_mutation")),
        ("capability_profile", json!({ "write_authorization": true })),
    ] {
        let mut forged = params.clone();
        if let Some(field) = field_path.strip_prefix("envelope.") {
            forged["envelope"][field] = forged_value;
        } else {
            forged[field_path] = forged_value;
        }

        let decoded = serde_json::from_value::<StatusRequest>(forged);
        assert!(
            decoded.is_err(),
            "{field_path} must not be accepted as public request authority"
        );
    }

    assert_eq!(fixture.counts()?.state_version, 0);
    Ok(())
}

#[test]
fn structured_store_unavailability_does_not_expose_sql_or_local_paths() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("store_unavailable")?;
    fs::remove_file(
        fixture
            .runtime_home_path()
            .join("projects")
            .join(fixture.project_id())
            .join("state.sqlite"),
    )?;

    let response = core(&fixture).status(
        fixture.status_request("req_missing_state_db", None),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;

    assert_rejected_code(&response.response_value, "MCP_UNAVAILABLE");
    assert_eq!(
        response.response_value["errors"][0]["details"]["store_failure_category"],
        "project_state_database_missing"
    );
    assert_public_response_has_no_internal_leak(
        &response.response_json,
        fixture.runtime_home_path(),
    );
    Ok(())
}

#[test]
fn committed_non_allow_prepare_write_audit_and_replay_are_exact() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("prepare_non_allow_audit")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "prepare_non_allow_audit")?;
    let before = fixture.counts()?;
    let mut request = fixture.prepare_write_request(
        "req_prepare_non_allow_audit",
        "idem_prepare_non_allow_audit",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];

    let first = service.prepare_write(
        request.clone(),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    let after_first = fixture.counts()?;
    let event_payload =
        assert_latest_prepare_write_event(&fixture, &first.response_value, "blocked")?;

    assert_eq!(first.response_value["decision"], "blocked");
    assert_prepare_reason(&first.response_value, "path_out_of_scope");
    assert_eq!(first.response_value["write_authorization"], Value::Null);
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);
    assert_eq!(
        after_first.write_authorizations,
        before.write_authorizations
    );
    assert_eq!(after_first.artifact_staging, before.artifact_staging);
    assert_eq!(after_first.artifacts, before.artifacts);
    assert_eq!(after_first.artifact_links, before.artifact_links);
    assert_eq!(after_first.evidence_summaries, before.evidence_summaries);
    assert_eq!(after_first.blockers, before.blockers);
    assert_eq!(
        event_payload["write_decision_reasons"][0]["category"],
        "scope"
    );
    assert_eq!(
        event_payload["write_decision_reasons"][0]["code"],
        "path_out_of_scope"
    );
    assert!(!event_payload["write_decision_reasons"][0]["related_refs"]
        .as_array()
        .expect("related_refs should be present")
        .is_empty());

    let replay = service.prepare_write(
        request.clone(),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert!(replay.replayed);
    assert_eq!(replay.response_json, first.response_json);
    assert_eq!(fixture.counts()?, after_first);

    let mismatch =
        service.prepare_write(request, invocation(&fixture, AccessClass::CoreMutation))?;
    assert!(!mismatch.replayed);
    assert_rejected_code(&mismatch.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(mismatch
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert!(!mismatch.response_json.contains("path_out_of_scope"));
    assert_eq!(fixture.counts()?, after_first);
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
    run.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id)).into();
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
    reuse.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id)).into();
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
    invalid_input.expected_sha256 = Some("sha256:0000".to_owned()).into();
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

    let scope_judgment = service.request_user_judgment(
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
    let scope_judgment_id = scope_judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("scope judgment id should be present")
        .to_owned();
    let before_scope_record = fixture.counts()?;
    let scope_recorded = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_scope_decision_record",
            idempotency_key: "idem_scope_decision_record",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: &scope_judgment_id,
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

    let sensitive_judgment = service.request_user_judgment(
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
    let sensitive_judgment_id = sensitive_judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("sensitive judgment id should be present")
        .to_owned();
    let before_sensitive = fixture.counts()?;
    let sensitive = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_sensitive_only_record",
            idempotency_key: "idem_sensitive_only_record",
            expected_state_version: Some(5),
            task_id: &task_id,
            user_judgment_id: &sensitive_judgment_id,
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

    let risk_judgment = service.request_user_judgment(
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
    let risk_judgment_id = risk_judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("risk judgment id should be present")
        .to_owned();
    let before_wrong_kind = fixture.counts()?;
    let wrong_kind = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_risk_wrong_answer",
            idempotency_key: "idem_risk_wrong_answer",
            expected_state_version: Some(7),
            task_id: &task_id,
            user_judgment_id: &risk_judgment_id,
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
            answer: answer_payload(JudgmentKind::FinalAcceptance),
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&wrong_kind.response_value, "VALIDATION_FAILED");
    assert_eq!(fixture.counts()?, before_wrong_kind);
    assert_eq!(fixture.user_judgment_status(&risk_judgment_id)?, "pending");
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
    run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Close claim supported by an artifact.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
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
    let mut risk_run = risk_fixture.record_run_request(
        "req_close_risk_basis",
        "idem_close_risk_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    risk_run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    risk_run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Close claim supported with a visible residual risk.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: vec![ResidualRiskInput {
            summary: "Manual verification remains partial.".to_owned(),
            consequence: "The user must accept the remaining manual verification risk.".to_owned(),
            acceptance_required: true,
            source_refs: Vec::new(),
        }],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let risk_run = risk_service.record_run(
        risk_run,
        invocation(&risk_fixture, AccessClass::RunRecording),
    )?;
    let after_evidence = risk_run.response_value["base"]["state_version"]
        .as_u64()
        .expect("risk basis state version should be present");
    let after_final = record_final_acceptance(
        &risk_fixture,
        &risk_service,
        &task_id,
        &change_unit_id,
        after_evidence,
        "risk",
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

#[test]
fn persisted_owner_state_corruption_fails_closed_without_effects() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("corrupt_completion")?;
    let service = core(&fixture);
    let (task_id, _) = create_task_with_change_unit(&fixture, &service, "corrupt_completion")?;
    fixture.set_task_owner_json_raw(
        &task_id,
        TaskOwnerJsonColumn::CompletionPolicy,
        "{not valid json",
    )?;
    let before = fixture.counts()?;

    let check = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_completion_check",
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
    assert_owner_state_unavailable(&check.response_value, "tasks", "completion_policy_json");
    assert_eq!(fixture.counts()?, before);

    let complete = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_completion_complete",
            idempotency_key: Some("idem_corrupt_completion_complete"),
            dry_run: false,
            expected_state_version: Some(before.state_version),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_owner_state_unavailable(&complete.response_value, "tasks", "completion_policy_json");
    assert_eq!(fixture.counts()?, before);

    let fixture = CoreFixture::new("corrupt_close_summary")?;
    let service = core(&fixture);
    let (task_id, _) = create_task_with_change_unit(&fixture, &service, "corrupt_close_summary")?;
    fixture.set_task_owner_json_raw(&task_id, TaskOwnerJsonColumn::CloseSummary, "[")?;
    let before = fixture.counts()?;
    let check = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_close_summary",
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
    assert_owner_state_unavailable(&check.response_value, "tasks", "close_summary_json");
    assert_eq!(fixture.counts()?, before);

    let fixture = CoreFixture::new("corrupt_close_basis")?;
    let service = core(&fixture);
    let (task_id, _) = create_task_with_change_unit(&fixture, &service, "corrupt_close_basis")?;
    fixture.set_task_owner_json_raw(&task_id, TaskOwnerJsonColumn::CurrentCloseBasis, "{")?;
    let before = fixture.counts()?;
    let check = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_close_basis",
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
    assert_owner_state_unavailable(&check.response_value, "tasks", "close_basis_json");
    assert_eq!(fixture.counts()?, before);

    let fixture = CoreFixture::new("corrupt_write_basis")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "corrupt_write_basis")?;
    fixture.set_change_unit_owner_json_raw(
        &change_unit_id,
        ChangeUnitOwnerJsonColumn::WriteBasis,
        "{",
    )?;
    let before = fixture.counts()?;
    let prepare = service.prepare_write(
        fixture.prepare_write_request(
            "req_corrupt_write_basis",
            "idem_corrupt_write_basis",
            Some(before.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_owner_state_unavailable(&prepare.response_value, "change_units", "write_basis_json");
    assert_eq!(fixture.counts()?, before);

    let fixture = CoreFixture::new("corrupt_bounded_paths")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "corrupt_bounded_paths")?;
    fixture.set_change_unit_owner_json_raw(
        &change_unit_id,
        ChangeUnitOwnerJsonColumn::BoundedPaths,
        "{\"unexpected\":true}",
    )?;
    let before = fixture.counts()?;
    let prepare = service.prepare_write(
        fixture.prepare_write_request(
            "req_corrupt_bounded_paths",
            "idem_corrupt_bounded_paths",
            Some(before.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_owner_state_unavailable(
        &prepare.response_value,
        "change_units",
        "bounded_paths_json",
    );
    assert_eq!(fixture.counts()?, before);

    let fixture = CoreFixture::new("corrupt_lifecycle")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "corrupt_lifecycle")?;
    fixture.set_change_unit_owner_json_raw(
        &change_unit_id,
        ChangeUnitOwnerJsonColumn::Lifecycle,
        "{",
    )?;
    let before = fixture.counts()?;
    let check = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_lifecycle",
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
    assert_owner_state_unavailable(&check.response_value, "change_units", "lifecycle_json");
    assert_eq!(fixture.counts()?, before);

    Ok(())
}

#[test]
fn optional_owner_json_null_is_absent_but_malformed_text_fails_closed() -> Result<(), Box<dyn Error>>
{
    let null_fixture = CoreFixture::new("optional_null")?;
    let null_service = core(&null_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&null_fixture, &null_service, "optional_null")?;
    let final_version = record_final_acceptance(
        &null_fixture,
        &null_service,
        &task_id,
        &change_unit_id,
        2,
        "optional_null",
    )?;
    let final_judgment_id = latest_judgment_id(&null_fixture)?;
    null_fixture.set_user_judgment_resolution_raw(&final_judgment_id, None)?;
    let before = null_fixture.counts()?;

    let check = null_service.close_task(
        null_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_optional_null_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(&null_fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(check.response_value["base"]["response_kind"], "result");
    assert_close_blocker(&check.response_value, "missing_final_acceptance");
    assert_eq!(null_fixture.counts()?, before);
    assert_eq!(before.state_version, final_version);

    let malformed_fixture = CoreFixture::new("optional_malformed")?;
    let malformed_service = core(&malformed_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&malformed_fixture, &malformed_service, "optional_bad")?;
    record_final_acceptance(
        &malformed_fixture,
        &malformed_service,
        &task_id,
        &change_unit_id,
        2,
        "optional_bad",
    )?;
    let final_judgment_id = latest_judgment_id(&malformed_fixture)?;
    malformed_fixture.set_user_judgment_resolution_raw(&final_judgment_id, Some("{"))?;
    let before = malformed_fixture.counts()?;
    let check = malformed_service.close_task(
        malformed_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_optional_bad_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(&malformed_fixture, AccessClass::ReadStatus),
    )?;
    assert_owner_state_unavailable(&check.response_value, "user_judgments", "resolution_json");
    assert_eq!(malformed_fixture.counts()?, before);
    Ok(())
}

#[test]
fn write_authorization_expiration_is_enforced_through_record_run() -> Result<(), Box<dyn Error>> {
    let t0 = fixed_time("2026-01-01T00:00:00Z")?;

    let usable = prepared_write_fixture("auth_usable", t0)?;
    let before_usable = usable.fixture.counts()?;
    let usable_response = service_at(&usable.fixture, t0 + Duration::seconds(14 * 60 + 59))
        .record_run(
            product_write_run(
                &usable.fixture,
                "req_auth_usable_run",
                "idem_auth_usable_run",
                before_usable.state_version,
                &usable.task_id,
                &usable.change_unit_id,
                &usable.write_authorization_id,
            ),
            invocation(&usable.fixture, AccessClass::RunRecording),
        )?;
    assert_eq!(
        usable_response.response_value["base"]["response_kind"],
        "result"
    );
    assert_eq!(
        usable
            .fixture
            .write_authorization_status(&usable.write_authorization_id)?,
        "consumed"
    );

    let expired = prepared_write_fixture("auth_expired_exact", t0)?;
    let before_expired = expired.fixture.counts()?;
    let expired_response = service_at(&expired.fixture, t0 + Duration::minutes(15)).record_run(
        product_write_run(
            &expired.fixture,
            "req_auth_expired_run",
            "idem_auth_expired_run",
            before_expired.state_version,
            &expired.task_id,
            &expired.change_unit_id,
            &expired.write_authorization_id,
        ),
        invocation(&expired.fixture, AccessClass::RunRecording),
    )?;
    assert_rejected_code(
        &expired_response.response_value,
        "WRITE_AUTHORIZATION_INVALID",
    );
    assert_eq!(
        expired_response.response_value["errors"][0]["details"]["authorization_reason"],
        "expired"
    );
    assert_eq!(expired.fixture.counts()?, before_expired);
    assert_eq!(
        expired
            .fixture
            .write_authorization_status(&expired.write_authorization_id)?,
        "active"
    );

    let capped = prepared_write_fixture("auth_capped", t0)?;
    capped.fixture.set_write_authorization_timestamps(
        &capped.write_authorization_id,
        &format_time(t0),
        &format_time(t0 + Duration::days(1)),
    )?;
    let before_capped = capped.fixture.counts()?;
    let capped_response = service_at(&capped.fixture, t0 + Duration::minutes(15)).record_run(
        product_write_run(
            &capped.fixture,
            "req_auth_capped_run",
            "idem_auth_capped_run",
            before_capped.state_version,
            &capped.task_id,
            &capped.change_unit_id,
            &capped.write_authorization_id,
        ),
        invocation(&capped.fixture, AccessClass::RunRecording),
    )?;
    assert_rejected_code(
        &capped_response.response_value,
        "WRITE_AUTHORIZATION_INVALID",
    );
    assert_eq!(capped.fixture.counts()?, before_capped);

    let stale = prepared_write_fixture("auth_stale_precedence", t0)?;
    service_at(&stale.fixture, t0).update_scope(
        stale.fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_auth_stale_scope",
            idempotency_key: "idem_auth_stale_scope",
            dry_run: false,
            expected_state_version: Some(stale.fixture.counts()?.state_version),
            task_id: &stale.task_id,
            operation: ChangeUnitOperation::ReplaceCurrent,
            scope_summary: "Replacement scope before stale authorization use.",
        }),
        invocation(&stale.fixture, AccessClass::CoreMutation),
    )?;
    let current_change_unit_id = stale
        .fixture
        .current_change_unit_id(&stale.task_id)?
        .expect("replacement Change Unit should be current");
    let before_stale = stale.fixture.counts()?;
    let stale_response = service_at(&stale.fixture, t0 + Duration::minutes(16)).record_run(
        product_write_run(
            &stale.fixture,
            "req_auth_stale_run",
            "idem_auth_stale_run",
            before_stale.state_version,
            &stale.task_id,
            &current_change_unit_id,
            &stale.write_authorization_id,
        ),
        invocation(&stale.fixture, AccessClass::RunRecording),
    )?;
    assert_rejected_code(&stale_response.response_value, "STATE_VERSION_CONFLICT");
    assert_eq!(stale.fixture.counts()?, before_stale);
    assert_eq!(
        stale
            .fixture
            .write_authorization_status(&stale.write_authorization_id)?,
        "stale"
    );

    Ok(())
}

#[test]
fn prepare_write_allocates_authorization_only_on_committed_allowed_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("auth_allocation")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(&fixture, &service, "auth_alloc")?;

    let mut blocked = fixture.prepare_write_request(
        "req_auth_alloc_blocked",
        "idem_auth_alloc_blocked",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    blocked.intended_paths = vec!["src/out_of_scope.rs".to_owned()];
    let before_blocked = fixture.counts()?;
    let blocked_response = service.prepare_write(
        blocked,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(blocked_response.response_value["decision"], "blocked");
    assert!(blocked_response.response_value["write_authorization_ref"].is_null());
    assert_eq!(
        fixture.counts()?.write_authorizations,
        before_blocked.write_authorizations
    );

    let before_dry_run = fixture.counts()?;
    let mut dry_run = fixture.prepare_write_request(
        "req_auth_alloc_dry",
        "idem_auth_alloc_dry",
        Some(before_dry_run.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    dry_run.envelope.dry_run = true;
    let dry_response = service.prepare_write(
        dry_run,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(
        dry_response.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(fixture.counts()?, before_dry_run);

    let allowed_request = fixture.prepare_write_request(
        "req_auth_alloc_allowed",
        "idem_auth_alloc_allowed",
        Some(before_dry_run.state_version),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let allowed = service.prepare_write(
        allowed_request.clone(),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    let after_allowed = fixture.counts()?;
    let authorization_id = allowed.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("allowed prepare_write should allocate an authorization")
        .to_owned();
    let timestamps = fixture.write_authorization_timestamps(&authorization_id)?;

    let replay = service.prepare_write(
        allowed_request,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert!(replay.replayed);
    assert_eq!(replay.response_json, allowed.response_json);
    assert_eq!(
        replay.response_value["write_authorization_ref"]["record_id"],
        authorization_id
    );
    assert_eq!(
        fixture.write_authorization_timestamps(&authorization_id)?,
        timestamps
    );
    assert_eq!(fixture.counts()?, after_allowed);
    Ok(())
}

fn core(fixture: &CoreFixture) -> CoreService {
    CoreService::new(fixture.runtime_home_path())
}

fn invocation(fixture: &CoreFixture, access_class: AccessClass) -> InvocationContext {
    InvocationContext {
        binding: AdapterSessionBinding::new(
            harness_types::ProjectId::new(fixture.project_id()),
            harness_types::SurfaceId::new(fixture.surface_id()),
            harness_types::SurfaceInstanceId::new(fixture.surface_instance_id()),
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        ),
        requested_access_class: access_class,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FixedClock {
    now: DateTime<Utc>,
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

struct PreparedWriteFixture {
    fixture: CoreFixture,
    task_id: String,
    change_unit_id: String,
    write_authorization_id: String,
}

fn fixed_time(value: &str) -> Result<DateTime<Utc>, Box<dyn Error>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn format_time(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn service_at(fixture: &CoreFixture, now: DateTime<Utc>) -> CoreService {
    CoreService::with_clock(fixture.runtime_home_path(), FixedClock { now })
}

fn prepared_write_fixture(
    suffix: &str,
    now: DateTime<Utc>,
) -> Result<PreparedWriteFixture, Box<dyn Error>> {
    let fixture = CoreFixture::new(suffix)?;
    let service = service_at(&fixture, now);
    let (task_id, change_unit_id) = create_task_with_change_unit(&fixture, &service, suffix)?;
    let response = service.prepare_write(
        fixture.prepare_write_request(
            &format!("req_prepare_{suffix}"),
            &format!("idem_prepare_{suffix}"),
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(response.response_value["decision"], "allowed");
    let write_authorization_id = response.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("write authorization ref should be present")
        .to_owned();
    Ok(PreparedWriteFixture {
        fixture,
        task_id,
        change_unit_id,
        write_authorization_id,
    })
}

fn product_write_run(
    fixture: &CoreFixture,
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: u64,
    task_id: &str,
    change_unit_id: &str,
    write_authorization_id: &str,
) -> harness_types::RecordRunRequest {
    let mut request = fixture.record_run_request(
        request_id,
        idempotency_key,
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec![DEFAULT_PRODUCT_PATH.to_owned()];
    request.write_authorization_id = Some(WriteAuthorizationId::new(write_authorization_id)).into();
    request
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
    request.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Close claim supported.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
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
    let judgment = service.request_user_judgment(
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
    let judgment_id = judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("final acceptance judgment id should be present")
        .to_owned();
    let response = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: &format!("req_final_record_{suffix}"),
            idempotency_key: &format!("idem_final_record_{suffix}"),
            expected_state_version: Some(expected_state_version + 1),
            task_id,
            user_judgment_id: &judgment_id,
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

fn assert_latest_prepare_write_event(
    fixture: &CoreFixture,
    response_value: &Value,
    decision: &str,
) -> Result<Value, Box<dyn Error>> {
    let event = fixture.latest_task_event()?;
    assert_eq!(event.event_kind, "write_decision_recorded");
    assert_eq!(
        event.state_version,
        response_value["base"]["state_version"]
            .as_u64()
            .expect("state_version should be present")
    );
    assert_eq!(event.event_payload["decision"], decision);
    assert!(event.event_payload["write_authorization_id"].is_null());
    assert!(event.event_payload.get("reason_codes").is_none());
    assert!(event.event_payload.get("intended_paths").is_none());
    assert!(event.event_payload.get("intended_operation").is_none());
    assert!(event.event_payload.get("sensitive_categories").is_none());
    assert_eq!(
        event.event_payload["write_decision_reasons"],
        response_value["write_decision_reasons"]
    );
    Ok(event.event_payload)
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

fn assert_owner_state_unavailable(response_value: &Value, table: &str, logical_column: &str) {
    assert_rejected_code(response_value, "MCP_UNAVAILABLE");
    let error = &response_value["errors"][0]["details"]["owner_state_error"];
    assert_eq!(error["table"], table);
    assert_eq!(error["logical_column"], logical_column);
    assert!(
        matches!(
            error["corruption_category"].as_str(),
            Some("corrupt_stored_json" | "corrupt_stored_value")
        ),
        "unexpected owner-state corruption details: {error:?}"
    );
}

fn latest_judgment_id(fixture: &CoreFixture) -> Result<String, Box<dyn Error>> {
    Ok(fixture.conn()?.query_row(
        "SELECT judgment_id
           FROM user_judgments
          WHERE project_id = ?1
          ORDER BY requested_at DESC, judgment_id DESC
          LIMIT 1",
        [fixture.project_id()],
        |row| row.get(0),
    )?)
}

fn assert_public_response_has_no_internal_leak(body: &str, runtime_home_path: &Path) {
    let runtime_home = runtime_home_path.to_string_lossy();
    assert!(!body.contains(runtime_home.as_ref()));
    for fragment in [
        "SELECT ",
        "INSERT INTO",
        "UPDATE ",
        "DELETE ",
        "constraint failed",
        "state.sqlite",
    ] {
        assert!(
            !body.contains(fragment),
            "public response leaked internal fragment {fragment}: {body}"
        );
    }
}
