use super::*;

const STAGE_RESULT_BOUNDARY_CLOCK: &str = "2026-06-18T00:00:00Z";
const STAGE_RESULT_BOUNDARY_EXPIRES_AT: &str = "2026-06-19T00:00:00Z";

fn content_type_for_serialized_stage_result_size(
    task_id: &str,
    handle_suffix: &str,
    target_bytes: usize,
) -> Result<String, Box<dyn Error>> {
    let content_type_prefix = "text/plain;";
    let result = StageArtifactResult {
        base: staging_created_result_base(Some(2), Vec::new()),
        evidence_state: EvidenceDisplayState::Prepared,
        staged_artifact_handle: StagedArtifactHandle {
            handle_id: StagedArtifactHandleId::new(prefixed_durable_id(
                DurableIdKind::StagedArtifact,
                handle_suffix,
            )),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: TaskId::new(task_id),
            created_by_actor_source: AGENT_ACTOR_SOURCE.parse()?,
            content_type: content_type_prefix.to_owned(),
            sha256: "0".repeat(64),
            size_bytes: "staging sample".len() as u64,
            redaction_state: RedactionState::None,
            expires_at: UtcTimestamp::parse(STAGE_RESULT_BOUNDARY_EXPIRES_AT)?,
            consumed: false,
        },
        expires_at: UtcTimestamp::parse(STAGE_RESULT_BOUNDARY_EXPIRES_AT)?,
    };
    let base_bytes = serde_json::to_vec(&result)?.len();
    assert!(
        base_bytes <= target_bytes,
        "target must fit the fixed StageArtifactResult fields"
    );
    Ok(format!(
        "{content_type_prefix}{}",
        "a".repeat(target_bytes - base_bytes)
    ))
}

fn stage_artifact_tmp_dir(harness: &MethodHarness) -> PathBuf {
    harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join("artifacts")
        .join("tmp")
}

#[test]
fn stage_artifact_creates_transient_handle_without_core_commit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_valid")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_valid",
        Some("idem_stage_valid"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "trace.log".to_owned();
    request.content_type = "text/plain; charset=utf-8".to_owned();
    request.safe_bytes_or_notice = "Local trace sample captured for debugging.".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present")
        .to_owned();
    let row = staged_artifact_row(&harness, &handle_id)?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "staging_created"
    );
    assert_eq!(response.response_value["base"]["state_version"], 2);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(
        response.response_value["staged_artifact_handle"]["consumed"],
        false
    );
    assert_eq!(response.response_value.get("artifact_ref"), None);
    assert_eq!(after.state_version, before.state_version);
    assert_eq!(after.artifact_staging, before.artifact_staging + 1);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.authority_events, before.authority_events);
    assert_eq!(after.tool_invocations, before.tool_invocations);
    assert_eq!(row.status, "staged");
    assert_eq!(row.redaction_state, "none");
    assert_eq!(row.created_by_actor_source, AGENT_ACTOR_SOURCE);
    assert!(row.tmp_path.starts_with("artifacts/tmp/"));
    assert!(row.tmp_path.ends_with(".txt"));
    assert!(harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join(&row.tmp_path)
        .exists());
    assert!(
        (23.99..=24.01).contains(&row.ttl_hours),
        "expected 24h TTL, got {}",
        row.ttl_hours
    );
    Ok(())
}

#[test]
fn staged_evidence_input_is_not_close_evidence_until_recorded() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_input_only_evidence")?;
    let required_criterion_id = active_acceptance_criterion_id(&harness, &task_id)?;
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_input_only_evidence",
        Some("idem_stage_input_only_evidence"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "close-trace.log".to_owned();
    request.safe_bytes_or_notice = "Trace captured before evidence was recorded.".to_owned();
    request.relation_hint = Some("close_evidence_input".to_owned()).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["evidence_state"], "prepared");
    let after_stage = harness.counts()?;
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present")
        .to_owned();
    let row = staged_artifact_row(&harness, &handle_id)?;
    let staged_path = harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join(&row.tmp_path);
    let repo_root = product_repo_root(&harness)?;

    assert_eq!(after_stage.state_version, before.state_version);
    assert_eq!(after_stage.artifact_staging, before.artifact_staging + 1);
    assert_eq!(after_stage.artifacts, before.artifacts);
    assert_eq!(after_stage.artifact_links, before.artifact_links);
    assert_eq!(after_stage.evidence_summaries, before.evidence_summaries);
    assert_eq!(
        after_stage.evidence_observations,
        before.evidence_observations
    );
    assert!(staged_path.exists());
    assert!(
        staged_path.starts_with(&harness.runtime_home_path),
        "staged input should live under Runtime Home: {}",
        staged_path.display()
    );
    assert!(
        !staged_path.starts_with(&repo_root),
        "staged input must not be stored in Product Repository: {}",
        staged_path.display()
    );

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_stage_input_only_evidence",
                None,
                false,
                None,
                Some(&task_id),
            ),
            continuity_page: None,
            include: StatusInclude {
                task: true,
                pending_user_actions: false,
                write_ticket: false,
                evidence: true,
                close: true,
                guarantees: false,
                continuity: false,
            },
        },
        invocation(OperationCategory::Read),
    )?;
    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_stage_input_only_evidence",
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

    assert_eq!(status.response_value["close_state"], "blocked");
    assert_eq!(
        status.response_value["summary_card"]["evidence"],
        "required_missing"
    );
    assert_eq!(
        status.response_value["summary_card"]["evidence"],
        status.response_value["evidence_gate"]["state"]
    );
    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_field_absent(&status.response_value["evidence_summary"], "evidence_state");
    let coverage_items = status.response_value["evidence_summary"]["coverage_items"]
        .as_array()
        .expect("coverage_items should be an array");
    let required_item = coverage_items
        .iter()
        .find(|item| {
            item["target"]["acceptance_criterion_id"].as_str()
                == Some(required_criterion_id.as_str())
        })
        .expect("required acceptance criterion should be present");
    assert_eq!(required_item["coverage_state"], "unsupported");
    assert_eq!(
        required_item["target"]["target_kind"],
        "acceptance_criterion"
    );
    assert_eq!(required_item["supporting_artifact_refs"], json!([]));
    assert_eq!(required_item["observation_refs"], json!([]));
    assert_eq!(
        status.response_value["evidence_summary"]["artifact_refs"],
        json!([])
    );
    assert_close_blocker(&status.response_value, "missing_current_close_basis");
    assert_close_blocker(&status.response_value, "evidence_claim_missing");
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_eq!(
        check.response_value["summary_card"]["evidence"],
        "required_missing"
    );
    assert_eq!(
        check.response_value["summary_card"]["evidence"],
        check.response_value["evidence_gate"]["state"]
    );
    assert_eq!(
        status.response_value["evidence_gate"],
        check.response_value["evidence_gate"]
    );
    assert_field_absent(&check.response_value["evidence_summary"], "evidence_state");
    assert_close_blocker(&check.response_value, "missing_current_close_basis");
    assert_close_blocker(&check.response_value, "evidence_claim_missing");
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    assert_eq!(harness.counts()?, after_stage);
    Ok(())
}

#[test]
fn stage_artifact_rejects_checksum_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_sha")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_sha",
        Some("idem_stage_sha"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "checksum mismatch sample".to_owned();
    request.expected_sha256 =
        Some("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_invalid_checksum_format_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_sha_format")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_sha_format",
        Some("idem_stage_sha_format"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "checksum format sample".to_owned();
    request.expected_sha256 = Some("sha256:0000".to_owned()).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert!(response.response_json.contains("64-character SHA-256"));
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_size_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_size")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_size",
        Some("idem_stage_size"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "size mismatch sample".to_owned();
    request.expected_size_bytes = Some(999).into();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_oversized_input_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_big")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_big",
        Some("idem_stage_big"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "huge.log".to_owned();
    request.safe_bytes_or_notice = "x".repeat(MAX_STAGED_BODY_BYTES + 1);
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_accepts_complete_result_at_serialized_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_result_boundary")?;
    let handle_suffix = "stage-result-boundary";
    harness.use_generator_and_clock(
        CountingDurableIdGenerator::new([handle_suffix]),
        ManualClock::at(STAGE_RESULT_BOUNDARY_CLOCK),
    );
    let before = harness.counts()?;
    let mut request =
        stage_artifact_request("req_stage_result_boundary", None, false, Some(2), &task_id);
    request.content_type = content_type_for_serialized_stage_result_size(
        &task_id,
        handle_suffix,
        crate::methods::stage_artifact::MAX_STAGE_ARTIFACT_RESULT_BYTES,
    )?;

    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_json.len(),
        crate::methods::stage_artifact::MAX_STAGE_ARTIFACT_RESULT_BYTES
    );
    assert!(response.response_value["staged_artifact_handle"]["handle_id"].is_string());
    assert!(response.response_value["staged_artifact_handle"]["expires_at"].is_string());
    assert_eq!(
        response.response_value["expires_at"],
        response.response_value["staged_artifact_handle"]["expires_at"]
    );
    let after = harness.counts()?;
    assert_eq!(after.state_version, before.state_version);
    assert_eq!(after.artifact_staging, before.artifact_staging + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations);
    assert!(stage_artifact_tmp_dir(&harness).is_dir());
    Ok(())
}

#[test]
fn stage_artifact_ttl_overflow_rejects_before_staging_effect() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_ttl_overflow")?;
    harness.use_generator_and_clock(
        CountingDurableIdGenerator::new(["stage-ttl-overflow"]),
        ManualClock::at("9999-12-31T23:50:00Z"),
    );
    let before = harness.counts()?;

    let response = harness.service.stage_artifact(
        stage_artifact_request("req_stage_ttl_overflow", None, false, Some(2), &task_id),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "expires_at"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_oversized_complete_result_before_staging_effect(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_result_oversized")?;
    let handle_suffix = "stage-result-oversized";
    harness.use_generator_and_clock(
        CountingDurableIdGenerator::new([handle_suffix]),
        ManualClock::at(STAGE_RESULT_BOUNDARY_CLOCK),
    );
    let tmp_dir = stage_artifact_tmp_dir(&harness);
    assert!(!tmp_dir.exists());
    let before = harness.counts()?;
    let mut request =
        stage_artifact_request("req_stage_result_oversized", None, false, Some(2), &task_id);
    request.content_type = content_type_for_serialized_stage_result_size(
        &task_id,
        handle_suffix,
        crate::methods::stage_artifact::MAX_STAGE_ARTIFACT_RESULT_BYTES + 1,
    )?;

    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "content_type"
    );
    assert!(response
        .response_json
        .contains("24 KiB staging result limit"));
    assert_eq!(harness.counts()?, before);
    assert!(!tmp_dir.exists());
    Ok(())
}

#[test]
fn stage_artifact_rejects_unsafe_secret_input_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_secret")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_secret",
        Some("idem_stage_secret"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "secrets.log".to_owned();
    request.safe_bytes_or_notice = "password=hunter2".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_unsupported_redaction_state() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::to_value(stage_artifact_request(
        "req_stage_bad_redaction",
        Some("idem_stage_bad_redaction"),
        false,
        Some(2),
        "task_redaction",
    ))?;
    value["redaction_state"] = json!("unsupported");

    let error = serde_json::from_value::<StageArtifactRequest>(value)
        .expect_err("unsupported redaction_state should not deserialize");
    assert!(error.to_string().contains("unknown variant"));
    Ok(())
}

#[test]
fn stage_artifact_dry_run_creates_no_handle_or_storage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_dry",
        Some("idem_stage_dry"),
        true,
        Some(2),
        &task_id,
    );
    request.display_name = "trace.md".to_owned();
    request.content_type = "text/markdown".to_owned();
    request.redaction_state = RedactionState::Redacted;
    request.safe_bytes_or_notice = "Redacted diagnostic excerpt.".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert!(response
        .response_value
        .get("staged_artifact_handle")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_dry_run_still_checks_stale_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry_stale")?;
    let before = harness.counts()?;

    let request = stage_artifact_request(
        "req_stage_dry_stale",
        Some("idem_stage_dry_stale"),
        true,
        Some(1),
        &task_id,
    );
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_invalid_input_does_not_bypass_invocation_preflight() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_invocation_first")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_invocation_first",
        Some("idem_stage_invocation_first"),
        true,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = String::new();
    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::Read))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_uses_verified_invocation_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_provenance")?;

    let mut request = stage_artifact_request(
        "req_stage_provenance",
        Some("idem_stage_provenance"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "binary.bin".to_owned();
    request.content_type = "application/octet-stream".to_owned();
    request.redaction_state = RedactionState::Blocked;
    request.safe_bytes_or_notice = "Binary output omitted; see local run context.".to_owned();

    let response = harness
        .service
        .stage_artifact(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(
        response.response_value["staged_artifact_handle"]["created_by_actor_source"],
        AGENT_ACTOR_SOURCE
    );
    assert_eq!(
        response.response_value["staged_artifact_handle"]["redaction_state"],
        "blocked"
    );
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present");
    let row = staged_artifact_row(&harness, handle_id)?;
    assert_eq!(row.created_by_actor_source, AGENT_ACTOR_SOURCE);
    Ok(())
}

#[test]
fn stage_artifact_rejects_caller_submitted_provenance_fields() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::to_value(stage_artifact_request(
        "req_stage_forged_provenance",
        Some("idem_stage_forged_provenance"),
        false,
        Some(2),
        "task_forged_provenance",
    ))?;
    value["created_by_actor_source"] = json!("forged_connection");
    value["created_by_actor_source"] = json!("forged_instance");

    let error = serde_json::from_value::<StageArtifactRequest>(value)
        .expect_err("caller-submitted provenance fields should be rejected");

    assert!(error.to_string().contains("created_by_actor_source"));
    Ok(())
}
