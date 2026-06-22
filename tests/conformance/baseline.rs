use std::{error::Error, fs, path::Path};

use chrono::{DateTime, Duration, Utc};
use harness_core::{
    rejected_response, tool_error, AdapterSessionBinding, Clock, CoreService, InvocationContext,
};
use harness_test_support::core_fixtures::{
    answer_payload, artifact_input_for_handle, supported_evidence_update,
    unsupported_evidence_update, ArtifactOwnerJsonColumn, ChangeUnitOwnerJsonColumn,
    CloseTaskFixture, CoreFixture, EvidenceSummaryOwnerJsonColumn, RecordJudgmentFixture,
    TaskOwnerJsonColumn, UpdateScopeFixture, UserJudgmentFixture, DEFAULT_PRODUCT_PATH,
};
use harness_types::{
    AccessClass, ArtifactInput, ArtifactInputId, ArtifactInputSourceKind, ArtifactRef,
    ChangeUnitOperation, CloseAssessmentInput, CloseIntent, CloseReason, EffectKind, ErrorCode,
    JudgmentKind, ResidualRiskInput, ResponseKind, RunId, StagedArtifactHandle, StateRecordKind,
    StateRecordRef, StatusRequest, UtcTimestamp, WriteAuthorizationId,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
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

    let mut risk_basis = fixture.record_run_request(
        "req_risk_judgment_basis",
        "idem_risk_judgment_basis",
        false,
        Some(6),
        &task_id,
        &change_unit_id,
    );
    risk_basis.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    risk_basis.close_assessment = Some(CloseAssessmentInput {
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
    let risk_basis =
        service.record_run(risk_basis, invocation(&fixture, AccessClass::RunRecording))?;
    let after_risk_basis = risk_basis.response_value["base"]["state_version"]
        .as_u64()
        .expect("risk basis state version should be present");

    let risk_judgment = service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_risk_judgment",
            idempotency_key: "idem_risk_judgment",
            dry_run: false,
            expected_state_version: Some(after_risk_basis),
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
            expected_state_version: Some(after_risk_basis + 1),
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
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&cancel_fixture, &cancel_service, "cancel")?;
    let after_authority = record_cancellation_authority(
        &cancel_fixture,
        &cancel_service,
        &task_id,
        &change_unit_id,
        2,
        "terminal",
    )?;
    let before_cancel = cancel_fixture.counts()?;
    let cancel = cancel_service.close_task(
        cancel_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_cancel",
            idempotency_key: Some("idem_cancel"),
            dry_run: false,
            expected_state_version: Some(after_authority),
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
fn required_resolution_json_null_is_rejected_and_malformed_text_fails_closed(
) -> Result<(), Box<dyn Error>> {
    let null_fixture = CoreFixture::new("required_resolution_null")?;
    let null_service = core(&null_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&null_fixture, &null_service, "required_resolution_null")?;
    let after_basis = record_close_evidence(
        &null_fixture,
        &null_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    record_final_acceptance(
        &null_fixture,
        &null_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "required_resolution_null",
    )?;
    let final_judgment_id = latest_judgment_id(&null_fixture)?;
    let before = null_fixture.counts()?;
    let error = null_fixture
        .set_user_judgment_resolution_raw(&final_judgment_id, None)
        .expect_err("resolved judgment rows require resolution_json");
    assert!(
        format!("{error:?}").contains("ConstraintViolation"),
        "expected SQLite constraint error, got {error:?}"
    );
    assert_eq!(null_fixture.counts()?, before);

    let malformed_fixture = CoreFixture::new("optional_malformed")?;
    let malformed_service = core(&malformed_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&malformed_fixture, &malformed_service, "optional_bad")?;
    let after_basis = record_close_evidence(
        &malformed_fixture,
        &malformed_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    record_final_acceptance(
        &malformed_fixture,
        &malformed_service,
        &task_id,
        &change_unit_id,
        after_basis,
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

#[test]
fn current_close_basis_lifecycle_is_publicly_observable() -> Result<(), Box<dyn Error>> {
    let risk_fixture = CoreFixture::new("basis_public_risk")?;
    let risk_service = core(&risk_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&risk_fixture, &risk_service, "basis_public_risk")?;
    let mut risk_run = risk_fixture.record_run_request(
        "req_basis_public_risk",
        "idem_basis_public_risk",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    risk_run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    risk_run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Close basis with generated risk identity.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: vec![ResidualRiskInput {
            summary: "Generated risk identity must not be caller text.".to_owned(),
            consequence: "The exact risk id must be accepted.".to_owned(),
            acceptance_required: true,
            source_refs: Vec::new(),
        }],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let risk_response = risk_service.record_run(
        risk_run,
        invocation(&risk_fixture, AccessClass::RunRecording),
    )?;
    let risk_id = risk_response.response_value["current_close_basis"]["residual_risks"][0]
        ["risk_id"]
        .as_str()
        .expect("risk id should be generated")
        .to_owned();
    assert_ne!(risk_id, "Generated risk identity must not be caller text.");

    let before_status = risk_fixture.counts()?;
    let status = risk_service.status(
        risk_fixture.status_request("req_basis_public_status", Some(&task_id)),
        invocation(&risk_fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(
        status.response_value["current_close_basis"]["residual_risks"][0]["risk_id"],
        risk_id
    );
    assert_eq!(risk_fixture.counts()?, before_status);

    let empty_fixture = CoreFixture::new("basis_public_empty")?;
    let empty_service = core(&empty_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&empty_fixture, &empty_service, "basis_public_empty")?;
    let mut empty_run = empty_fixture.record_run_request(
        "req_basis_public_empty",
        "idem_basis_public_empty",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    empty_run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    empty_run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Close basis with no identified residual risks.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    empty_service.record_run(
        empty_run,
        invocation(&empty_fixture, AccessClass::RunRecording),
    )?;
    let status = empty_service.status(
        empty_fixture.status_request("req_basis_public_empty_status", Some(&task_id)),
        invocation(&empty_fixture, AccessClass::ReadStatus),
    )?;
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(
        status.response_value["current_close_basis"]["residual_risks"],
        json!([])
    );

    let clear = empty_fixture.record_run_request(
        "req_basis_public_clear",
        "idem_basis_public_clear",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    let clear_response =
        empty_service.record_run(clear, invocation(&empty_fixture, AccessClass::RunRecording))?;
    assert!(clear_response.response_value["current_close_basis"].is_null());
    let status = empty_service.status(
        empty_fixture.status_request("req_basis_public_cleared_status", Some(&task_id)),
        invocation(&empty_fixture, AccessClass::ReadStatus),
    )?;
    assert!(status.response_value["current_close_basis"].is_null());
    let before_final = empty_fixture.counts()?;
    let final_without_basis = empty_service.request_user_judgment(
        empty_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_basis_public_final_without_basis",
            idempotency_key: "idem_basis_public_final_without_basis",
            dry_run: false,
            expected_state_version: Some(4),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(&empty_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&final_without_basis.response_value, "DECISION_UNRESOLVED");
    assert_eq!(empty_fixture.counts()?, before_final);

    let scope_fixture = CoreFixture::new("basis_public_scope")?;
    let scope_service = core(&scope_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&scope_fixture, &scope_service, "basis_public_scope")?;
    record_close_evidence(
        &scope_fixture,
        &scope_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let scope_change = scope_service.update_scope(
        scope_fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_basis_public_scope_change",
            idempotency_key: "idem_basis_public_scope_change",
            dry_run: false,
            expected_state_version: Some(3),
            task_id: &task_id,
            operation: ChangeUnitOperation::KeepCurrent,
            scope_summary: "Material scope change invalidates close basis.",
        }),
        invocation(&scope_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(
        scope_change.response_value["base"]["response_kind"],
        "result"
    );
    let status = scope_service.status(
        scope_fixture.status_request("req_basis_public_scope_status", Some(&task_id)),
        invocation(&scope_fixture, AccessClass::ReadStatus),
    )?;
    assert!(status.response_value["current_close_basis"].is_null());
    Ok(())
}

#[test]
fn judgment_compatibility_is_exact_for_close_and_write_requirements() -> Result<(), Box<dyn Error>>
{
    let scope_fixture = CoreFixture::new("compat_scope_final")?;
    let scope_service = core(&scope_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&scope_fixture, &scope_service, "compat_scope_final")?;
    let after_basis = record_close_evidence(
        &scope_fixture,
        &scope_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let (after_final, final_id) = record_final_acceptance_with_id(
        &scope_fixture,
        &scope_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "compat_scope_final",
    )?;
    scope_service.update_scope(
        scope_fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_compat_scope_final_change",
            idempotency_key: "idem_compat_scope_final_change",
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            operation: ChangeUnitOperation::KeepCurrent,
            scope_summary: "Scope change makes final acceptance stale.",
        }),
        invocation(&scope_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(scope_fixture.user_judgment_status(&final_id)?, "stale");
    assert_eq!(
        scope_fixture.user_judgment_basis_status(&final_id)?,
        "stale"
    );

    let run_fixture = CoreFixture::new("compat_run_final")?;
    let run_service = core(&run_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&run_fixture, &run_service, "compat_run_final")?;
    let after_basis = record_close_evidence(
        &run_fixture,
        &run_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let (after_final, final_id) = record_final_acceptance_with_id(
        &run_fixture,
        &run_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "compat_run_final",
    )?;
    record_close_evidence(
        &run_fixture,
        &run_service,
        &task_id,
        &change_unit_id,
        after_final,
        true,
    )?;
    assert_eq!(run_fixture.user_judgment_status(&final_id)?, "stale");

    let partial_fixture = CoreFixture::new("compat_risk_partial")?;
    let partial_service = core(&partial_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&partial_fixture, &partial_service, "compat_risk_partial")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &partial_fixture,
        &partial_service,
        &task_id,
        &change_unit_id,
        2,
        "compat_risk_partial",
        vec![
            residual_risk_input("Risk A needs exact acceptance."),
            residual_risk_input("Risk B needs exact acceptance."),
        ],
    )?;
    let risk_judgment = partial_service.request_user_judgment(
        partial_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_compat_risk_partial",
            idempotency_key: "idem_compat_risk_partial",
            dry_run: false,
            expected_state_version: Some(after_basis),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
        }),
        invocation(&partial_fixture, AccessClass::CoreMutation),
    )?;
    let risk_judgment_id = response_record_id(&risk_judgment.response_value, "user_judgment_ref");
    let partial = partial_service.record_user_judgment(
        partial_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_compat_risk_partial_record",
            idempotency_key: "idem_compat_risk_partial_record",
            expected_state_version: Some(after_basis + 1),
            task_id: &task_id,
            user_judgment_id: &risk_judgment_id,
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
            answer: residual_risk_acceptance_payload(&[risk_ids[0].clone()]),
        }),
        invocation(&partial_fixture, AccessClass::CoreMutation),
    )?;
    let after_partial = partial.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version");
    let after_final = record_final_acceptance(
        &partial_fixture,
        &partial_service,
        &task_id,
        &change_unit_id,
        after_partial,
        "compat_risk_partial",
    )?;
    let close = partial_service.close_task(
        partial_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_compat_risk_partial_close",
            idempotency_key: Some("idem_compat_risk_partial_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedWithRiskAccepted),
            superseding_task_id: None,
        }),
        invocation(&partial_fixture, AccessClass::CoreMutation),
    )?;
    assert_close_blocker(&close.response_value, "missing_residual_risk_acceptance");

    let text_fixture = CoreFixture::new("compat_risk_text")?;
    let text_service = core(&text_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&text_fixture, &text_service, "compat_risk_text")?;
    let (after_old, old_ids) = record_close_basis_with_risks(
        &text_fixture,
        &text_service,
        &task_id,
        &change_unit_id,
        2,
        "compat_risk_text_old",
        vec![residual_risk_input("Same visible risk text.")],
    )?;
    let (after_current, current_ids) = record_close_basis_with_risks(
        &text_fixture,
        &text_service,
        &task_id,
        &change_unit_id,
        after_old,
        "compat_risk_text_current",
        vec![residual_risk_input("Same visible risk text.")],
    )?;
    assert_ne!(old_ids[0], current_ids[0]);
    let risk_judgment = text_service.request_user_judgment(
        text_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_compat_risk_text",
            idempotency_key: "idem_compat_risk_text",
            dry_run: false,
            expected_state_version: Some(after_current),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
        }),
        invocation(&text_fixture, AccessClass::CoreMutation),
    )?;
    let risk_judgment_id = response_record_id(&risk_judgment.response_value, "user_judgment_ref");
    let before_wrong = text_fixture.counts()?;
    let wrong_risk = text_service.record_user_judgment(
        text_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_compat_risk_text_record",
            idempotency_key: "idem_compat_risk_text_record",
            expected_state_version: Some(after_current + 1),
            task_id: &task_id,
            user_judgment_id: &risk_judgment_id,
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
            answer: residual_risk_acceptance_payload(&old_ids),
        }),
        invocation(&text_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&wrong_risk.response_value, "VALIDATION_FAILED");
    assert_eq!(text_fixture.counts()?, before_wrong);
    assert_eq!(
        text_fixture.user_judgment_status(&risk_judgment_id)?,
        "pending"
    );

    assert_sensitive_approval_mismatch("path", |request| {
        request.intended_paths = vec!["tests/export.rs".to_owned()];
    })?;
    assert_sensitive_approval_mismatch("category", |request| {
        request.sensitive_categories = vec!["credential".to_owned()];
    })?;
    assert_sensitive_approval_change_unit_mismatch()?;

    let pending_fixture = CoreFixture::new("compat_pending_superseded")?;
    let pending_service = core(&pending_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &pending_fixture,
        &pending_service,
        "compat_pending_superseded",
    )?;
    let pending = pending_service.request_user_judgment(
        pending_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_compat_pending",
            idempotency_key: "idem_compat_pending",
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ProductDecision,
        }),
        invocation(&pending_fixture, AccessClass::CoreMutation),
    )?;
    let pending_id = response_record_id(&pending.response_value, "user_judgment_ref");
    pending_service.update_scope(
        pending_fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_compat_pending_scope",
            idempotency_key: "idem_compat_pending_scope",
            dry_run: false,
            expected_state_version: Some(3),
            task_id: &task_id,
            operation: ChangeUnitOperation::KeepCurrent,
            scope_summary: "Scope change supersedes pending judgment.",
        }),
        invocation(&pending_fixture, AccessClass::CoreMutation),
    )?;
    let before_answer = pending_fixture.counts()?;
    let stale_answer = pending_service.record_user_judgment(
        pending_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_compat_pending_answer",
            idempotency_key: "idem_compat_pending_answer",
            expected_state_version: Some(4),
            task_id: &task_id,
            user_judgment_id: &pending_id,
            judgment_kind: JudgmentKind::ProductDecision,
            answer: answer_payload(JudgmentKind::ProductDecision),
        }),
        invocation(&pending_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&stale_answer.response_value, "DECISION_UNRESOLVED");
    assert_eq!(pending_fixture.counts()?, before_answer);

    Ok(())
}

#[test]
fn basisless_user_judgments_are_rejected_by_storage_constraints() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("basis_required")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "basis_required")?;
    let after_basis =
        record_close_evidence(&fixture, &service, &task_id, &change_unit_id, 2, true)?;
    let (_, final_id) = record_final_acceptance_with_id(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        after_basis,
        "basis_required",
    )?;
    let before = fixture.counts()?;

    let error = fixture
        .clear_user_judgment_basis(&final_id)
        .expect_err("basis_json is required for stored judgments");
    assert_constraint_error(error);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn status_projection_matches_public_close_check_and_stays_read_only() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("status_projection")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "status_projection")?;
    record_close_evidence(&fixture, &service, &task_id, &change_unit_id, 2, true)?;
    let before = fixture.counts()?;

    let status = service.status(
        fixture.status_request("req_status_projection", Some(&task_id)),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;
    let check = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_status_projection_check",
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

    assert_eq!(status.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "supported"
    );
    assert!(status.response_value["current_close_basis"].is_object());
    assert_eq!(
        status.response_value["close_blockers"],
        check.response_value["blockers"]
    );
    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_ne!(
        status.response_value["guarantee_display"]["level"],
        "detective"
    );
    assert_eq!(fixture.counts()?, before);

    let t0 = fixed_time("2026-06-18T00:00:00Z")?;
    let expired = prepared_write_fixture("status_expired_projection", t0)?;
    let before_status = expired.fixture.counts()?;
    let expired_status = service_at(&expired.fixture, t0 + Duration::minutes(15)).status(
        expired
            .fixture
            .status_request("req_status_expired_projection", Some(&expired.task_id)),
        invocation(&expired.fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(
        expired_status.response_value["write_authority_summary"]["status"],
        "expired"
    );
    assert_eq!(
        expired
            .fixture
            .write_authorization_status(&expired.write_authorization_id)?,
        "active"
    );
    assert_eq!(expired.fixture.counts()?, before_status);
    Ok(())
}

#[test]
fn public_negative_authority_option_selection_remains_non_authoritative(
) -> Result<(), Box<dyn Error>> {
    let accepted_fixture = CoreFixture::new("negative_final_accepted")?;
    let accepted_service = core(&accepted_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&accepted_fixture, &accepted_service, "negative_accepted")?;
    let after_basis = record_close_evidence(
        &accepted_fixture,
        &accepted_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let after_final = record_final_acceptance(
        &accepted_fixture,
        &accepted_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "negative_accepted",
    )?;
    let closed = accepted_service.close_task(
        accepted_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_negative_accepted_close",
            idempotency_key: Some("idem_negative_accepted_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&accepted_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");

    let rejected_fixture = CoreFixture::new("negative_final_rejected")?;
    let rejected_service = core(&rejected_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &rejected_fixture,
        &rejected_service,
        "negative_final_rejected",
    )?;
    let after_basis = record_close_evidence(
        &rejected_fixture,
        &rejected_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let (after_final, judgment_id) = record_authority_judgment_with_option(
        &rejected_fixture,
        &rejected_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "negative_final_rejected",
        JudgmentKind::FinalAcceptance,
        "reject",
        rejected_authority_answer_payload(JudgmentKind::FinalAcceptance, &[]),
    )?;
    let response = rejected_service.close_task(
        rejected_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_negative_final_close_rejected",
            idempotency_key: Some("idem_negative_final_close_rejected"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&rejected_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(
        rejected_fixture.user_judgment_resolution_outcome(&judgment_id)?,
        Some("rejected".to_owned())
    );

    let actor_fixture = CoreFixture::new("negative_final_actor")?;
    let actor_service = core(&actor_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&actor_fixture, &actor_service, "negative_actor")?;
    let after_basis = record_close_evidence(
        &actor_fixture,
        &actor_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let judgment = actor_service.request_user_judgment(
        actor_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_negative_actor_final",
            idempotency_key: "idem_negative_actor_final",
            dry_run: false,
            expected_state_version: Some(after_basis),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(&actor_fixture, AccessClass::CoreMutation),
    )?;
    assert_current_authority_options(&judgment.response_value);
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let mut agent_record = actor_fixture.record_judgment_request(RecordJudgmentFixture {
        request_id: "req_negative_actor_final_record",
        idempotency_key: "idem_negative_actor_final_record",
        expected_state_version: Some(after_basis + 1),
        task_id: &task_id,
        user_judgment_id: &judgment_id,
        judgment_kind: JudgmentKind::FinalAcceptance,
        answer: answer_payload(JudgmentKind::FinalAcceptance),
    });
    agent_record.envelope.actor_kind = harness_types::ActorKind::Agent;
    let before_agent_record = actor_fixture.counts()?;
    let agent_rejected = actor_service.record_user_judgment(
        agent_record,
        invocation(&actor_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&agent_rejected.response_value, "VALIDATION_FAILED");
    assert_eq!(actor_fixture.counts()?, before_agent_record);
    assert_eq!(actor_fixture.user_judgment_status(&judgment_id)?, "pending");
    let actor_blocked = actor_service.close_task(
        actor_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_negative_actor_close",
            idempotency_key: Some("idem_negative_actor_close"),
            dry_run: false,
            expected_state_version: Some(after_basis + 1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&actor_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(actor_blocked.response_value["close_state"], "blocked");
    assert_close_blocker(&actor_blocked.response_value, "missing_final_acceptance");

    let fixture = CoreFixture::new("negative_risk_rejected")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "negative_risk_rejected")?;
    let (after_basis, risk_ids) = record_close_basis_with_risks(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        2,
        "rejected",
        vec![residual_risk_input(
            "Risk needs exact accepted user coverage.",
        )],
    )?;
    let (after_record, judgment_id) = record_authority_judgment_with_option(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        after_basis,
        "negative_risk_rejected",
        JudgmentKind::ResidualRiskAcceptance,
        "reject",
        rejected_authority_answer_payload(JudgmentKind::ResidualRiskAcceptance, &risk_ids),
    )?;
    let status = service.status(
        fixture.status_request("req_negative_risk_status_rejected", Some(&task_id)),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(status.response_value["base"]["state_version"], after_record);
    assert_eq!(
        fixture.user_judgment_resolution_outcome(&judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(
        status.response_value["risk_acceptance_coverage"][0]["accepted"],
        false
    );
    assert_eq!(
        status.response_value["risk_acceptance_coverage"][0]["accepted_by_judgment_refs"],
        json!([])
    );
    assert_close_blocker(&status.response_value, "missing_residual_risk_acceptance");

    let sensitive_fixture = CoreFixture::new("negative_sensitive_accepted")?;
    let sensitive_service = core(&sensitive_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &sensitive_fixture,
        &sensitive_service,
        "negative_sensitive_accepted",
    )?;
    let (after_approval, judgment_id) = record_sensitive_approval(
        &sensitive_fixture,
        &sensitive_service,
        &task_id,
        &change_unit_id,
        2,
        "negative_sensitive_accepted",
    )?;
    let mut prepare = sensitive_fixture.prepare_write_request(
        "req_negative_sensitive_allowed",
        "idem_negative_sensitive_allowed",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.intended_operation = "local_sensitive_step".to_owned();
    prepare.sensitive_categories = vec!["network".to_owned()];
    let allowed = sensitive_service.prepare_write(
        prepare,
        invocation(&sensitive_fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(allowed.response_value["decision"], "allowed");
    assert_eq!(
        allowed.response_value["active_user_judgment_refs"][0]["record_id"],
        judgment_id
    );

    let fixture = CoreFixture::new("negative_sensitive_rejected")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "negative_sensitive_rejected")?;
    let (after_approval, judgment_id) = record_authority_judgment_with_option(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        2,
        "negative_sensitive_rejected",
        JudgmentKind::SensitiveApproval,
        "reject",
        rejected_authority_answer_payload(JudgmentKind::SensitiveApproval, &[]),
    )?;
    let mut prepare = fixture.prepare_write_request(
        "req_negative_sensitive_prepare_rejected",
        "idem_negative_sensitive_prepare_rejected",
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.intended_operation = "local_sensitive_step".to_owned();
    prepare.sensitive_categories = vec!["network".to_owned()];
    let response = service.prepare_write(
        prepare,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(
        fixture.user_judgment_resolution_outcome(&judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_authorization"].is_null());

    let conflict_fixture = CoreFixture::new("negative_answer_conflict")?;
    let conflict_service = core(&conflict_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&conflict_fixture, &conflict_service, "answer_conflict")?;
    let judgment = conflict_service.request_user_judgment(
        conflict_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_negative_answer_conflict",
            idempotency_key: "idem_negative_answer_conflict",
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ScopeDecision,
        }),
        invocation(&conflict_fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let mut request = conflict_fixture.record_judgment_request(RecordJudgmentFixture {
        request_id: "req_negative_answer_conflict_record",
        idempotency_key: "idem_negative_answer_conflict_record",
        expected_state_version: Some(3),
        task_id: &task_id,
        user_judgment_id: &judgment_id,
        judgment_kind: JudgmentKind::ScopeDecision,
        answer: answer_payload(JudgmentKind::ScopeDecision),
    });
    request.selected_option_id = harness_types::UserJudgmentOptionId::new("reject");
    let before = conflict_fixture.counts()?;
    let rejected = conflict_service.record_user_judgment(
        request,
        invocation(&conflict_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&rejected.response_value, "VALIDATION_FAILED");
    assert_eq!(conflict_fixture.counts()?, before);
    assert_eq!(
        conflict_fixture.user_judgment_status(&judgment_id)?,
        "pending"
    );

    let mut missing_selected = serde_json::to_value(conflict_fixture.record_judgment_request(
        RecordJudgmentFixture {
            request_id: "req_negative_missing_option",
            idempotency_key: "idem_negative_missing_option",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::FinalAcceptance,
            answer: answer_payload(JudgmentKind::FinalAcceptance),
        },
    ))?;
    missing_selected
        .as_object_mut()
        .expect("record judgment request should be an object")
        .remove("selected_option_id");
    assert!(
        serde_json::from_value::<harness_types::RecordUserJudgmentRequest>(missing_selected)
            .is_err()
    );
    Ok(())
}

#[test]
fn public_sensitive_lifecycle_preserves_full_scope_through_close() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("sensitive_public_lifecycle_conf")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "sensitive_public_lifecycle")?;

    let (after_sensitive, _) = record_sensitive_approval(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        2,
        "public_lifecycle",
    )?;
    let mut prepare = fixture.prepare_write_request(
        "req_sensitive_public_prepare",
        "idem_sensitive_public_prepare",
        Some(after_sensitive),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare.intended_operation = "local_sensitive_step".to_owned();
    prepare.sensitive_categories = vec!["network".to_owned()];
    let prepared = service.prepare_write(
        prepare,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(prepared.response_value["decision"], "allowed");
    let write_authorization_id =
        response_record_id(&prepared.response_value, "write_authorization_ref");
    let after_prepare = prepared.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let mut run = product_write_run(
        &fixture,
        "req_sensitive_public_run",
        "idem_sensitive_public_run",
        after_prepare,
        &task_id,
        &change_unit_id,
        &write_authorization_id,
    );
    run.observed_changes.sensitive_categories = vec!["network".to_owned()];
    run.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Sensitive product write is ready for close.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: vec!["network".to_owned()],
        recovery_constraints: Vec::new(),
    })
    .into();
    let recorded = service.record_run(run, invocation(&fixture, AccessClass::RunRecording))?;
    let requirement =
        &recorded.response_value["current_close_basis"]["sensitive_action_requirements"][0];
    assert_eq!(requirement["action_kind"], "local_sensitive_step");
    assert_eq!(
        requirement["normalized_paths"],
        json!([DEFAULT_PRODUCT_PATH])
    );
    assert_eq!(requirement["sensitive_categories"], json!(["network"]));
    assert_eq!(requirement["baseline_ref"], "baseline_fixture");
    assert_eq!(requirement["change_unit_id"], change_unit_id);
    assert_eq!(
        requirement["source_write_authorization_ref"]["record_id"],
        write_authorization_id
    );
    let after_run = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present");

    let status = service.status(
        fixture.status_request("req_sensitive_public_status_after_run", Some(&task_id)),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;
    assert_eq!(
        status.response_value["current_close_basis"]["sensitive_action_requirements"][0],
        *requirement
    );

    let after_final = record_final_acceptance(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        after_run,
        "sensitive_public_lifecycle",
    )?;
    let closed = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_sensitive_public_close",
            idempotency_key: Some("idem_sensitive_public_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    Ok(())
}

#[test]
fn cancellation_and_pending_relevance_are_operation_specific() -> Result<(), Box<dyn Error>> {
    let missing_fixture = CoreFixture::new("cancel_missing_authority")?;
    let missing_service = core(&missing_fixture);
    let (task_id, _) =
        create_task_with_change_unit(&missing_fixture, &missing_service, "cancel_missing")?;
    let missing = missing_service.close_task(
        missing_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_cancel_missing",
            idempotency_key: Some("idem_cancel_missing"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(&missing_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(missing.response_value["close_state"], "blocked");
    assert_close_blocker(&missing.response_value, "missing_cancellation_authority");

    let fixture = CoreFixture::new("cancel_negative_rejected")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "cancel_negative_rejected")?;
    let (after_authority, judgment_id) = record_authority_judgment_with_option(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        2,
        "cancel_negative_rejected",
        JudgmentKind::Cancellation,
        "reject",
        rejected_authority_answer_payload(JudgmentKind::Cancellation, &[]),
    )?;
    let response = service.close_task(
        fixture.close_task_request(CloseTaskFixture {
            request_id: "req_cancel_negative_rejected",
            idempotency_key: Some("idem_cancel_negative_rejected"),
            dry_run: false,
            expected_state_version: Some(after_authority),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(
        fixture.user_judgment_resolution_outcome(&judgment_id)?,
        Some("rejected".to_owned())
    );
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "cancellation_rejected");

    let stale_fixture = CoreFixture::new("cancel_scope_stale")?;
    let stale_service = core(&stale_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&stale_fixture, &stale_service, "cancel_stale")?;
    let after_authority = record_cancellation_authority(
        &stale_fixture,
        &stale_service,
        &task_id,
        &change_unit_id,
        2,
        "stale",
    )?;
    let scope = stale_service.update_scope(
        stale_fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_cancel_scope_stale",
            idempotency_key: "idem_cancel_scope_stale",
            dry_run: false,
            expected_state_version: Some(after_authority),
            task_id: &task_id,
            operation: ChangeUnitOperation::ReplaceCurrent,
            scope_summary: "Replacement scope makes cancellation authority stale.",
        }),
        invocation(&stale_fixture, AccessClass::CoreMutation),
    )?;
    let after_scope = scope.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let stale = stale_service.close_task(
        stale_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_cancel_stale_close",
            idempotency_key: Some("idem_cancel_stale_close"),
            dry_run: false,
            expected_state_version: Some(after_scope),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(&stale_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(stale.response_value["close_state"], "blocked");
    assert_close_blocker(&stale.response_value, "cancellation_judgment_stale");

    let final_pending_fixture = CoreFixture::new("cancel_ignores_pending_final")?;
    let final_pending_service = core(&final_pending_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &final_pending_fixture,
        &final_pending_service,
        "cancel_ignores_final",
    )?;
    let after_basis = record_close_evidence(
        &final_pending_fixture,
        &final_pending_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    final_pending_service.request_user_judgment(
        final_pending_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_cancel_pending_final",
            idempotency_key: "idem_cancel_pending_final",
            dry_run: false,
            expected_state_version: Some(after_basis),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(&final_pending_fixture, AccessClass::CoreMutation),
    )?;
    let after_cancel_authority = record_cancellation_authority(
        &final_pending_fixture,
        &final_pending_service,
        &task_id,
        &change_unit_id,
        after_basis + 1,
        "pending_final",
    )?;
    let cancelled = final_pending_service.close_task(
        final_pending_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_cancel_pending_final_close",
            idempotency_key: Some("idem_cancel_pending_final_close"),
            dry_run: false,
            expected_state_version: Some(after_cancel_authority),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(&final_pending_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(cancelled.response_value["close_state"], "cancelled");

    for kind in [JudgmentKind::FinalAcceptance, JudgmentKind::Cancellation] {
        let fixture = CoreFixture::new(&format!("pending_prepare_{kind:?}"))?;
        let service = core(&fixture);
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&fixture, &service, &format!("pending_prepare_{kind:?}"))?;
        let mut expected_version = 2;
        if kind == JudgmentKind::FinalAcceptance {
            expected_version =
                record_close_evidence(&fixture, &service, &task_id, &change_unit_id, 2, true)?;
        }
        service.request_user_judgment(
            fixture.user_judgment_request(UserJudgmentFixture {
                request_id: &format!("req_pending_prepare_{kind:?}"),
                idempotency_key: &format!("idem_pending_prepare_{kind:?}"),
                dry_run: false,
                expected_state_version: Some(expected_version),
                task_id: &task_id,
                change_unit_id: Some(&change_unit_id),
                judgment_kind: kind,
            }),
            invocation(&fixture, AccessClass::CoreMutation),
        )?;
        let prepared = service.prepare_write(
            fixture.prepare_write_request(
                &format!("req_pending_prepare_write_{kind:?}"),
                &format!("idem_pending_prepare_write_{kind:?}"),
                Some(expected_version + 1),
                Some(&task_id),
                Some(&change_unit_id),
            ),
            invocation(&fixture, AccessClass::WriteAuthorization),
        )?;
        assert_eq!(prepared.response_value["decision"], "allowed");
    }

    let sensitive_pending_fixture = CoreFixture::new("pending_sensitive_prepare")?;
    let sensitive_pending_service = core(&sensitive_pending_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &sensitive_pending_fixture,
        &sensitive_pending_service,
        "pending_sensitive",
    )?;
    sensitive_pending_service.request_user_judgment(
        sensitive_pending_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_pending_sensitive_prepare",
            idempotency_key: "idem_pending_sensitive_prepare",
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::SensitiveApproval,
        }),
        invocation(&sensitive_pending_fixture, AccessClass::CoreMutation),
    )?;
    let mut sensitive_prepare = sensitive_pending_fixture.prepare_write_request(
        "req_pending_sensitive_prepare_write",
        "idem_pending_sensitive_prepare_write",
        Some(3),
        Some(&task_id),
        Some(&change_unit_id),
    );
    sensitive_prepare.intended_operation = "local_sensitive_step".to_owned();
    sensitive_prepare.sensitive_categories = vec!["network".to_owned()];
    let sensitive_blocked = sensitive_pending_service.prepare_write(
        sensitive_prepare,
        invocation(&sensitive_pending_fixture, AccessClass::WriteAuthorization),
    )?;
    assert_ne!(sensitive_blocked.response_value["decision"], "allowed");
    assert!(sensitive_blocked.response_value["write_authorization"].is_null());

    let close_pending_fixture = CoreFixture::new("pending_close_complete")?;
    let close_pending_service = core(&close_pending_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &close_pending_fixture,
        &close_pending_service,
        "pending_close",
    )?;
    let after_basis = record_close_evidence(
        &close_pending_fixture,
        &close_pending_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    close_pending_service.request_user_judgment(
        close_pending_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_pending_close_final",
            idempotency_key: "idem_pending_close_final",
            dry_run: false,
            expected_state_version: Some(after_basis),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(&close_pending_fixture, AccessClass::CoreMutation),
    )?;
    let close = close_pending_service.close_task(
        close_pending_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_pending_close_complete",
            idempotency_key: Some("idem_pending_close_complete"),
            dry_run: false,
            expected_state_version: Some(after_basis + 1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&close_pending_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(close.response_value["close_state"], "blocked");
    assert_close_blocker(&close.response_value, "pending_user_judgment");

    let info_fixture = CoreFixture::new("pending_info_close")?;
    let info_service = core(&info_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&info_fixture, &info_service, "pending_info")?;
    let mut info_request = info_fixture.user_judgment_request(UserJudgmentFixture {
        request_id: "req_pending_info",
        idempotency_key: "idem_pending_info",
        dry_run: false,
        expected_state_version: Some(2),
        task_id: &task_id,
        change_unit_id: Some(&change_unit_id),
        judgment_kind: JudgmentKind::TechnicalDecision,
    });
    info_request.required_for = vec![harness_types::JudgmentRequiredFor::Informational];
    info_service.request_user_judgment(
        info_request,
        invocation(&info_fixture, AccessClass::CoreMutation),
    )?;
    let after_basis = record_close_evidence(
        &info_fixture,
        &info_service,
        &task_id,
        &change_unit_id,
        3,
        true,
    )?;
    let after_final = record_final_acceptance(
        &info_fixture,
        &info_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "pending_info",
    )?;
    let closed = info_service.close_task(
        info_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_pending_info_close",
            idempotency_key: Some("idem_pending_info_close"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(&info_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(closed.response_value["close_state"], "closed");
    Ok(())
}

#[test]
fn canonical_close_refs_and_artifact_integrity_remain_truthful() -> Result<(), Box<dyn Error>> {
    for (index, (record_kind, record_id)) in [
        (StateRecordKind::WriteAuthorization, "wa_fabricated"),
        (StateRecordKind::UserJudgment, "uj_fabricated"),
        (StateRecordKind::Blocker, "blocker_fabricated"),
        (StateRecordKind::TaskEvent, "evt_fabricated"),
    ]
    .into_iter()
    .enumerate()
    {
        let fixture = CoreFixture::new(&format!("canonical_unsupported_{index}"))?;
        let service = core(&fixture);
        let (task_id, change_unit_id) = create_task_with_change_unit(
            &fixture,
            &service,
            &format!("canonical_unsupported_{index}"),
        )?;
        let mut request = fixture.record_run_request(
            &format!("req_canonical_unsupported_{index}"),
            &format!("idem_canonical_unsupported_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(CloseAssessmentInput {
            result_summary: "Unsupported refs must not enter close authority.".to_owned(),
            result_refs: vec![state_record_ref(
                &fixture,
                &task_id,
                record_kind,
                record_id,
                Some(999),
            )],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();
        let before = fixture.counts()?;
        let response =
            service.record_run(request, invocation(&fixture, AccessClass::RunRecording))?;
        assert_rejected_code(&response.response_value, "VALIDATION_FAILED");
        assert_eq!(fixture.counts()?, before);
    }

    let missing_fixture = CoreFixture::new("canonical_missing_allowed")?;
    let missing_service = core(&missing_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&missing_fixture, &missing_service, "missing_allowed")?;
    let mut missing = missing_fixture.record_run_request(
        "req_canonical_missing_allowed",
        "idem_canonical_missing_allowed",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    missing.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Missing allowed refs still need stored records.".to_owned(),
        result_refs: vec![state_record_ref(
            &missing_fixture,
            &task_id,
            StateRecordKind::Run,
            "run_missing",
            Some(2),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let before = missing_fixture.counts()?;
    let response = missing_service.record_run(
        missing,
        invocation(&missing_fixture, AccessClass::RunRecording),
    )?;
    assert_rejected_code(&response.response_value, "VALIDATION_FAILED");
    assert_eq!(missing_fixture.counts()?, before);

    let cross_fixture = CoreFixture::new("canonical_cross_refs")?;
    let cross_service = core(&cross_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&cross_fixture, &cross_service, "cross_refs")?;
    for (index, record_ref) in [
        state_record_ref_with_project(
            &cross_fixture,
            &task_id,
            "project_other",
            StateRecordKind::Artifact,
            "artifact_cross_project",
            Some(2),
        ),
        state_record_ref_with_project(
            &cross_fixture,
            "task_other",
            cross_fixture.project_id(),
            StateRecordKind::Run,
            "run_cross_task",
            Some(2),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut request = cross_fixture.record_run_request(
            &format!("req_canonical_cross_{index}"),
            &format!("idem_canonical_cross_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(CloseAssessmentInput {
            result_summary: "Cross-owner refs must not enter close authority.".to_owned(),
            result_refs: vec![record_ref],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();
        let before = cross_fixture.counts()?;
        let response = cross_service.record_run(
            request,
            invocation(&cross_fixture, AccessClass::RunRecording),
        )?;
        assert_rejected_code(&response.response_value, "VALIDATION_FAILED");
        assert_eq!(cross_fixture.counts()?, before);
    }

    let canonical_fixture = CoreFixture::new("canonical_dedup")?;
    let canonical_service = core(&canonical_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&canonical_fixture, &canonical_service, "canonical_dedup")?;
    let mut request = canonical_fixture.record_run_request(
        "req_canonical_dedup",
        "idem_canonical_dedup",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_canonical_dedup")).into();
    request.evidence_updates = vec![supported_evidence_update("Canonical close basis claim.")];
    let future_run_ref = state_record_ref(
        &canonical_fixture,
        &task_id,
        StateRecordKind::Run,
        "run_canonical_dedup",
        Some(999),
    );
    let past_run_ref = state_record_ref(
        &canonical_fixture,
        &task_id,
        StateRecordKind::Run,
        "run_canonical_dedup",
        Some(1),
    );
    let mut risk = residual_risk_input("Caller-versioned risk source.");
    risk.acceptance_required = false;
    risk.source_refs = vec![future_run_ref.clone(), past_run_ref.clone()];
    request.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Canonical refs are stored.".to_owned(),
        result_refs: vec![future_run_ref, past_run_ref],
        residual_risks: vec![risk],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let response = canonical_service.record_run(
        request,
        invocation(&canonical_fixture, AccessClass::RunRecording),
    )?;
    let basis = &response.response_value["current_close_basis"];
    let result_refs = basis["result_refs"]
        .as_array()
        .expect("result refs should be present");
    assert_eq!(
        result_refs
            .iter()
            .filter(|record_ref| record_ref["record_kind"] == "run"
                && record_ref["record_id"] == "run_canonical_dedup")
            .count(),
        1
    );
    assert!(result_refs.iter().any(|record_ref| {
        record_ref["record_kind"] == "run"
            && record_ref["record_id"] == "run_canonical_dedup"
            && record_ref["state_version"] == 3
    }));
    assert!(result_refs.iter().any(|record_ref| {
        record_ref["record_kind"] == "change_unit"
            && record_ref["record_id"] == change_unit_id
            && record_ref["state_version"] == 3
    }));
    assert!(result_refs
        .iter()
        .any(|record_ref| record_ref["record_kind"] == "evidence_summary"
            && record_ref["state_version"] == 3));
    assert_eq!(
        basis["residual_risks"][0]["source_refs"][0]["state_version"],
        3
    );

    let final_judgment = canonical_service.request_user_judgment(
        canonical_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_canonical_final_basis",
            idempotency_key: "idem_canonical_final_basis",
            dry_run: false,
            expected_state_version: Some(3),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(&canonical_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(
        final_judgment.response_value["user_judgment"]["basis"]["result_refs"],
        basis["result_refs"]
    );

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let artifact_fixture = CoreFixture::new("artifact_integrity_real")?;
    let artifact_service = core(&artifact_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&artifact_fixture, &artifact_service, "artifact_integrity")?;
    let mut stage = artifact_fixture.stage_artifact_request(
        "req_artifact_integrity_stage",
        Some("idem_artifact_integrity_stage"),
        false,
        Some(2),
        &task_id,
    );
    stage.safe_bytes_or_notice = "hello".to_owned();
    stage.content_type = "text/plain".to_owned();
    let staged = artifact_service.stage_artifact(
        stage,
        invocation(&artifact_fixture, AccessClass::ArtifactRegistration),
    )?;
    let handle: StagedArtifactHandle =
        serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;
    let mut run = artifact_fixture.record_run_request(
        "req_artifact_integrity_run",
        "idem_artifact_integrity_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_integrity",
        handle,
        Some("validation_report"),
        Some("Artifact integrity is recorded."),
    )];
    let recorded = artifact_service.record_run(
        run,
        invocation(&artifact_fixture, AccessClass::RunRecording),
    )?;
    let artifact = &recorded.response_value["registered_artifacts"][0];
    assert_eq!(artifact["content_type"], "text/plain");
    assert_eq!(artifact["sha256"], HELLO_SHA256);
    assert_eq!(artifact["size_bytes"], 5);
    assert_eq!(artifact["integrity_status"], "verified");

    let zero_fixture = CoreFixture::new("artifact_integrity_zero")?;
    let zero_service = core(&zero_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&zero_fixture, &zero_service, "artifact_zero")?;
    let mut zero_stage = zero_fixture.stage_artifact_request(
        "req_artifact_zero_stage",
        Some("idem_artifact_zero_stage"),
        false,
        Some(2),
        &task_id,
    );
    zero_stage.safe_bytes_or_notice = String::new();
    zero_stage.expected_sha256 = Some(EMPTY_SHA256.to_owned()).into();
    zero_stage.expected_size_bytes = Some(0).into();
    let staged = zero_service.stage_artifact(
        zero_stage,
        invocation(&zero_fixture, AccessClass::ArtifactRegistration),
    )?;
    let handle: StagedArtifactHandle =
        serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;
    let mut run = zero_fixture.record_run_request(
        "req_artifact_zero_run",
        "idem_artifact_zero_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_zero",
        handle,
        Some("empty_report"),
        Some("Zero-byte artifact was registered."),
    )];
    let zero =
        zero_service.record_run(run, invocation(&zero_fixture, AccessClass::RunRecording))?;
    assert_eq!(
        zero.response_value["registered_artifacts"][0]["sha256"],
        EMPTY_SHA256
    );
    assert_eq!(
        zero.response_value["registered_artifacts"][0]["size_bytes"],
        0
    );

    let legacy_fixture = CoreFixture::new("artifact_integrity_legacy")?;
    let legacy_service = core(&legacy_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&legacy_fixture, &legacy_service, "artifact_legacy")?;
    let staged = stage_artifact_for_record_run(&legacy_fixture, &legacy_service, &task_id)?;
    let mut run = legacy_fixture.record_run_request(
        "req_artifact_legacy_run",
        "idem_artifact_legacy_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_legacy",
        staged,
        Some("validation_report"),
        Some("Legacy integrity evidence."),
    )];
    run.evidence_updates = vec![supported_evidence_update("Legacy integrity evidence.")];
    run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Legacy integrity evidence.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let response =
        legacy_service.record_run(run, invocation(&legacy_fixture, AccessClass::RunRecording))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();
    legacy_fixture.set_artifact_integrity(&artifact_id, "legacy_unknown", None, None, None)?;
    let status = legacy_service.status(
        legacy_fixture.status_request("req_artifact_legacy_status", Some(&task_id)),
        invocation(&legacy_fixture, AccessClass::ReadStatus),
    )?;
    let artifact_ref = &status.response_value["evidence_summary"]["coverage_items"][0]
        ["supporting_artifact_refs"][0];
    assert_eq!(artifact_ref["integrity_status"], "legacy_unknown");
    assert!(artifact_ref["content_type"].is_null());
    assert!(artifact_ref["sha256"].is_null());
    assert!(artifact_ref["size_bytes"].is_null());
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn persisted_state_corruption_public_entries_fail_closed_without_effects(
) -> Result<(), Box<dyn Error>> {
    let request_fixture = CoreFixture::new("corrupt_public_request")?;
    let request_service = core(&request_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&request_fixture, &request_service, "corrupt_public_request")?;
    let judgment = request_service.request_user_judgment(
        request_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_corrupt_public_request",
            idempotency_key: "idem_corrupt_public_request",
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ProductDecision,
        }),
        invocation(&request_fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    request_fixture.set_user_judgment_request_raw(
        &judgment_id,
        r#"{"presentation":17,"question":"must not leak secret-request-path","required_for":["close_complete"],"expires_at":null}"#,
    )?;
    let before = request_fixture.counts()?;
    let response = request_service.record_user_judgment(
        request_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_corrupt_public_request_record",
            idempotency_key: "idem_corrupt_public_request_record",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::ProductDecision,
            answer: answer_payload(JudgmentKind::ProductDecision),
        }),
        invocation(&request_fixture, AccessClass::CoreMutation),
    )?;
    assert_owner_state_unavailable(&response.response_value, "user_judgments", "request_json");
    assert_eq!(request_fixture.counts()?, before);

    let resolution_fixture = CoreFixture::new("corrupt_public_resolution")?;
    let resolution_service = core(&resolution_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &resolution_fixture,
        &resolution_service,
        "corrupt_public_resolution",
    )?;
    let after_basis = record_close_evidence(
        &resolution_fixture,
        &resolution_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    record_final_acceptance(
        &resolution_fixture,
        &resolution_service,
        &task_id,
        &change_unit_id,
        after_basis,
        "corrupt_public_resolution",
    )?;
    let judgment_id = latest_judgment_id(&resolution_fixture)?;
    resolution_fixture.set_user_judgment_resolution_raw(&judgment_id, Some("{"))?;
    let before = resolution_fixture.counts()?;
    let response = resolution_service.close_task(
        resolution_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_public_resolution_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(&resolution_fixture, AccessClass::ReadStatus),
    )?;
    assert_owner_state_unavailable(
        &response.response_value,
        "user_judgments",
        "resolution_json",
    );
    assert_eq!(resolution_fixture.counts()?, before);

    let basis_fixture = CoreFixture::new("corrupt_public_basis")?;
    let basis_service = core(&basis_fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&basis_fixture, &basis_service, "corrupt_public_basis")?;
    let judgment = basis_service.request_user_judgment(
        basis_fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_corrupt_public_basis",
            idempotency_key: "idem_corrupt_public_basis",
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ProductDecision,
        }),
        invocation(&basis_fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    basis_fixture.set_user_judgment_basis_raw(
        &judgment_id,
        r#"{"scope_revision":"bad","compatibility_status":"current"}"#,
    )?;
    let before = basis_fixture.counts()?;
    let response = basis_service.record_user_judgment(
        basis_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_corrupt_public_basis_record",
            idempotency_key: "idem_corrupt_public_basis_record",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::ProductDecision,
            answer: answer_payload(JudgmentKind::ProductDecision),
        }),
        invocation(&basis_fixture, AccessClass::CoreMutation),
    )?;
    assert_owner_state_unavailable(&response.response_value, "user_judgments", "basis_json");
    assert_eq!(basis_fixture.counts()?, before);

    let artifact_fixture = CoreFixture::new("corrupt_public_artifact")?;
    let artifact_service = core(&artifact_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &artifact_fixture,
        &artifact_service,
        "corrupt_public_artifact",
    )?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &artifact_fixture,
        &artifact_service,
        &task_id,
        &change_unit_id,
        2,
        "corrupt_public_artifact",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    artifact_fixture.set_artifact_owner_json_raw(
        &artifact_id,
        ArtifactOwnerJsonColumn::Producer,
        "{",
    )?;
    let mut run = artifact_fixture.record_run_request(
        "req_corrupt_public_artifact_reuse",
        "idem_corrupt_public_artifact_reuse",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_corrupt_public",
        artifact_ref,
    )];
    let before = artifact_fixture.counts()?;
    let response = artifact_service.record_run(
        run,
        invocation(&artifact_fixture, AccessClass::RunRecording),
    )?;
    assert_owner_state_unavailable(&response.response_value, "artifacts", "producer_json");
    assert_eq!(artifact_fixture.counts()?, before);

    let provenance_fixture = CoreFixture::new("corrupt_public_provenance")?;
    let provenance_service = core(&provenance_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &provenance_fixture,
        &provenance_service,
        "corrupt_public_provenance",
    )?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &provenance_fixture,
        &provenance_service,
        &task_id,
        &change_unit_id,
        2,
        "corrupt_public_provenance",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let artifact_state_ref =
        artifact_state_ref(&provenance_fixture, &task_id, &artifact_id, state_version);
    let mut basis_run = provenance_fixture.record_run_request(
        "req_corrupt_public_provenance_basis",
        "idem_corrupt_public_provenance_basis",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    basis_run.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_corrupt_public_provenance",
        artifact_ref,
    )];
    basis_run.close_assessment = Some(CloseAssessmentInput {
        result_summary: "Close basis references corrupt provenance artifact.".to_owned(),
        result_refs: vec![artifact_state_ref],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    provenance_service.record_run(
        basis_run,
        invocation(&provenance_fixture, AccessClass::RunRecording),
    )?;
    provenance_fixture.set_artifact_source_staging_handle_raw(&artifact_id, None)?;
    let before = provenance_fixture.counts()?;
    let response = provenance_service.close_task(
        provenance_fixture.close_task_request(CloseTaskFixture {
            request_id: "req_corrupt_public_provenance_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(&provenance_fixture, AccessClass::ReadStatus),
    )?;
    assert_owner_state_unavailable(
        &response.response_value,
        "artifacts",
        "source_staging_handle_id",
    );
    assert_eq!(provenance_fixture.counts()?, before);

    let evidence_fixture = CoreFixture::new("corrupt_public_evidence")?;
    let evidence_service = core(&evidence_fixture);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &evidence_fixture,
        &evidence_service,
        "corrupt_public_evidence",
    )?;
    record_close_evidence(
        &evidence_fixture,
        &evidence_service,
        &task_id,
        &change_unit_id,
        2,
        true,
    )?;
    let evidence_summary_id = evidence_fixture.latest_evidence_summary_id(&task_id)?;
    evidence_fixture.set_evidence_summary_owner_json_raw(
        &evidence_summary_id,
        EvidenceSummaryOwnerJsonColumn::Metadata,
        r#"{"updated_by_run_id":123}"#,
    )?;
    let before = evidence_fixture.counts()?;
    let response = evidence_service.status(
        evidence_fixture.status_request("req_corrupt_public_evidence_status", Some(&task_id)),
        invocation(&evidence_fixture, AccessClass::ReadStatus),
    )?;
    assert_owner_state_unavailable(
        &response.response_value,
        "evidence_summaries",
        "metadata_json",
    );
    assert_eq!(evidence_fixture.counts()?, before);
    Ok(())
}

#[test]
fn timestamp_semantics_use_rfc3339_instants_without_sleep() -> Result<(), Box<dyn Error>> {
    let t0 = fixed_time("2026-06-18T00:00:00Z")?;

    let invalid_fixture = CoreFixture::new("timestamp_invalid_judgment")?;
    let invalid_service = service_at(&invalid_fixture, t0);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&invalid_fixture, &invalid_service, "timestamp_invalid")?;
    let mut request = invalid_fixture.user_judgment_request(UserJudgmentFixture {
        request_id: "req_timestamp_invalid",
        idempotency_key: "idem_timestamp_invalid",
        dry_run: false,
        expected_state_version: Some(2),
        task_id: &task_id,
        change_unit_id: Some(&change_unit_id),
        judgment_kind: JudgmentKind::ProductDecision,
    });
    request.expires_at = Some(UtcTimestamp::parse("2026-06-18T00:00:00Z")?).into();
    let before = invalid_fixture.counts()?;
    let response = invalid_service.request_user_judgment(
        request,
        invocation(&invalid_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&response.response_value, "VALIDATION_FAILED");
    assert_eq!(invalid_fixture.counts()?, before);

    let offset_fixture = CoreFixture::new("timestamp_offset_judgment")?;
    let offset_service = service_at(&offset_fixture, t0);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&offset_fixture, &offset_service, "timestamp_offset")?;
    let mut request = offset_fixture.user_judgment_request(UserJudgmentFixture {
        request_id: "req_timestamp_offset",
        idempotency_key: "idem_timestamp_offset",
        dry_run: false,
        expected_state_version: Some(2),
        task_id: &task_id,
        change_unit_id: Some(&change_unit_id),
        judgment_kind: JudgmentKind::ProductDecision,
    });
    request.expires_at = Some(UtcTimestamp::parse("2026-06-18T09:00:01+09:00")?).into();
    let judgment = offset_service.request_user_judgment(
        request,
        invocation(&offset_fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let recorded = offset_service.record_user_judgment(
        offset_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_timestamp_offset_record",
            idempotency_key: "idem_timestamp_offset_record",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::ProductDecision,
            answer: answer_payload(JudgmentKind::ProductDecision),
        }),
        invocation(&offset_fixture, AccessClass::CoreMutation),
    )?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");

    let boundary_fixture = CoreFixture::new("timestamp_judgment_boundary")?;
    let boundary_service = service_at(&boundary_fixture, t0);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&boundary_fixture, &boundary_service, "timestamp_boundary")?;
    let mut request = boundary_fixture.user_judgment_request(UserJudgmentFixture {
        request_id: "req_timestamp_boundary",
        idempotency_key: "idem_timestamp_boundary",
        dry_run: false,
        expected_state_version: Some(2),
        task_id: &task_id,
        change_unit_id: Some(&change_unit_id),
        judgment_kind: JudgmentKind::ProductDecision,
    });
    request.expires_at = Some(UtcTimestamp::parse("2026-06-18T00:00:01Z")?).into();
    let judgment = boundary_service.request_user_judgment(
        request,
        invocation(&boundary_fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let before = boundary_fixture.counts()?;
    let expired = service_at(&boundary_fixture, t0 + Duration::seconds(1)).record_user_judgment(
        boundary_fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_timestamp_boundary_record",
            idempotency_key: "idem_timestamp_boundary_record",
            expected_state_version: Some(3),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::ProductDecision,
            answer: answer_payload(JudgmentKind::ProductDecision),
        }),
        invocation(&boundary_fixture, AccessClass::CoreMutation),
    )?;
    assert_rejected_code(&expired.response_value, "DECISION_UNRESOLVED");
    assert_eq!(boundary_fixture.counts()?, before);

    let stage_before = CoreFixture::new("timestamp_stage_before")?;
    let stage_service = service_at(&stage_before, t0);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&stage_before, &stage_service, "timestamp_stage_before")?;
    let mut handle = stage_artifact_for_record_run(&stage_before, &stage_service, &task_id)?;
    handle.expires_at = UtcTimestamp::parse("2026-06-19T09:00:00+09:00")?;
    let mut run = stage_before.record_run_request(
        "req_timestamp_stage_before_run",
        "idem_timestamp_stage_before_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_timestamp_stage_before",
        handle,
        Some("timestamp_report"),
        Some("Timestamp staged artifact before boundary."),
    )];
    let before = stage_before.counts()?;
    let recorded = service_at(
        &stage_before,
        t0 + Duration::hours(24) - Duration::seconds(1),
    )
    .record_run(run, invocation(&stage_before, AccessClass::RunRecording))?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");
    assert_eq!(stage_before.counts()?.runs, before.runs + 1);

    let stage_exact = CoreFixture::new("timestamp_stage_exact")?;
    let stage_service = service_at(&stage_exact, t0);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&stage_exact, &stage_service, "timestamp_stage_exact")?;
    let handle = stage_artifact_for_record_run(&stage_exact, &stage_service, &task_id)?;
    let mut run = stage_exact.record_run_request(
        "req_timestamp_stage_exact_run",
        "idem_timestamp_stage_exact_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_timestamp_stage_exact",
        handle,
        Some("timestamp_report"),
        Some("Timestamp staged artifact exact boundary."),
    )];
    let before = stage_exact.counts()?;
    let expired = service_at(&stage_exact, t0 + Duration::hours(24))
        .record_run(run, invocation(&stage_exact, AccessClass::RunRecording))?;
    assert_rejected_code(&expired.response_value, "VALIDATION_FAILED");
    assert_eq!(
        expired.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(stage_exact.counts()?, before);

    let corrupt_fixture = CoreFixture::new("timestamp_stage_corrupt")?;
    let corrupt_service = service_at(&corrupt_fixture, t0);
    let (task_id, change_unit_id) = create_task_with_change_unit(
        &corrupt_fixture,
        &corrupt_service,
        "timestamp_stage_corrupt",
    )?;
    let handle = stage_artifact_for_record_run(&corrupt_fixture, &corrupt_service, &task_id)?;
    corrupt_fixture.set_staged_artifact_expires_at(handle.handle_id.as_str(), "tomorrow")?;
    let mut run = corrupt_fixture.record_run_request(
        "req_timestamp_stage_corrupt_run",
        "idem_timestamp_stage_corrupt_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_timestamp_stage_corrupt",
        handle,
        Some("timestamp_report"),
        Some("Timestamp staged artifact corrupt expiration."),
    )];
    let before = corrupt_fixture.counts()?;
    let response =
        corrupt_service.record_run(run, invocation(&corrupt_fixture, AccessClass::RunRecording))?;
    assert_owner_state_unavailable(&response.response_value, "artifact_staging", "expires_at");
    assert_eq!(corrupt_fixture.counts()?, before);
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
        &format!("req_close_evidence_{supported}_{expected_state_version}"),
        &format!("idem_close_evidence_{supported}_{expected_state_version}"),
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

fn record_cancellation_authority(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<u64, Box<dyn Error>> {
    let judgment = service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: &format!("req_cancel_auth_{suffix}"),
            idempotency_key: &format!("idem_cancel_auth_{suffix}"),
            dry_run: false,
            expected_state_version: Some(expected_state_version),
            task_id,
            change_unit_id: Some(change_unit_id),
            judgment_kind: JudgmentKind::Cancellation,
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let response = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: &format!("req_cancel_auth_record_{suffix}"),
            idempotency_key: &format!("idem_cancel_auth_record_{suffix}"),
            expected_state_version: Some(expected_state_version + 1),
            task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::Cancellation,
            answer: answer_payload(JudgmentKind::Cancellation),
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present"))
}

fn record_final_acceptance_with_id(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<(u64, String), Box<dyn Error>> {
    let judgment = service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: &format!("req_final_id_{suffix}"),
            idempotency_key: &format!("idem_final_id_{suffix}"),
            dry_run: false,
            expected_state_version: Some(expected_state_version),
            task_id,
            change_unit_id: Some(change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let response = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: &format!("req_final_id_record_{suffix}"),
            idempotency_key: &format!("idem_final_id_record_{suffix}"),
            expected_state_version: Some(expected_state_version + 1),
            task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::FinalAcceptance,
            answer: answer_payload(JudgmentKind::FinalAcceptance),
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    Ok((
        response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"),
        judgment_id,
    ))
}

#[allow(clippy::too_many_arguments)]
fn record_authority_judgment_with_option(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    judgment_kind: JudgmentKind,
    selected_option_id: &str,
    answer: harness_types::RecordUserJudgmentPayload,
) -> Result<(u64, String), Box<dyn Error>> {
    let judgment = service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: &format!("req_authority_{suffix}"),
            idempotency_key: &format!("idem_authority_{suffix}"),
            dry_run: false,
            expected_state_version: Some(expected_state_version),
            task_id,
            change_unit_id: Some(change_unit_id),
            judgment_kind,
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    assert_current_authority_options(&judgment.response_value);
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let mut request = fixture.record_judgment_request(RecordJudgmentFixture {
        request_id: &format!("req_authority_record_{suffix}"),
        idempotency_key: &format!("idem_authority_record_{suffix}"),
        expected_state_version: Some(expected_state_version + 1),
        task_id,
        user_judgment_id: &judgment_id,
        judgment_kind,
        answer,
    });
    request.selected_option_id = harness_types::UserJudgmentOptionId::new(selected_option_id);
    let response =
        service.record_user_judgment(request, invocation(fixture, AccessClass::CoreMutation))?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["selected_option_id"],
        selected_option_id
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["machine_action"],
        selected_option_id
    );
    let expected_outcome = match selected_option_id {
        "accept" => "accepted",
        "reject" => "rejected",
        "defer" => "deferred",
        _ => panic!("unexpected authority option id {selected_option_id}"),
    };
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolution_outcome"],
        expected_outcome
    );
    Ok((
        response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"),
        judgment_id,
    ))
}

fn record_close_basis_with_risks(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    residual_risks: Vec<ResidualRiskInput>,
) -> Result<(u64, Vec<String>), Box<dyn Error>> {
    let mut request = fixture.record_run_request(
        &format!("req_basis_risk_{suffix}"),
        &format!("idem_basis_risk_{suffix}"),
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.evidence_updates = vec![supported_evidence_update("Close claim supported.")];
    request.close_assessment = Some(CloseAssessmentInput {
        result_summary: format!("Close basis for {suffix}."),
        result_refs: Vec::new(),
        residual_risks,
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let response = service.record_run(request, invocation(fixture, AccessClass::RunRecording))?;
    let risk_ids = response.response_value["current_close_basis"]["residual_risks"]
        .as_array()
        .expect("residual risks should be present")
        .iter()
        .map(|risk| {
            risk["risk_id"]
                .as_str()
                .expect("risk id should be present")
                .to_owned()
        })
        .collect::<Vec<_>>();
    Ok((
        response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"),
        risk_ids,
    ))
}

fn residual_risk_input(summary: &str) -> ResidualRiskInput {
    ResidualRiskInput {
        summary: summary.to_owned(),
        consequence: "The named residual risk remains close-relevant.".to_owned(),
        acceptance_required: true,
        source_refs: Vec::new(),
    }
}

fn residual_risk_acceptance_payload(
    risk_ids: &[String],
) -> harness_types::RecordUserJudgmentPayload {
    let mut payload = answer_payload(JudgmentKind::ResidualRiskAcceptance);
    payload.residual_risk_acceptance = Some(json_object(json!({ "risk_ids": risk_ids }))).into();
    payload
}

fn rejected_authority_answer_payload(
    judgment_kind: JudgmentKind,
    risk_ids: &[String],
) -> harness_types::RecordUserJudgmentPayload {
    let mut payload = answer_payload(judgment_kind);
    match judgment_kind {
        JudgmentKind::ScopeDecision => {
            payload.scope_decision = Some(json_object(json!({
                "requested_scope_summary": "Expanded scope that must not apply silently.",
                "decision": "rejected"
            })))
            .into();
        }
        JudgmentKind::SensitiveApproval => {}
        JudgmentKind::FinalAcceptance => {
            payload.final_acceptance = Some(json_object(json!({
                "judgment": {
                    "decision": "rejected",
                    "basis": "The visible close basis is not accepted."
                }
            })))
            .into();
        }
        JudgmentKind::ResidualRiskAcceptance => {
            payload.residual_risk_acceptance = Some(json_object(json!({
                "risk_ids": risk_ids,
                "decision": "rejected"
            })))
            .into();
        }
        JudgmentKind::Cancellation => {
            payload.cancellation = Some(json_object(json!({
                "decision": "rejected",
                "reason": "The user rejected cancellation."
            })))
            .into();
        }
        JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision => {
            panic!("non-authority judgment kind does not use rejected authority payloads")
        }
    }
    payload
}

fn assert_sensitive_approval_mismatch<F>(suffix: &str, mutate: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&mut harness_types::PrepareWriteRequest),
{
    let fixture = CoreFixture::new(&format!("compat_sensitive_{suffix}"))?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, &format!("compat_sensitive_{suffix}"))?;
    let (after_approval, _) =
        record_sensitive_approval(&fixture, &service, &task_id, &change_unit_id, 2, suffix)?;
    let mut request = fixture.prepare_write_request(
        &format!("req_compat_sensitive_{suffix}"),
        &format!("idem_compat_sensitive_{suffix}"),
        Some(after_approval),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_operation = "local_sensitive_step".to_owned();
    request.sensitive_categories = vec!["network".to_owned()];
    mutate(&mut request);
    let response = service.prepare_write(
        request,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_authorization"].is_null());
    Ok(())
}

fn assert_sensitive_approval_change_unit_mismatch() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("compat_sensitive_change_unit")?;
    let service = core(&fixture);
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&fixture, &service, "compat_sensitive_change_unit")?;
    let (after_approval, _) = record_sensitive_approval(
        &fixture,
        &service,
        &task_id,
        &change_unit_id,
        2,
        "change_unit",
    )?;
    let replace = service.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_compat_sensitive_cu_replace",
            idempotency_key: "idem_compat_sensitive_cu_replace",
            dry_run: false,
            expected_state_version: Some(after_approval),
            task_id: &task_id,
            operation: ChangeUnitOperation::ReplaceCurrent,
            scope_summary: "Replacement scope for sensitive approval.",
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");
    let after_replace = replace.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version should be present");
    let mut request = fixture.prepare_write_request(
        "req_compat_sensitive_cu_prepare",
        "idem_compat_sensitive_cu_prepare",
        Some(after_replace),
        Some(&task_id),
        Some(&replacement_change_unit_id),
    );
    request.intended_operation = "local_sensitive_step".to_owned();
    request.sensitive_categories = vec!["network".to_owned()];
    let response = service.prepare_write(
        request,
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;
    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert!(response.response_value["write_authorization"].is_null());
    Ok(())
}

fn record_sensitive_approval(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<(u64, String), Box<dyn Error>> {
    let judgment = service.request_user_judgment(
        fixture.user_judgment_request(UserJudgmentFixture {
            request_id: &format!("req_sensitive_approval_{suffix}"),
            idempotency_key: &format!("idem_sensitive_approval_{suffix}"),
            dry_run: false,
            expected_state_version: Some(expected_state_version),
            task_id,
            change_unit_id: Some(change_unit_id),
            judgment_kind: JudgmentKind::SensitiveApproval,
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    let response = service.record_user_judgment(
        fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: &format!("req_sensitive_approval_record_{suffix}"),
            idempotency_key: &format!("idem_sensitive_approval_record_{suffix}"),
            expected_state_version: Some(expected_state_version + 1),
            task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::SensitiveApproval,
            answer: answer_payload(JudgmentKind::SensitiveApproval),
        }),
        invocation(fixture, AccessClass::CoreMutation),
    )?;
    Ok((
        response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"),
        judgment_id,
    ))
}

fn promote_artifact_for_record_run(
    fixture: &CoreFixture,
    service: &CoreService,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<(u64, ArtifactRef), Box<dyn Error>> {
    let handle = stage_artifact_for_record_run(fixture, service, task_id)?;
    let mut request = fixture.record_run_request(
        &format!("req_promote_artifact_{suffix}"),
        &format!("idem_promote_artifact_{suffix}"),
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        &format!("artifact_input_promote_{suffix}"),
        handle,
        Some("validation_report"),
        Some("Artifact registered for corruption coverage."),
    )];
    let response = service.record_run(request, invocation(fixture, AccessClass::RunRecording))?;
    let artifact_ref =
        serde_json::from_value(response.response_value["registered_artifacts"][0].clone())?;
    Ok((
        response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"),
        artifact_ref,
    ))
}

fn existing_artifact_input(artifact_input_id: &str, artifact_ref: ArtifactRef) -> ArtifactInput {
    let expected_sha256 = artifact_ref.sha256.as_ref().cloned();
    let expected_size_bytes = artifact_ref.size_bytes.as_ref().copied();
    let redaction_state = artifact_ref.redaction_state;
    ArtifactInput {
        artifact_input_id: ArtifactInputId::new(artifact_input_id),
        source_kind: ArtifactInputSourceKind::ExistingArtifact,
        staged_artifact_handle: None.into(),
        existing_artifact_ref: Some(artifact_ref).into(),
        relation_hint: Some("validation_report".to_owned()).into(),
        claim: Some("Reused artifact for corruption coverage.".to_owned()).into(),
        expected_sha256: expected_sha256.into(),
        expected_size_bytes: expected_size_bytes.into(),
        redaction_state: Some(redaction_state).into(),
    }
}

fn artifact_state_ref(
    fixture: &CoreFixture,
    task_id: &str,
    artifact_id: &str,
    state_version: u64,
) -> harness_types::StateRecordRef {
    harness_types::StateRecordRef {
        record_kind: harness_types::StateRecordKind::Artifact,
        record_id: harness_types::RecordId::new(artifact_id),
        project_id: harness_types::ProjectId::new(fixture.project_id()),
        task_id: Some(harness_types::TaskId::new(task_id)).into(),
        state_version: Some(state_version).into(),
    }
}

fn state_record_ref(
    fixture: &CoreFixture,
    task_id: &str,
    record_kind: StateRecordKind,
    record_id: &str,
    state_version: Option<u64>,
) -> StateRecordRef {
    state_record_ref_with_project(
        fixture,
        task_id,
        fixture.project_id(),
        record_kind,
        record_id,
        state_version,
    )
}

fn state_record_ref_with_project(
    _fixture: &CoreFixture,
    task_id: &str,
    project_id: &str,
    record_kind: StateRecordKind,
    record_id: &str,
    state_version: Option<u64>,
) -> StateRecordRef {
    StateRecordRef {
        record_kind,
        record_id: harness_types::RecordId::new(record_id),
        project_id: harness_types::ProjectId::new(project_id),
        task_id: Some(harness_types::TaskId::new(task_id)).into(),
        state_version: state_version.into(),
    }
}

fn response_record_id(response_value: &Value, field: &str) -> String {
    response_value[field]["record_id"]
        .as_str()
        .expect("record_id should be present")
        .to_owned()
}

fn assert_current_authority_options(response_value: &Value) {
    let options = response_value["user_judgment"]["options"]
        .as_array()
        .expect("authority judgment options should be present");
    let option_ids = options
        .iter()
        .map(|option| option["option_id"].as_str().expect("option id"))
        .collect::<Vec<_>>();
    assert_eq!(option_ids, vec!["accept", "reject"]);
    for option in options {
        match option["option_id"].as_str().expect("option id") {
            "accept" => {
                assert_eq!(option["machine_action"], "accept");
                assert_eq!(option["resolution_outcome"], "accepted");
                assert_eq!(option["is_default"], true);
            }
            "reject" => {
                assert_eq!(option["machine_action"], "reject");
                assert_eq!(option["resolution_outcome"], "rejected");
                assert_eq!(option["is_default"], false);
            }
            other => panic!("unexpected authority option id {other}"),
        }
    }
}

fn json_object(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(object) => object,
        _ => panic!("expected JSON object"),
    }
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
    let blockers = response_value
        .get("blockers")
        .or_else(|| response_value.get("close_blockers"))
        .expect("blockers or close_blockers should be present");
    let codes = blockers
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

fn assert_constraint_error(error: impl std::fmt::Debug) {
    let details = format!("{error:?}");
    assert!(
        details.contains("ConstraintViolation"),
        "expected SQLite constraint error, got {details}"
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
