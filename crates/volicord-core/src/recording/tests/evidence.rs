use super::*;

#[test]
fn user_channel_observation_is_strong_and_reuse_revalidates_its_authority_chain(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "user_evidence_authority")?;
    let criterion_id = volicord_types::ids::AcceptanceCriterionId::new(
        active_acceptance_criterion_id(&harness, &task_id)?,
    );
    set_active_acceptance_criterion_requirement(&harness, &task_id, EvidenceRequirement::Required)?;
    let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "user_evidence_authority",
    )?;
    let target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: criterion_id,
    };
    let (after_user, user_action_resolution_ref) = request_and_resolve_user_observation(
        &harness,
        UserObservationFixture {
            task_id: &task_id,
            change_unit_id: &change_unit_id,
            expected_state_version: after_artifact,
            suffix: "user_evidence_authority",
            target: target.clone(),
            artifact_ref: &artifact_ref,
            relevance_status: EvidenceRelevanceStatus::Supported,
        },
    )?;

    let mut record = record_run_request(
        "req_record_user_evidence",
        "idem_record_user_evidence",
        false,
        Some(after_user),
        &task_id,
        &change_unit_id,
    );
    record.evidence_updates = vec![EvidenceCoverageUpdate {
        target: target.clone(),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: vec![artifact_ref.clone()],
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
        input_refs: vec![user_action_resolution_ref.clone()],
        source_refs: Vec::new(),
        output_artifact_refs: vec![artifact_ref.clone()],
        limitations: Vec::new(),
        observed_at: volicord_types::values::UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
    }];
    record.close_assessment = Some(close_assessment_with_risks(
        "User-observed evidence is current.",
        Vec::new(),
    ))
    .into();
    let recorded = harness
        .service
        .record_run(record, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");
    assert_eq!(
        recorded.response_value["evidence_observations"][0]["source_kind"],
        "user_observation"
    );
    assert_eq!(
        recorded.response_value["evidence_observations"][0]["producer_anchor"]["producer_kind"],
        "user_channel_observation"
    );
    assert_no_close_blocker(
        &recorded.response_value["state"],
        "evidence_provenance_insufficient",
    );
    let after_record = recorded.response_value["base"]["state_version"]
        .as_u64()
        .expect("record state version");
    let original_observation_ref: StateRecordRef = serde_json::from_value(
        recorded.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            .clone(),
    )?;

    let mut reuse = record_run_request(
        "req_reuse_user_evidence",
        "idem_reuse_user_evidence",
        false,
        Some(after_record),
        &task_id,
        &change_unit_id,
    );
    reuse.evidence_updates = vec![EvidenceCoverageUpdate {
        target: target.clone(),
        coverage_state: EvidenceCoverageUpdateState::Supported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: vec![original_observation_ref],
        supporting_artifact_refs: vec![artifact_ref],
        gap_refs: Vec::new(),
    }];
    reuse.close_assessment = Some(close_assessment_with_risks(
        "Reused user-observed evidence is current.",
        Vec::new(),
    ))
    .into();
    let reused = harness
        .service
        .record_run(reuse, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(reused.response_value["base"]["response_kind"], "result");
    assert_eq!(
        reused.response_value["evidence_observations"][0]["source_kind"],
        "reused_evidence"
    );
    assert_no_close_blocker(
        &reused.response_value["state"],
        "evidence_provenance_insufficient",
    );

    let conn = harness.conn()?;
    let resolution_json: String = conn.query_row(
        "SELECT resolution_json
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_resolution_ref.record_id.as_str()],
        |row| row.get(0),
    )?;
    let mut resolution_json: serde_json::Value = serde_json::from_str(&resolution_json)?;
    resolution_json["observation"]["relevance_status"] =
        serde_json::Value::String("contradicted".to_owned());
    conn.execute(
        "UPDATE user_action_resolutions
            SET resolution_json = ?3
          WHERE project_id = ?1
            AND user_action_resolution_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            user_action_resolution_ref.record_id.as_str(),
            serde_json::to_string(&resolution_json)?
        ],
    )?;
    let close = harness.service.check_close(
        check_close_request(CloseTaskFixture {
            request_id: "req_check_tampered_user_evidence",
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
    assert_close_blocker(&close.response_value, "evidence_provenance_insufficient");
    Ok(())
}

#[test]
fn user_channel_observation_preserves_relevance_resolution_time_and_supported_only_reuse(
) -> Result<(), Box<dyn Error>> {
    for (suffix, relevance_status, coverage_state, relevance_value, caller_observed_at) in [
        (
            "supported",
            EvidenceRelevanceStatus::Supported,
            EvidenceCoverageUpdateState::Supported,
            "supported",
            "2000-01-01T00:00:00Z",
        ),
        (
            "contradicted",
            EvidenceRelevanceStatus::Contradicted,
            EvidenceCoverageUpdateState::Contradicted,
            "contradicted",
            "2999-01-01T00:00:00Z",
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("user_relevance_{suffix}"))?;
        let criterion_id = volicord_types::ids::AcceptanceCriterionId::new(
            active_acceptance_criterion_id(&harness, &task_id)?,
        );
        set_active_acceptance_criterion_requirement(
            &harness,
            &task_id,
            EvidenceRequirement::Required,
        )?;
        let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            &format!("user_relevance_{suffix}"),
        )?;
        let target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: criterion_id,
        };
        let (after_resolution, resolution_ref) = request_and_resolve_user_observation(
            &harness,
            UserObservationFixture {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                expected_state_version: after_artifact,
                suffix: &format!("user_relevance_{suffix}"),
                target: target.clone(),
                artifact_ref: &artifact_ref,
                relevance_status,
            },
        )?;
        let resolved_at: String = harness.conn()?.query_row(
            "SELECT resolved_at
               FROM user_action_resolutions
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
            rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
            |row| row.get(0),
        )?;
        assert_ne!(resolved_at, caller_observed_at);

        let mut record = record_run_request(
            &format!("req_user_relevance_{suffix}"),
            &format!("idem_user_relevance_{suffix}"),
            false,
            Some(after_resolution),
            &task_id,
            &change_unit_id,
        );
        record.evidence_updates = vec![EvidenceCoverageUpdate {
            target: target.clone(),
            coverage_state,
            provenance: None,
            supporting_run_refs: Vec::new(),
            observation_refs: Vec::new(),
            supporting_artifact_refs: vec![artifact_ref.clone()],
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
            output_artifact_refs: vec![artifact_ref.clone()],
            limitations: Vec::new(),
            observed_at: volicord_types::values::UtcTimestamp::parse(caller_observed_at)?,
        }];
        record.close_assessment = Some(close_assessment_with_risks(
            &format!("User-observed {suffix} evidence is preserved."),
            Vec::new(),
        ))
        .into();
        let replay_request = record.clone();
        let before_record = harness.counts()?;
        let recorded = harness
            .service
            .record_run(record, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(recorded.response_value["base"]["response_kind"], "result");
        let observation = &recorded.response_value["evidence_observations"][0];
        assert_eq!(observation["source_kind"], "user_observation");
        assert_eq!(observation["assurance_level"], "user_observed");
        assert_eq!(observation["observed_by_actor_source"], "local_user");
        assert_eq!(
            observation["producer_anchor"]["producer_kind"],
            "user_channel_observation"
        );
        assert_eq!(
            observation["relevance_assessment"]["status"],
            relevance_value
        );
        assert_eq!(
            observation["relevance_assessment"]["assessed_by_actor_source"],
            "local_user"
        );
        assert_eq!(observation["observed_at"], resolved_at);
        assert_eq!(
            recorded.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
            relevance_value
        );
        assert_no_close_blocker(
            &recorded.response_value["state"],
            "evidence_provenance_insufficient",
        );
        let after_record = harness.counts()?;
        assert_eq!(after_record.runs, before_record.runs + 1);
        assert_eq!(
            after_record.evidence_observations,
            before_record.evidence_observations + 1
        );
        let observation_id = observation["observation_id"]
            .as_str()
            .expect("committed observation id");
        let (stored_observed_at, stored_metadata): (String, String) = harness.conn()?.query_row(
            "SELECT observed_at, metadata_json
                   FROM evidence_observations
                  WHERE project_id = ?1
                    AND evidence_observation_id = ?2",
            rusqlite::params![PROJECT_ID, observation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(stored_observed_at, resolved_at);
        let stored_metadata: Value = serde_json::from_str(&stored_metadata)?;
        assert_eq!(
            stored_metadata["relevance_assessment"]["status"],
            relevance_value
        );
        let before_replay = harness.counts()?;
        let before_replay_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        let replayed = harness
            .service
            .record_run(replay_request, invocation(OperationCategory::AgentWorkflow))?;
        assert!(replayed.replayed);
        assert_eq!(replayed.response_json, recorded.response_json);
        assert_eq!(harness.counts()?, before_replay);
        let after_replay_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_replay_floor, before_replay_floor);

        let observation_ref: StateRecordRef = serde_json::from_value(
            recorded.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
                .clone(),
        )?;
        if relevance_status == EvidenceRelevanceStatus::Supported {
            let original_resolution_json: String = harness.conn()?.query_row(
                "SELECT resolution_json
                   FROM user_action_resolutions
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
                rusqlite::params![PROJECT_ID, resolution_ref.record_id.as_str()],
                |row| row.get(0),
            )?;
            let mut mismatched_resolution: Value = serde_json::from_str(&original_resolution_json)?;
            mismatched_resolution["observation"]["relevance_status"] =
                Value::String("contradicted".to_owned());
            harness.conn()?.execute(
                "UPDATE user_action_resolutions
                    SET resolution_json = ?3
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
                rusqlite::params![
                    PROJECT_ID,
                    resolution_ref.record_id.as_str(),
                    serde_json::to_string(&mismatched_resolution)?
                ],
            )?;
        }

        let committed_state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .expect("committed record state version");
        for dry_run in [true, false] {
            let branch = if dry_run { "dry" } else { "commit" };
            let mut reuse = record_run_request(
                &format!("req_user_relevance_reuse_{suffix}_{branch}"),
                &format!("idem_user_relevance_reuse_{suffix}_{branch}"),
                dry_run,
                Some(committed_state_version),
                &task_id,
                &change_unit_id,
            );
            reuse.evidence_updates = vec![EvidenceCoverageUpdate {
                target: target.clone(),
                coverage_state: EvidenceCoverageUpdateState::Supported,
                provenance: None,
                supporting_run_refs: Vec::new(),
                observation_refs: vec![observation_ref.clone()],
                supporting_artifact_refs: vec![artifact_ref.clone()],
                gap_refs: Vec::new(),
            }];
            let before_rejection = harness.counts()?;
            let before_rejection_floor: String = harness.conn()?.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            let rejected = harness
                .service
                .record_run(reuse, invocation(OperationCategory::AgentWorkflow))?;
            assert_eq!(
                rejected.response_value["base"]["response_kind"], "rejected",
                "case {suffix}, dry_run={dry_run}"
            );
            assert_eq!(
                rejected.response_value["errors"][0]["details"]["field"],
                "evidence_updates[].observation_refs"
            );
            assert_eq!(harness.counts()?, before_rejection);
            let after_rejection_floor: String = harness.conn()?.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [PROJECT_ID],
                |row| row.get(0),
            )?;
            assert_eq!(after_rejection_floor, before_rejection_floor);
        }
    }
    Ok(())
}

#[test]
fn not_applicable_rejects_required_criterion_but_commits_for_optional_criterion(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_not_applicable_requirement")?;
    let (after_required, required_criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        2,
        "run_not_applicable_required",
        &[(
            "Criterion requiring evidence.",
            EvidenceRequirement::Required,
        )],
    )?;
    let required_id = required_criteria[0].acceptance_criterion_id.clone();
    let required_update = EvidenceCoverageUpdate {
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: required_id.clone(),
        },
        coverage_state: EvidenceCoverageUpdateState::NotApplicable,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    };
    let mut required_request = record_run_request(
        "req_run_not_applicable_required",
        "idem_run_not_applicable_required",
        false,
        Some(after_required),
        &task_id,
        &change_unit_id,
    );
    required_request.evidence_updates = vec![required_update];
    let before_rejection = harness.counts()?;
    let rejected = harness.service.record_run(
        required_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        rejected.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].coverage_state"
    );
    assert_eq!(harness.counts()?, before_rejection);

    let (after_optional, optional_criteria) = replace_acceptance_criteria_for_test(
        &harness,
        &task_id,
        after_required,
        "run_not_applicable_optional",
        &[(
            "Criterion where evidence is optional.",
            EvidenceRequirement::Optional,
        )],
    )?;
    assert_eq!(optional_criteria[0].acceptance_criterion_id, required_id);
    let mut optional_request = record_run_request(
        "req_run_not_applicable_optional",
        "idem_run_not_applicable_optional",
        false,
        Some(after_optional),
        &task_id,
        &change_unit_id,
    );
    optional_request.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: required_id,
        },
        coverage_state: EvidenceCoverageUpdateState::NotApplicable,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let committed = harness.service.record_run(
        optional_request,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(
        committed.response_value["evidence_summary"]["coverage_items"][0]["coverage_state"],
        "not_applicable"
    );
    Ok(())
}

#[test]
fn supplemental_evidence_claim_identity_is_task_scoped_and_statement_is_immutable(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (first_task_id, first_change_unit_id) =
        create_task_with_change_unit(&harness, "task_scoped_evidence_claim")?;
    let shared_claim_id = volicord_types::ids::EvidenceClaimId::new("claim_shared_between_tasks");

    let mut first_run = record_run_request(
        "req_task_scoped_claim_first",
        "idem_task_scoped_claim_first",
        false,
        Some(2),
        &first_task_id,
        &first_change_unit_id,
    );
    first_run.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: shared_claim_id.clone(),
            statement: "Statement owned by the first Task.".to_owned(),
        },
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let first_response = harness
        .service
        .record_run(first_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(first_response.response_value["base"]["state_version"], 3);

    let second_intake = harness.service.intake(
        intake_request(
            "req_task_scoped_claim_second_task",
            "idem_task_scoped_claim_second_task",
            false,
            Some(3),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_task_id = response_record_id(&second_intake.response_value, "task_ref");
    let second_scope = harness.service.update_scope(
        update_scope_request(
            "req_task_scoped_claim_second_scope",
            "idem_task_scoped_claim_second_scope",
            false,
            Some(4),
            &second_task_id,
            ChangeUnitOperation::CreateCurrent,
            "Second Task claim scope.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_change_unit_id = response_record_id(&second_scope.response_value, "change_unit_ref");
    advance_work_task_for_test(
        &harness,
        "task_scoped_evidence_claim_second",
        &second_task_id,
        &second_change_unit_id,
    )?;

    let mut second_run = record_run_request(
        "req_task_scoped_claim_second",
        "idem_task_scoped_claim_second",
        false,
        Some(5),
        &second_task_id,
        &second_change_unit_id,
    );
    second_run.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: shared_claim_id.clone(),
            statement: "Independent statement owned by the second Task.".to_owned(),
        },
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let second_response = harness
        .service
        .record_run(second_run, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(second_response.response_value["base"]["state_version"], 6);

    let claims = {
        let conn = harness.conn()?;
        let mut statement = conn.prepare(
            "SELECT task_id, statement
               FROM evidence_claims
              WHERE project_id = ?1
                AND evidence_claim_id = ?2
              ORDER BY task_id ASC",
        )?;
        let rows = statement
            .query_map(
                rusqlite::params![PROJECT_ID, shared_claim_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        rows
    };
    assert_eq!(claims.len(), 2);
    assert!(claims.iter().any(|(task_id, statement)| {
        task_id == &first_task_id && statement == "Statement owned by the first Task."
    }));
    assert!(claims.iter().any(|(task_id, statement)| {
        task_id == &second_task_id && statement == "Independent statement owned by the second Task."
    }));

    let before_mutation = harness.counts()?;
    let mut mutated = record_run_request(
        "req_task_scoped_claim_mutation",
        "idem_task_scoped_claim_mutation",
        false,
        Some(6),
        &second_task_id,
        &second_change_unit_id,
    );
    mutated.evidence_updates = vec![EvidenceCoverageUpdate {
        target: EvidenceTarget::SupplementalClaim {
            evidence_claim_id: shared_claim_id,
            statement: "Attempted mutation within the second Task.".to_owned(),
        },
        coverage_state: EvidenceCoverageUpdateState::Unsupported,
        provenance: None,
        supporting_run_refs: Vec::new(),
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }];
    let rejected = harness
        .service
        .record_run(mutated, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before_mutation);
    Ok(())
}

#[test]
fn record_run_rejects_supported_evidence_without_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_missing_provenance")?;
    let mut request = record_run_request(
        "req_run_missing_provenance",
        "idem_run_missing_provenance",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let mut evidence_update = supported_evidence_update("Claim without provenance.");
    evidence_update.provenance = None;
    request.evidence_updates = vec![evidence_update];
    let before = harness.counts()?;

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "evidence_updates[].provenance"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}
