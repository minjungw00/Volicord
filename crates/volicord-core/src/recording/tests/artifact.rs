use super::*;

#[test]
fn record_run_rejects_cross_project_artifact_and_cross_task_run_refs_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "cross_refs")?;

    for (index, record_ref) in [
        test_state_record_ref(
            StateRecordKind::Artifact,
            "artifact_cross_project",
            "project_other",
            &task_id,
            Some(2),
        ),
        test_state_record_ref(
            StateRecordKind::Run,
            "run_cross_task",
            PROJECT_ID,
            "task_other",
            Some(2),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_cross_ref_{index}"),
            &format!("idem_cross_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.run_id = Some(RunId::new(format!("run_cross_ref_{index}"))).into();
        request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
            result_summary: "Cross-owner refs must not enter close authority.".to_owned(),
            result_refs: vec![record_ref],
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            recovery_constraints: Vec::new(),
        })
        .into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
    }

    Ok(())
}

#[test]
fn user_channel_observation_rejects_tampered_exact_artifact_binding_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_evidence_exact_artifacts")?;
    let criterion_id = volicord_types::ids::AcceptanceCriterionId::new(
        active_acceptance_criterion_id(&harness, &task_id)?,
    );
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let (after_first_artifact, first_artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_evidence_exact_first",
    )?;
    let (after_second_artifact, second_artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        after_first_artifact,
        "user_evidence_exact_second",
    )?;
    let target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: criterion_id,
    };
    let artifact_refs = vec![first_artifact_ref, second_artifact_ref];
    let requested = harness.service.request_user_action(
        volicord_types::methods::RequestUserActionRequest {
            envelope: envelope(
                "req_user_action_observation_exact_artifacts",
                Some("idem_user_action_observation_exact_artifacts"),
                false,
                Some(after_second_artifact),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            change_unit_id: Some(ChangeUnitId::new(&change_unit_id)).into(),
            action: volicord_types::schema::UserActionDraft::EvidenceObservation(
                volicord_types::schema::UserActionEvidenceObservationDraft {
                    question: "Do these exact artifacts support the selected target?".to_owned(),
                    context_summary: "The user must inspect both exact candidate artifacts."
                        .to_owned(),
                    target_candidates: vec![target.clone()],
                    artifact_candidate_ids: artifact_refs
                        .iter()
                        .map(|artifact| artifact.artifact_id.clone())
                        .collect(),
                },
            ),
            required_for: vec![volicord_types::values::UserActionRequiredFor::RecordRun],
            expires_at: None.into(),
        },
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id =
        response_record_id(&requested.response_value, "user_action_request_ref");
    let resolved = harness.service.resolve_user_action(
        volicord_types::methods::ResolveUserActionRequest {
            envelope: envelope(
                "req_user_action_observation_exact_artifacts_resolve",
                Some("submission_user_action_observation_exact_artifacts"),
                false,
                None,
                Some(&task_id),
            ),
            user_action_request_id: volicord_types::ids::UserActionRequestId::new(
                user_action_request_id,
            ),
            channel_submission_id: "submission_user_action_observation_exact_artifacts".to_owned(),
            resolution: volicord_types::schema::UserActionResolutionInput::EvidenceObservation {
                target: target.clone(),
                artifact_ids: artifact_refs
                    .iter()
                    .map(|artifact| artifact.artifact_id.clone())
                    .collect(),
                relevance_status: EvidenceRelevanceStatus::Supported,
                summary: "The user assessed both exact candidate artifacts.".to_owned(),
            },
        },
        invocation(OperationCategory::UserOnly),
    )?;
    let after_user_action = resolved.response_value["base"]["state_version"]
        .as_u64()
        .expect("user-action resolution state version");
    let resolution_ref: StateRecordRef =
        serde_json::from_value(resolved.response_value["user_action_resolution_ref"].clone())?;

    let record_request = |suffix: &str, dry_run: bool| {
        let request_id = format!("req_user_evidence_exact_{suffix}");
        let idempotency_key = format!("idem_user_evidence_exact_{suffix}");
        let mut record = record_run_request(
            &request_id,
            &idempotency_key,
            dry_run,
            Some(after_user_action),
            &task_id,
            &change_unit_id,
        );
        record.evidence_updates = vec![EvidenceCoverageUpdate {
            target: target.clone(),
            coverage_state: EvidenceCoverageUpdateState::Supported,
            provenance: None,
            supporting_run_refs: Vec::new(),
            observation_refs: Vec::new(),
            supporting_artifact_refs: artifact_refs.clone(),
            gap_refs: Vec::new(),
        }];
        record.evidence_observations = vec![EvidenceObservationInput {
            target: target.clone(),
            source_kind: EvidenceSourceKind::UserObservation,
            assurance_level: EvidenceAssuranceLevel::UserObserved,
            observed_by_actor_source: None.into(),
            tool_name: None.into(),
            tool_invocation_id: None.into(),
            tool_metadata: JsonObject::new(),
            input_refs: vec![resolution_ref.clone()],
            source_refs: Vec::new(),
            output_artifact_refs: artifact_refs.clone(),
            limitations: Vec::new(),
            observed_at: volicord_types::values::UtcTimestamp::parse("2026-06-18T00:00:00Z")
                .expect("fixture timestamp"),
        }];
        record
    };

    let before_control = harness.counts()?;
    let before_control_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    let control = harness.service.record_run(
        record_request("control", true),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(control.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(harness.counts()?, before_control);
    let after_control_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_control_floor, before_control_floor);

    let conn = harness.conn()?;
    let original_resolution_json: String = conn.query_row(
        "SELECT resolution_json
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
        |row| row.get(0),
    )?;
    let original_resolution: Value = serde_json::from_str(&original_resolution_json)?;
    let mut tampered_resolutions = Vec::new();
    for (suffix, pointer, replacement) in [
        (
            "display_name",
            "/observation/output_artifact_refs/0/display_name",
            json!("tampered-display-name.bin"),
        ),
        (
            "content_type",
            "/observation/output_artifact_refs/0/content_type",
            json!("application/tampered"),
        ),
        (
            "redaction_state",
            "/observation/output_artifact_refs/0/redaction_state",
            json!("redacted"),
        ),
        (
            "producer_identity",
            "/observation/output_artifact_refs/0/created_by_run_ref/record_id",
            json!("run_tampered_exact_output"),
        ),
        (
            "producer_presence",
            "/observation/output_artifact_refs/0/created_by_run_ref",
            Value::Null,
        ),
        (
            "producer_actor",
            "/observation/output_artifact_refs/0/created_by_actor_source",
            json!("local_user"),
        ),
        (
            "storage_ref",
            "/observation/output_artifact_refs/0/storage_ref",
            json!("artifact://tampered-storage-ref"),
        ),
    ] {
        let mut tampered = original_resolution.clone();
        *tampered
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("fixture pointer should exist: {pointer}")) = replacement;
        tampered_resolutions.push((suffix, tampered));
    }
    let mut duplicate = original_resolution.clone();
    let duplicated_ref = duplicate["observation"]["output_artifact_refs"][0].clone();
    duplicate["observation"]["output_artifact_refs"][1] = duplicated_ref;
    tampered_resolutions.push(("duplicate_artifact_id", duplicate));

    for (suffix, tampered) in tampered_resolutions {
        conn.execute(
            "UPDATE user_action_resolutions
                SET resolution_json = ?3
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
            rusqlite::params![
                PROJECT_ID,
                resolution_ref.record_id.as_str(),
                serde_json::to_string(&tampered)?
            ],
        )?;
        for dry_run in [true, false] {
            let branch = if dry_run { "dry" } else { "commit" };
            let before = harness.counts()?;
            let before_floor: String = conn.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            let response = harness.service.record_run(
                record_request(&format!("{suffix}_{branch}"), dry_run),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(
                response.response_value["base"]["response_kind"], "rejected",
                "case {suffix}, dry_run={dry_run}"
            );
            assert_eq!(
                harness.counts()?,
                before,
                "case {suffix}, dry_run={dry_run}"
            );
            let after_floor: String = conn.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            assert_eq!(
                after_floor, before_floor,
                "case {suffix}, dry_run={dry_run}"
            );
        }
    }
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolution_json = ?3
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            resolution_ref.record_id.as_str(),
            original_resolution_json
        ],
    )?;
    Ok(())
}

#[test]
fn cooperative_observation_cannot_be_promoted_by_reuse_or_stale_artifact_ref(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "reused_observation")?;
    let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "reused_observation",
    )?;
    let target = EvidenceTarget::SupplementalClaim {
        evidence_claim_id: volicord_types::ids::EvidenceClaimId::new("claim_reused_observation"),
        statement: "Strong observation can be reused in the current scope.".to_owned(),
    };

    let mut first = record_run_request(
        "req_reused_observation_source",
        "idem_reused_observation_source",
        false,
        Some(after_artifact),
        &task_id,
        &change_unit_id,
    );
    let mut first_artifact_input = existing_artifact_input(
        "artifact_input_reused_observation_source",
        artifact_ref.clone(),
    );
    first_artifact_input.evidence_target = Some(target.clone()).into();
    first.artifact_inputs = vec![first_artifact_input];
    let mut first_update =
        supported_evidence_update("Strong observation can be reused in the current scope.");
    first_update.target = target.clone();
    first.evidence_updates = vec![first_update];
    let source_response = harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    let source_state_version = source_response.response_value["base"]["state_version"]
        .as_u64()
        .expect("source state version should be present");
    let source_observation_id = source_response.response_value["evidence_observations"][0]
        ["observation_id"]
        .as_str()
        .expect("source observation ID should be present")
        .to_owned();
    assert_eq!(
        source_response.response_value["evidence_observations"][0]["source_kind"],
        "agent_report"
    );

    let source_observation_ref = state_ref(
        StateRecordKind::EvidenceObservation,
        &source_observation_id,
        &ProjectId::new(PROJECT_ID),
        Some(&TaskId::new(&task_id)),
        Some(source_state_version),
    );
    let mut caller_artifact_ref = artifact_ref.clone();
    caller_artifact_ref.display_name = "caller-supplied-stale-name.bin".to_owned();
    caller_artifact_ref.content_type = None.into();
    caller_artifact_ref.sha256 = None.into();
    caller_artifact_ref.size_bytes = None.into();
    caller_artifact_ref.integrity_status = ArtifactIntegrityStatus::Corrupt;
    caller_artifact_ref.availability = ArtifactAvailability::Missing;
    caller_artifact_ref.created_by_run_ref = None.into();
    caller_artifact_ref.created_by_actor_source = None.into();
    caller_artifact_ref.storage_ref = None.into();

    let mut reuse = record_run_request(
        "req_reused_observation_current",
        "idem_reused_observation_current",
        false,
        Some(source_state_version),
        &task_id,
        &change_unit_id,
    );
    reuse.evidence_updates = vec![EvidenceCoverageUpdate {
        target: target.clone(),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: vec![source_observation_ref],
        supporting_artifact_refs: vec![caller_artifact_ref],
        gap_refs: Vec::new(),
    }];
    let before_reuse = harness.counts()?;
    let reused_response = harness
        .service
        .record_run(reuse, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        reused_response.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        reused_response.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].observation_refs"
    );
    assert_eq!(harness.counts()?, before_reuse);
    Ok(())
}

#[test]
fn record_run_promotes_zero_byte_artifact_with_real_empty_sha256() -> Result<(), Box<dyn Error>> {
    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_zero_artifact")?;
    let mut stage_request = stage_artifact_request(
        "req_stage_zero_artifact",
        Some("idem_stage_zero_artifact"),
        false,
        Some(2),
        &task_id,
    );
    stage_request.safe_bytes_or_notice = String::new();
    stage_request.expected_sha256 = Some(EMPTY_SHA256.to_owned()).into();
    stage_request.expected_size_bytes = Some(0).into();
    let stage_response = harness
        .service
        .stage_artifact(stage_request, invocation(OperationCategory::AgentWorkflow))?;
    let handle: StagedArtifactHandle =
        serde_json::from_value(stage_response.response_value["staged_artifact_handle"].clone())?;

    let mut request = record_run_request(
        "req_run_zero_artifact",
        "idem_run_zero_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_zero",
        handle,
        Some("empty_report"),
        Some("Zero-byte artifact was registered."),
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present");
    let artifact_row = persistent_artifact_row(&harness, artifact_id)?;

    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["sha256"],
        EMPTY_SHA256
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["size_bytes"],
        0
    );
    assert_eq!(artifact_row.integrity_status, "verified");
    assert_eq!(artifact_row.sha256.as_deref(), Some(EMPTY_SHA256));
    assert_eq!(artifact_row.size_bytes, Some(0));
    Ok(())
}

#[test]
fn corrupt_artifact_blocks_evidence_and_close() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "corrupt_evidence_artifact")?;
    let acceptance_criterion_id = active_acceptance_criterion_id(&harness, &task_id)?;
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "corrupt_evidence_artifact", 2)?;

    let mut request = record_run_request(
        "req_run_corrupt_evidence_artifact",
        "idem_run_corrupt_evidence_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let mut artifact_input = artifact_input_for_handle(
        "artifact_input_corrupt",
        handle,
        Some("validation_report"),
        Some("Corrupt integrity evidence."),
    );
    artifact_input.evidence_target = Some(EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: volicord_types::ids::AcceptanceCriterionId::new(
            &acceptance_criterion_id,
        ),
    })
    .into();
    request.artifact_inputs = vec![artifact_input];
    request.evidence_updates = vec![evidence_update_for_acceptance_criterion(
        supported_evidence_update("Corrupt integrity evidence."),
        &volicord_types::ids::AcceptanceCriterionId::new(acceptance_criterion_id),
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Corrupt integrity evidence.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();

    set_artifact_integrity(&harness, &artifact_id, "corrupt", None, None, None)?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_corrupt_evidence_artifact",
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
    let artifact_ref = &status.response_value["evidence_summary"]["coverage_items"][0]
        ["supporting_artifact_refs"][0];

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "stale"
    );
    assert_eq!(artifact_ref["availability"], "integrity_failed");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert!(artifact_ref["content_type"].is_null());
    assert!(artifact_ref["sha256"].is_null());
    assert!(artifact_ref["size_bytes"].is_null());
    assert_close_blocker(&status.response_value, "artifact_unavailable");

    let check = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_close_corrupt_evidence_artifact",
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
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn corrupt_artifact_is_not_linkable_as_existing_artifact() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "corrupt_artifact")?;
    let (state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "corrupt")?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    let before = harness.counts()?;
    set_artifact_integrity(
        &harness,
        &artifact_id,
        "corrupt",
        artifact_ref.content_type.as_ref().map(String::as_str),
        artifact_ref.sha256.as_ref().map(String::as_str),
        artifact_ref.size_bytes.as_ref().copied(),
    )?;

    let mut request = record_run_request(
        "req_run_corrupt_existing",
        "idem_run_corrupt_existing",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_corrupt_existing",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "ARTIFACT_MISSING"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn verified_existing_artifact_ref_missing_integrity_fact_is_rejected() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "missing_ref_fact")?;
    let (state_version, mut artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "missing_ref")?;
    artifact_ref.sha256 = RequiredNullable::null();
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_existing_ref_fact",
        "idem_run_missing_existing_ref_fact",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_missing_existing_ref_fact",
        artifact_ref,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn missing_persistent_artifact_body_blocks_evidence_and_close_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "missing_body")?;
    let before_counts = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, fixture.artifact_id())?;

    fs::remove_file(&fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "stale"
    );
    assert_eq!(artifact_ref["availability"], "missing");
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&status, &harness.runtime_home_path);

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&check, &harness.runtime_home_path);
    assert_eq!(harness.counts()?, before_counts);
    assert_eq!(
        persistent_artifact_row(&harness, fixture.artifact_id())?,
        before_row
    );
    Ok(())
}

#[test]
fn changed_persistent_artifact_body_blocks_evidence_and_close_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "changed_body")?;
    let before_counts = harness.counts()?;
    let before_row = persistent_artifact_row(&harness, fixture.artifact_id())?;

    fs::write(&fixture.body_path, b"{\"fixture\":\"changed\"}")?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(
        status.response_value["evidence_summary"]["status"],
        "insufficient"
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "stale"
    );
    assert_eq!(artifact_ref["availability"], "integrity_failed");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert_close_blocker(&status.response_value, "artifact_unavailable");

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_eq!(harness.counts()?, before_counts);
    assert_eq!(
        persistent_artifact_row(&harness, fixture.artifact_id())?,
        before_row
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_escape_persistent_artifact_body_is_unusable_without_path_leak(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "symlink_escape")?;
    let before_counts = harness.counts()?;
    let outside_path = harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join("outside-artifact-store.json");
    fs::write(&outside_path, b"{\"fixture\":\"symlink_escape\"}")?;
    fs::remove_file(&fixture.body_path)?;
    std::os::unix::fs::symlink(&outside_path, &fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(artifact_ref["availability"], "unusable");
    assert_eq!(artifact_ref["integrity_status"], "corrupt");
    assert_close_blocker(&status.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&status, &harness.runtime_home_path);

    let check = close_check(&harness, &fixture.task_id)?;
    assert_close_blocker(&check.response_value, "artifact_unavailable");
    assert_public_response_has_no_internal_leak(&check, &harness.runtime_home_path);
    assert_eq!(harness.counts()?, before_counts);
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_within_artifact_store_keeps_persistent_artifact_usable() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let fixture = current_artifact_evidence_and_close_fixture(&harness, "symlink_inside")?;
    let original_bytes = fs::read(&fixture.body_path)?;
    let inside_target = fixture
        .body_path
        .parent()
        .expect("artifact body has parent")
        .join("symlink-inside-target.json");
    fs::write(&inside_target, original_bytes)?;
    fs::remove_file(&fixture.body_path)?;
    std::os::unix::fs::symlink(&inside_target, &fixture.body_path)?;

    let status = status_with_evidence_and_close(&harness, &fixture.task_id)?;
    let artifact_ref = status_evidence_artifact_ref(&status.response_value);

    assert_eq!(artifact_ref["availability"], "available");
    assert_eq!(artifact_ref["integrity_status"], "verified");
    assert_no_close_blocker(&status.response_value, "artifact_unavailable");
    Ok(())
}

#[test]
fn record_run_corrupt_staged_artifact_metadata_rejects_without_effect() -> Result<(), Box<dyn Error>>
{
    for (suffix, artifact_json) in [
        ("malformed", corrupt_owner_json()),
        ("non_object", "[]"),
        ("missing_display_name", "{}"),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_stage_metadata_{suffix}"))?;
        let handle =
            stage_artifact_for_record_run(&harness, &task_id, &format!("metadata_{suffix}"), 2)?;
        let handle_id = handle.handle_id.as_str().to_owned();
        set_artifact_staging_artifact_json(&harness, &handle_id, artifact_json)?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_run_stage_metadata_{suffix}"),
            &format!("idem_run_stage_metadata_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            &format!("artifact_input_metadata_{suffix}"),
            handle,
            None,
            None,
        )];
        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_owner_state_rejection(
            &response,
            "artifact_staging",
            &handle_id,
            "artifact_json",
            &harness.runtime_home_path,
        );
        assert_eq!(harness.counts()?, before, "case {suffix}");
        assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    }
    Ok(())
}

#[test]
fn record_run_staged_artifact_actor_source_mismatch_rejects_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_source")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_source", 2)?;
    handle.created_by_actor_source = ActorSource::agent_connection("forged_connection");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_source",
        "idem_run_stage_source",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_source",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_actor_source_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_expired_staged_artifact_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_expired")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_expired", 2)?;
    expire_staged_artifact(&harness, handle.handle_id.as_str())?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_expired",
        "idem_run_stage_expired",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_expired",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_staged_artifact_uses_semantic_expiry_boundary() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_boundary")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_boundary", 2)?;
    clock.advance(Duration::seconds(24 * 60 * 60 - 1));

    let mut request = record_run_request(
        "req_run_stage_boundary_before",
        "idem_run_stage_boundary_before",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_boundary_before",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");

    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock.clone());
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_boundary_exact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_boundary_exact", 2)?;
    clock.advance(Duration::hours(24));
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_boundary_exact",
        "idem_run_stage_boundary_exact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_boundary_exact",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_staged_artifact_accepts_equivalent_offset_expiration() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_offset")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_offset", 2)?;
    handle.expires_at = volicord_types::values::UtcTimestamp::parse("2026-06-19T09:00:00+09:00")?;

    let mut request = record_run_request(
        "req_run_stage_offset",
        "idem_run_stage_offset",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_offset",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    Ok(())
}

#[test]
fn record_run_invalid_stored_staged_artifact_expiration_is_corrupt_state(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T00:00:00Z");
    harness.use_clock(clock);
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_stage_bad_expires")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_bad_expires", 2)?;
    set_staged_artifact_expires_at(&harness, handle.handle_id.as_str(), "tomorrow")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_bad_expires",
        "idem_run_stage_bad_expires",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_bad_expires",
        handle.clone(),
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_owner_state_value_rejection(
        &response,
        "artifact_staging",
        handle.handle_id.as_str(),
        "expires_at",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        artifact_staging_status(&harness, handle.handle_id.as_str())?,
        "staged"
    );
    Ok(())
}

#[test]
fn record_run_rejects_future_reversed_and_out_of_range_staging_windows_without_effect(
) -> Result<(), Box<dyn Error>> {
    for (suffix, created_at, expires_at) in [
        (
            "future_created",
            "2026-06-18T00:00:01Z",
            "2026-06-19T00:00:00Z",
        ),
        (
            "reversed_window",
            "2026-06-19T00:00:00Z",
            "2026-06-18T00:00:00Z",
        ),
        (
            "out_of_range_created",
            "9999-12-31T23:59:59-23:59",
            "9999-12-31T23:59:59Z",
        ),
    ] {
        let mut harness = MethodHarness::new()?;
        harness.use_clock(ManualClock::at("2026-06-18T00:00:00Z"));
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_stage_{suffix}"))?;
        let handle = stage_artifact_for_record_run(&harness, &task_id, suffix, 2)?;
        set_staged_artifact_window(&harness, handle.handle_id.as_str(), created_at, expires_at)?;
        let before = harness.counts()?;
        let before_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;

        let mut request = record_run_request(
            &format!("req_run_stage_{suffix}"),
            &format!("idem_run_stage_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            &format!("artifact_input_{suffix}"),
            handle.clone(),
            None,
            None,
        )];
        let result = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow));
        match suffix {
            "future_created" => {
                assert!(matches!(result, Err(CorePipelineError::Invariant { .. })));
            }
            "reversed_window" | "out_of_range_created" => {
                let response = result?;
                assert_store_rejection(&response, "PERSISTED_DATA_CORRUPT", "corrupt_stored_value");
            }
            _ => unreachable!("closed fixture cases"),
        }
        assert_eq!(harness.counts()?, before, "case {suffix}");
        assert_eq!(
            artifact_staging_status(&harness, handle.handle_id.as_str())?,
            "staged",
            "case {suffix}"
        );
        let after_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor, "case {suffix}");
    }
    Ok(())
}

#[test]
fn record_run_body_checksum_mismatch_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_body_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    fs::write(
        staged_artifact_body_path(&harness, &handle_id)?,
        vec![b'x'; handle.size_bytes as usize],
    )?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_sha",
        "idem_run_body_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_sha",
        handle,
        Some("validation_report"),
        Some("Tampered body should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Tampered body should not promote.",
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Tampered body should not promote.",
        Vec::new(),
    ))
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_body_size_mismatch_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_body_size")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_size", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    fs::write(
        staged_artifact_body_path(&harness, &handle_id)?,
        vec![b'x'; handle.size_bytes as usize + 1],
    )?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_size",
        "idem_run_body_size",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_size",
        handle,
        Some("validation_report"),
        Some("Resized body should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Resized body should not promote.",
    )];
    request.close_assessment = Some(close_assessment_with_risks(
        "Resized body should not promote.",
        Vec::new(),
    ))
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_staging_path_outside_artifact_store_rolls_back_all_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_body_path_outside")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_body_path_outside", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    set_artifact_staging_tmp_path(&harness, &handle_id, "tmp/not-under-artifacts.txt")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_body_path_outside",
        "idem_run_body_path_outside",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_body_path_outside",
        handle,
        Some("validation_report"),
        Some("Invalid staging path should not promote."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Invalid staging path should not promote.",
    )];

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "PERSISTED_DATA_CORRUPT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}
