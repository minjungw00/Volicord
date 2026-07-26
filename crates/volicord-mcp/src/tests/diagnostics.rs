use super::*;

#[test]
fn platform_diagnostic_code_is_preserved_in_persisted_mcp_facts() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-platform-diagnostic-persistence")?;
    let admission =
        volicord_test_support::TestRuntimeHomeMutation::acquire(fixture.runtime_home_path())?;
    let context = admission.context()?;
    let data = crate::diagnostics::data_for_diagnostic(
        crate::diagnostics::McpDiagnostic::Platform(
            volicord_platform_fs::PlatformDiagnosticKind::UnsupportedTarget,
        ),
        &crate::diagnostics::McpDiagnosticContext {
            observed_at: volicord_types::values::UtcTimestamp::parse("2026-07-22T01:02:03Z")?,
            connection_id: Some(fixture.connection_id().to_owned()),
            integration_revision: None,
            runtime_session_id: None,
            requested_revision: None,
            selected_revision: None,
            negotiated_revision: None,
            supported_revisions: crate::diagnostics::production_supported_revisions(),
            attempted_client_name: None,
            attempted_client_version: None,
            json_rpc_error_code: None,
            safe_error_data: None,
            tool_name: None,
            missing_tools: Vec::new(),
        },
    )?;
    let occurrence = volicord_types::diagnostics::OccurrenceDiagnosticFinding::try_new(data, None)?;
    volicord_store::diagnostic_findings::insert_occurrence_finding(&context, &occurrence)?;

    let stored =
        stored_diagnostic_findings_by_ids(fixture.runtime_home_path(), &[occurrence.id()])?;
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].to_diagnostic_finding().code().as_str(),
        "platform.target.unsupported"
    );
    Ok(())
}

#[test]
fn stdio_workflow_metrics_record_exact_tools_list_method_outcomes_and_status_rereads(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-workflow-metrics")?;
    let private_marker = "private_prompt_marker_must_not_be_persisted";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call(
            3,
            AgentToolId::STATUS.wire_name(),
            json!({ "detail": "workflow" }),
        ),
        tools_call(
            4,
            AgentToolId::STATUS.wire_name(),
            json!({ "detail": "workflow" }),
        ),
        tools_call(
            5,
            AgentToolId::CHECK_CLOSE.wire_name(),
            json!({
                "private_marker": private_marker
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 5);
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(responses[3]["result"]["isError"], false);
    assert_eq!(responses[4]["result"]["isError"], true);
    let exact_tools_list_bytes = u64::try_from(serde_json::to_vec(&responses[1]["result"])?.len())?;
    let metrics =
        read_workflow_metric_aggregates(fixture.runtime_home_path(), fixture.project_id())?;

    let tools_list = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::ToolsListSerializedBytes,
        None,
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(tools_list.sample_count, 1);
    assert_eq!(tools_list.host_kind.as_deref(), Some("codex"));
    assert_eq!(tools_list.value_total, exact_tools_list_bytes);
    assert_eq!(tools_list.value_min, exact_tools_list_bytes);
    assert_eq!(tools_list.value_max, exact_tools_list_bytes);

    let successful_status = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::McpMethodCall,
        Some(MethodName::Status),
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(successful_status.sample_count, 2);
    assert_eq!(successful_status.value_total, 2);
    let invalid_check_close = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::McpMethodCall,
        Some(MethodName::CheckClose),
        Some(WorkflowMetricOutcome::ValidationFailure),
    );
    assert_eq!(invalid_check_close.sample_count, 1);
    assert_eq!(invalid_check_close.value_total, 1);
    let status_reread = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::StatusReread,
        None,
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(status_reread.sample_count, 1);
    assert_eq!(status_reread.value_total, 1);

    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(private_marker));
    Ok(())
}

#[test]
fn stdio_diagnostics_count_validation_retry_without_storing_request_content(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-validation-retry")?;
    let before = fixture.counts()?;
    let sensitive_sentinel = "diagnostic-request-secret-and-file-/private/example.txt";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({"unexpected_private_value": sensitive_sentinel}),
        ),
        tools_call(3, AgentToolId::STATUS.wire_name(), json!({})),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[1]["result"]["isError"], true);
    assert_eq!(responses[2]["result"]["isError"], false);
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home_path(), None)?.expect("diagnostics session");
    let status = diagnostics
        .tools
        .iter()
        .find(|tool| tool.tool_name == AgentToolId::STATUS.wire_name())
        .expect("status metrics");
    assert_eq!(status.call_count, 2);
    assert_eq!(status.validation_failures, 1);
    assert_eq!(status.retries_after_validation_failure, 1);
    assert_eq!(status.core_reached_count, 1);
    assert_eq!(fixture.counts()?, before);
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(sensitive_sentinel));
    Ok(())
}

#[test]
fn stdio_diagnostics_never_store_unknown_caller_tool_names() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-unknown-tool-private")?;
    let sensitive_tool_name = "token=abc123-private-tool-name";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, sensitive_tool_name, json!({})),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses[1]["error"]["code"], -32602);
    assert!(
        read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none(),
        "untrusted tool metadata must not bind or create a managed diagnostics session"
    );
    assert!(
        !diagnostics_db_path(fixture.runtime_home_path()).exists(),
        "rejected untrusted metadata must not create the diagnostics store"
    );
    Ok(())
}

#[test]
fn corrupt_diagnostics_store_is_nonfatal_to_mcp_core_result() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-corrupt-nonfatal")?;
    fs::write(
        diagnostics_db_path(fixture.runtime_home_path()),
        b"not a sqlite diagnostics database",
    )?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call(3, AgentToolId::STATUS.wire_name(), json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert!(responses[1]["result"]["tools"].is_array());
    assert_eq!(responses[2]["result"]["isError"], false);
    let response = volicord_response_from_tool(&responses[2])?;
    assert_eq!(response["base"]["response_kind"], "result");
    assert_eq!(response["base"]["effect_kind"], "read_only");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn corrupt_diagnostics_store_is_nonfatal_to_managed_codex_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-managed-diagnostics-corrupt-nonfatal")?;
    fs::write(
        diagnostics_db_path(fixture.runtime_home_path()),
        b"not a sqlite diagnostics database",
    )?;
    let native_session_id = "native.session.corrupt-diagnostics";
    let native_thread_id = "native.thread.corrupt-diagnostics";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call_with_codex_metadata(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({}),
            native_session_id,
            native_thread_id,
            "turn.one",
        ),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::STATUS.wire_name(),
            json!({}),
            native_session_id,
            native_thread_id,
            "turn.two",
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(
        responses[1]["result"]["isError"], false,
        "first response: {:?}",
        responses[1]
    );
    assert_eq!(
        responses[2]["result"]["isError"], false,
        "second response: {:?}",
        responses[2]
    );
    let serialized = serde_json::to_string(&responses)?;
    assert!(!serialized.contains(native_session_id));
    assert!(!serialized.contains(native_thread_id));
    Ok(())
}
