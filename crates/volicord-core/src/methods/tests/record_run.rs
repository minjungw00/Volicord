use super::*;

#[test]
fn record_run_without_product_write_commits_run_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_no_write")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let response = harness.service.record_run(
        record_run_request(
            "req_run_no_write",
            "idem_run_no_write",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_typed_result_contract::<RecordRunResult>(&response);
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["run_summary"]["observed_changes"]["product_file_write_observed"],
        false
    );
    let run_id = run_id_from_record_run(&response.response_value);
    assert_eq!(run_scope_revision(&harness, &run_id)?, 1);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_tickets, before.write_tickets);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.authority_events, before.authority_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let after_revision = task_revision(&harness, &task_id)?;
    assert_eq!(
        after_revision.close_basis_revision,
        before_revision.close_basis_revision + 1
    );
    assert!(after_revision.current_close_basis.is_none());
    assert!(response.response_value["current_close_basis"].is_null());
    Ok(())
}

#[test]
fn record_run_rejects_branch_change_after_write_ticket_issue_without_consumption(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let intake = harness.service.intake(
        intake_request(
            "req_run_workspace_task",
            "idem_run_workspace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let original = crate::pipeline::GitWorkspaceContext {
        git_common_dir: "/tmp/volicord-run-workspace/.git".to_owned(),
        worktree_id: format!("sha256:{}", "5".repeat(64)),
        branch_ref: Some("refs/heads/original".to_owned()),
        head_sha: Some("5".repeat(40)),
        workspace_fingerprint: format!("sha256:{}", "6".repeat(64)),
    };
    let scoped = harness.service.update_scope(
        update_scope_request(
            "req_run_workspace_scope",
            "idem_run_workspace_scope",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Bind the ticket to the original branch.",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    let change_unit_id = response_record_id(&scoped.response_value, "change_unit_ref");
    let ticket = harness.service.prepare_write(
        prepare_write_request(
            "req_run_workspace_ticket",
            "idem_run_workspace_ticket",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(original.clone()),
    )?;
    assert_eq!(ticket.response_value["decision"], "allowed");
    let write_ticket_id = response_record_id(&ticket.response_value, "write_ticket_ref");
    let before = harness.counts()?;

    let mut changed = original;
    changed.branch_ref = Some("refs/heads/other".to_owned());
    changed.head_sha = Some("7".repeat(40));
    changed.workspace_fingerprint = format!("sha256:{}", "8".repeat(64));
    let response = harness.service.record_run(
        product_write_record_run_request(
            "req_run_workspace_changed",
            "idem_run_workspace_changed",
            3,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            "run_workspace_changed",
        ),
        invocation(OperationCategory::AgentWorkflow).with_git_workspace_context(changed),
    )?;

    assert_write_ticket_invalid_reason(&response, "workspace_context_mismatch");
    assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_missing_write_ticket_rejects_product_write_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_missing_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_auth",
        "idem_run_missing_auth",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_TICKET_REQUIRED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_promotes_staged_artifact_and_updates_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_artifact", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let expected_content_type = handle.content_type.clone();
    let expected_sha256 = handle.sha256.clone();
    let expected_size_bytes = handle.size_bytes;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_artifact",
        "idem_run_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_report",
        handle,
        Some("validation_report"),
        Some("Search-result count validation passed."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Search-result count validation passed.",
    )];
    request.evidence_observations = vec![EvidenceObservationInput {
        target: supplemental_evidence_target("Search-result count validation passed."),
        source_kind: EvidenceSourceKind::ExternalTool,
        assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
        observed_by_actor_source: None.into(),
        tool_name: Some("search-count-validator".to_owned()).into(),
        tool_invocation_id: None.into(),
        tool_metadata: Map::from_iter([("validator".to_owned(), json!("search-count"))]),
        input_refs: Vec::new(),
        source_refs: vec![
            volicord_types::schema::SourceRef::ExternalUri(
                volicord_types::schema::ExternalUriSource {
                    uri: "https://example.invalid/search-spec".to_owned(),
                    retrieved_at: volicord_types::values::UtcTimestamp::parse(
                        "2026-06-17T23:59:00Z",
                    )?,
                    content_sha256: "d".repeat(64),
                },
            ),
            volicord_types::schema::SourceRef::UserContext(
                volicord_types::schema::UserContextSource {
                    context_id: "message_search_requirement".to_owned(),
                },
            ),
        ],
        output_artifact_refs: Vec::new(),
        limitations: vec!["External tool output is not product correctness proof.".to_owned()],
        observed_at: volicord_types::values::UtcTimestamp::parse("2026-06-18T00:00:00Z")?,
    }];
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let after = harness.counts()?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();
    let observation = &response.response_value["evidence_observations"][0];
    let observation_id = observation["observation_id"]
        .as_str()
        .expect("observation id should be present")
        .to_owned();
    let artifact_row = persistent_artifact_row(&harness, &artifact_id)?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["registered_artifacts"][0]["integrity_status"],
        "verified"
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["content_type"],
        expected_content_type
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["sha256"],
        expected_sha256
    );
    assert_eq!(
        response.response_value["registered_artifacts"][0]["size_bytes"],
        expected_size_bytes
    );
    assert_eq!(artifact_row.integrity_status, "verified");
    assert_eq!(
        artifact_row.content_type.as_deref(),
        Some(expected_content_type.as_str())
    );
    let body_path = artifact_row
        .body_path
        .as_deref()
        .expect("promoted artifact should store a body path");
    let staging_row = staged_artifact_row(&harness, &handle_id)?;
    assert!(
        body_path.starts_with("tmp/"),
        "persistent body_path should be artifact-store-relative: {body_path}"
    );
    assert!(
        !body_path.starts_with("artifacts/"),
        "persistent body_path must not include the project-home artifact prefix"
    );
    assert_eq!(staging_row.tmp_path, format!("artifacts/{body_path}"));
    assert_eq!(
        artifact_row.sha256.as_deref(),
        Some(expected_sha256.as_str())
    );
    assert_eq!(artifact_row.size_bytes, Some(expected_size_bytes));
    assert_eq!(artifact_row.status, "available");
    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["supporting_run_refs"][0]
            ["record_kind"],
        "run"
    );
    assert_eq!(observation["source_kind"], "agent_report");
    assert_eq!(observation["assurance_level"], "cooperative_report");
    assert_eq!(
        observation["producer_anchor"]["producer_kind"],
        "unverified_caller"
    );
    assert_eq!(observation["relevance_assessment"]["status"], "unassessed");
    assert_eq!(observation["observed_by_actor_source"], AGENT_ACTOR_SOURCE);
    assert_eq!(observation["tool_metadata"]["validator"], "search-count");
    assert_eq!(observation["source_refs"][0]["source_kind"], "external_uri");
    assert_eq!(observation["source_refs"][1]["source_kind"], "user_context");
    assert_eq!(
        observation["output_artifact_refs"][0]["artifact_id"],
        artifact_id
    );
    assert!(
        observation_id.starts_with("evidence_observation_"),
        "generated observation id should use the durable prefix: {observation_id}"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            ["record_kind"],
        "evidence_observation"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["observation_refs"][0]
            ["record_id"],
        observation_id
    );
    assert_eq!(
        response.response_value["evidence_summary"]["observation_refs"][0]["record_id"],
        observation_id
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.artifacts, before.artifacts + 1);
    assert_eq!(after.artifact_links, before.artifact_links + 3);
    assert_eq!(after.evidence_summaries, before.evidence_summaries + 1);
    assert_eq!(
        after.evidence_observations,
        before.evidence_observations + 1
    );
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "consumed");
    let stored_source_refs: String = harness.conn()?.query_row(
        "SELECT source_refs_json FROM evidence_observations WHERE project_id = ?1 AND evidence_observation_id = ?2",
        rusqlite::params![PROJECT_ID, observation_id],
        |row| row.get(0),
    )?;
    let stored_source_refs: serde_json::Value = serde_json::from_str(&stored_source_refs)?;
    assert_eq!(stored_source_refs, observation["source_refs"]);
    assert!(artifact_owner_link_exists(&harness, &artifact_id, "run")?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_summary"
    )?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_observation"
    )?);
    Ok(())
}

#[test]
fn record_run_checksum_mismatch_rejects_and_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;

    let mut input = artifact_input_for_handle("artifact_input_sha", handle, None, None);
    input.expected_sha256 =
        Some("0000000000000000000000000000000000000000000000000000000000000000".to_owned()).into();
    let mut request = record_run_request(
        "req_run_stage_sha",
        "idem_run_stage_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![input];
    request.close_assessment = Some(close_assessment_with_risks(
        "Rejected close basis.",
        Vec::new(),
    ))
    .into();
    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_checksum_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_dry_run_and_idempotency_replay_have_no_extra_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_replay")?;
    let before_dry = harness.counts()?;
    let dry_run = harness.service.record_run(
        record_run_request(
            "req_run_dry",
            "idem_run_dry",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(harness.counts()?, before_dry);

    let request = record_run_request(
        "req_run_replay",
        "idem_run_replay",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let first = harness.service.record_run(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}
