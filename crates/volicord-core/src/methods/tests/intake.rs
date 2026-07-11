use super::*;

#[test]
fn intake_commits_once_and_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let request = intake_request(
        "req_intake",
        "idem_intake",
        false,
        Some(0),
        RequestedMode::Auto,
    );

    let first = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;

    assert_eq!(first.response_value["base"]["response_kind"], "result");
    assert_eq!(
        first.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(first.response_value["base"]["state_version"], 1);
    assert_eq!(first.response_value["state"]["mode"], "work");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.tasks, before.tasks + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);

    let second = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn intake_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let response = harness.service.intake(
        intake_request(
            "req_intake_dry",
            "idem_intake_dry",
            true,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn intake_persists_normalized_non_authoritative_source_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let mut request = intake_request(
        "req_intake_source",
        "idem_intake_source",
        false,
        Some(0),
        RequestedMode::Advisor,
    );
    request.initial_source_refs = vec![volicord_types::SourceRef::RepositoryFile(
        volicord_types::RepositoryFileSource {
            repository_path: "notes/./source.md".to_owned(),
            baseline_commit_sha: "a".repeat(40),
            content_sha256: "b".repeat(64),
            line_range: Some(volicord_types::SourceLineRange {
                start_line: 2,
                end_line: 4,
            })
            .into(),
        },
    )];

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    let task_id = response.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id should be present");
    let bounded_context: String = harness.conn()?.query_row(
        "SELECT bounded_context_json FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    let bounded_context: serde_json::Value = serde_json::from_str(&bounded_context)?;
    assert_eq!(
        bounded_context["initial_source_refs"][0]["source"]["repository_path"],
        "notes/source.md"
    );
    assert_eq!(bounded_context["initial_context_refs"], json!([]));
    Ok(())
}

#[test]
fn intake_rejects_structurally_invalid_source_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let mut request = intake_request(
        "req_intake_bad_source",
        "idem_intake_bad_source",
        false,
        Some(0),
        RequestedMode::Advisor,
    );
    request.initial_source_refs = vec![volicord_types::SourceRef::ExternalUri(
        volicord_types::ExternalUriSource {
            uri: "https://user@example.invalid/spec".to_owned(),
            retrieved_at: volicord_types::UtcTimestamp::parse("2026-07-12T00:00:00Z")?,
            content_sha256: "c".repeat(64),
        },
    )];

    let response = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(harness.counts()?, before);
    Ok(())
}
