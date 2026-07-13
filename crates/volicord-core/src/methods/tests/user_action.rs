use super::*;

fn request_id(response: &PipelineResponse) -> String {
    response_record_id(&response.response_value, "user_action_request_ref")
}

fn current_state_version(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
    Ok(harness.counts()?.state_version)
}

#[test]
fn custom_clock_is_bounded_by_persisted_and_same_handle_floors() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-06-18T01:00:00Z");
    harness.use_clock(clock.clone());
    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let first = harness.service.project_now(&store)?;
    assert_eq!(first.to_string(), "2026-06-18T01:00:00Z");
    clock.advance(Duration::hours(-1));
    let same_handle = harness.service.project_now(&store)?;
    assert_eq!(same_handle, first);
    drop(store);

    let persisted_floor = "2999-07-13T12:34:56.789Z";
    harness.conn()?.execute(
        "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
        rusqlite::params![PROJECT_ID, persisted_floor],
    )?;
    harness.use_clock(ManualClock::at("2026-06-18T00:00:00Z"));
    let reopened = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    assert_eq!(
        harness.service.project_now(&reopened)?.to_string(),
        persisted_floor
    );
    Ok(())
}

#[test]
fn chrono_max_custom_clock_is_rejected_without_effects() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let before_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    harness.use_clock(ManualClock::from_datetime(DateTime::<Utc>::MAX_UTC));

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_extreme_clock", None, false, None, None),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(harness.counts()?, before);
    let after_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(after_floor, before_floor);
    Ok(())
}

#[test]
fn each_prepared_operation_samples_the_core_clock_once() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "clock_sample_once")?;
    let clock = ManualClock::at("2999-07-13T12:34:56.789Z");
    harness.use_clock(clock.clone());

    let before_request = clock.sample_count();
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_clock_sample_once",
            "idem_clock_sample_once",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(requested.response_value["base"]["response_kind"], "result");
    assert_eq!(clock.sample_count() - before_request, 1);

    let (requested_at, updated_at) = harness.conn()?.query_row(
        "SELECT r.requested_at, p.updated_at
           FROM user_action_requests r
           JOIN project_state p ON p.project_id = r.project_id
          WHERE r.project_id = ?1
            AND r.user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, request_id(&requested)],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    assert_eq!(requested_at, "2999-07-13T12:34:56.789Z");
    assert_eq!(updated_at, requested_at);

    let before_status = clock.sample_count();
    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_clock_sample_once_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(status.response_value["base"]["response_kind"], "result");
    assert_eq!(clock.sample_count() - before_status, 1);
    Ok(())
}

#[test]
fn evidence_user_action_ttl_overflow_rejects_without_effects() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "evidence_action_ttl_overflow")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "evidence_action_ttl_overflow",
    )?;
    harness.use_clock(ManualClock::at("9999-12-31T23:50:00Z"));
    let before = harness.counts()?;

    let response = harness.service.request_user_action(
        observation_action_request(
            "req_evidence_action_ttl_overflow",
            "idem_evidence_action_ttl_overflow",
            state_version,
            &task_id,
            &change_unit_id,
            supplemental_evidence_target("TTL overflow must not persist."),
            vec![artifact_ref.artifact_id],
        ),
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
fn evidence_user_action_extended_stored_ttl_rejects_resolution_without_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "evidence_action_extended_stored_ttl")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "evidence_action_extended_stored_ttl",
    )?;
    let target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: volicord_types::AcceptanceCriterionId::new(
            active_acceptance_criterion_id(&harness, &task_id)?,
        ),
    };
    let artifact_id = artifact_ref.artifact_id.clone();
    let requested = harness.service.request_user_action(
        observation_action_request(
            "req_evidence_action_extended_stored_ttl",
            "idem_evidence_action_extended_stored_ttl",
            state_version,
            &task_id,
            &change_unit_id,
            target.clone(),
            vec![artifact_id.clone()],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        requested.response_value["base"]["response_kind"], "result",
        "{}",
        requested.response_value
    );
    let action_id = request_id(&requested);
    let conn = harness.conn()?;
    let (request_json, requested_at): (String, String) = conn.query_row(
        "SELECT request_json, requested_at
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, action_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let extended_expires_at = UtcTimestamp::parse(&requested_at)?
        .checked_add(Duration::minutes(16))?
        .to_string();
    let mut request_json: Value = serde_json::from_str(&request_json)?;
    request_json["expires_at"] = json!(extended_expires_at);
    conn.execute(
        "UPDATE user_action_requests
            SET request_json = ?3,
                expires_at = ?4
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            action_id,
            request_json.to_string(),
            extended_expires_at
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
        let submission_id = format!("submission_evidence_extended_ttl_{branch}");
        let response = harness.service.resolve_user_action(
            volicord_types::ResolveUserActionRequest {
                envelope: envelope(
                    &format!("req_resolve_evidence_extended_ttl_{branch}"),
                    Some(&submission_id),
                    dry_run,
                    None,
                    Some(&task_id),
                ),
                user_action_request_id: volicord_types::UserActionRequestId::new(action_id.clone()),
                channel_submission_id: submission_id,
                resolution: volicord_types::UserActionResolutionInput::EvidenceObservation {
                    target: target.clone(),
                    artifact_ids: vec![artifact_id.clone()],
                    relevance_status: EvidenceRelevanceStatus::Supported,
                    summary: "The exact candidate artifact supports the target.".to_owned(),
                },
            },
            invocation(OperationCategory::UserOnly),
        )?;

        assert_eq!(
            response.response_value["base"]["response_kind"], "rejected",
            "dry_run={dry_run}"
        );
        assert_eq!(
            response.response_value["errors"][0]["code"], "MCP_UNAVAILABLE",
            "dry_run={dry_run}"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["owner_state_error"]["logical_column"],
            "expires_at",
            "dry_run={dry_run}"
        );
        assert_eq!(harness.counts()?, before, "dry_run={dry_run}");
        let after_floor: String = conn.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor, "dry_run={dry_run}");
    }
    Ok(())
}

#[test]
fn explicit_choice_expiry_outside_canonical_range_rejects_dry_run_and_commit_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "choice_expiry_range")?;
    let before = harness.counts()?;
    let before_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    let out_of_range =
        UtcTimestamp::parse("9999-12-31T23:59:59-23:59").expect("Chrono accepts +10000 UTC");

    for dry_run in [true, false] {
        let mut request = user_action_request(
            if dry_run {
                "req_choice_expiry_range_dry"
            } else {
                "req_choice_expiry_range_commit"
            },
            if dry_run {
                "idem_choice_expiry_range_dry"
            } else {
                "idem_choice_expiry_range_commit"
            },
            dry_run,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        );
        request.expires_at = RequiredNullable::some(out_of_range.clone());
        let response = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
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
        let after_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor);
    }
    Ok(())
}

#[test]
fn oversized_derived_capture_form_rejects_before_user_action_commit() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "capture_form_size")?;
    let (state_version, mut artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "capture_form_size",
    )?;
    let target = supplemental_evidence_target("Artifact registered for corruption coverage.");
    artifact_ref.display_name.clear();
    let body_for = |artifact_ref: volicord_types::ArtifactRef| {
        UserActionRequestBody::EvidenceObservation(
            volicord_types::UserActionEvidenceObservationRequestBody {
                question: "q".to_owned(),
                context_summary: "c".to_owned(),
                target_candidates: vec![target.clone()],
                artifact_candidates: vec![artifact_ref],
            },
        )
    };
    let base_size = canonical_json_size_bytes(&body_for(artifact_ref.clone()))?;
    artifact_ref.display_name = "x".repeat(USER_ACTION_FORM_MAX_BYTES - base_size);
    let boundary_body = body_for(artifact_ref.clone());
    assert_eq!(
        canonical_json_size_bytes(&boundary_body)?,
        USER_ACTION_FORM_MAX_BYTES
    );
    let form_error = boundary_body
        .capture_form()
        .expect_err("derived form overhead must exceed the body byte boundary");
    assert_eq!(form_error.field(), "form");

    let mut producer_json: Value = serde_json::from_str(&harness.conn()?.query_row(
        "SELECT producer_json FROM artifacts
          WHERE project_id = ?1 AND artifact_id = ?2",
        rusqlite::params![PROJECT_ID, artifact_ref.artifact_id.as_str()],
        |row| row.get::<_, String>(0),
    )?)?;
    producer_json["display_name"] = json!(artifact_ref.display_name.clone());
    harness.conn()?.execute(
        "UPDATE artifacts SET producer_json = ?3
          WHERE project_id = ?1 AND artifact_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            artifact_ref.artifact_id.as_str(),
            producer_json.to_string()
        ],
    )?;
    let before = harness.counts()?;

    let mut request = observation_action_request(
        "req_capture_form_size",
        "idem_capture_form_size",
        state_version,
        &task_id,
        &change_unit_id,
        target,
        vec![artifact_ref.artifact_id],
    );
    let volicord_types::UserActionDraft::EvidenceObservation(observation) = &mut request.action
    else {
        unreachable!("observation helper must create an evidence-observation draft")
    };
    observation.question = "q".to_owned();
    observation.context_summary = "c".to_owned();
    let response = harness
        .service
        .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "form"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

fn resolve_observation_request(
    request_id: &str,
    channel_submission_id: &str,
    task_id: &str,
    user_action_request_id: &str,
    target: EvidenceTarget,
    artifact_ids: Vec<volicord_types::ArtifactId>,
    relevance_status: EvidenceRelevanceStatus,
) -> volicord_types::ResolveUserActionRequest {
    volicord_types::ResolveUserActionRequest {
        envelope: envelope(
            request_id,
            Some(channel_submission_id),
            false,
            None,
            Some(task_id),
        ),
        user_action_request_id: volicord_types::UserActionRequestId::new(user_action_request_id),
        channel_submission_id: channel_submission_id.to_owned(),
        resolution: volicord_types::UserActionResolutionInput::EvidenceObservation {
            target,
            artifact_ids,
            relevance_status,
            summary: "The user assessed the exact candidate bytes.".to_owned(),
        },
    }
}

fn record_user_action_matrix_close_basis(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    suffix: &str,
    with_residual_risk: bool,
) -> Result<u64, Box<dyn Error>> {
    let mut request = record_run_request(
        &format!("req_matrix_basis_{suffix}"),
        &format!("idem_matrix_basis_{suffix}"),
        false,
        Some(2),
        task_id,
        change_unit_id,
    );
    request.evidence_updates = Vec::new();
    request.close_assessment = Some(close_assessment_with_risks(
        "Current close basis for the user-action compatibility matrix.",
        if with_residual_risk {
            vec![residual_risk_input(
                "A visible risk needs an explicit user disposition.",
            )]
        } else {
            Vec::new()
        },
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(
        response.response_value["base"]["response_kind"], "result",
        "matrix close basis should commit: {}",
        response.response_value
    );
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("matrix close-basis state version"))
}

#[test]
fn required_for_compatibility_covers_all_action_kinds_and_operations() -> Result<(), Box<dyn Error>>
{
    #[derive(Clone, Copy, Debug)]
    enum ActionCase {
        Choice(JudgmentKind),
        EvidenceObservation,
    }

    let requirements = [
        volicord_types::UserActionRequiredFor::ScopeUpdate,
        volicord_types::UserActionRequiredFor::PrepareWrite,
        volicord_types::UserActionRequiredFor::RecordRun,
        volicord_types::UserActionRequiredFor::CloseComplete,
        volicord_types::UserActionRequiredFor::CloseCancel,
        volicord_types::UserActionRequiredFor::CloseSupersede,
        volicord_types::UserActionRequiredFor::Informational,
    ];
    let cases = [
        (
            ActionCase::Choice(JudgmentKind::ProductDecision),
            "product",
            [true, true, true, true, false, true, true],
        ),
        (
            ActionCase::Choice(JudgmentKind::TechnicalDecision),
            "technical",
            [true, true, true, true, false, true, true],
        ),
        (
            ActionCase::Choice(JudgmentKind::ScopeDecision),
            "scope",
            [true, true, true, true, false, true, true],
        ),
        (
            ActionCase::Choice(JudgmentKind::SensitiveApproval),
            "sensitive",
            [false, true, true, true, false, true, true],
        ),
        (
            ActionCase::Choice(JudgmentKind::FinalAcceptance),
            "final",
            [false, false, false, true, false, false, true],
        ),
        (
            ActionCase::Choice(JudgmentKind::ResidualRiskAcceptance),
            "risk",
            [false, false, false, true, false, false, true],
        ),
        (
            ActionCase::Choice(JudgmentKind::Cancellation),
            "cancel",
            [false, false, false, false, true, false, true],
        ),
        (
            ActionCase::EvidenceObservation,
            "observation",
            [false, false, true, true, false, false, true],
        ),
    ];

    for (action, action_label, expected) in cases {
        for (requirement_index, (required_for, should_accept)) in
            requirements.into_iter().zip(expected).enumerate()
        {
            let suffix = format!("matrix_{action_label}_{requirement_index}");
            let harness = MethodHarness::new()?;
            let (task_id, change_unit_id) = create_task_with_change_unit(&harness, &suffix)?;
            let mut expected_state_version = 2;
            let mut artifact_ref = None;
            match action {
                ActionCase::Choice(JudgmentKind::FinalAcceptance) => {
                    expected_state_version = record_user_action_matrix_close_basis(
                        &harness,
                        &task_id,
                        &change_unit_id,
                        &suffix,
                        false,
                    )?;
                }
                ActionCase::Choice(JudgmentKind::ResidualRiskAcceptance) => {
                    expected_state_version = record_user_action_matrix_close_basis(
                        &harness,
                        &task_id,
                        &change_unit_id,
                        &suffix,
                        true,
                    )?;
                }
                ActionCase::EvidenceObservation => {
                    let (state_version, artifact) = promote_artifact_for_record_run(
                        &harness,
                        &task_id,
                        &change_unit_id,
                        2,
                        &suffix,
                    )?;
                    expected_state_version = state_version;
                    artifact_ref = Some(artifact);
                }
                ActionCase::Choice(_) => {}
            }

            let mut request = match action {
                ActionCase::Choice(kind) => user_action_request(
                    &format!("req_{suffix}"),
                    &format!("idem_{suffix}"),
                    false,
                    Some(expected_state_version),
                    &task_id,
                    Some(&change_unit_id),
                    kind,
                ),
                ActionCase::EvidenceObservation => observation_action_request(
                    &format!("req_{suffix}"),
                    &format!("idem_{suffix}"),
                    expected_state_version,
                    &task_id,
                    &change_unit_id,
                    supplemental_evidence_target("Artifact registered for corruption coverage."),
                    vec![artifact_ref
                        .as_ref()
                        .expect("observation artifact")
                        .artifact_id
                        .clone()],
                ),
            };
            request.required_for = vec![required_for];
            let before = harness.counts()?;
            let response = harness
                .service
                .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
            let after = harness.counts()?;

            assert_eq!(
                response.response_value["base"]["response_kind"],
                if should_accept { "result" } else { "rejected" },
                "unexpected matrix result for {action:?} / {required_for:?}: {}",
                response.response_value
            );
            if should_accept {
                assert_eq!(after.user_action_requests, before.user_action_requests + 1);
                assert_eq!(after.state_version, before.state_version + 1);
            } else {
                assert_eq!(after, before);
                assert_eq!(
                    response.response_value["errors"][0]["details"]["field"],
                    "required_for"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn duplicate_required_for_rejects_before_dry_run_or_commit_effects() -> Result<(), Box<dyn Error>> {
    for dry_run in [false, true] {
        let harness = MethodHarness::new()?;
        let suffix = if dry_run { "dry" } else { "commit" };
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("duplicate_required_for_{suffix}"))?;
        let mut request = user_action_request(
            &format!("req_duplicate_required_for_{suffix}"),
            &format!("idem_duplicate_required_for_{suffix}"),
            dry_run,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        );
        request.required_for = vec![
            volicord_types::UserActionRequiredFor::RecordRun,
            volicord_types::UserActionRequiredFor::RecordRun,
        ];
        let before = harness.counts()?;

        let response = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(response.response_value["base"]["dry_run"], dry_run);
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["field"],
            "required_for"
        );
        assert_eq!(harness.counts()?, before);
    }
    Ok(())
}

#[test]
fn choice_affected_refs_reject_cross_boundary_refs_without_effect() -> Result<(), Box<dyn Error>> {
    #[derive(Clone, Copy)]
    enum InvalidRef {
        Project,
        TaskContext,
        TaskRecord,
    }

    for (case, field, label) in [
        (
            InvalidRef::Project,
            "action.affected_refs.project_id",
            "cross_project",
        ),
        (
            InvalidRef::TaskContext,
            "action.affected_refs.task_id",
            "cross_task_context",
        ),
        (
            InvalidRef::TaskRecord,
            "action.affected_refs.task_id",
            "cross_task_record",
        ),
    ] {
        for dry_run in [false, true] {
            let harness = MethodHarness::new()?;
            let mode = if dry_run { "dry" } else { "commit" };
            let (task_id, change_unit_id) =
                create_task_with_change_unit(&harness, &format!("affected_ref_{label}_{mode}"))?;
            let mut request = user_action_request(
                &format!("req_affected_ref_{label}_{mode}"),
                &format!("idem_affected_ref_{label}_{mode}"),
                dry_run,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            );
            let affected_ref = match case {
                InvalidRef::Project => StateRecordRef {
                    record_kind: StateRecordKind::ChangeUnit,
                    record_id: RecordId::new(&change_unit_id),
                    project_id: ProjectId::new("project_other"),
                    task_id: Some(TaskId::new(&task_id)).into(),
                    produced_at_state_version: Some(2).into(),
                },
                InvalidRef::TaskContext => StateRecordRef {
                    record_kind: StateRecordKind::ChangeUnit,
                    record_id: RecordId::new(&change_unit_id),
                    project_id: ProjectId::new(PROJECT_ID),
                    task_id: Some(TaskId::new("task_other")).into(),
                    produced_at_state_version: Some(2).into(),
                },
                InvalidRef::TaskRecord => StateRecordRef {
                    record_kind: StateRecordKind::Task,
                    record_id: RecordId::new("task_other"),
                    project_id: ProjectId::new(PROJECT_ID),
                    task_id: None.into(),
                    produced_at_state_version: Some(2).into(),
                },
            };
            let volicord_types::UserActionDraft::Choice(choice) = &mut request.action else {
                unreachable!("product decision fixture is choice-shaped")
            };
            choice.affected_refs = vec![affected_ref];
            let before = harness.counts()?;

            let response = harness
                .service
                .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;

            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(response.response_value["base"]["dry_run"], dry_run);
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "VALIDATION_FAILED"
            );
            assert_eq!(
                response.response_value["errors"][0]["details"]["field"],
                field
            );
            assert_eq!(harness.counts()?, before);
        }
    }
    Ok(())
}

#[test]
fn valid_affected_refs_commit_while_context_refs_remain_display_only() -> Result<(), Box<dyn Error>>
{
    for dry_run in [true, false] {
        let harness = MethodHarness::new()?;
        let mode = if dry_run { "dry" } else { "commit" };
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("display_context_ref_{mode}"))?;
        let mut request = user_action_request(
            &format!("req_display_context_ref_{mode}"),
            &format!("idem_display_context_ref_{mode}"),
            dry_run,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        );
        let display_ref = StateRecordRef {
            record_kind: StateRecordKind::Task,
            record_id: RecordId::new("task_display_only"),
            project_id: ProjectId::new("project_display_only"),
            task_id: Some(TaskId::new("task_display_only")).into(),
            produced_at_state_version: Some(99).into(),
        };
        let volicord_types::UserActionDraft::Choice(choice) = &mut request.action else {
            unreachable!("product decision fixture is choice-shaped")
        };
        choice.affected_refs = vec![StateRecordRef {
            record_kind: StateRecordKind::ChangeUnit,
            record_id: RecordId::new(&change_unit_id),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: Some(TaskId::new(&task_id)).into(),
            produced_at_state_version: Some(2).into(),
        }];
        choice.context.related_refs = vec![display_ref.clone()];
        let before = harness.counts()?;

        let response = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;

        if dry_run {
            assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
            assert_eq!(harness.counts()?, before);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "result");
            let after = harness.counts()?;
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.user_action_requests, before.user_action_requests + 1);
            assert_eq!(
                response.response_value["user_action_request"]["body"]["context"]["related_refs"],
                json!([display_ref])
            );
        }
    }
    Ok(())
}

#[test]
fn all_eight_action_kinds_create_one_canonical_pending_request() -> Result<(), Box<dyn Error>> {
    let choice_cases = [
        (JudgmentKind::ProductDecision, "product_decision"),
        (JudgmentKind::TechnicalDecision, "technical_decision"),
        (JudgmentKind::ScopeDecision, "scope_decision"),
        (JudgmentKind::SensitiveApproval, "sensitive_approval"),
        (JudgmentKind::FinalAcceptance, "final_acceptance"),
        (
            JudgmentKind::ResidualRiskAcceptance,
            "residual_risk_acceptance",
        ),
        (JudgmentKind::Cancellation, "cancellation"),
    ];

    for (index, (kind, expected_kind)) in choice_cases.into_iter().enumerate() {
        let harness = MethodHarness::new()?;
        let suffix = format!("action_kind_{index}");
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, &suffix)?;
        if kind == JudgmentKind::FinalAcceptance {
            record_close_evidence(&harness, &task_id, &change_unit_id, 2, &suffix, true)?;
        } else if kind == JudgmentKind::ResidualRiskAcceptance {
            record_close_basis_with_risks(
                &harness,
                &task_id,
                &change_unit_id,
                2,
                &suffix,
                vec![residual_risk_input("A visible residual risk remains.")],
            )?;
        }
        let expected_state_version = current_state_version(&harness)?;
        let before = harness.counts()?;
        let response = harness.service.request_user_action(
            user_action_request(
                &format!("req_{suffix}"),
                &format!("idem_{suffix}"),
                false,
                Some(expected_state_version),
                &task_id,
                Some(&change_unit_id),
                kind,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let after = harness.counts()?;

        assert_eq!(
            response.response_value["base"]["response_kind"], "result",
            "action kind {kind:?} should commit: {}",
            response.response_value
        );
        assert_eq!(
            response.response_value["user_action_request"]["action_kind"],
            expected_kind
        );
        assert_eq!(
            response.response_value["user_action_request"]["status"],
            "pending"
        );
        assert_eq!(after.user_action_requests, before.user_action_requests + 1);
        assert_eq!(
            after.user_action_resolutions,
            before.user_action_resolutions
        );
        assert_eq!(after.state_version, before.state_version + 1);
    }

    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "action_kind_observation")?;
    let (state_version, artifact_ref) = promote_artifact_for_record_run(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "action_kind_observation",
    )?;
    let before = harness.counts()?;
    let response = harness.service.request_user_action(
        observation_action_request(
            "req_action_kind_observation",
            "idem_action_kind_observation",
            state_version,
            &task_id,
            &change_unit_id,
            supplemental_evidence_target("Artifact registered for corruption coverage."),
            vec![artifact_ref.artifact_id.clone()],
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after = harness.counts()?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["user_action_request"]["action_kind"],
        "evidence_observation"
    );
    assert_eq!(after.user_action_requests, before.user_action_requests + 1);
    assert_eq!(
        after.user_action_resolutions,
        before.user_action_resolutions
    );
    Ok(())
}

#[test]
fn agent_connection_cannot_resolve_user_action() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "agent_denied")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_agent_denied_request",
            "idem_agent_denied_request",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;
    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_agent_denied_resolve",
            "submission_agent_denied",
            None,
            &task_id,
            &request_id(&requested),
            "accept",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_action_status(&harness, &request_id(&requested))?,
        "pending"
    );

    let git_bound_user_invocation = invocation(OperationCategory::UserOnly)
        .with_git_workspace_context(crate::GitWorkspaceContext {
            git_common_dir: "/tmp/volicord-user-only-resolution.git".to_owned(),
            worktree_id: format!("sha256:{}", "a".repeat(64)),
            branch_ref: Some("refs/heads/user-only-resolution".to_owned()),
            head_sha: Some("b".repeat(40)),
            workspace_fingerprint: format!("sha256:{}", "c".repeat(64)),
        });
    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_user_git_denied_resolve",
            "submission_user_git_denied",
            None,
            &task_id,
            &request_id(&requested),
            "accept",
        ),
        git_bound_user_invocation,
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_action_status(&harness, &request_id(&requested))?,
        "pending"
    );
    Ok(())
}

#[test]
fn resolution_omits_expected_state_and_survives_unrelated_commit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "current_pin")?;
    let first = harness.service.request_user_action(
        user_action_request(
            "req_current_pin_first",
            "idem_current_pin_first",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let mut unrelated = user_action_request(
        "req_current_pin_unrelated",
        "idem_current_pin_unrelated",
        false,
        Some(3),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::TechnicalDecision,
    );
    unrelated.required_for = vec![volicord_types::UserActionRequiredFor::Informational];
    let second = harness
        .service
        .request_user_action(unrelated, invocation(OperationCategory::AgentWorkflow))?;
    let before = harness.counts()?;
    let request = resolve_user_action_request(
        "req_current_pin_resolve",
        "submission_current_pin",
        None,
        &task_id,
        &request_id(&first),
        "accept",
    );
    assert!(request.envelope.expected_state_version.as_ref().is_none());
    let resolved = harness
        .service
        .resolve_user_action(request, invocation(OperationCategory::UserOnly))?;
    let after = harness.counts()?;

    assert_eq!(resolved.response_value["base"]["response_kind"], "result");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(
        after.user_action_resolutions,
        before.user_action_resolutions + 1
    );
    assert_eq!(
        user_action_status(&harness, &request_id(&first))?,
        "resolved"
    );
    assert_eq!(
        user_action_status(&harness, &request_id(&second))?,
        "pending"
    );
    Ok(())
}

#[test]
fn resolution_uses_core_clock_at_expiry_boundary() -> Result<(), Box<dyn Error>> {
    for (suffix, advance, expected_kind) in [
        ("before", Duration::seconds(9), "result"),
        ("equal", Duration::seconds(10), "rejected"),
    ] {
        let mut harness = MethodHarness::new()?;
        let clock = ManualClock::at("2026-07-13T00:00:00Z");
        harness.use_clock(clock.clone());
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
        let mut request = user_action_request(
            &format!("req_expiry_{suffix}"),
            &format!("idem_expiry_{suffix}"),
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        );
        request.expires_at =
            Some(volicord_types::UtcTimestamp::parse("2026-07-13T00:00:10Z")?).into();
        let requested = harness
            .service
            .request_user_action(request, invocation(OperationCategory::AgentWorkflow))?;
        clock.advance(advance);
        let before = harness.counts()?;
        let response = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!("req_expiry_resolve_{suffix}"),
                &format!("submission_expiry_{suffix}"),
                None,
                &task_id,
                &request_id(&requested),
                "accept",
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        assert_eq!(
            response.response_value["base"]["response_kind"],
            expected_kind
        );
        if expected_kind == "result" {
            assert_eq!(
                harness.counts()?.user_action_resolutions,
                before.user_action_resolutions + 1
            );
        } else {
            assert_eq!(harness.counts()?, before);
            let status = harness.service.status(
                StatusRequest {
                    envelope: envelope(
                        &format!("req_expiry_status_{suffix}"),
                        None,
                        false,
                        None,
                        Some(&task_id),
                    ),
                    include: StatusInclude {
                        pending_user_actions: true,
                        ..status_include()
                    },
                },
                invocation(OperationCategory::Read),
            )?;
            assert!(status.response_value["pending_user_actions"]
                .as_array()
                .expect("pending actions")
                .is_empty());
            assert!(status.response_value["pending_user_action_inbox_items"]
                .as_array()
                .expect("pending inbox")
                .is_empty());
        }
    }
    Ok(())
}

#[test]
fn resolution_rejects_explicit_future_requested_at_corruption_without_effects(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-07-13T00:00:10Z");
    harness.use_clock(clock);
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "future_requested_at")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_future_requested_at",
            "idem_future_requested_at",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    harness.conn()?.execute(
        "UPDATE user_action_requests
            SET requested_at = '2026-07-13T00:00:10.001Z'
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, request_id(&requested)],
    )?;
    let (requested_at, project_floor): (String, String) = harness.conn()?.query_row(
        "SELECT request.requested_at, project.updated_at
           FROM user_action_requests AS request
           JOIN project_state AS project
             ON project.project_id = request.project_id
          WHERE request.project_id = ?1
            AND request.user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, request_id(&requested)],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(requested_at, "2026-07-13T00:00:10.001Z");
    assert_eq!(project_floor, "2026-07-13T00:00:10Z");
    let before = harness.counts()?;

    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_resolve_future_requested_at",
            "submission_resolve_future_requested_at",
            None,
            &task_id,
            &request_id(&requested),
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["owner_state_error"]["logical_column"],
        "requested_at"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn null_to_current_change_unit_transition_supersedes_pending_request() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_null_cu_task",
            "idem_null_cu_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_null_cu_action",
            "idem_null_cu_action",
            false,
            Some(1),
            &task_id,
            None,
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    harness.service.update_scope(
        update_scope_request(
            "req_null_cu_scope",
            "idem_null_cu_scope",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Create the first current Change Unit.",
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let before = harness.counts()?;
    let response = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_null_cu_resolve",
            "submission_null_cu",
            None,
            &task_id,
            &request_id(&requested),
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_action_basis_status(&harness, &request_id(&requested))?,
        "superseded"
    );
    Ok(())
}

#[test]
fn exact_replay_is_stable_and_payload_conflict_has_no_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "replay")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_replay_action",
            "idem_replay_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let request = resolve_user_action_request(
        "req_replay_resolution",
        "submission_replay_resolution",
        None,
        &task_id,
        &request_id(&requested),
        "accept",
    );
    let first = harness
        .service
        .resolve_user_action(request.clone(), invocation(OperationCategory::UserOnly))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .resolve_user_action(request.clone(), invocation(OperationCategory::UserOnly))?;
    assert_eq!(second.response_value, first.response_value);
    assert!(second.replayed);
    assert_eq!(harness.counts()?, after_first);

    let cross_channel = harness.service.resolve_user_action(
        request.clone(),
        InvocationContext::new(
            ProjectId::new(PROJECT_ID),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            volicord_types::VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
        ),
    )?;
    assert_eq!(
        cross_channel.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        cross_channel.response_value["errors"][0]["code"],
        "INVOCATION_CONTEXT_MISMATCH"
    );
    assert_eq!(harness.counts()?, after_first);

    let mut conflict = request;
    conflict.resolution = volicord_types::UserActionResolutionInput::Choice {
        selected_option_id: volicord_types::UserActionOptionId::new("decline"),
        note: Some("A different payload must conflict.".to_owned()).into(),
    };
    let rejected = harness
        .service
        .resolve_user_action(conflict, invocation(OperationCategory::UserOnly))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn resolution_submission_id_enforces_256_visible_ascii_bytes_at_core_boundary(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "submission_id_boundary")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_submission_id_boundary_request",
            "idem_submission_id_boundary_request",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let action_id = request_id(&requested);
    let before = harness.counts()?;
    let too_long = "x".repeat(257);
    let rejected = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_submission_id_boundary_reject",
            &too_long,
            None,
            &task_id,
            &action_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(user_action_status(&harness, &action_id)?, "pending");

    let accepted = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_submission_id_boundary_accept",
            &"x".repeat(256),
            None,
            &task_id,
            &action_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    assert_eq!(accepted.response_value["base"]["response_kind"], "result");
    assert_eq!(user_action_status(&harness, &action_id)?, "resolved");
    Ok(())
}

#[test]
fn same_connection_resume_replays_exact_origin_after_state_advance_and_denies_other_connection(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-07-13T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "resume_origin")?;
    let original = harness.service.request_user_action(
        user_action_request(
            "req_resume_origin",
            "idem_resume_origin",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id = request_id(&original);
    let original_state_version = original.response_value["base"]["state_version"]
        .as_u64()
        .expect("origin state version");
    let after_origin = harness.counts()?;

    let resumed = harness
        .service
        .resume_user_action_request(
            ProjectId::new(PROJECT_ID),
            volicord_types::UserActionRequestId::new(&user_action_request_id),
            invocation(OperationCategory::AgentWorkflow),
        )?
        .expect("same connection should resume a direct request-user-action origin");
    assert!(resumed.replayed);
    assert_eq!(resumed.response_json, original.response_json);
    assert_eq!(resumed.response_value, original.response_value);
    assert_eq!(resumed.operation_result_ref, original.operation_result_ref);
    assert_eq!(harness.counts()?, after_origin);

    let rotated_basis = InvocationContext::new(
        ProjectId::new(PROJECT_ID),
        ActorSource::agent_connection(CONNECTION_ID),
        OperationCategory::AgentWorkflow,
        "rotated_agent_registration_basis",
    );
    let changed_workspace = invocation(OperationCategory::AgentWorkflow)
        .with_git_workspace_context(crate::GitWorkspaceContext {
            git_common_dir: "/tmp/volicord-resume-changed-workspace.git".to_owned(),
            worktree_id: format!("sha256:{}", "a".repeat(64)),
            branch_ref: Some("refs/heads/resume-changed".to_owned()),
            head_sha: Some("b".repeat(40)),
            workspace_fingerprint: format!("sha256:{}", "c".repeat(64)),
        });
    for changed_context in [rotated_basis, changed_workspace] {
        let recovered = harness
            .service
            .resume_user_action_request(
                ProjectId::new(PROJECT_ID),
                volicord_types::UserActionRequestId::new(&user_action_request_id),
                changed_context,
            )?
            .expect("same Agent Connection retains read-only origin recovery access");
        assert!(recovered.replayed);
        assert_eq!(recovered.response_json, original.response_json);
        assert_eq!(harness.counts()?, after_origin);
    }

    clock.advance(Duration::seconds(5));
    let unrelated = harness.service.request_user_action(
        user_action_request(
            "req_resume_unrelated",
            "idem_resume_unrelated",
            false,
            Some(original_state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let advanced_state_version = unrelated.response_value["base"]["state_version"]
        .as_u64()
        .expect("advanced state version");
    assert!(advanced_state_version > original_state_version);

    let current = harness
        .service
        .current_user_action_projection(
            &ProjectId::new(PROJECT_ID),
            &volicord_types::UserActionRequestId::new(&user_action_request_id),
        )?
        .expect("origin should retain a current projection");
    assert_eq!(current.observed_state_version, advanced_state_version);
    assert_eq!(
        current.observed_at,
        volicord_types::UtcTimestamp::parse("2026-07-13T00:00:05Z")?
    );
    assert_eq!(current.status, volicord_types::UserActionStatus::Pending);
    let after_advance = harness.counts()?;

    let resumed_after_advance = harness
        .service
        .resume_user_action_request(
            ProjectId::new(PROJECT_ID),
            volicord_types::UserActionRequestId::new(&user_action_request_id),
            invocation(OperationCategory::AgentWorkflow),
        )?
        .expect("same connection should resume after unrelated state advance");
    assert!(resumed_after_advance.replayed);
    assert_eq!(resumed_after_advance.response_json, original.response_json);
    assert_eq!(
        resumed_after_advance.operation_result_ref,
        original.operation_result_ref
    );
    assert_eq!(harness.counts()?, after_advance);

    let wrong_connection = harness.service.resume_user_action_request(
        ProjectId::new(PROJECT_ID),
        volicord_types::UserActionRequestId::new(user_action_request_id),
        invocation_with_actor(
            ActorSource::agent_connection("connection_other"),
            OperationCategory::AgentWorkflow,
        ),
    )?;
    assert!(wrong_connection.is_none());
    assert_eq!(harness.counts()?, after_advance);
    Ok(())
}

#[test]
fn origin_resume_rejects_tampered_request_and_inbox_authority_without_effect(
) -> Result<(), Box<dyn Error>> {
    for variant in ["request_body", "inbox_question", "inbox_form"] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("origin_tamper_{variant}"))?;
        let idempotency_key = format!("idem_origin_tamper_{variant}");
        let original = harness.service.request_user_action(
            user_action_request(
                &format!("req_origin_tamper_{variant}"),
                &idempotency_key,
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let action_id = request_id(&original);
        let mut tampered = original.response_value.clone();
        match variant {
            "request_body" => {
                tampered["user_action_request"]["body"]["question"] =
                    json!("Tampered immutable question");
            }
            "inbox_question" => {
                tampered["inbox_item"]["question"] = json!("Tampered inbox question");
            }
            "inbox_form" => {
                tampered["inbox_item"]["form"]["note_max_chars"] = json!(1);
            }
            _ => unreachable!(),
        }
        harness.conn()?.execute(
            "UPDATE tool_invocations
                SET response_json = ?4
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            rusqlite::params![
                PROJECT_ID,
                MethodName::RequestUserAction.as_str(),
                idempotency_key,
                serde_json::to_string(&tampered)?
            ],
        )?;
        let before = harness.counts()?;
        let before_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;

        let result = harness.service.resume_user_action_request(
            ProjectId::new(PROJECT_ID),
            UserActionRequestId::new(action_id),
            invocation(OperationCategory::AgentWorkflow),
        );
        assert!(result.is_err(), "variant {variant} must fail closed");
        assert_eq!(harness.counts()?, before, "variant {variant}");
        let after_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor, "variant {variant}");
    }
    Ok(())
}

#[test]
fn read_snapshot_prevents_mixed_projection_rows_across_concurrent_resolution_commit(
) -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    let clock = ManualClock::at("2026-07-13T00:00:00Z");
    harness.use_clock(clock.clone());
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "read_snapshot")?;
    let original = harness.service.request_user_action(
        user_action_request(
            "req_read_snapshot_origin",
            "idem_read_snapshot_origin",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id = request_id(&original);
    let origin_state_version = original.response_value["base"]["state_version"]
        .as_u64()
        .expect("origin state version");

    let conn = harness.conn()?;
    let journal_mode = conn.query_row("PRAGMA journal_mode = WAL", [], |row| {
        row.get::<_, String>(0)
    })?;
    assert_eq!(journal_mode, "wal");
    drop(conn);

    let store =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let runtime_home = harness.runtime_home_path.clone();
    let writer_task_id = task_id.clone();
    let writer_action_id = user_action_request_id.clone();
    let writer_clock = clock.clone();
    let snapshot_now = volicord_types::UtcTimestamp::parse("2026-07-13T00:00:01Z")?;
    let (snapshot_state_version, committed_state_version) =
        store.with_read_snapshot(|snapshot| {
            let snapshot_state_version = snapshot.project_state()?.state_version;
            assert_eq!(snapshot_state_version, origin_state_version);

            writer_clock.advance(Duration::seconds(1));
            let writer = std::thread::spawn(move || {
                CoreService::with_clock(&runtime_home, writer_clock)
                    .resolve_user_action(
                        resolve_user_action_request(
                            "req_read_snapshot_resolution",
                            "submission_read_snapshot_resolution",
                            None,
                            &writer_task_id,
                            &writer_action_id,
                            "accept",
                        ),
                        invocation(OperationCategory::UserOnly),
                    )
                    .expect("concurrent resolution should commit while the WAL snapshot is open")
            });
            let committed = writer
                .join()
                .expect("concurrent resolution thread should not panic");
            let committed_state_version = committed.response_value["base"]["state_version"]
                .as_u64()
                .unwrap_or_else(|| {
                    panic!(
                        "resolution should commit with a state version: {}",
                        committed.response_json
                    )
                });
            assert!(committed_state_version > snapshot_state_version);

            assert_eq!(
                snapshot.project_state()?.state_version,
                snapshot_state_version
            );
            let record = snapshot
                .user_action_record(&user_action_request_id, &snapshot_now)?
                .expect("origin request should remain visible in its read snapshot");
            assert_eq!(record.status, volicord_types::UserActionStatus::Pending);
            assert!(record.resolution.is_none());
            let origin_replay = snapshot
                .tool_invocation(
                    MethodName::RequestUserAction,
                    &IdempotencyKey::new("idem_read_snapshot_origin"),
                )?
                .expect("origin replay should remain visible in its read snapshot");
            assert_eq!(
                origin_replay.committed_state_version,
                snapshot_state_version
            );
            assert!(snapshot
                .tool_invocation(
                    MethodName::ResolveUserAction,
                    &IdempotencyKey::new("submission_read_snapshot_resolution"),
                )?
                .is_none());

            Ok((snapshot_state_version, committed_state_version))
        })?;

    let fresh =
        CoreProjectStore::open_read_only(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    assert_eq!(
        fresh.project_state()?.state_version,
        committed_state_version
    );
    assert!(committed_state_version > snapshot_state_version);
    let resolved = fresh
        .user_action_record(&user_action_request_id, &snapshot_now)?
        .expect("fresh read should observe the resolved request");
    assert_eq!(resolved.status, volicord_types::UserActionStatus::Resolved);
    assert!(resolved.resolution.is_some());
    let resolution_replay = fresh
        .tool_invocation(
            MethodName::ResolveUserAction,
            &IdempotencyKey::new("submission_read_snapshot_resolution"),
        )?
        .expect("fresh read should observe the resolution replay");
    assert_eq!(
        resolution_replay.committed_state_version,
        committed_state_version
    );
    Ok(())
}

#[test]
fn concurrent_distinct_submissions_create_only_one_resolution() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "concurrent")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_concurrent_action",
            "idem_concurrent_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let action_id = request_id(&requested);
    let runtime_home = harness.runtime_home_path.clone();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut outcomes = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for index in 0..2 {
            let runtime_home = runtime_home.clone();
            let task_id = task_id.clone();
            let action_id = action_id.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(scope.spawn(move || {
                let service = CoreService::new(runtime_home);
                let request = resolve_user_action_request(
                    &format!("req_concurrent_resolution_{index}"),
                    &format!("submission_concurrent_{index}"),
                    None,
                    &task_id,
                    &action_id,
                    "accept",
                );
                barrier.wait();
                service
                    .resolve_user_action(request, invocation(OperationCategory::UserOnly))
                    .map(|response| {
                        response.response_value["base"]["response_kind"]
                            .as_str()
                            .unwrap_or("missing")
                            .to_owned()
                    })
                    .map_err(|error| error.to_string())
            }));
        }
        for handle in handles {
            outcomes.push(handle.join().expect("resolution thread should not panic"));
        }
    });
    let outcomes = outcomes.into_iter().collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        outcomes.iter().filter(|value| *value == "result").count(),
        1
    );
    assert_eq!(
        outcomes.iter().filter(|value| *value == "rejected").count(),
        1
    );
    assert_eq!(
        harness.counts()?.user_action_resolutions,
        1,
        "the one-to-one storage invariant must win the race"
    );
    Ok(())
}

#[test]
fn observation_resolution_rejects_candidate_mismatch_and_changed_bytes(
) -> Result<(), Box<dyn Error>> {
    for scenario in ["candidate", "bytes", "content_type"] {
        let harness = MethodHarness::new()?;
        let suffix = scenario;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, suffix)?;
        let (state_version, artifact_ref) =
            promote_artifact_for_record_run(&harness, &task_id, &change_unit_id, 2, suffix)?;
        let target = supplemental_evidence_target("Artifact registered for corruption coverage.");
        let requested = harness.service.request_user_action(
            observation_action_request(
                &format!("req_observation_{suffix}"),
                &format!("idem_observation_{suffix}"),
                state_version,
                &task_id,
                &change_unit_id,
                target.clone(),
                vec![artifact_ref.artifact_id.clone()],
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(
            requested.response_value["base"]["response_kind"], "result",
            "observation request should commit: {}",
            requested.response_value
        );
        if scenario == "bytes" {
            fs::write(
                persistent_artifact_body_path(&harness, artifact_ref.artifact_id.as_str())?,
                b"changed after request",
            )?;
        } else if scenario == "content_type" {
            harness.conn()?.execute(
                "UPDATE artifacts SET content_type = 'text/plain'
                  WHERE project_id = ?1 AND artifact_id = ?2",
                rusqlite::params![PROJECT_ID, artifact_ref.artifact_id.as_str()],
            )?;
        }
        let before = harness.counts()?;
        let response = harness.service.resolve_user_action(
            resolve_observation_request(
                &format!("req_observation_resolve_{suffix}"),
                &format!("submission_observation_{suffix}"),
                &task_id,
                &request_id(&requested),
                if scenario == "candidate" {
                    supplemental_evidence_target("A target not in the stored form.")
                } else {
                    target
                },
                vec![artifact_ref.artifact_id.clone()],
                EvidenceRelevanceStatus::Supported,
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        assert_eq!(
            response.response_value["base"]["response_kind"], "rejected",
            "{scenario} mutation must reject: {}",
            response.response_value
        );
        assert_eq!(harness.counts()?, before);
    }
    Ok(())
}

#[test]
fn immediate_projection_matches_authoritative_status_after_each_commit(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "projection")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_projection_action",
            "idem_projection_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_request = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_projection_status_pending",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let mut immediate_state = requested.response_value["state"].clone();
    let mut reread_state = after_request.response_value["active_task"].clone();
    immediate_state
        .as_object_mut()
        .expect("immediate state")
        .remove("guarantee_display");
    immediate_state
        .as_object_mut()
        .expect("immediate state")
        .remove("guard_health");
    reread_state
        .as_object_mut()
        .expect("reread state")
        .remove("guarantee_display");
    reread_state
        .as_object_mut()
        .expect("reread state")
        .remove("guard_health");
    assert_eq!(immediate_state, reread_state);
    assert_eq!(
        requested.response_value["state"]["pending_user_action_refs"],
        after_request.response_value["pending_user_actions"]
    );

    let resolved = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_projection_resolution",
            "submission_projection_resolution",
            None,
            &task_id,
            &request_id(&requested),
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let after_resolution = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_projection_status_resolved",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    let mut immediate_state = resolved.response_value["state"].clone();
    let mut reread_state = after_resolution.response_value["active_task"].clone();
    immediate_state
        .as_object_mut()
        .expect("immediate state")
        .remove("guarantee_display");
    immediate_state
        .as_object_mut()
        .expect("immediate state")
        .remove("guard_health");
    reread_state
        .as_object_mut()
        .expect("reread state")
        .remove("guarantee_display");
    reread_state
        .as_object_mut()
        .expect("reread state")
        .remove("guard_health");
    assert_eq!(immediate_state, reread_state);
    assert!(after_resolution.response_value["pending_user_actions"]
        .as_array()
        .expect("pending actions")
        .is_empty());
    Ok(())
}

#[test]
fn current_projection_rejects_tampered_resolution_replay_context_and_output_without_effect(
) -> Result<(), Box<dyn Error>> {
    for variant in [
        "resolution_body",
        "resolution_ref_version",
        "derived_refs",
        "verification_basis",
        "git_workspace_context",
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("resolution_tamper_{variant}"))?;
        let requested = harness.service.request_user_action(
            user_action_request(
                &format!("req_resolution_tamper_request_{variant}"),
                &format!("idem_resolution_tamper_request_{variant}"),
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;
        let action_id = request_id(&requested);
        let submission_id = format!("submission_resolution_tamper_{variant}");
        let resolved = harness.service.resolve_user_action(
            resolve_user_action_request(
                &format!("req_resolution_tamper_resolve_{variant}"),
                &submission_id,
                None,
                &task_id,
                &action_id,
                "accept",
            ),
            invocation(OperationCategory::UserOnly),
        )?;
        assert_eq!(resolved.response_value["base"]["response_kind"], "result");

        match variant {
            "resolution_body" | "resolution_ref_version" | "derived_refs" => {
                let mut response = resolved.response_value.clone();
                if variant == "resolution_body" {
                    response["user_action_resolution"]["body"]["note"] =
                        json!("tampered private note");
                } else if variant == "resolution_ref_version" {
                    let version = response["user_action_resolution_ref"]
                        ["produced_at_state_version"]
                        .as_u64()
                        .expect("resolution ref version");
                    response["user_action_resolution_ref"]["produced_at_state_version"] =
                        json!(version + 1);
                } else {
                    assert!(
                        !response["derived_refs"]
                            .as_array()
                            .expect("derived refs array")
                            .is_empty(),
                        "accepted decision fixture must derive continuity"
                    );
                    response["derived_refs"] = json!([]);
                }
                harness.conn()?.execute(
                    "UPDATE tool_invocations
                        SET response_json = ?4
                      WHERE project_id = ?1
                        AND tool_name = ?2
                        AND idempotency_key = ?3",
                    rusqlite::params![
                        PROJECT_ID,
                        MethodName::ResolveUserAction.as_str(),
                        submission_id,
                        serde_json::to_string(&response)?
                    ],
                )?;
            }
            "verification_basis" => {
                harness.conn()?.execute(
                    "UPDATE tool_invocations
                        SET verification_basis = 'tampered_user_channel_basis'
                      WHERE project_id = ?1
                        AND tool_name = ?2
                        AND idempotency_key = ?3",
                    rusqlite::params![
                        PROJECT_ID,
                        MethodName::ResolveUserAction.as_str(),
                        submission_id
                    ],
                )?;
            }
            "git_workspace_context" => {
                let git_context = json!({
                    "git_common_dir": "/tmp/volicord-resolution-tamper.git",
                    "worktree_id": format!("sha256:{}", "a".repeat(64)),
                    "branch_ref": "refs/heads/tamper",
                    "head_sha": "b".repeat(40),
                    "workspace_fingerprint": format!("sha256:{}", "c".repeat(64))
                });
                harness.conn()?.execute(
                    "UPDATE tool_invocations
                        SET git_workspace_context_json = ?4
                      WHERE project_id = ?1
                        AND tool_name = ?2
                        AND idempotency_key = ?3",
                    rusqlite::params![
                        PROJECT_ID,
                        MethodName::ResolveUserAction.as_str(),
                        submission_id,
                        serde_json::to_string(&git_context)?
                    ],
                )?;
            }
            _ => unreachable!(),
        }
        let before = harness.counts()?;
        let before_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        let projection = harness.service.current_user_action_projection(
            &ProjectId::new(PROJECT_ID),
            &UserActionRequestId::new(action_id),
        );
        assert!(projection.is_err(), "variant {variant} must fail closed");
        assert_eq!(harness.counts()?, before, "variant {variant}");
        let after_floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, before_floor, "variant {variant}");
    }
    Ok(())
}

#[test]
fn accepted_decision_continuity_has_no_fabricated_rationale() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "continuity")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_continuity_action",
            "idem_continuity_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let mut resolution = resolve_user_action_request(
        "req_continuity_resolution",
        "submission_continuity_resolution",
        None,
        &task_id,
        &request_id(&requested),
        "accept",
    );
    resolution.resolution = volicord_types::UserActionResolutionInput::Choice {
        selected_option_id: volicord_types::UserActionOptionId::new("accept"),
        note: None.into(),
    };
    let response = harness
        .service
        .resolve_user_action(resolution, invocation(OperationCategory::UserOnly))?;
    let rows = harness.continuity_records()?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "decision");
    let rationale: Option<String> = harness.conn()?.query_row(
        "SELECT rationale
           FROM project_continuity_records
          WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert_eq!(rationale, None);
    assert!(response.response_value["derived_refs"]
        .as_array()
        .expect("derived refs")
        .iter()
        .any(|record_ref| record_ref["record_kind"] == "project_continuity_record"));
    Ok(())
}

#[test]
fn later_continuity_with_older_resolution_as_supplemental_source_does_not_pollute_projection(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "continuity_supplemental_source")?;
    let first_request = harness.service.request_user_action(
        user_action_request(
            "req_continuity_supplemental_first",
            "idem_continuity_supplemental_first",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let first_action_id = request_id(&first_request);
    let first_resolution = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_continuity_supplemental_first_resolution",
            "submission_continuity_supplemental_first",
            None,
            &task_id,
            &first_action_id,
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let first_resolution_ref: StateRecordRef = serde_json::from_value(
        first_resolution.response_value["user_action_resolution_ref"].clone(),
    )?;
    let expected_first_derived: Vec<StateRecordRef> =
        serde_json::from_value(first_resolution.response_value["derived_refs"].clone())?;
    assert_eq!(expected_first_derived.len(), 1);

    let second_request = harness.service.request_user_action(
        user_action_request(
            "req_continuity_supplemental_second",
            "idem_continuity_supplemental_second",
            false,
            Some(current_state_version(&harness)?),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let second_resolution = harness.service.resolve_user_action(
        resolve_user_action_request(
            "req_continuity_supplemental_second_resolution",
            "submission_continuity_supplemental_second",
            None,
            &task_id,
            &request_id(&second_request),
            "accept",
        ),
        invocation(OperationCategory::UserOnly),
    )?;
    let second_continuity_id = second_resolution.response_value["derived_refs"][0]["record_id"]
        .as_str()
        .expect("second continuity id")
        .to_owned();
    let source_refs_json: String = harness.conn()?.query_row(
        "SELECT source_refs_json
           FROM project_continuity_records
          WHERE project_id = ?1
            AND continuity_record_id = ?2",
        rusqlite::params![PROJECT_ID, second_continuity_id],
        |row| row.get(0),
    )?;
    let mut source_refs: Vec<StateRecordRef> = serde_json::from_str(&source_refs_json)?;
    assert_ne!(source_refs.first(), Some(&first_resolution_ref));
    source_refs.push(first_resolution_ref);
    harness.conn()?.execute(
        "UPDATE project_continuity_records
            SET source_refs_json = ?3
          WHERE project_id = ?1
            AND continuity_record_id = ?2",
        rusqlite::params![
            PROJECT_ID,
            second_continuity_id,
            serde_json::to_string(&source_refs)?
        ],
    )?;
    let before = harness.counts()?;

    let projection = harness
        .service
        .current_user_action_projection(
            &ProjectId::new(PROJECT_ID),
            &UserActionRequestId::new(first_action_id),
        )?
        .expect("first resolution projection remains readable");
    assert_eq!(projection.derived_refs, expected_first_derived);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn residual_risk_private_note_stays_only_in_resolution_body() -> Result<(), Box<dyn Error>> {
    const PRIVATE_NOTE: &str = "PRIVATE_RESIDUAL_RISK_NOTE_SENTINEL_7f4c9e";

    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "private_residual_risk_note")?;
    let (after_basis, _) = record_close_basis_with_risks(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "private_residual_risk_note",
        vec![residual_risk_input(
            "A private-note regression risk remains.",
        )],
    )?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_private_residual_risk_action",
            "idem_private_residual_risk_action",
            false,
            Some(after_basis),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id = request_id(&requested);
    let mut resolution = resolve_user_action_request(
        "req_private_residual_risk_resolution",
        "submission_private_residual_risk_resolution",
        None,
        &task_id,
        &user_action_request_id,
        "accept",
    );
    resolution.resolution = volicord_types::UserActionResolutionInput::Choice {
        selected_option_id: volicord_types::UserActionOptionId::new("accept"),
        note: Some(PRIVATE_NOTE.to_owned()).into(),
    };
    let resolved = harness
        .service
        .resolve_user_action(resolution, invocation(OperationCategory::UserOnly))?;
    assert_eq!(resolved.response_value["base"]["response_kind"], "result");

    let resolution_json: String = harness.conn()?.query_row(
        "SELECT resolution_json
           FROM user_action_resolutions
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![PROJECT_ID, user_action_request_id],
        |row| row.get(0),
    )?;
    assert!(resolution_json.contains(PRIVATE_NOTE));
    let continuity_rows: Vec<(String, Option<String>)> = {
        let conn = harness.conn()?;
        let mut stmt = conn.prepare(
            "SELECT kind, rationale
               FROM project_continuity_records
              WHERE project_id = ?1
              ORDER BY continuity_record_id",
        )?;
        let rows = stmt.query_map([PROJECT_ID], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    assert_eq!(continuity_rows, vec![("accepted_risk".to_owned(), None)]);

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_private_residual_risk_status",
                None,
                false,
                None,
                Some(&task_id),
            ),
            include: status_include(),
        },
        invocation(OperationCategory::Read),
    )?;
    assert!(!status.response_json.contains(PRIVATE_NOTE));

    let agent_safe = harness
        .service
        .current_user_action_projection(
            &ProjectId::new(PROJECT_ID),
            &volicord_types::UserActionRequestId::new(&user_action_request_id),
        )?
        .expect("resolved user action should have a current safe projection");
    assert_eq!(
        agent_safe.status,
        volicord_types::UserActionStatus::Resolved
    );
    let agent_safe_json = serde_json::to_string(&agent_safe.user_action_resolution)?;
    assert!(!agent_safe_json.contains(PRIVATE_NOTE));

    let source_replay_response: String = harness.conn()?.query_row(
        "SELECT response_json
           FROM tool_invocations
          WHERE project_id = ?1
            AND operation_category = 'user_only'",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    assert!(source_replay_response.contains(PRIVATE_NOTE));

    let store = CoreProjectStore::open(&harness.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
    let repo_root = store.project_record().repo_root.clone();
    drop(store);
    let export = volicord_store::export::read_authority_bundle_snapshot(
        &harness.runtime_home_path,
        &repo_root,
    )?;
    let mut exported_resolution_rows = 0;
    let mut exported_continuity_rows = 0;
    let mut exported_user_only_replay_rows = 0;
    for record in export.records {
        let row_json = serde_json::to_string(&record.row)?;
        if record.table == "user_action_resolutions" {
            exported_resolution_rows += 1;
            assert!(row_json.contains(PRIVATE_NOTE));
        } else {
            assert!(
                !row_json.contains(PRIVATE_NOTE),
                "private note escaped through exported {} row",
                record.table
            );
        }
        if record.table == "project_continuity_records" {
            exported_continuity_rows += 1;
            assert!(record.row["rationale"].is_null());
        }
        if record.table == "tool_invocations" && record.row["operation_category"] == "user_only" {
            exported_user_only_replay_rows += 1;
            assert!(record.row["response_json"].is_null());
        }
    }
    assert_eq!(exported_resolution_rows, 1);
    assert_eq!(exported_continuity_rows, 1);
    assert_eq!(exported_user_only_replay_rows, 1);

    let diagnostics_path =
        volicord_store::diagnostics::diagnostics_db_path(&harness.runtime_home_path);
    assert!(!diagnostics_path.exists());
    Ok(())
}

#[test]
fn canonical_bounds_reject_n_plus_one_empty_and_over_32k_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bounds")?;

    let mut at_limit = user_action_request(
        "req_bounds_n",
        "idem_bounds_n",
        false,
        Some(2),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    let volicord_types::UserActionDraft::Choice(choice) = &mut at_limit.action else {
        unreachable!("choice fixture")
    };
    choice.options = Some(
        (0..volicord_types::USER_ACTION_TARGET_CANDIDATE_LIMIT)
            .map(|index| volicord_types::UserActionOptionInput {
                option_id: volicord_types::UserActionOptionId::new(format!("option_{index}")),
                label: format!("Option {index}"),
                description: "A bounded option.".to_owned(),
                consequence: "Only this option is selected.".to_owned(),
                is_default: index == 0,
            })
            .collect(),
    )
    .into();
    let accepted = harness
        .service
        .request_user_action(at_limit, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(accepted.response_value["base"]["response_kind"], "result");

    let mut over_limit = user_action_request(
        "req_bounds_n_plus_one",
        "idem_bounds_n_plus_one",
        false,
        Some(3),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    let volicord_types::UserActionDraft::Choice(choice) = &mut over_limit.action else {
        unreachable!("choice fixture")
    };
    choice.options = Some(
        (0..=volicord_types::USER_ACTION_TARGET_CANDIDATE_LIMIT)
            .map(|index| volicord_types::UserActionOptionInput {
                option_id: volicord_types::UserActionOptionId::new(format!("over_{index}")),
                label: format!("Option {index}"),
                description: "An over-limit option.".to_owned(),
                consequence: "This request must reject.".to_owned(),
                is_default: index == 0,
            })
            .collect(),
    )
    .into();
    let before = harness.counts()?;
    let rejected = harness
        .service
        .request_user_action(over_limit, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);

    let mut empty_observation = observation_action_request(
        "req_bounds_empty",
        "idem_bounds_empty",
        3,
        &task_id,
        &change_unit_id,
        supplemental_evidence_target("No candidate may be empty."),
        Vec::new(),
    );
    let volicord_types::UserActionDraft::EvidenceObservation(observation) =
        &mut empty_observation.action
    else {
        unreachable!("observation fixture")
    };
    observation.target_candidates.clear();
    let rejected = harness.service.request_user_action(
        empty_observation,
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);

    let mut oversized = user_action_request(
        "req_bounds_32k",
        "idem_bounds_32k",
        false,
        Some(3),
        &task_id,
        Some(&change_unit_id),
        JudgmentKind::ProductDecision,
    );
    let volicord_types::UserActionDraft::Choice(choice) = &mut oversized.action else {
        unreachable!("choice fixture")
    };
    choice.context.summary = "x".repeat(volicord_types::USER_ACTION_FORM_MAX_BYTES + 1);
    let rejected = harness
        .service
        .request_user_action(oversized, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn local_web_token_is_bound_and_consumed_atomically() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "local_web")?;
    let requested = harness.service.request_user_action(
        user_action_request(
            "req_local_web_action",
            "idem_local_web_action",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let action_id = request_id(&requested);
    let token_hash = create_local_web_token_for_user_action(
        &harness,
        "local-web-token-user-action",
        &action_id,
    )?;
    let completion_metadata = crate::LocalWebConsentCompletionMetadata {
        selection_recording: Some("recorded".to_owned()),
        endpoint: Some("/local-web-consent/complete".to_owned()),
    };
    let channel_submission_id = crate::local_web_channel_submission_id(
        &ProjectId::new(PROJECT_ID),
        &UserActionRequestId::new(action_id.clone()),
        "local-web-token-user-action",
        CONNECTION_ID,
        &completion_metadata,
    )?;
    let local_request = crate::LocalWebConsentUserActionRequest {
        request: resolve_user_action_request(
            "req_local_web_resolve",
            &channel_submission_id,
            None,
            &task_id,
            &action_id,
            "accept",
        ),
        token: "local-web-token-user-action".to_owned(),
        expected_connection_internal_id: CONNECTION_ID.to_owned(),
        completion_metadata_json: serde_json::to_string(&completion_metadata)?,
    };
    assert!(!format!("{local_request:?}").contains("local-web-token-user-action"));
    let before_generic = harness.counts()?;
    let before_generic_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    let mut dry_run = local_request.clone();
    dry_run.request.envelope.dry_run = true;
    let rejected = harness.service.resolve_local_web_consent_user_action(
        dry_run,
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
    )?;
    assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        rejected.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before_generic);
    assert_eq!(local_web_token_status(&harness, &token_hash)?.0, "pending");
    for generic_invocation in [
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
        InvocationContext::new(
            ProjectId::new(PROJECT_ID),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            format!(" {VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB} "),
        ),
    ] {
        let rejected = harness
            .service
            .resolve_user_action(local_request.request.clone(), generic_invocation)?;
        assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            rejected.response_value["errors"][0]["code"],
            "INVOCATION_CONTEXT_MISMATCH"
        );
        assert_eq!(harness.counts()?, before_generic);
        assert_eq!(local_web_token_status(&harness, &token_hash)?.0, "pending");
        let floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(floor, before_generic_floor);
    }

    let response = harness.service.resolve_local_web_consent_user_action(
        local_request.clone(),
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert!(!response
        .response_json
        .contains("local-web-token-user-action"));
    let (status, consumed_at, completed_at) = local_web_token_status(&harness, &token_hash)?;
    assert_eq!(status, "consumed");
    assert!(consumed_at.is_some());
    assert!(completed_at.is_some());
    assert_eq!(user_action_status(&harness, &action_id)?, "resolved");

    let raw_token_occurrences: i64 = harness.conn()?.query_row(
        "SELECT COUNT(*)
           FROM (
             SELECT request_hash AS value FROM tool_invocations WHERE project_id = ?1
             UNION ALL SELECT response_json FROM tool_invocations WHERE project_id = ?1
             UNION ALL SELECT request_hash FROM authority_events WHERE project_id = ?1
             UNION ALL SELECT payload_json FROM authority_events WHERE project_id = ?1
             UNION ALL SELECT channel_submission_id FROM user_action_resolutions WHERE project_id = ?1
             UNION ALL SELECT resolution_json FROM user_action_resolutions WHERE project_id = ?1
             UNION ALL SELECT token_hash FROM user_action_channel_tokens WHERE project_id = ?1
             UNION ALL SELECT created_metadata_json FROM user_action_channel_tokens WHERE project_id = ?1
             UNION ALL SELECT completion_metadata_json FROM user_action_channel_tokens WHERE project_id = ?1
           )
          WHERE instr(value, ?2) > 0",
        rusqlite::params![PROJECT_ID, "local-web-token-user-action"],
        |row| row.get(0),
    )?;
    assert_eq!(
        raw_token_occurrences, 0,
        "raw local-web token reached durable state"
    );

    let after_commit = harness.counts()?;
    let after_commit_floor: String = harness.conn()?.query_row(
        "SELECT updated_at FROM project_state WHERE project_id = ?1",
        [PROJECT_ID],
        |row| row.get(0),
    )?;
    let mut semantically_equivalent = local_request.clone();
    semantically_equivalent.completion_metadata_json =
        " { \"endpoint\" : \"/local-web-consent/complete\", \"selection_recording\" : \"recorded\" } "
            .to_owned();
    let replay = harness.service.resolve_local_web_consent_user_action(
        semantically_equivalent,
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
    )?;
    assert!(replay.replayed);
    assert_eq!(replay.response_json, response.response_json);
    assert_eq!(harness.counts()?, after_commit);
    let exact_replay = harness.service.resolve_local_web_consent_user_action(
        local_request.clone(),
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
    )?;
    assert!(exact_replay.replayed);
    assert_eq!(exact_replay.response_json, response.response_json);
    assert_eq!(harness.counts()?, after_commit);

    let mut wrong_token = local_request.clone();
    wrong_token.token = "different-local-web-token-user-action".to_owned();
    let mut invalid_token_shape = local_request.clone();
    invalid_token_shape.token.clear();
    let mut wrong_connection = local_request.clone();
    wrong_connection.expected_connection_internal_id = "conn_wrong_local_web".to_owned();
    let mut wrong_metadata = local_request.clone();
    wrong_metadata.completion_metadata_json =
        serde_json::to_string(&crate::LocalWebConsentCompletionMetadata {
            selection_recording: Some("recorded".to_owned()),
            endpoint: Some("/different-local-web-endpoint".to_owned()),
        })?;
    let mut unknown_metadata = local_request.clone();
    unknown_metadata.completion_metadata_json =
        r#"{"selection_recording":"recorded","endpoint":"/local-web-consent/complete","unknown":true}"#
            .to_owned();
    let mut handcrafted_submission = local_request.clone();
    handcrafted_submission.request.channel_submission_id = "local_web:handcrafted".to_owned();
    handcrafted_submission.request.envelope.idempotency_key =
        Some(IdempotencyKey::new("local_web:handcrafted")).into();
    for attempted in [
        wrong_token,
        invalid_token_shape,
        wrong_connection,
        wrong_metadata,
        unknown_metadata,
        handcrafted_submission,
    ] {
        let rejected = harness.service.resolve_local_web_consent_user_action(
            attempted,
            local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
        )?;
        assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, after_commit);
        let floor: String = harness.conn()?.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(floor, after_commit_floor);
    }
    for generic_invocation in [
        local_web_invocation(ActorSource::LocalUser, OperationCategory::UserOnly),
        InvocationContext::new(
            ProjectId::new(PROJECT_ID),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            format!(" {VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB} "),
        ),
    ] {
        let rejected = harness
            .service
            .resolve_user_action(local_request.request.clone(), generic_invocation)?;
        assert_eq!(rejected.response_value["base"]["response_kind"], "rejected");
        assert_eq!(harness.counts()?, after_commit);
        assert_eq!(local_web_token_status(&harness, &token_hash)?.0, "consumed");
    }
    Ok(())
}
