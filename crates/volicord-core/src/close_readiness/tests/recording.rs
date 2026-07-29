use super::*;

#[test]
fn record_run_non_null_close_assessment_creates_current_basis() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_basis")?;
    let generator = CountingDurableIdGenerator::new(["run_basis", "event_basis"]);
    let clock = ManualClock::at("2026-06-18T12:00:00Z");
    harness.use_generator_and_clock(generator, clock);

    let mut request = record_run_request(
        "req_run_basis",
        "idem_run_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(close_assessment_with_risks(
        "Recorded close basis.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;
    let basis = revision
        .current_close_basis
        .expect("current close basis should be stored");

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(basis.task_id.as_str(), task_id);
    assert_eq!(basis.change_unit_id.as_str(), change_unit_id);
    assert_eq!(basis.scope_revision, 1);
    assert_eq!(basis.close_basis_revision, revision.close_basis_revision);
    assert_eq!(basis.result_summary, "Recorded close basis.");
    assert!(basis.residual_risks.is_empty());
    assert_eq!(basis.updated_at.to_string(), "2026-06-18T12:00:00Z");
    assert_eq!(
        response.response_value["current_close_basis"]["residual_risks"],
        json!([])
    );
    assert!(
        response.response_value["current_close_basis"]["result_refs"]
            .as_array()
            .expect("result_refs should be present")
            .iter()
            .filter_map(|record_ref| record_ref["record_kind"].as_str())
            .any(|kind| kind == "run")
    );
    Ok(())
}

#[test]
fn current_compatible_run_ref_can_enter_close_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "current_run_ref")?;

    let mut first = record_run_request(
        "req_current_run_ref_first",
        "idem_current_run_ref_first",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    first.run_id = Some(RunId::new("run_current_ref_first")).into();
    let first_response = harness
        .service
        .record_run(first, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(first_response.response_value["base"]["state_version"], 3);

    let mut second = record_run_request(
        "req_current_run_ref_second",
        "idem_current_run_ref_second",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    second.run_id = Some(RunId::new("run_current_ref_second")).into();
    second.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Current prior Run can support this close basis.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_current_ref_first",
            PROJECT_ID,
            &task_id,
            Some(999),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(second, invocation(OperationCategory::AgentWorkflow))?;
    let basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current basis should be stored");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_current_ref_first"
            && record_ref.produced_at_state_version.as_ref() == Some(&4)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_current_ref_second"
            && record_ref.produced_at_state_version.as_ref() == Some(&4)
    }));
    Ok(())
}

#[test]
fn record_run_rejects_superseded_change_unit_run_ref_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "old_unit_run_ref")?;

    let mut old = record_run_request(
        "req_old_unit_run",
        "idem_old_unit_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    old.run_id = Some(RunId::new("run_old_unit")).into();
    harness
        .service
        .record_run(old, invocation(OperationCategory::AgentWorkflow))?;

    let replace = harness.service.update_scope(
        update_scope_request(
            "req_old_unit_replace",
            "idem_old_unit_replace",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement current scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_old_unit_rejected",
        "idem_old_unit_rejected",
        false,
        Some(4),
        &task_id,
        &replacement_change_unit_id,
    );
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Old unit Run must not become current.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_old_unit",
            PROJECT_ID,
            &task_id,
            Some(3),
        )],
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
    Ok(())
}

#[test]
fn record_run_scope_revision_is_required_by_storage_constraint() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_scope_required")?;

    let mut request = record_run_request(
        "req_scope_required_run",
        "idem_scope_required_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_scope_required")).into();
    harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;

    let error = harness
        .conn()?
        .execute(
            "UPDATE runs
                SET scope_revision = NULL
              WHERE project_id = ?1
                AND run_id = 'run_scope_required'",
            rusqlite::params![PROJECT_ID],
        )
        .expect_err("runs.scope_revision is required");
    assert_constraint_error(error);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_rejects_baseline_incompatible_run_ref_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "baseline_run_ref")?;

    let mut baseline = record_run_request(
        "req_baseline_run",
        "idem_baseline_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    baseline.run_id = Some(RunId::new("run_baseline_mismatch")).into();
    harness
        .service
        .record_run(baseline, invocation(OperationCategory::AgentWorkflow))?;
    set_run_observed_baseline(&harness, "run_baseline_mismatch", "baseline_other")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_baseline_ref_rejected",
        "idem_baseline_ref_rejected",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Baseline-mismatched Run must not become current.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            "run_baseline_mismatch",
            PROJECT_ID,
            &task_id,
            Some(3),
        )],
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
    Ok(())
}

#[test]
fn historical_verified_artifact_reuse_requires_new_current_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "artifact_reuse")?;
    let (artifact_state_version, artifact_ref) =
        promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, "artifact_reuse")?;
    let old_run_id = latest_run_id(&harness, &task_id)?;

    let replace = harness.service.update_scope(
        update_scope_request(
            "req_artifact_reuse_replace",
            "idem_artifact_reuse_replace",
            false,
            Some(artifact_state_version),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement scope for artifact reuse.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let replacement_change_unit_id = response_record_id(&replace.response_value, "change_unit_ref");

    let mut direct_old_run = record_run_request(
        "req_artifact_reuse_old_run",
        "idem_artifact_reuse_old_run",
        false,
        Some(artifact_state_version + 1),
        &task_id,
        &replacement_change_unit_id,
    );
    direct_old_run.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Old Run must not be reused directly.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            &old_run_id,
            PROJECT_ID,
            &task_id,
            Some(artifact_state_version),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let before_reject = harness.counts()?;
    let rejected = harness
        .service
        .record_run(direct_old_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before_reject);

    let mut current_reuse = record_run_request(
        "req_artifact_reuse_current",
        "idem_artifact_reuse_current",
        false,
        Some(artifact_state_version + 1),
        &task_id,
        &replacement_change_unit_id,
    );
    current_reuse.run_id = Some(RunId::new("run_artifact_reuse_current")).into();
    current_reuse.artifact_inputs = vec![existing_artifact_input(
        "artifact_input_reuse_current",
        artifact_ref.clone(),
    )];
    current_reuse.evidence_updates = vec![supported_evidence_update(
        "Historical verified artifact reused by a current Run.",
    )];
    current_reuse.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Artifact reuse is recorded by a current Run.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Artifact,
            artifact_ref.artifact_id.as_str(),
            PROJECT_ID,
            &task_id,
            Some(artifact_state_version),
        )],
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(current_reuse, invocation(OperationCategory::AgentWorkflow))?;
    let basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current basis should be stored");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        run_scope_revision(&harness, "run_artifact_reuse_current")?,
        2
    );
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_artifact_reuse_current"
    }));
    assert!(basis.result_refs.iter().all(|record_ref| {
        record_ref.record_kind != StateRecordKind::Run
            || record_ref.record_id.as_str() != old_run_id
    }));
    Ok(())
}

#[test]
fn record_run_generates_opaque_residual_risk_ids_on_commit() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_risks")?;
    let generator = CountingDurableIdGenerator::new(["risk_alpha", "risk_beta", "event_risks"]);
    let clock = ManualClock::at("2026-06-18T12:30:00Z");
    harness.use_generator_and_clock(generator.clone(), clock);

    let mut request = record_run_request(
        "req_run_risks",
        "idem_run_risks",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_risks_supplied")).into();
    request.close_assessment = Some(close_assessment_with_risks(
        "Recorded close basis with risks.",
        vec![
            residual_risk_input("First residual risk."),
            residual_risk_input("Second residual risk."),
        ],
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let risk_ids = response.response_value["current_close_basis"]["residual_risks"]
        .as_array()
        .expect("residual risks should be an array")
        .iter()
        .map(|risk| {
            risk["risk_id"]
                .as_str()
                .expect("risk id should be present")
                .to_owned()
        })
        .collect::<Vec<_>>();
    let (_, event_payload, _) = latest_authority_event(&harness)?;

    assert_eq!(risk_ids, vec!["risk_risk_alpha", "risk_risk_beta"]);
    assert_eq!(generator.count(DurableIdKind::Risk), 2);
    assert_eq!(event_payload["residual_risk_ids"], json!(risk_ids));
    assert_eq!(
        event_payload["source_run_ref"]["record_id"],
        "run_risks_supplied"
    );
    assert_eq!(event_payload["scope_revision"], 1);
    assert_eq!(event_payload["close_basis_revision"], 2);
    Ok(())
}

#[test]
fn record_run_rejects_unsupported_close_basis_ref_kinds_without_effect(
) -> Result<(), Box<dyn Error>> {
    let unsupported = [
        (StateRecordKind::WriteTicket, "wa_fabricated"),
        (StateRecordKind::UserActionRequest, "uj_fabricated"),
        (StateRecordKind::Blocker, "blocker_fabricated"),
        (StateRecordKind::TaskEvent, "evt_fabricated"),
        (StateRecordKind::ProjectState, "project_state_fabricated"),
        (StateRecordKind::Task, "task_fabricated"),
        (StateRecordKind::AgentConnection, "connection_fabricated"),
    ];

    for (index, (record_kind, record_id)) in unsupported.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("unsupported_ref_{index}"))?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_unsupported_ref_{index}"),
            &format!("idem_unsupported_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
            result_summary: "Unsupported refs must not enter close authority.".to_owned(),
            result_refs: vec![test_state_record_ref(
                record_kind,
                record_id,
                PROJECT_ID,
                &task_id,
                Some(999),
            )],
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
fn record_run_rejects_nonexistent_allowed_close_basis_refs_without_effect(
) -> Result<(), Box<dyn Error>> {
    let allowed_but_missing = [
        (StateRecordKind::Run, "run_missing"),
        (StateRecordKind::Artifact, "artifact_missing"),
        (StateRecordKind::EvidenceSummary, "evidence_missing"),
        (StateRecordKind::ChangeUnit, "cu_missing"),
    ];

    for (index, (record_kind, record_id)) in allowed_but_missing.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("missing_ref_{index}"))?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            &format!("req_missing_ref_{index}"),
            &format!("idem_missing_ref_{index}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
            result_summary: "Missing allowed refs still need stored records.".to_owned(),
            result_refs: vec![test_state_record_ref(
                record_kind,
                record_id,
                PROJECT_ID,
                &task_id,
                Some(2),
            )],
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
fn record_run_rejects_corrupt_artifact_close_basis_ref_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "corrupt_basis_artifact")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "corrupt_basis_artifact",
    )?;
    let artifact_id = artifact_ref.artifact_id.as_str().to_owned();
    set_artifact_integrity(
        &harness,
        &artifact_id,
        "corrupt",
        artifact_ref.content_type.as_deref(),
        artifact_ref.sha256.as_deref(),
        artifact_ref.size_bytes.as_ref().copied(),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_unverified_artifact_basis",
        "idem_unverified_artifact_basis",
        false,
        Some(state_version),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Unverified artifact must not enter close authority.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::Artifact,
            &artifact_id,
            PROJECT_ID,
            &task_id,
            Some(999),
        )],
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
    Ok(())
}

#[test]
fn record_run_rejects_noncurrent_evidence_summary_close_basis_ref_without_effect(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2999-07-13T12:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "noncurrent_evidence")?;
    let first_state =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "old_evidence", true)?;
    let old_evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    clock.advance(Duration::milliseconds(1));
    let current_state = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        first_state,
        "new_evidence",
        true,
    )?;
    let current_evidence_summary_id = latest_evidence_summary_id(&harness, &task_id)?;
    let summaries = {
        let conn = harness.conn()?;
        let mut stmt = conn.prepare(
            "SELECT evidence_summary_id, produced_at_state_version
               FROM evidence_summaries
              WHERE project_id = ?1 AND task_id = ?2
              ORDER BY produced_at_state_version DESC",
        )?;
        let summaries = stmt
            .query_map(rusqlite::params![PROJECT_ID, task_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        summaries
    };
    assert_ne!(
        old_evidence_summary_id, current_evidence_summary_id,
        "second evidence commit must establish a distinct latest summary: {summaries:?}"
    );
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_noncurrent_evidence_basis",
        "idem_noncurrent_evidence_basis",
        false,
        Some(current_state),
        &task_id,
        &change_unit_id,
    );
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Old evidence summary must not enter current close authority.".to_owned(),
        result_refs: vec![test_state_record_ref(
            StateRecordKind::EvidenceSummary,
            &old_evidence_summary_id,
            PROJECT_ID,
            &task_id,
            Some(first_state),
        )],
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
    Ok(())
}

#[test]
fn record_run_canonicalizes_deduplicates_and_adds_current_close_basis_refs(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_refs")?;
    let mut request = record_run_request(
        "req_canonical_refs",
        "idem_canonical_refs",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_canonical_refs")).into();
    request.evidence_updates = vec![supported_evidence_update("Canonical close basis claim.")];
    let future_run_ref = test_state_record_ref(
        StateRecordKind::Run,
        "run_canonical_refs",
        PROJECT_ID,
        &task_id,
        Some(999),
    );
    let past_run_ref = test_state_record_ref(
        StateRecordKind::Run,
        "run_canonical_refs",
        PROJECT_ID,
        &task_id,
        Some(1),
    );
    let mut risk = residual_risk_input("Caller-versioned risk source.");
    risk.acceptance_required = false;
    risk.source_refs = vec![future_run_ref.clone(), past_run_ref.clone()];
    request.close_assessment = Some(volicord_types::schema::CloseAssessmentInput {
        result_summary: "Canonical refs are stored.".to_owned(),
        result_refs: vec![future_run_ref, past_run_ref],
        residual_risks: vec![risk],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;
    let basis = revision
        .current_close_basis
        .expect("current close basis should be stored");

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(basis.result_refs.len(), 3);
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::Run
            && record_ref.record_id.as_str() == "run_canonical_refs"
            && record_ref.produced_at_state_version.as_ref() == Some(&3)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::ChangeUnit
            && record_ref.record_id.as_str() == change_unit_id
            && record_ref.produced_at_state_version.as_ref() == Some(&3)
    }));
    assert!(basis.result_refs.iter().any(|record_ref| {
        record_ref.record_kind == StateRecordKind::EvidenceSummary
            && record_ref.produced_at_state_version.as_ref() == Some(&3)
    }));
    assert_eq!(
        basis
            .evidence_summary_ref
            .as_ref()
            .and_then(|record_ref| record_ref.produced_at_state_version.as_ref().copied()),
        Some(3)
    );
    assert_eq!(basis.residual_risks[0].source_refs.len(), 1);
    assert_eq!(
        basis.residual_risks[0].source_refs[0]
            .produced_at_state_version
            .as_ref(),
        Some(&3)
    );
    Ok(())
}

#[test]
fn final_acceptance_judgment_basis_uses_canonical_close_basis_refs() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "canonical_final")?;
    let state_version = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "canonical_final",
        true,
    )?;
    let close_basis = task_revision(&harness, &task_id)?
        .current_close_basis
        .expect("current close basis should be stored");

    let response = harness.service.request_user_action(
        user_action_request(
            "req_canonical_final",
            "idem_canonical_final",
            false,
            Some(state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let request_id = response_record_id(&response.response_value, "user_action_request_ref");
    assert_eq!(
        response.response_value["user_action_request_summary"],
        pending_user_action_summary(&request_id)
    );
    let facts = local_pending_user_action_facts(&harness, &task_id)?;
    let projected = facts
        .actions
        .iter()
        .find(|item| item.request.user_action_request_id.as_str() == request_id)
        .expect("trusted User Channel projection should retain the close-basis request");
    assert_eq!(
        projected.request.basis.result_refs(),
        close_basis.result_refs.as_slice()
    );
    assert!(close_basis.result_refs.iter().all(|record_ref| {
        record_ref.produced_at_state_version.as_ref() == Some(&state_version)
    }));
    Ok(())
}

#[test]
fn record_run_null_close_assessment_invalidates_existing_basis() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_clear_basis")?;

    let mut establish = record_run_request(
        "req_run_establish_basis",
        "idem_run_establish_basis",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    establish.close_assessment = Some(close_assessment_with_risks(
        "Established basis.",
        Vec::new(),
    ))
    .into();
    harness
        .service
        .record_run(establish, invocation(OperationCategory::AgentWorkflow))?;
    assert!(task_revision(&harness, &task_id)?
        .current_close_basis
        .is_some());

    let clear = record_run_request(
        "req_run_clear_basis",
        "idem_run_clear_basis",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    let response = harness
        .service
        .record_run(clear, invocation(OperationCategory::AgentWorkflow))?;
    let revision = task_revision(&harness, &task_id)?;

    assert!(response.response_value["current_close_basis"].is_null());
    assert_eq!(revision.close_basis_revision, 3);
    assert!(revision.current_close_basis.is_none());
    Ok(())
}

#[test]
fn record_run_dry_run_allocates_no_residual_risk_ids() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_dry_risk")?;
    let generator = CountingDurableIdGenerator::new(Vec::<&str>::new());
    let clock = ManualClock::at("2026-06-18T13:00:00Z");
    harness.use_generator_and_clock(generator.clone(), clock);
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut request = record_run_request(
        "req_run_dry_risk",
        "idem_run_dry_risk",
        true,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.run_id = Some(RunId::new("run_dry_risk_supplied")).into();
    request.close_assessment = Some(close_assessment_with_risks(
        "Dry-run close basis.",
        vec![residual_risk_input("Dry-run residual risk.")],
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(generator.count(DurableIdKind::Risk), 0);
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    Ok(())
}
