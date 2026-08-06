use super::*;

#[test]
fn connection_context_resolves_and_preflight_reports_allowed_project() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-context")?;

    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    assert_eq!(
        context.connection_internal_id.as_str(),
        fixture.connection_id()
    );
    assert_eq!(context.mode, AgentConnectionMode::Workflow);

    let report = preflight_check(
        |name| {
            if name == "VOLICORD_HOME" {
                Some(fixture.runtime_home_path().as_os_str().to_owned())
            } else {
                None
            }
        },
        fixture.runtime_home_path(),
        fixture.connection_id(),
        None,
    )?;
    assert_eq!(report.connection_id, fixture.connection_id());
    assert_eq!(report.mode, "workflow");
    assert_eq!(report.allowed_projects, 1);
    assert_eq!(report.available_projects, 1);
    assert_eq!(report.registry_read, "passed");
    assert_eq!(report.project_state_read, "passed");
    assert_eq!(report.writeability.status, "not_checked");
    assert_eq!(
        report.writeability.requirement,
        "requires_active_verification"
    );
    assert_eq!(report.effective_tool_mode, "requires_active_verification");
    assert_eq!(report.tools_list_schema_validation, "passed");
    assert_eq!(report.tool_naming_style, "dotted_namespace");
    let effective_tools =
        mcp_tools_for_mode_and_storage(context.mode, McpStorageCapability::Unknown);
    assert_eq!(report.host_callable_tools.len(), effective_tools.len());
    assert!(effective_tools.iter().all(|tool| report
        .host_callable_tools
        .iter()
        .any(|identity| identity.raw_tool_name == tool.id.wire_name())));
    let guard_probe = report
        .host_callable_tools
        .iter()
        .find(|identity| identity.raw_tool_name == AgentToolId::GUARD_PROBE.wire_name())
        .expect("Guard probe callable identity");
    assert_eq!(
        guard_probe.profile,
        HostContractProfileId::CodexMcpCallableNames.as_str()
    );
    assert!(guard_probe
        .callable_name
        .ends_with("__volicord_guard_probe"));
    assert_eq!(
        guard_probe.server_key,
        report.host_callable_tools[0].server_key
    );
    Ok(())
}

#[test]
fn mcp_preflight_defers_writeability_and_effective_tool_mode() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readwrite-mode")?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_eq!(report.registry_read, "passed");
    assert_eq!(report.project_state_read, "passed");
    assert_eq!(report.writeability.status, "not_checked");
    assert_eq!(report.effective_tool_mode, "requires_active_verification");
    assert_eq!(report.tools_list_schema_validation, "passed");
    assert_eq!(report.tool_naming_style, "dotted_namespace");
    assert_eq!(report.projects[0].state_read, "passed");
    assert_eq!(report.projects[0].state_write, "not_checked");
    Ok(())
}

#[test]
fn mcp_preflight_does_not_mutate_project_state() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-no-mutate")?;
    let before_version = read_only_state_version(&fixture)?;
    let before_sessions = read_only_table_count(&fixture, "host_sessions")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_eq!(report.writeability.status, "not_checked");
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "host_sessions")?,
        before_sessions
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    Ok(())
}

#[test]
fn mcp_preflight_succeeds_with_readonly_project_state() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readonly-state")?;
    let _guard = make_project_state_readonly(&fixture)?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_eq!(report.registry_read, "passed");
    assert_eq!(report.project_state_read, "passed");
    assert_eq!(report.writeability.status, "not_checked");
    assert_eq!(report.effective_tool_mode, "requires_active_verification");
    assert_eq!(report.tools_list_schema_validation, "passed");
    assert_eq!(report.tool_naming_style, "dotted_namespace");
    assert_eq!(report.projects[0].state_read, "passed");
    assert_eq!(report.projects[0].state_write, "not_checked");
    Ok(())
}
#[test]
fn runtime_tool_mode_still_degrades_on_readonly_storage() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readonly-tool-mode")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;
    let names = tool_names(&adapter.tools()?);

    assert_eq!(report.effective_tool_mode, "requires_active_verification");
    assert!(names.contains(&AgentToolId::STATUS.wire_name()));
    assert!(names.contains(&AgentToolId::GET_OPERATION_RESULT.wire_name()));
    assert!(names.contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    assert!(names.contains(&AgentToolId::CHECK_CLOSE.wire_name()));
    assert!(names.contains(&AgentToolId::LIST_PROJECTS.wire_name()));
    assert!(!names.contains(&AgentToolId::INTAKE.wire_name()));
    Ok(())
}

#[test]
fn public_stdio_records_manual_cli_session_source() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-public-stdio-manual-source")?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();
    run_stdio(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let source = registry.query_row(
        "SELECT session_source FROM mcp_runtime_sessions ORDER BY process_started_at DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )?;
    assert_eq!(source, "manual_cli");
    Ok(())
}

#[test]
fn managed_codex_launch_stays_effect_free_until_exact_call_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-binding")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(adapter, BufReader::new(input), &mut output)?;

    assert!(output.is_empty());
    assert_eq!(read_only_table_count(&fixture, "host_sessions")?, 0);
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none());
    Ok(())
}

#[test]
fn managed_codex_tools_list_buffers_metrics_until_call_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-tools-list-binding")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert!(responses[1]["result"]["tools"].is_array());
    assert_eq!(read_only_table_count(&fixture, "host_sessions")?, 0);
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none());
    Ok(())
}

#[test]
fn managed_stdio_records_authoritative_protocol_milestones_with_future_client_data(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-authoritative-runtime-milestones")?;
    let mut initialize = initialize_request(1, json!({}));
    initialize["params"]["clientInfo"]["name"] = json!("future-cooperative-client");
    initialize["params"]["clientInfo"]["version"] = json!("999.0-preview+custom");
    let input = Cursor::new(json_lines(&[
        initialize,
        initialized_notification(),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::LIST_PROJECTS.wire_name(),
            json!({}),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            CODEX_TEST_TURN_ID,
        ),
    ])?);
    let mut output = Vec::new();
    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;
    assert_eq!(stdio_responses(&output)?.len(), 3);
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let runtime_session_id = registry.query_row(
        "SELECT runtime_session_id
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1 AND session_source = 'managed_host'
          ORDER BY process_started_at DESC, runtime_session_id DESC
          LIMIT 1",
        [fixture.connection_id()],
        |row| row.get::<_, String>(0),
    )?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("authoritative runtime session");
    assert_eq!(
        runtime.attempted_client_name.as_deref(),
        Some("future-cooperative-client")
    );
    assert_eq!(
        runtime.attempted_client_version.as_deref(),
        Some("999.0-preview+custom")
    );
    assert_eq!(
        runtime.requested_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert_eq!(
        runtime.selected_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert_eq!(
        runtime.negotiated_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert!(runtime.initialize_completed_at.is_some());
    assert!(runtime.initialized_notification_at.is_some());
    assert_eq!(runtime.required_tools_present, Some(true));
    assert_eq!(
        runtime.verification_tool_name.as_deref(),
        Some(AgentToolId::LIST_PROJECTS.wire_name())
    );
    assert!(runtime.verification_tool_observed_at.is_some());
    assert!(runtime.graceful_close_at.is_some());
    assert!(runtime.terminal_finding_id.is_none());
    Ok(())
}

#[test]
fn successful_non_designated_read_only_tools_do_not_record_round_trip_evidence(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-nondesignated-read-only-round-trip")?;
    let _repository = volicord_test_support::core_fixtures::PlanningRepository::initialize_at(
        fixture.product_repo_path(),
    )?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _, _) = create_implementation_task(&fixture)?;
    let update_form = action_form_for_variant(
        &setup_adapter,
        &task_id,
        MethodName::UpdateScope,
        WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
    )?;
    let mut update_arguments = Value::Object(update_form.canonical_minimal_request);
    update_arguments["baseline_ref"] =
        json!(volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF);
    let committed =
        setup_adapter.call_tool(AgentToolId::UPDATE_SCOPE.wire_name(), update_arguments)?;
    let operation_result_ref = committed
        .operation_result_ref
        .ok_or("update_scope operation result ref")?;
    let check_close_action_form_ref =
        action_form_ref_for_method(&setup_adapter, &task_id, MethodName::CheckClose)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::STATUS.wire_name(),
            json!({ "detail": "workflow", "task_id": null }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            "fixture_codex_turn_status",
        ),
        tools_call_with_codex_metadata(
            4,
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            json!({ "operation_result_ref": operation_result_ref, "cursor": null }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            "fixture_codex_turn_result",
        ),
        tools_call_with_codex_metadata(
            5,
            AgentToolId::CHECK_CLOSE.wire_name(),
            json!({
                "action_form_ref": check_close_action_form_ref,
                "task_id": task_id
            }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            "fixture_codex_turn_close",
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
    for response in &responses[2..] {
        assert_eq!(response["result"]["isError"], false, "{response}");
    }
    let runtime =
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .ok_or("managed runtime")?;
    assert!(runtime.verification_tool_name.is_none());
    assert!(runtime.verification_tool_observed_at.is_none());
    Ok(())
}

#[test]
fn failed_designated_tool_call_does_not_record_round_trip_evidence() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-failed-designated-round-trip")?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::LIST_PROJECTS.wire_name(),
            json!({ "unexpected": true }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            CODEX_TEST_TURN_ID,
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
        responses[2]["result"]["isError"], true,
        "unexpected failed designated-tool response: {}",
        responses[2]
    );
    let runtime =
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .ok_or("managed runtime")?;
    assert!(runtime.verification_tool_name.is_none());
    assert!(runtime.verification_tool_observed_at.is_none());
    Ok(())
}

#[test]
fn failed_initialize_retains_attempted_client_and_requested_revision() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-failed-initialize-attempt")?;
    let mut initialize = initialize_request(1, json!("invalid-capabilities"));
    initialize["params"]["protocolVersion"] = json!("2099-01-01");
    initialize["params"]["clientInfo"]["name"] = json!("future-client");
    initialize["params"]["clientInfo"]["version"] = json!("2099.7");
    let mut output = Vec::new();
    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[initialize])?)),
        &mut output,
    )?;
    assert!(stdio_responses(&output)?[0]["error"].is_object());
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let runtime_session_id = registry.query_row(
        "SELECT runtime_session_id FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
          ORDER BY process_started_at DESC, runtime_session_id DESC LIMIT 1",
        [fixture.connection_id()],
        |row| row.get::<_, String>(0),
    )?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("failed initialize runtime session");
    assert_eq!(
        runtime.attempted_client_name.as_deref(),
        Some("future-client")
    );
    assert_eq!(runtime.attempted_client_version.as_deref(), Some("2099.7"));
    assert_eq!(
        runtime.requested_protocol_version.as_deref(),
        Some("2099-01-01")
    );
    assert!(runtime.selected_protocol_version.is_none());
    assert!(runtime.negotiated_protocol_version.is_none());
    assert!(runtime.initialize_completed_at.is_none());
    let terminal_id = runtime.terminal_finding_id.ok_or("terminal finding")?;
    let terminal = stored_diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        &[volicord_types::diagnostics::DiagnosticFindingId::parse(
            terminal_id,
        )?],
    )?
    .into_iter()
    .next()
    .ok_or("persisted terminal finding")?;
    let terminal = terminal.to_diagnostic_finding();
    assert_eq!(
        terminal.code().as_str(),
        "mcp.protocol.capability_shape_invalid"
    );
    assert_eq!(
        terminal.runtime_session_id().map(|value| value.as_str()),
        Some(runtime_session_id.as_str())
    );
    Ok(())
}

#[test]
fn managed_stdio_tool_call_records_bounded_metrics() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-tool-call-metrics")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
        initialized_notification(),
        tools_call(
            3,
            "volicord.status",
            json!({ "detail": "workflow", "task_id": null }),
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    let status = volicord_response_from_tool(&responses[2])?;
    assert_eq!(status["base"]["response_kind"], "result");
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
    let status_call = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::McpMethodCall,
        Some(MethodName::Status),
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(status_call.sample_count, 1);
    assert_eq!(status_call.value_total, 1);
    Ok(())
}

#[test]
fn managed_codex_new_client_version_uses_protocol_and_call_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-new-codex-version")?;
    let observed_version = "0.145.0";
    let input = Cursor::new(json_lines(&[
        initialize_request_with_client_info(
            1,
            json!({}),
            CODEX_MANAGED_MCP_CLIENT_NAME,
            observed_version,
        ),
        initialized_notification(),
        tools_call_with_codex_metadata(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({"detail":"workflow", "task_id": null}),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            CODEX_TEST_TURN_ID,
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["result"]["isError"], false);
    Ok(())
}

#[test]
fn managed_codex_binding_allows_new_turn_and_rejects_session_or_thread_rebind(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-codex-binding-immutable")?;
    let native_session_id = "native.session.root";
    let native_thread_id = "native.thread.root";
    let capability_sentinel = "capability.must.not.persist";
    let initialize_sentinel = "initialize.payload.must.not.persist";
    let client_extension_sentinel = "client.extension.must.not.persist";
    let tool_payload_sentinel = "tool.payload.must.not.persist";
    let mut initialize = initialize_request(1, json!({}));
    initialize["params"]["capabilities"]["future_capability"] = json!(capability_sentinel);
    initialize["params"]["future_initialize_field"] = json!(initialize_sentinel);
    initialize["params"]["clientInfo"]["future_client_field"] = json!(client_extension_sentinel);
    let mut first_call = tools_call_with_codex_metadata(
        2,
        "volicord.status",
        json!({"detail":"workflow", "task_id": null}),
        native_session_id,
        native_thread_id,
        "turn.one",
    );
    first_call["params"]["_meta"]["future_tool_payload"] = json!(tool_payload_sentinel);
    let input = Cursor::new(json_lines(&[
        initialize,
        initialized_notification(),
        first_call,
        tools_call_with_codex_metadata(
            3,
            "volicord.status",
            json!({"detail":"workflow", "task_id": null}),
            native_session_id,
            native_thread_id,
            "turn.two",
        ),
        tools_call_with_codex_metadata(
            4,
            "volicord.status",
            json!({"detail":"workflow", "task_id": null}),
            native_session_id,
            "native.thread.other",
            "turn.three",
        ),
        tools_call_with_codex_metadata(
            5,
            "volicord.status",
            json!({"detail":"workflow", "task_id": null}),
            "native.session.other",
            native_thread_id,
            "turn.four",
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
    assert!(responses[1]["result"].is_object());
    assert!(responses[2]["result"].is_object());
    assert_eq!(responses[3]["error"]["code"], -32602);
    assert_eq!(responses[4]["error"]["code"], -32602);
    let diagnostic = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("first exact call must bind the managed runtime diagnostic session");
    assert_eq!(diagnostic.totals.tool_call_count, 2);
    let persisted = serde_json::to_string(&diagnostic)?;
    for raw in [
        native_session_id,
        native_thread_id,
        "turn.one",
        "turn.two",
        "native.thread.other",
        "native.session.other",
        capability_sentinel,
        initialize_sentinel,
        client_extension_sentinel,
        tool_payload_sentinel,
    ] {
        assert!(
            !persisted.contains(raw),
            "raw host-native session correlation metadata leaked: {raw}"
        );
    }
    Ok(())
}

#[test]
fn invalid_codex_call_metadata_has_zero_durable_or_core_effect() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-invalid-marker-watch-skip")?;
    let adapter = project_bound_adapter(&fixture)?;
    let before_state_version = read_only_state_version(&fixture)?;
    let before_agent_sessions = read_only_table_count(&fixture, "host_sessions")?;
    let before_tool_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(
            2,
            "tools/call",
            json!({
                "name": AgentToolId::LIST_PROJECTS.wire_name(),
                "arguments": {},
                "_meta": {
                    "threadId": "thread invalid marker",
                    "x-codex-turn-metadata": {
                        "session_id": CODEX_TEST_SESSION_ID,
                        "thread_id": CODEX_TEST_THREAD_ID,
                        "turn_id": CODEX_TEST_TURN_ID
                    }
                }
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["error"]["code"], -32602);
    assert!(!serde_json::to_string(&responses)?.contains("thread invalid marker"));
    assert_eq!(read_only_state_version(&fixture)?, before_state_version);
    assert_eq!(
        read_only_table_count(&fixture, "host_sessions")?,
        before_agent_sessions
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_tool_invocations
    );
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none());
    let runtime =
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .ok_or("managed runtime for malformed host metadata")?;
    assert!(runtime.verification_tool_name.is_none());
    assert!(runtime.verification_tool_observed_at.is_none());
    let findings = diagnostic_occurrences_for_runtime_session(
        fixture.runtime_home_path(),
        &runtime.runtime_session_id,
    )?;
    assert!(findings
        .iter()
        .any(|finding| finding.data().code().as_str() == "host.codex.metadata_malformed"));
    let persisted_findings = serde_json::to_string(
        &findings
            .iter()
            .map(|finding| finding.to_diagnostic_finding())
            .collect::<Vec<_>>(),
    )?;
    assert!(!persisted_findings.contains("thread invalid marker"));
    assert!(!persisted_findings.contains(CODEX_TEST_SESSION_ID));
    assert!(!persisted_findings.contains(CODEX_TEST_THREAD_ID));
    Ok(())
}

#[test]
fn invalid_tool_shapes_do_not_bind_and_a_later_exact_codex_call_recovers(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-codex-prebinding-validation-order")?;
    let rejected_session_id = "native.session.rejected-before-binding";
    let accepted_session_id = "native.session.accepted-after-recovery";
    let mut non_object_arguments = tools_call_with_codex_metadata(
        3,
        "volicord.status",
        json!({}),
        rejected_session_id,
        "native.thread.rejected-before-binding",
        "turn.rejected.arguments",
    );
    non_object_arguments["params"]["arguments"] = json!([]);
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call_with_codex_metadata(
            2,
            "volicord.unknown",
            json!({}),
            rejected_session_id,
            "native.thread.rejected-before-binding",
            "turn.rejected.tool",
        ),
        non_object_arguments,
        request(
            4,
            "tools/call",
            json!({"name":"volicord.status","arguments":{}}),
        ),
        tools_call_with_codex_metadata(
            5,
            "volicord.status",
            json!({"detail":"workflow", "task_id": null}),
            accepted_session_id,
            "native.thread.accepted-after-recovery",
            "turn.accepted",
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
    for response in &responses[1..4] {
        assert_eq!(response["error"]["code"], -32602);
    }
    assert!(responses[4]["result"].is_object());

    let rejected = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
        &HostNativeCorrelation::CodexMcp(CodexMcpCorrelation {
            session_id: HostSessionId::parse(rejected_session_id)?,
            thread_id: HostThreadId::parse("native.thread.rejected")?,
            turn_id: HostTurnId::parse("turn.rejected")?,
        }),
    )?
    .session_id;
    assert!(
        agent_session(fixture.runtime_home_path(), fixture.project_id(), &rejected,)?.is_none()
    );

    let accepted = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
        &HostNativeCorrelation::CodexMcp(CodexMcpCorrelation {
            session_id: HostSessionId::parse(accepted_session_id)?,
            thread_id: HostThreadId::parse("native.thread.accepted-after-recovery")?,
            turn_id: HostTurnId::parse("turn.accepted")?,
        }),
    )?
    .session_id;
    assert!(
        agent_session(fixture.runtime_home_path(), fixture.project_id(), &accepted,)?.is_some()
    );
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_some());
    let serialized = serde_json::to_string(&responses)?;
    assert!(!serialized.contains(rejected_session_id));
    assert!(!serialized.contains(accepted_session_id));
    Ok(())
}
