use super::*;

fn lowercase_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn workspace_context(seed: char) -> crate::pipeline::GitWorkspaceContext {
    crate::pipeline::GitWorkspaceContext {
        git_common_dir: "/tmp/volicord-evidence-capture/.git".to_owned(),
        worktree_id: format!("sha256:{}", seed.to_string().repeat(64)),
        branch_ref: Some("refs/heads/evidence-capture".to_owned()),
        head_sha: Some(seed.to_string().repeat(40)),
        workspace_fingerprint: format!(
            "sha256:{}",
            seed.to_ascii_uppercase().to_string().repeat(64)
        ),
    }
}

fn create_workspace_bound_task(
    harness: &MethodHarness,
    suffix: &str,
) -> Result<(String, String, String, crate::pipeline::GitWorkspaceContext), Box<dyn Error>> {
    let intake = harness.service.intake(
        intake_request(
            &format!("req_capture_{suffix}_task"),
            &format!("idem_capture_{suffix}_task"),
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let workspace = workspace_context('a');
    let scope = harness.service.update_scope(
        update_scope_request(
            &format!("req_capture_{suffix}_scope"),
            &format!("idem_capture_{suffix}_scope"),
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bind evidence capture to one workspace.",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let change_unit_id = response_record_id(&scope.response_value, "change_unit_ref");
    let criterion_id = active_acceptance_criterion_id(harness, &task_id)?;
    Ok((task_id, change_unit_id, criterion_id, workspace))
}

fn capture_request(
    request_id: &str,
    idempotency_key: Option<&str>,
    dry_run: bool,
    expected_state_version: u64,
    basis: (&str, &str, &str),
    capture: EvidenceCaptureSpec,
) -> PrepareEvidenceCaptureRequest {
    let (task_id, change_unit_id, criterion_id) = basis;
    PrepareEvidenceCaptureRequest {
        envelope: envelope(
            request_id,
            idempotency_key,
            dry_run,
            Some(expected_state_version),
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: ChangeUnitId::new(change_unit_id),
        baseline_ref: BaselineRef::new("baseline_test"),
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(criterion_id),
        },
        capture,
    }
}

fn fulfill_command_receipt(
    harness: &MethodHarness,
    intent_id: &str,
    exit_code: i32,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    let mut store =
        CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let intent = store
        .evidence_capture_intent_record(intent_id)?
        .expect("capture intent should exist");
    let observed_at = UtcTimestamp::from_datetime(
        *UtcTimestamp::parse(&intent.created_at)?.as_datetime() + Duration::minutes(1),
    )
    .to_canonical_string();
    let observed_outcome = json!({
        "exit_code": exit_code,
        "stdout_sha256": lowercase_sha256(b"fixture stdout"),
        "stdout_size_bytes": 14,
        "stderr_sha256": lowercase_sha256(b""),
        "stderr_size_bytes": 0
    });
    let result_sha256 = lowercase_sha256(&volicord_types::canonical_json_bytes(&observed_outcome)?);
    let source = json!({
        "connection_id": CONNECTION_ID,
        "session_id": null,
        "guard_installation_id": null,
        "guard_event_ids": [],
        "watch_observation_refs": [],
        "host_invocation_id": format!("host_invocation_command_{suffix}")
    });
    let expected_outcome: Value = serde_json::from_str(&intent.expected_outcome_json)?;
    let safe_receipt = json!({
        "schema_version": "volicord.evidence_capture_receipt.v1",
        "capture_kind": "verified_command_execution",
        "capture_intent_id": intent_id,
        "input_sha256": intent.input_sha256,
        "result_sha256": result_sha256,
        "expected_outcome": expected_outcome,
        "observed_outcome": observed_outcome,
        "source": source,
        "complete": true,
        "limitations": [EVIDENCE_CAPTURE_COMMAND_LIMITATION],
        "redaction_state": "redacted",
        "observed_by_actor_source": AGENT_ACTOR_SOURCE,
        "observed_at": observed_at
    });
    store.fulfill_evidence_capture_source(
        volicord_store::evidence_capture::EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: format!("evidence_capture_receipt_{suffix}"),
            evidence_capture_intent_id: intent_id.to_owned(),
            staging_handle_id: format!("staged_capture_receipt_{suffix}"),
            task_id: intent.task_id,
            capture_kind: intent.capture_kind,
            input_sha256: intent.input_sha256,
            result_sha256,
            expected_outcome_json: intent.expected_outcome_json,
            observed_outcome_json: serde_json::to_string(&observed_outcome)?,
            source_refs_json: "[]".to_owned(),
            observed_by_actor_source: AGENT_ACTOR_SOURCE.to_owned(),
            observed_at: observed_at.clone(),
            limitations_json: serde_json::to_string(&json!([EVIDENCE_CAPTURE_COMMAND_LIMITATION]))?,
            safe_receipt_json: serde_json::to_string(&safe_receipt)?,
            created_at: observed_at,
            staging_expires_at: intent.expires_at,
            metadata_json: serde_json::to_string(&json!({"source": source}))?,
        },
    )?;
    Ok(())
}

fn fulfill_registered_source_receipt(
    harness: &MethodHarness,
    intent_id: &str,
    observed_outcome: Value,
    source: Value,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    let mut store =
        CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let intent = store
        .evidence_capture_intent_record(intent_id)?
        .expect("capture intent should exist");
    let observed_at = UtcTimestamp::from_datetime(
        *UtcTimestamp::parse(&intent.created_at)?.as_datetime() + Duration::minutes(1),
    )
    .to_canonical_string();
    let result_sha256 = lowercase_sha256(&volicord_types::canonical_json_bytes(&observed_outcome)?);
    let expected_outcome: Value = serde_json::from_str(&intent.expected_outcome_json)?;
    let capture: EvidenceCaptureSpec = serde_json::from_str(&intent.capture_spec_json)?;
    let limitation = match capture {
        EvidenceCaptureSpec::VerifiedToolInvocation { .. }
        | EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind: volicord_types::ConnectionObservationSourceKind::GuardEvent,
            ..
        } => EVIDENCE_CAPTURE_GUARD_LIMITATION,
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind: volicord_types::ConnectionObservationSourceKind::SessionWatcher,
            ..
        } => EVIDENCE_CAPTURE_WATCHER_LIMITATION,
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => EVIDENCE_CAPTURE_COMMAND_LIMITATION,
    };
    let safe_receipt = json!({
        "schema_version": "volicord.evidence_capture_receipt.v1",
        "capture_kind": intent.capture_kind,
        "capture_intent_id": intent_id,
        "input_sha256": intent.input_sha256,
        "result_sha256": result_sha256,
        "expected_outcome": expected_outcome,
        "observed_outcome": observed_outcome,
        "source": source,
        "complete": true,
        "limitations": [limitation],
        "redaction_state": "redacted",
        "observed_by_actor_source": AGENT_ACTOR_SOURCE,
        "observed_at": observed_at
    });
    store.fulfill_evidence_capture_source(
        volicord_store::evidence_capture::EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: format!("evidence_capture_receipt_{suffix}"),
            evidence_capture_intent_id: intent_id.to_owned(),
            staging_handle_id: format!("staged_capture_receipt_{suffix}"),
            task_id: intent.task_id,
            capture_kind: intent.capture_kind,
            input_sha256: intent.input_sha256,
            result_sha256,
            expected_outcome_json: intent.expected_outcome_json,
            observed_outcome_json: serde_json::to_string(&observed_outcome)?,
            source_refs_json: "[]".to_owned(),
            observed_by_actor_source: AGENT_ACTOR_SOURCE.to_owned(),
            observed_at: observed_at.clone(),
            limitations_json: serde_json::to_string(&json!([limitation]))?,
            safe_receipt_json: serde_json::to_string(&safe_receipt)?,
            created_at: observed_at,
            staging_expires_at: intent.expires_at,
            metadata_json: serde_json::to_string(&json!({"source": source}))?,
        },
    )?;
    Ok(())
}

fn record_run_with_capture(
    task_id: &str,
    change_unit_id: &str,
    criterion_id: &str,
    intent_ref: StateRecordRef,
    suffix: &str,
) -> RecordRunRequest {
    let target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new(criterion_id),
    };
    let mut request = record_run_request(
        &format!("req_capture_record_run_{suffix}"),
        &format!("idem_capture_record_run_{suffix}"),
        false,
        Some(3),
        task_id,
        change_unit_id,
    );
    request.evidence_observations = vec![EvidenceObservationInput {
        target: target.clone(),
        source_kind: EvidenceSourceKind::ExternalTool,
        assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
        observed_by_actor_source: RequiredNullable::null(),
        tool_name: RequiredNullable::null(),
        tool_invocation_id: RequiredNullable::null(),
        tool_metadata: JsonObject::new(),
        input_refs: vec![intent_ref],
        source_refs: Vec::new(),
        output_artifact_refs: Vec::new(),
        limitations: Vec::new(),
        observed_at: UtcTimestamp::parse("2000-01-01T00:00:00Z").expect("timestamp"),
    }];
    request.evidence_updates = vec![EvidenceCoverageUpdate {
        target,
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    request.close_assessment = Some(volicord_types::CloseAssessmentInput {
        result_summary: "Evidence capture result is current.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    request
}

#[test]
fn command_capture_defaults_are_persisted_once_and_replay_is_exact() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "command")?;
    let clock = ManualClock::at("2026-07-13T01:00:00Z");
    harness.use_clock(clock);
    let request = capture_request(
        "req_capture_command",
        Some("idem_capture_command"),
        false,
        2,
        (&task_id, &change_unit_id, &criterion_id),
        EvidenceCaptureSpec::VerifiedCommandExecution {
            command_sha256: "a".repeat(64),
            command_label: "  cargo   test  ".to_owned(),
            expected_exit_code: RequiredNullable::null(),
        },
    );
    let before = harness.counts()?;
    let committed = harness.service.prepare_evidence_capture(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let after = harness.counts()?;

    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(committed.response_value["base"]["state_version"], 3);
    assert_eq!(
        committed.response_value["capture_intent_ref"]["record_kind"],
        "evidence_capture_intent"
    );
    assert_eq!(
        committed.response_value["capture_intent"]["capture"]["expected_exit_code"],
        0
    );
    assert_eq!(
        committed.response_value["capture_intent"]["capture"]["command_label"],
        "cargo   test"
    );
    assert_eq!(
        committed.response_value["capture_intent"]["expected_outcome"],
        json!({"expected_exit_code": 0})
    );
    assert_eq!(
        committed.response_value["capture_intent"]["created_at"],
        "2026-07-13T01:00:00Z"
    );
    assert_eq!(
        committed.response_value["expires_at"],
        "2026-07-13T01:15:00Z"
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(
        after.evidence_capture_intents,
        before.evidence_capture_intents + 1
    );
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(
        after.evidence_capture_receipts,
        before.evidence_capture_receipts
    );
    assert_eq!(after.evidence_producers, before.evidence_producers);

    let intent_id = response_record_id(&committed.response_value, "capture_intent_ref");
    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let row = store
        .evidence_capture_intent_record(&intent_id)?
        .expect("committed intent should be immediately readable");
    assert_eq!(row.capture_kind, "verified_command_execution");
    assert_eq!(row.input_sha256, "a".repeat(64));
    assert_eq!(row.requesting_connection_internal_id, CONNECTION_ID);
    assert_eq!(row.created_at, "2026-07-13T01:00:00Z");
    assert_eq!(row.expires_at, "2026-07-13T01:15:00Z");
    assert_eq!(
        serde_json::from_str::<Value>(&row.capture_spec_json)?,
        committed.response_value["capture_intent"]["capture"]
    );

    let replayed = harness.service.prepare_evidence_capture(
        request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    assert!(replayed.replayed);
    assert_eq!(replayed.response_json, committed.response_json);
    assert_eq!(harness.counts()?, after);
    Ok(())
}

#[test]
fn capture_variants_apply_omission_defaults_from_one_owner() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "variants")?;
    let cases = [
        (
            EvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name: "  cargo-test  ".to_owned(),
                tool_input_sha256: "b".repeat(64),
                expected_success: RequiredNullable::null(),
            },
            "verified_tool_invocation",
            "expected_success",
            json!(true),
        ),
        (
            EvidenceCaptureSpec::RegisteredConnectionObservation {
                source_kind: volicord_types::ConnectionObservationSourceKind::SessionWatcher,
                observation_input_sha256: "c".repeat(64),
                expected_complete: RequiredNullable::null(),
            },
            "registered_connection_observation",
            "expected_complete",
            json!(true),
        ),
    ];
    for (index, (capture, kind, expected_field, expected_value)) in cases.into_iter().enumerate() {
        let expected_state_version = 2 + index as u64;
        let session_id = format!("session_capture_variant_{index}");
        let response = harness.service.prepare_evidence_capture(
            capture_request(
                &format!("req_capture_variant_{index}"),
                Some(&format!("idem_capture_variant_{index}")),
                false,
                expected_state_version,
                (&task_id, &change_unit_id, &criterion_id),
                capture,
            ),
            invocation_with_session(OperationCategory::AgentWorkflow, &session_id)
                .with_git_workspace_context(workspace.clone()),
        )?;
        assert_eq!(
            response.response_value["capture_intent"]["capture"]["capture_kind"],
            kind
        );
        assert_eq!(
            response.response_value["capture_intent"]["expected_outcome"][expected_field],
            expected_value
        );
    }
    Ok(())
}

#[test]
fn invalid_or_unbound_capture_and_dry_run_have_no_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "negative")?;
    let base_request = capture_request(
        "req_capture_negative",
        Some("idem_capture_negative"),
        false,
        2,
        (&task_id, &change_unit_id, &criterion_id),
        EvidenceCaptureSpec::VerifiedCommandExecution {
            command_sha256: "not-a-digest".to_owned(),
            command_label: "cargo test".to_owned(),
            expected_exit_code: RequiredNullable::null(),
        },
    );
    let before = harness.counts()?;
    let malformed = harness.service.prepare_evidence_capture(
        base_request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(
        malformed.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);

    let missing_workspace = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_missing_workspace",
            Some("idem_capture_missing_workspace"),
            false,
            2,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name: "fixture".to_owned(),
                tool_input_sha256: "d".repeat(64),
                expected_success: RequiredNullable::null(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        missing_workspace.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);

    let missing_session = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_missing_session",
            Some("idem_capture_missing_session"),
            false,
            2,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::RegisteredConnectionObservation {
                source_kind: volicord_types::ConnectionObservationSourceKind::GuardEvent,
                observation_input_sha256: "f".repeat(64),
                expected_complete: RequiredNullable::null(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(
        missing_session.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        missing_session.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);

    let dry_run = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_dry_run",
            None,
            true,
            2,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::RegisteredConnectionObservation {
                source_kind: volicord_types::ConnectionObservationSourceKind::GuardEvent,
                observation_input_sha256: "e".repeat(64),
                expected_complete: RequiredNullable::null(),
            },
        ),
        invocation_with_session(OperationCategory::AgentWorkflow, "session_capture_dry_run")
            .with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert!(dry_run.response_value.get("capture_intent_ref").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_finalizes_command_provenance_without_self_approving_criterion(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "finalize")?;
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let clock = ManualClock::at("2026-07-13T01:00:00Z");
    harness.use_clock(clock.clone());
    let prepared = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_finalize",
            Some("idem_capture_finalize"),
            false,
            2,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256: "f".repeat(64),
                command_label: "true".to_owned(),
                expected_exit_code: RequiredNullable::null(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let intent_ref: StateRecordRef =
        serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;
    fulfill_command_receipt(&harness, intent_ref.record_id.as_str(), 0, "finalize")?;
    clock.advance(Duration::minutes(2));
    let before = harness.counts()?;
    let response = harness.service.record_run(
        record_run_with_capture(
            &task_id,
            &change_unit_id,
            &criterion_id,
            intent_ref.clone(),
            "finalize",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let after = harness.counts()?;

    assert_eq!(
        response.response_value["base"]["response_kind"],
        "result",
        "{}",
        serde_json::to_string_pretty(&response.response_value)?
    );
    let observation = &response.response_value["evidence_observations"][0];
    assert_eq!(observation["source_kind"], "external_tool");
    assert_eq!(observation["assurance_level"], "external_tool_result");
    assert_eq!(
        observation["producer_anchor"]["producer_kind"],
        "verified_command_execution"
    );
    assert_eq!(
        observation["producer_anchor"]["producer_ref"]["record_kind"],
        "evidence_producer"
    );
    assert_eq!(observation["relevance_assessment"]["status"], "unassessed");
    assert_eq!(
        observation["relevance_assessment"]["assessment_ref"]["record_id"],
        intent_ref.record_id.as_str()
    );
    assert_eq!(observation["observed_at"], "2026-07-13T01:01:00Z");
    assert_eq!(observation["tool_name"], "volicord.command_runner");
    assert_eq!(
        observation["output_artifact_refs"],
        response.response_value["registered_artifacts"]
    );
    assert_eq!(
        response.response_value["state"]["evidence_gate"]["state"],
        "partial",
        "{}",
        serde_json::to_string_pretty(&response.response_value)?
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "supported"
    );
    assert_close_blocker(
        &response.response_value["state"],
        "evidence_claim_unsupported",
    );
    assert_eq!(after.evidence_producers, before.evidence_producers + 1);
    assert_eq!(
        after.evidence_observations,
        before.evidence_observations + 1
    );
    assert_eq!(after.artifacts, before.artifacts + 1);
    assert_eq!(after.state_version, before.state_version + 1);

    let producer_id = observation["producer_anchor"]["producer_ref"]["record_id"]
        .as_str()
        .expect("producer id");
    let observation_id = observation["observation_id"]
        .as_str()
        .expect("observation id");
    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let producer_row = store
        .evidence_producer_record(producer_id)?
        .expect("producer should be immediately readable");
    let producer: EvidenceProducer = serde_json::from_str(&producer_row.canonical_producer_json)?;
    assert_eq!(
        response.response_value["evidence_producers"],
        json!([producer.clone()]),
        "the public method result must expose the exact producer body committed in the same plan"
    );
    assert_eq!(producer.evidence_producer_id.as_str(), producer_id);
    assert_eq!(
        producer.capture_intent_id.as_str(),
        intent_ref.record_id.as_str()
    );
    assert_eq!(
        producer.receipt_artifact_refs,
        serde_json::from_value::<Vec<ArtifactRef>>(observation["output_artifact_refs"].clone())?
    );
    let observation_record = store
        .evidence_observation_record(observation_id)?
        .expect("observation should be immediately readable");
    assert_eq!(
        super::super::record_run::stored_evidence_observation_provenance_class(
            &store,
            &observation_record,
            &super::super::record_run::StoredEvidenceProvenanceBasis {
                project_id: &ProjectId::new(PROJECT_ID),
                task_id: &TaskId::new(&task_id),
                change_unit_id: &change_unit_id,
                scope_revision: 1,
                baseline_ref: Some("baseline_test"),
                target: &EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(&criterion_id),
                },
            },
        )?,
        EvidenceProvenanceClass::Strong
    );
    drop(store);

    let original_producer_json = producer_row.canonical_producer_json.clone();
    let mut tampered_producer = producer;
    tampered_producer
        .limitations
        .push("unregistered_extra_limitation".to_owned());
    harness.conn()?.execute(
        "UPDATE evidence_producers
            SET canonical_producer_json = ?3
          WHERE project_id = ?1 AND evidence_producer_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            producer_id,
            volicord_types::canonical_json_string(&tampered_producer)?
        ],
    )?;
    let tampered_store =
        CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    assert_ne!(
        super::super::record_run::stored_evidence_observation_provenance_class(
            &tampered_store,
            &observation_record,
            &super::super::record_run::StoredEvidenceProvenanceBasis {
                project_id: &ProjectId::new(PROJECT_ID),
                task_id: &TaskId::new(&task_id),
                change_unit_id: &change_unit_id,
                scope_revision: 1,
                baseline_ref: Some("baseline_test"),
                target: &EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(&criterion_id),
                },
            },
        )?,
        EvidenceProvenanceClass::Strong
    );
    drop(tampered_store);
    harness.conn()?.execute(
        "UPDATE evidence_producers
            SET canonical_producer_json = ?3
          WHERE project_id = ?1 AND evidence_producer_id = ?2",
        rusqlite::params![PROJECT_ID, producer_id, original_producer_json],
    )?;

    harness.conn()?.execute(
        "UPDATE evidence_producers
            SET producer_kind = 'verified_tool_invocation'
          WHERE project_id = ?1 AND evidence_producer_id = ?2",
        rusqlite::params![PROJECT_ID, producer_id],
    )?;
    let relabeled_store =
        CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    assert_ne!(
        super::super::record_run::stored_evidence_observation_provenance_class(
            &relabeled_store,
            &observation_record,
            &super::super::record_run::StoredEvidenceProvenanceBasis {
                project_id: &ProjectId::new(PROJECT_ID),
                task_id: &TaskId::new(&task_id),
                change_unit_id: &change_unit_id,
                scope_revision: 1,
                baseline_ref: Some("baseline_test"),
                target: &EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(&criterion_id),
                },
            },
        )?,
        EvidenceProvenanceClass::Strong
    );
    drop(relabeled_store);
    harness.conn()?.execute(
        "UPDATE evidence_producers
            SET producer_kind = 'verified_command_execution'
          WHERE project_id = ?1 AND evidence_producer_id = ?2",
        rusqlite::params![PROJECT_ID, producer_id],
    )?;

    let original_tool_metadata_json = observation_record.tool_metadata_json.clone();
    let mut tampered_tool_metadata = serde_json::from_str::<Value>(&original_tool_metadata_json)?;
    tampered_tool_metadata["raw_tool_response"] = Value::String("must not persist".to_owned());
    harness.conn()?.execute(
        "UPDATE evidence_observations
            SET tool_metadata_json = ?3
          WHERE project_id = ?1 AND evidence_observation_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            observation_id,
            volicord_types::canonical_json_string(&tampered_tool_metadata)?
        ],
    )?;
    let tampered_observation_store =
        CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let tampered_observation_record = tampered_observation_store
        .evidence_observation_record(observation_id)?
        .expect("tampered observation should remain readable");
    assert_ne!(
        super::super::record_run::stored_evidence_observation_provenance_class(
            &tampered_observation_store,
            &tampered_observation_record,
            &super::super::record_run::StoredEvidenceProvenanceBasis {
                project_id: &ProjectId::new(PROJECT_ID),
                task_id: &TaskId::new(&task_id),
                change_unit_id: &change_unit_id,
                scope_revision: 1,
                baseline_ref: Some("baseline_test"),
                target: &EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(&criterion_id),
                },
            },
        )?,
        EvidenceProvenanceClass::Strong
    );
    drop(tampered_observation_store);
    harness.conn()?.execute(
        "UPDATE evidence_observations
            SET tool_metadata_json = ?3
          WHERE project_id = ?1 AND evidence_observation_id = ?2",
        rusqlite::params![PROJECT_ID, observation_id, original_tool_metadata_json],
    )?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_capture_finalize_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read).with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(status.response_value["evidence_gate"]["state"], "partial");
    assert_close_blocker(&status.response_value, "evidence_claim_unsupported");

    let observation_ref: StateRecordRef = serde_json::from_value(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            .clone(),
    )?;
    let before_relevance_reuse = harness.counts()?;
    let mut relevance_reuse = record_run_request(
        "req_capture_reuse_unassessed",
        "idem_capture_reuse_unassessed",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    relevance_reuse.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(&criterion_id),
        },
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: vec![observation_ref],
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let rejected_relevance_reuse = harness.service.record_run(
        relevance_reuse,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(
        rejected_relevance_reuse.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        rejected_relevance_reuse.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before_relevance_reuse);

    let before_reuse = harness.counts()?;
    let mut reuse_request = record_run_with_capture(
        &task_id,
        &change_unit_id,
        &criterion_id,
        intent_ref,
        "reuse_same_intent",
    );
    reuse_request.envelope.expected_state_version = Some(4).into();
    let reuse = harness.service.record_run(
        reuse_request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace),
    )?;
    assert_eq!(reuse.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        reuse.response_value["errors"][0]["code"],
        "EVIDENCE_INSUFFICIENT"
    );
    assert_eq!(harness.counts()?, before_reuse);
    Ok(())
}

#[test]
fn contradicted_and_corrupt_capture_paths_fail_closed_without_false_support(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "contradicted")?;
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let clock = ManualClock::at("2026-07-13T01:00:00Z");
    harness.use_clock(clock.clone());
    let prepared = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_contradicted",
            Some("idem_capture_contradicted"),
            false,
            2,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256: "1".repeat(64),
                command_label: "cargo test".to_owned(),
                expected_exit_code: RequiredNullable::null(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let intent_ref: StateRecordRef =
        serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;
    fulfill_command_receipt(&harness, intent_ref.record_id.as_str(), 1, "contradicted")?;
    clock.advance(Duration::minutes(2));
    let contradicted = harness.service.record_run(
        record_run_with_capture(
            &task_id,
            &change_unit_id,
            &criterion_id,
            intent_ref,
            "contradicted",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(
        contradicted.response_value["evidence_observations"][0]["relevance_assessment"]["status"],
        "contradicted",
        "{}",
        serde_json::to_string_pretty(&contradicted.response_value)?
    );
    assert_ne!(
        contradicted.response_value["state"]["evidence_gate"]["state"],
        "sufficient"
    );

    let second_prepared = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_corrupt",
            Some("idem_capture_corrupt"),
            false,
            4,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256: "2".repeat(64),
                command_label: "cargo check".to_owned(),
                expected_exit_code: RequiredNullable::null(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let second_ref: StateRecordRef =
        serde_json::from_value(second_prepared.response_value["capture_intent_ref"].clone())?;
    fulfill_command_receipt(&harness, second_ref.record_id.as_str(), 0, "corrupt")?;
    let conn = harness.conn()?;
    let stored_safe_receipt: String = conn.query_row(
        "SELECT safe_receipt_json FROM evidence_capture_receipts
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
        rusqlite::params![PROJECT_ID, second_ref.record_id.as_str()],
        |row| row.get(0),
    )?;
    let mut corrupted_safe_receipt = serde_json::from_str::<Value>(&stored_safe_receipt)?;
    corrupted_safe_receipt["observed_outcome"]["stdout"] =
        Value::String("raw output must not persist".to_owned());
    let corrupted_result_sha256 = lowercase_sha256(&volicord_types::canonical_json_bytes(
        &corrupted_safe_receipt["observed_outcome"],
    )?);
    corrupted_safe_receipt["result_sha256"] = Value::String(corrupted_result_sha256.clone());
    let corrupted_observed_outcome_json =
        volicord_types::canonical_json_string(&corrupted_safe_receipt["observed_outcome"])?;
    let corrupted_safe_receipt = volicord_types::canonical_json_string(&corrupted_safe_receipt)?;
    let corrupted_safe_receipt_sha256 = lowercase_sha256(corrupted_safe_receipt.as_bytes());
    let corrupted_safe_receipt_size = i64::try_from(corrupted_safe_receipt.len())?;
    conn.execute(
        "UPDATE evidence_capture_receipts
            SET result_sha256 = ?3,
                observed_outcome_json = ?4,
                safe_receipt_json = ?5,
                safe_receipt_sha256 = ?6,
                safe_receipt_size_bytes = ?7
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            second_ref.record_id.as_str(),
            corrupted_result_sha256,
            corrupted_observed_outcome_json,
            corrupted_safe_receipt,
            corrupted_safe_receipt_sha256,
            corrupted_safe_receipt_size
        ],
    )?;
    let before_corrupt = harness.counts()?;
    let mut corrupt_request = record_run_with_capture(
        &task_id,
        &change_unit_id,
        &criterion_id,
        second_ref.clone(),
        "corrupt",
    );
    corrupt_request.envelope.expected_state_version = Some(5).into();
    let corrupt = harness.service.record_run(
        corrupt_request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    assert_eq!(corrupt.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        corrupt.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before_corrupt);

    let mut oversized_safe_receipt = serde_json::from_str::<Value>(&stored_safe_receipt)?;
    oversized_safe_receipt["source"]["host_invocation_id"] =
        Value::String("x".repeat(MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES + 1));
    let oversized_metadata = volicord_types::canonical_json_string(&json!({
        "source": oversized_safe_receipt["source"].clone()
    }))?;
    let original_observed_outcome_json =
        volicord_types::canonical_json_string(&oversized_safe_receipt["observed_outcome"])?;
    let original_result_sha256 = oversized_safe_receipt["result_sha256"]
        .as_str()
        .expect("fixture result digest")
        .to_owned();
    let oversized_safe_receipt = volicord_types::canonical_json_string(&oversized_safe_receipt)?;
    let oversized_safe_receipt_sha256 = lowercase_sha256(oversized_safe_receipt.as_bytes());
    let oversized_safe_receipt_size = i64::try_from(oversized_safe_receipt.len())?;
    conn.execute(
        "UPDATE evidence_capture_receipts
            SET result_sha256 = ?3,
                observed_outcome_json = ?4,
                safe_receipt_json = ?5,
                safe_receipt_sha256 = ?6,
                safe_receipt_size_bytes = ?7,
                metadata_json = ?8
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            second_ref.record_id.as_str(),
            original_result_sha256,
            original_observed_outcome_json,
            oversized_safe_receipt,
            oversized_safe_receipt_sha256,
            oversized_safe_receipt_size,
            oversized_metadata
        ],
    )?;
    let mut oversized_request = record_run_with_capture(
        &task_id,
        &change_unit_id,
        &criterion_id,
        second_ref,
        "oversized_reread",
    );
    oversized_request.envelope.expected_state_version = Some(5).into();
    let oversized = harness.service.record_run(
        oversized_request,
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace),
    )?;
    assert_eq!(
        oversized.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        oversized.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before_corrupt);
    Ok(())
}

#[test]
fn missing_source_claim_rejects_finalization_without_core_effects() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id, criterion_id, workspace) =
        create_workspace_bound_task(&harness, "missing_source_claim")?;
    let clock = ManualClock::at("2026-07-13T01:00:00Z");
    harness.use_clock(clock.clone());
    let prepared = harness.service.prepare_evidence_capture(
        capture_request(
            "req_capture_missing_source_claim",
            Some("idem_capture_missing_source_claim"),
            false,
            2,
            (&task_id, &change_unit_id, &criterion_id),
            EvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256: "3".repeat(64),
                command_label: "cargo test".to_owned(),
                expected_exit_code: RequiredNullable::null(),
            },
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace.clone()),
    )?;
    let intent_ref: StateRecordRef =
        serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;
    fulfill_command_receipt(
        &harness,
        intent_ref.record_id.as_str(),
        0,
        "missing_source_claim",
    )?;
    harness.conn()?.execute(
        "DELETE FROM evidence_capture_source_claims
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
        rusqlite::params![PROJECT_ID, intent_ref.record_id.as_str()],
    )?;
    clock.advance(Duration::minutes(2));
    let before = harness.counts()?;
    let response = harness.service.record_run(
        record_run_with_capture(
            &task_id,
            &change_unit_id,
            &criterion_id,
            intent_ref,
            "missing_source_claim",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_future_and_expired_capture_receipts_without_effects(
) -> Result<(), Box<dyn Error>> {
    for (suffix, advance) in [
        ("future_receipt", Duration::zero()),
        ("expired_intent", Duration::minutes(15)),
    ] {
        let mut harness = MethodHarness::new()?;
        let (task_id, change_unit_id, criterion_id, workspace) =
            create_workspace_bound_task(&harness, suffix)?;
        let clock = ManualClock::at("2026-07-13T01:00:00Z");
        harness.use_clock(clock.clone());
        let prepared = harness.service.prepare_evidence_capture(
            capture_request(
                &format!("req_capture_{suffix}"),
                Some(&format!("idem_capture_{suffix}")),
                false,
                2,
                (&task_id, &change_unit_id, &criterion_id),
                EvidenceCaptureSpec::VerifiedCommandExecution {
                    command_sha256: "9".repeat(64),
                    command_label: "freshness boundary".to_owned(),
                    expected_exit_code: RequiredNullable::null(),
                },
            ),
            invocation(OperationCategory::AgentWorkflow)
                .with_git_workspace_context(workspace.clone()),
        )?;
        let intent_ref: StateRecordRef =
            serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;
        fulfill_command_receipt(&harness, intent_ref.record_id.as_str(), 0, suffix)?;
        clock.advance(advance);
        let before = harness.counts()?;
        let response = harness.service.record_run(
            record_run_with_capture(&task_id, &change_unit_id, &criterion_id, intent_ref, suffix),
            invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(workspace),
        )?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "EVIDENCE_INSUFFICIENT"
        );
        assert_eq!(harness.counts()?, before);
    }
    Ok(())
}

#[test]
fn tool_and_connection_receipts_finalize_to_their_exact_producer_classes(
) -> Result<(), Box<dyn Error>> {
    struct Case {
        suffix: &'static str,
        capture: EvidenceCaptureSpec,
        observed_outcome: Value,
        source: Value,
        source_kind: EvidenceSourceKind,
        assurance_level: EvidenceAssuranceLevel,
        producer_kind: &'static str,
        tool_name: Option<&'static str>,
        tool_invocation_id: Option<&'static str>,
        session_id: &'static str,
    }
    let cases = vec![
        Case {
            suffix: "tool",
            capture: EvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name: "fixture_tool".to_owned(),
                tool_input_sha256: "3".repeat(64),
                expected_success: RequiredNullable::null(),
            },
            observed_outcome: json!({
                "success": true,
                "exit_code": null,
                "tool_result_sha256": "4".repeat(64),
                "tool_result_size_bytes": 12
            }),
            source: json!({
                "connection_id": CONNECTION_ID,
                "session_id": "session_tool",
                "guard_installation_id": "guard_installation_tool",
                "guard_event_ids": ["guard_pre_tool", "guard_post_tool"],
                "watch_observation_refs": [],
                "host_invocation_id": "host_invocation_tool"
            }),
            source_kind: EvidenceSourceKind::ExternalTool,
            assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
            producer_kind: "verified_tool_invocation",
            tool_name: Some("fixture_tool"),
            tool_invocation_id: Some("host_invocation_tool"),
            session_id: "session_tool",
        },
        Case {
            suffix: "connection",
            capture: EvidenceCaptureSpec::RegisteredConnectionObservation {
                source_kind: volicord_types::ConnectionObservationSourceKind::SessionWatcher,
                observation_input_sha256: "5".repeat(64),
                expected_complete: RequiredNullable::null(),
            },
            observed_outcome: json!({
                "complete": true,
                "snapshot_algorithm": WATCH_SNAPSHOT_ALGORITHM,
                "snapshot_digest": "6".repeat(64),
                "observation_sha256": "5".repeat(64)
            }),
            source: json!({
                "connection_id": CONNECTION_ID,
                "session_id": "session_watch",
                "guard_installation_id": null,
                "guard_event_ids": [],
                "watch_observation_refs": ["watch_observation_1"],
                "host_invocation_id": null
            }),
            source_kind: EvidenceSourceKind::ConnectionObservation,
            assurance_level: EvidenceAssuranceLevel::RegisteredConnectionObserved,
            producer_kind: "registered_connection_observation",
            tool_name: None,
            tool_invocation_id: None,
            session_id: "session_watch",
        },
    ];

    for case in cases {
        let mut harness = MethodHarness::new()?;
        let (task_id, change_unit_id, criterion_id, workspace) =
            create_workspace_bound_task(&harness, case.suffix)?;
        let clock = ManualClock::at("2026-07-13T01:00:00Z");
        harness.use_clock(clock.clone());
        let prepared = harness.service.prepare_evidence_capture(
            capture_request(
                &format!("req_capture_{}", case.suffix),
                Some(&format!("idem_capture_{}", case.suffix)),
                false,
                2,
                (&task_id, &change_unit_id, &criterion_id),
                case.capture,
            ),
            invocation_with_session(OperationCategory::AgentWorkflow, case.session_id)
                .with_git_workspace_context(workspace.clone()),
        )?;
        let intent_ref: StateRecordRef =
            serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;
        fulfill_registered_source_receipt(
            &harness,
            intent_ref.record_id.as_str(),
            case.observed_outcome,
            case.source,
            case.suffix,
        )?;
        clock.advance(Duration::minutes(2));
        let mut run_request = record_run_with_capture(
            &task_id,
            &change_unit_id,
            &criterion_id,
            intent_ref,
            case.suffix,
        );
        run_request.evidence_observations[0].source_kind = case.source_kind;
        run_request.evidence_observations[0].assurance_level = case.assurance_level;
        let response = harness.service.record_run(
            run_request,
            invocation_with_session(OperationCategory::AgentWorkflow, case.session_id)
                .with_git_workspace_context(workspace),
        )?;
        let observation = &response.response_value["evidence_observations"][0];
        assert_eq!(
            observation["producer_anchor"]["producer_kind"],
            case.producer_kind
        );
        assert_eq!(observation["relevance_assessment"]["status"], "unassessed");
        assert_eq!(observation["tool_name"], json!(case.tool_name));
        assert_eq!(
            observation["tool_invocation_id"],
            json!(case.tool_invocation_id)
        );
    }
    Ok(())
}
