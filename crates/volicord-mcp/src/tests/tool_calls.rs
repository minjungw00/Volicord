use super::*;

#[test]
fn known_tool_validation_aggregates_independent_issues_without_core_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-aggregated-validation")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let arguments = json!({
        "kind": "unsupported",
        "observed_changes": {
            "changed_paths": "not-an-array"
        },
        "unexpected": true
    });

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("independent argument failures should be rejected together");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);

    for field in ["task_id", "change_unit_id", "baseline_ref", "summary"] {
        tool_error_issue(&response, &format!("/{field}"), "MCP_ARGUMENT_REQUIRED");
    }
    tool_error_issue(&response, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    tool_error_issue(&response, "/kind", "MCP_ARGUMENT_ENUM_VALUE");
    tool_error_issue(
        &response,
        "/observed_changes/changed_paths",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    for field in [
        "product_file_write_observed",
        "sensitive_categories",
        "baseline_ref",
    ] {
        tool_error_issue(
            &response,
            &format!("/observed_changes/{field}"),
            "MCP_ARGUMENT_REQUIRED",
        );
    }
    assert!(response["issues"]
        .as_array()
        .is_some_and(|issues| issues.len() > 8));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn nullable_object_union_prefers_matching_branch_and_keeps_nested_issues(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-nullable-object-validation")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["close_assessment"] = json!({});

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("empty close assessment should expose its nested missing fields");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);

    for field in [
        "result_summary",
        "result_refs",
        "residual_risks",
        "sensitive_categories",
        "recovery_constraints",
    ] {
        tool_error_issue(
            &response,
            &format!("/close_assessment/{field}"),
            "MCP_ARGUMENT_REQUIRED",
        );
    }
    assert!(response["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .all(|issue| issue["path"] != "/close_assessment"
            || issue["code"] != "MCP_ARGUMENT_TYPE_MISMATCH"));
    Ok(())
}

#[test]
fn decoder_only_failure_is_one_structured_issue_without_core_effects() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-decoder-only-validation")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let mut arguments = canonical_example_value(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
        .expect_err("invalid timestamp format should fail typed decoding");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);

    assert_eq!(response["issues"].as_array().map(Vec::len), Some(1));
    assert_eq!(response["reported_issue_count"], 1);
    assert_eq!(response["truncated"], false);
    tool_error_issue(&response, "", "MCP_ARGUMENT_DECODE_FAILED");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn decoder_failure_precedes_readonly_storage_rejection() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-decoder-before-readonly-precondition")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;
    let before = fixture.counts()?;
    let mut arguments = canonical_example_value(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
        .expect_err("typed argument decoding should precede storage preconditions");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);

    assert_eq!(response["code"], "MCP_INVALID_ARGUMENTS");
    tool_error_issue(&response, "", "MCP_ARGUMENT_DECODE_FAILED");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
#[cfg(unix)]
fn mcp_workflow_connection_degrades_tool_list_when_storage_readonly() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-readonly-tools-list")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

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
        Some(fixture.project_id()),
    )?;
    assert!(report.contains("available_projects: 1"));
    assert!(report.contains("project[0].available: true"));

    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        json!(ProtocolRegistry::production()
            .preferred_server_profile()
            .revision()
            .as_str())
    );
    let names = tool_names_from_list_response(&responses[1]);
    assert_eq!(
        names,
        vec![
            AgentToolId::STATUS.wire_name(),
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        ]
    );
    assert!(responses[1].get("error").is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_readonly_storage_exposes_read_tools_and_user_action_resume() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-exposes-read-tools")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let names = tool_names(&adapter.tools()?);

    assert!(names.contains(&AgentToolId::STATUS.wire_name()));
    assert!(names.contains(&AgentToolId::GET_OPERATION_RESULT.wire_name()));
    assert!(names.contains(&AgentToolId::LIST_PROJECTS.wire_name()));
    assert!(names.contains(&AgentToolId::CHECK_CLOSE.wire_name()));
    assert!(names.contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    assert!(!names.contains(&AgentToolId::INTAKE.wire_name()));
    assert!(!names.contains(&AgentToolId::RECORD_RUN.wire_name()));
    assert!(!names.contains(&AgentToolId::CLOSE_TASK.wire_name()));
    Ok(())
}

#[test]
fn mcp_readwrite_storage_exposes_workflow_tools() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readwrite-exposes-workflow")?;
    let adapter = adapter(&fixture)?;

    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();

    assert_eq!(tool_names(&adapter.tools()?), expected);
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_status_succeeds_with_readonly_storage() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-status")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({ "detail": "workflow" }),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(
        response.response_value["status_summary"],
        "No current Task is selected."
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_status_does_not_advance_state_version() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-status-version")?;
    let before_version = read_only_state_version(&fixture)?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let adapter = adapter(&fixture)?;
    let before_sessions = read_only_table_count(&fixture, "host_sessions")?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response =
        adapter.call_tool(AgentToolId::STATUS.wire_name(), json!({ "detail": "full" }))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
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
fn stdio_operation_result_retrieval_is_exact_bounded_and_read_only_visible(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-operation-result-exact-page")?;
    let setup_adapter = adapter(&fixture)?;
    let committed = setup_adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))?;
    let operation_result_ref = committed
        .operation_result_ref
        .clone()
        .ok_or("committed agent-workflow result should expose a lookup ref")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let read_only_adapter = adapter(&fixture)?;
    assert!(tool_names(&read_only_adapter.tools()?)
        .contains(&AgentToolId::GET_OPERATION_RESULT.wire_name()));
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            json!({ "operation_result_ref": operation_result_ref.clone() }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(read_only_adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    let structured = &result["structuredContent"];
    assert_eq!(result["isError"], false);
    assert_eq!(structured["base"]["response_kind"], "result");
    assert_eq!(structured["start_offset_bytes"], 0);
    assert_eq!(structured["complete"], true);
    assert!(structured["next_cursor"].is_null());
    assert_eq!(structured["chunk_utf8"], committed.response_json);
    assert_eq!(structured["historical"], true);
    assert_eq!(structured["current_authority_refresh_required"], true);
    let primary_text = result["content"][0]["text"]
        .as_str()
        .ok_or("operation-result compatibility text should be present")?;
    assert!(primary_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
    assert!(serde_json::from_str::<Value>(primary_text).is_err());
    assert!(!primary_text.contains(
        structured["chunk_utf8"]
            .as_str()
            .ok_or("chunk_utf8 should be a string")?
    ));
    assert!(serde_json::to_vec(result)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);

    let stale_adapter = adapter(&fixture)?;
    set_connection_enabled(fixture.runtime_home_path(), fixture.connection_id(), false)?;
    let disabled = stale_adapter
        .call_tool(
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            json!({ "operation_result_ref": operation_result_ref }),
        )
        .expect_err("every result page should recheck current connection access");
    assert!(
        disabled.to_string().contains("disabled"),
        "unexpected disabled-connection error: {disabled}"
    );
    Ok(())
}

#[test]
fn stdio_budget_omission_reconstructs_exact_result_after_state_advance(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-operation-result-budget-chain")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    let mut next_request_id = 10_u64;
    let mut call_stdio = |tool_name: &str, arguments: Value| -> Result<Value, Box<dyn Error>> {
        let initialize_id = next_request_id;
        let tool_id = next_request_id + 1;
        next_request_id += 2;
        let input = Cursor::new(json_lines(&[
            initialize_request(initialize_id, json!({})),
            initialized_notification(),
            tools_call(tool_id, tool_name, arguments),
        ])?);
        let mut output = Vec::new();
        run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        Ok(responses[1].clone())
    };

    let bounded_unicode_text =
        |label: &str, index: usize| format!("{label}-{index}:{}", "결과🙂".repeat(2_000));
    let omitted_exact_marker = "OMITTED_EXACT_OPERATION_RESULT_MARKER";
    let goal_summary = format!("{}:{omitted_exact_marker}", bounded_unicode_text("goal", 0));
    let scope_boundary = bounded_unicode_text("scope", 0);
    let non_goals = (0..6)
        .map(|index| bounded_unicode_text("non-goal", index))
        .collect::<Vec<_>>();
    let acceptance_criteria = (0..6)
        .map(|index| {
            json!({
                "acceptance_criterion_id": null,
                "statement": bounded_unicode_text("criterion", index),
                "evidence_requirement": "required"
            })
        })
        .collect::<Vec<_>>();
    let autonomy_boundary = bounded_unicode_text("autonomy", 0);
    let change_unit_summary = bounded_unicode_text("change-unit", 0);

    let omitted = call_stdio(
        AgentToolId::UPDATE_SCOPE.wire_name(),
        json!({
            "detail": "full",
            "task_id": task_id,
            "goal_summary": goal_summary,
            "scope_boundary": scope_boundary,
            "non_goals": non_goals,
            "acceptance_criteria": acceptance_criteria,
            "autonomy_boundary": autonomy_boundary,
            "change_unit": {
                "operation": "create_current",
                "scope_summary": change_unit_summary,
                "affected_paths": ["src/operation-result.rs"]
            }
        }),
    )?;
    let omitted_result = &omitted["result"];
    let omitted_structured = &omitted_result["structuredContent"];
    assert_eq!(omitted_result["isError"], false);
    assert_eq!(omitted_structured["code"], "MCP_RESPONSE_BUDGET_EXCEEDED");
    assert_eq!(omitted_structured["requested_detail"], "full");
    assert_eq!(omitted_structured["reached_core"], true);
    assert_eq!(omitted_structured["committed"], true);
    assert_eq!(omitted_structured["effect_applied"], true);
    assert_eq!(omitted_structured["response_projection_omitted"], true);
    assert_eq!(omitted_structured["status_read_required"], true);
    assert!(omitted_structured["method_result"].get("state").is_none());
    assert!(serde_json::to_vec(omitted_result)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
    assert!(!serde_json::to_string(&omitted)?.contains(omitted_exact_marker));

    let operation_result_ref_value = omitted_structured["operation_result_ref"].clone();
    let operation_result_ref: OperationResultRef =
        serde_json::from_value(operation_result_ref_value.clone())?;
    assert_eq!(operation_result_ref.source_method, MethodName::UpdateScope);
    assert_eq!(
        omitted_structured["authority_receipt"]["state_version"],
        operation_result_ref.committed_state_version
    );
    let stored = fixture
        .store()?
        .operation_result(
            operation_result_ref.source_method,
            &operation_result_ref.source_idempotency_key,
        )?
        .ok_or("budget-omitted exact result should remain in the replay row")?;
    assert_eq!(
        stored.response_size_bytes,
        operation_result_ref.response_size_bytes
    );
    assert_eq!(stored.response_sha256, operation_result_ref.response_sha256);
    assert!(stored.response_json.len() > MAX_MCP_FULL_MUTATION_RESULT_BYTES);
    assert!(stored.response_json.contains(omitted_exact_marker));

    let advanced = call_stdio(
        AgentToolId::UPDATE_SCOPE.wire_name(),
        json!({
            "task_id": task_id,
            "change_unit": { "operation": "keep_current" }
        }),
    )?;
    let advanced_structured = &advanced["result"]["structuredContent"];
    assert_eq!(advanced["result"]["isError"], false);
    assert!(advanced_structured.get("code").is_none());
    let advanced_state_version = advanced_structured["authority_receipt"]["state_version"]
        .as_u64()
        .ok_or("state-advance receipt should expose state_version")?;
    assert!(advanced_state_version > operation_result_ref.committed_state_version);
    let after_advance = fixture.counts()?;

    let mut cursor = None;
    let mut reconstructed = String::new();
    let mut pages = 0_usize;
    loop {
        let mut arguments = json!({
            "operation_result_ref": operation_result_ref_value.clone()
        });
        if let Some(next_cursor) = cursor.take() {
            arguments["cursor"] = Value::String(next_cursor);
        }
        let response = call_stdio(AgentToolId::GET_OPERATION_RESULT.wire_name(), arguments)?;
        let result = &response["result"];
        let page = &result["structuredContent"];
        assert_eq!(result["isError"], false);
        assert_eq!(page["base"]["response_kind"], "result");
        assert_eq!(page["base"]["effect_kind"], "read_only");
        assert_eq!(page["operation_result_ref"], operation_result_ref_value);
        assert_eq!(page["start_offset_bytes"], reconstructed.len() as u64);
        let chunk = page["chunk_utf8"]
            .as_str()
            .ok_or("operation-result page should contain UTF-8 text")?;
        assert!(chunk.len() <= volicord_types::MAX_OPERATION_RESULT_PAGE_BYTES);
        reconstructed.push_str(chunk);
        assert_eq!(page["end_offset_bytes"], reconstructed.len() as u64);
        assert_eq!(page["historical"], true);
        assert_eq!(page["current_authority_refresh_required"], true);
        assert!(serde_json::to_vec(result)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        pages += 1;
        assert!(
            pages < 100,
            "bounded retrieval should make forward progress"
        );
        if page["complete"] == true {
            assert!(page["next_cursor"].is_null());
            break;
        }
        cursor = Some(
            page["next_cursor"]
                .as_str()
                .ok_or("incomplete operation-result page should expose a cursor")?
                .to_owned(),
        );
    }
    assert!(pages > 1);
    assert_eq!(reconstructed.as_bytes(), stored.response_json.as_bytes());

    let status = call_stdio(
        AgentToolId::STATUS.wire_name(),
        json!({ "detail": "summary", "task_id": task_id }),
    )?;
    let status_structured = &status["result"]["structuredContent"];
    assert_eq!(status["result"]["isError"], false);
    assert_eq!(
        status_structured["authority_receipt"]["state_version"],
        advanced_state_version
    );
    assert_eq!(fixture.counts()?, after_advance);
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_write_tool_returns_unavailable_when_storage_readonly() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-write-reject")?;
    let before_version = read_only_state_version(&fixture)?;
    let before_events = read_only_table_count(&fixture, "authority_events")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(
        response.response_value["errors"][0]["message"],
        "Volicord project state is not writable in the current MCP host environment."
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["storage_capability"],
        "read_only"
    );
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "authority_events")?,
        before_events
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn readonly_degraded_user_action_tool_rejects_create_but_allows_exact_resume(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-user-action-resume")?;
    let adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&adapter)?;
    let created = adapter.call_tool(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        product_action_args(&fixture, &task_id, state_version),
    )?;
    assert!(!created.replayed);
    let exact_origin = created.response_value.clone();
    let exact_operation_result_ref = created.operation_result_ref.clone();
    let user_action_request_id = created.response_value["user_action_request_summary"]
        ["user_action_request_id"]
        .as_str()
        .ok_or("request-user-action result should identify its request")?
        .to_owned();
    let before_version = read_only_state_version(&fixture)?;
    let before_events = read_only_table_count(&fixture, "authority_events")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let before_requests = read_only_table_count(&fixture, "user_action_requests")?;
    let _guard = make_project_state_readonly(&fixture)?;

    assert!(tool_names(&adapter.tools()?).contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    let rejected_create = adapter.call_tool(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        product_action_args(&fixture, &task_id, before_version),
    )?;
    assert_eq!(
        rejected_create.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        rejected_create.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );

    let resumed = adapter.call_tool(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        resume_user_action_args(&fixture, &user_action_request_id),
    )?;
    assert!(resumed.replayed);
    assert_eq!(resumed.response_value, exact_origin);
    assert_eq!(resumed.operation_result_ref, exact_operation_result_ref);
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "authority_events")?,
        before_events
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    assert_eq!(
        read_only_table_count(&fixture, "user_action_requests")?,
        before_requests
    );
    Ok(())
}

#[test]
fn adapter_auto_selects_single_project_and_injects_connection_invocation(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-auto-select")?;
    let adapter = adapter(&fixture)?;

    let response = adapter.call_tool("volicord.status", json!({}))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_invocation
        .expect("Core should verify adapter invocation");
    assert_eq!(verified.project_id.as_str(), fixture.project_id());
    assert_eq!(verified.actor_source.to_string(), fixture.actor_source());
    assert_eq!(verified.operation_category, OperationCategory::Read);
    Ok(())
}

#[test]
fn read_only_mode_rejects_agent_workflow_calls_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-read-only")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let cases = [
        (
            AgentToolId::INTAKE.wire_name(),
            json!({
                "plain_language_request": "Exercise read-only rejection.",
                "requested_mode": "work",
                "resume_policy": "create_new",
                "acceptance_policy": null,
                "lineage": null,
                "initial_scope": {
                    "boundary": "Read-only rejection.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "No Core mutation occurs.",
                        "evidence_requirement": "required"
                    }]
                },
                "initial_context_refs": []
            }),
        ),
        (
            AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
            json!({
                "task_id": "task_read_only_capture",
                "change_unit_id": "cu_read_only_capture",
                "baseline_ref": "baseline_read_only_capture",
                "target": {
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "criterion_read_only_capture"
                },
                "capture": {
                    "capture_kind": "verified_command_execution",
                    "command_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "command_label": "Read-only rejection validation"
                }
            }),
        ),
    ];

    for (tool_name, arguments) in cases {
        let error = adapter
            .call_tool(tool_name, arguments)
            .expect_err("read_only should reject agent workflow calls");
        assert!(error.to_string().contains("mode read_only"));
        assert!(error.to_string().contains("agent_workflow"));
    }
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_aggregated_validation_error_is_structured_and_has_no_core_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-aggregated-validation")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::RECORD_RUN.wire_name(),
            json!({
                "kind": "unsupported",
                "observed_changes": {},
                "unexpected": true
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_INVALID_ARGUMENTS");
    assert_eq!(error["tool_name"], AgentToolId::RECORD_RUN.wire_name());
    assert_eq!(error["retryable"], true);
    tool_error_issue(&error, "/task_id", "MCP_ARGUMENT_REQUIRED");
    tool_error_issue(&error, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    tool_error_issue(&error, "/kind", "MCP_ARGUMENT_ENUM_VALUE");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn known_tool_validation_error_bounds_issue_fields_and_complete_result(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-bounded-aggregate-validation")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let mut arguments = Map::new();
    arguments.insert("kind".to_owned(), Value::String("x".repeat(16 * 1024)));
    for index in 0..(MAX_VALIDATION_ISSUES * 3) {
        arguments.insert(
            format!("unexpected_{index}_{}", "\0".repeat(1024)),
            Value::Bool(true),
        );
    }

    let error = adapter
        .call_tool(
            AgentToolId::RECORD_RUN.wire_name(),
            Value::Object(arguments),
        )
        .expect_err("pathological invalid arguments should be rejected");
    let result = tool_execution_error_result(AgentToolId::RECORD_RUN.wire_name(), &error);
    let response = structured_error_result(&result);
    let issues = response["issues"].as_array().expect("issues");

    assert!(!issues.is_empty());
    assert!(
        issues.len() < MAX_VALIDATION_ISSUES,
        "escape-heavy issues should exercise the whole-result byte cap"
    );
    assert_eq!(response["reported_issue_count"], issues.len());
    assert_eq!(response["truncated"], true);
    for issue in issues {
        assert!(issue["path"].as_str().expect("issue path").len() <= MAX_MCP_TOOL_ISSUE_PATH_BYTES);
        assert!(
            issue["message"].as_str().expect("issue message").len()
                <= MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES
        );
    }
    assert!(
        serde_json::to_vec(&result)?.len() <= MAX_MCP_TOOL_ERROR_RESULT_BYTES,
        "complete CallToolResult should honor the compact JSON byte limit"
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_adapter_precondition_error_uses_requested_tool_and_structured_flags(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-adapter-precondition")?;
    add_allowed_project(&fixture, "project_stdio_precondition_other")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, AgentToolId::STATUS.wire_name(), json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
    assert_eq!(error["tool_name"], AgentToolId::STATUS.wire_name());
    assert_eq!(error["retryable"], false);
    assert_eq!(error["reported_issue_count"], 1);
    assert_eq!(error["truncated"], false);
    tool_error_issue(
        &error,
        "/project_selector",
        "MCP_ADAPTER_PRECONDITION_FAILED",
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn project_bound_stdio_rejects_a_guessed_repository_name_as_project_selector(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-guessed-project-selector")?;
    let before = fixture.counts()?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({
                "detail": "workflow",
                "project_selector": "product-repo"
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
    assert_eq!(error["tool_name"], AgentToolId::STATUS.wire_name());
    let issue = tool_error_issue(
        &error,
        "/project_selector",
        "MCP_ADAPTER_PRECONDITION_FAILED",
    );
    let message = issue["message"].as_str().expect("routing issue message");
    assert!(message.contains("outside this MCP transport project allowlist"));
    assert!(!message.contains("HTTP serve"));
    assert!(message.contains(&format!("Use {}", AgentToolId::LIST_PROJECTS.wire_name())));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_pending_user_action_returns_cli_inbox_recovery() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-cli-inbox-recovery")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            product_action_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        AgentToolId::REQUEST_USER_ACTION.wire_name()
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    let summary = &workflow["user_action_request_summary"];
    assert_eq!(summary["status"], "pending");
    assert_eq!(summary["next_actor"], "user");
    assert!(summary["user_action_request_id"]
        .as_str()
        .is_some_and(|request_id| !request_id.is_empty()));
    assert!(workflow.get("inbox_item").is_none());
    assert!(workflow.get("user_action_request").is_none());
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("pending UserAction requires the user"));
    assert!(fallback.contains("`volicord inbox`"));
    assert!(!fallback.contains("volicord inbox resolve"));
    assert!(!fallback.contains("request.operation=resume"));
    assert!(values[1]["result"].get("_meta").is_none());
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn stdio_record_guard_uses_the_cli_inbox_without_projecting_the_private_form(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-record-guard-cli-inbox")?;
    install_record_guard(&fixture)?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            product_action_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
    )?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        AgentToolId::REQUEST_USER_ACTION.wire_name()
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    assert_eq!(workflow["user_action_request_summary"]["status"], "pending");
    assert_eq!(
        workflow["user_action_request_summary"]["next_actor"],
        "user"
    );
    assert!(workflow.get("inbox_item").is_none());
    assert!(workflow.get("user_action_request").is_none());
    let fallback_texts = values[1]["result"]["content"]
        .as_array()
        .expect("tool content")
        .iter()
        .filter_map(|content| content["text"].as_str())
        .collect::<Vec<_>>();
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("pending UserAction requires the user")));
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("`volicord inbox`")));
    assert!(fallback_texts
        .iter()
        .all(|text| !text.contains("prompt_capture") && !text.contains("volicord inbox resolve")));
    assert!(values[1]["result"].get("_meta").is_none());
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn request_user_action_agent_projection_is_only_the_exact_pending_user_summary(
) -> Result<(), Box<dyn Error>> {
    const QUESTION_MARKER: &str = "MODEL_VISIBLE_USER_ACTION_QUESTION_MUST_NOT_ESCAPE";
    const OPTION_MARKER: &str = "MODEL_VISIBLE_USER_ACTION_OPTION_MUST_NOT_ESCAPE";
    const CONTEXT_MARKER: &str = "MODEL_VISIBLE_USER_ACTION_CONTEXT_MUST_NOT_ESCAPE";

    let fixture = CoreFixture::new("mcp-agent-user-action-summary")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let mut arguments = product_action_args(&fixture, &task_id, state_version);
    arguments["request"]["action"]["question"] = json!(QUESTION_MARKER);
    arguments["request"]["action"]["context"]["summary"] = json!(CONTEXT_MARKER);
    arguments["request"]["action"]["options"][0]["label"] = json!(OPTION_MARKER);
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let tool_result = &values[1]["result"];
    assert_eq!(tool_result["isError"], false);
    let model_visible = json!({
        "content": tool_result["content"].clone(),
        "structuredContent": tool_result["structuredContent"].clone()
    });
    let model_visible_text = serde_json::to_string(&model_visible)?;
    let mut violations = Vec::new();

    for marker in [QUESTION_MARKER, OPTION_MARKER, CONTEXT_MARKER] {
        if model_visible_text.contains(marker) {
            violations.push(format!("agent projection exposed private marker {marker}"));
        }
    }
    for forbidden_key in [
        "user_action_request",
        "user_action_request_ref",
        "inbox_item",
        "request_ref",
        "question",
        "options",
        "form",
        "preferred_capture_path",
        "command",
        "url",
        "token",
    ] {
        if !json_values_for_key(&model_visible, forbidden_key).is_empty() {
            violations.push(format!(
                "agent projection exposed forbidden field {forbidden_key}"
            ));
        }
    }

    let summaries = json_values_for_key(&model_visible, "user_action_request_summary");
    if summaries.is_empty() {
        violations.push("agent projection omitted user_action_request_summary".to_owned());
    }
    for summary in summaries {
        let Some(summary) = summary.as_object() else {
            violations.push("user_action_request_summary was not an object".to_owned());
            continue;
        };
        let actual_keys = summary.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected_keys = ["next_actor", "status", "user_action_request_id"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if actual_keys != expected_keys {
            violations.push(format!(
                "user_action_request_summary keys were {actual_keys:?}, expected {expected_keys:?}"
            ));
        }
        if summary.get("status") != Some(&json!("pending")) {
            violations.push("user_action_request_summary.status was not pending".to_owned());
        }
        if summary.get("next_actor") != Some(&json!("user")) {
            violations.push("user_action_request_summary.next_actor was not user".to_owned());
        }
        if summary
            .get("user_action_request_id")
            .and_then(Value::as_str)
            .is_none_or(|request_id| request_id.is_empty())
        {
            violations.push(
                "user_action_request_summary.user_action_request_id was not a non-empty string"
                    .to_owned(),
            );
        }
    }

    assert!(
        violations.is_empty(),
        "unsafe request_user_action agent projection:\n{}",
        violations.join("\n")
    );
    Ok(())
}

#[test]
fn all_eight_user_action_kinds_preserve_the_cli_inbox_boundary() -> Result<(), Box<dyn Error>> {
    let cases = [
        McpUserActionLeakageCase::choice(
            "product_decision",
            &["close_complete"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "technical_decision",
            &["close_complete"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "scope_decision",
            &["scope_update"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "sensitive_approval",
            &["prepare_write", "close_complete"],
            McpUserActionCloseBasis::None,
            true,
        ),
        McpUserActionLeakageCase::choice(
            "final_acceptance",
            &["close_complete"],
            McpUserActionCloseBasis::NoResidualRisks,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "residual_risk_acceptance",
            &["close_complete"],
            McpUserActionCloseBasis::VisibleResidualRisk,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "cancellation",
            &["close_cancel"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::evidence_observation(),
    ];

    for case in cases {
        let fixture = CoreFixture::new(&format!("mcp-user-action-leakage-{}", case.name))?;
        let prepared = prepare_mcp_user_action_leakage_case(&fixture, case)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({})),
            initialized_notification(),
            tools_call(
                2,
                AgentToolId::REQUEST_USER_ACTION.wire_name(),
                prepared.arguments,
            ),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        assert_eq!(values.len(), 2, "{}: unexpected MCP exchange", case.name);
        let tool_result = &values[1]["result"];
        assert_eq!(
            tool_result["isError"], false,
            "{}: {tool_result}",
            case.name
        );
        let response = volicord_response_from_tool(&values[1])?;
        let summary = &response["agent_workflow_result"]["user_action_request_summary"];
        let summary_keys = summary
            .as_object()
            .unwrap_or_else(|| panic!("{}: pending summary must be an object", case.name))
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            summary_keys,
            BTreeSet::from(["next_actor", "status", "user_action_request_id"]),
            "{}: pending summary must use the exact model-visible three-field shape",
            case.name
        );
        assert_eq!(summary["status"], "pending", "{}", case.name);
        assert_eq!(summary["next_actor"], "user", "{}", case.name);
        assert!(
            summary["user_action_request_id"]
                .as_str()
                .is_some_and(|request_id| !request_id.is_empty()),
            "{}: pending summary must identify the request",
            case.name
        );

        let model_visible = json!({
            "content": tool_result["content"].clone(),
            "structuredContent": tool_result["structuredContent"].clone(),
        });
        for forbidden_key in [
            "user_action_request",
            "user_action_request_ref",
            "request_ref",
            "inbox_item",
            "question",
            "options",
            "context",
            "context_summary",
            "form",
            "preferred_capture_path",
            "answer_path_availability",
            "user_channel_availability",
            "fallbacks",
            "command",
            "url",
            "token",
            "verification_code",
            "sensitive_action_scope",
        ] {
            assert!(
                json_values_for_key(&model_visible, forbidden_key).is_empty(),
                "{}: model-visible result exposed forbidden key {forbidden_key}",
                case.name
            );
        }
        let model_visible_text = serde_json::to_string(&model_visible)?;
        for forbidden_text in prepared.private_markers.iter().map(String::as_str).chain([
            "http://",
            "/consent?",
            "token=",
        ]) {
            assert!(
                !model_visible_text.contains(forbidden_text),
                "{}: model-visible result exposed forbidden text {forbidden_text:?}",
                case.name
            );
        }

        assert!(tool_result.get("_meta").is_none(), "{}", case.name);
        assert!(
            tool_result["content"]
                .as_array()
                .is_some_and(|content| content.iter().any(|item| item["text"]
                    .as_str()
                    .is_some_and(|text| text.contains("`volicord inbox`")))),
            "{}",
            case.name
        );

        let record = stored_action_record(&fixture, &prepared.task_id, &response)?;
        assert_eq!(
            serde_json::to_value(record.request.action_kind)?,
            json!(case.name),
            "{}: fixture must exercise the intended action kind",
            case.name
        );
        assert!(
            record.resolution.is_none(),
            "{}: handoff delivery must not resolve the action",
            case.name
        );
        assert_eq!(
            fixture.counts()?.user_action_resolutions,
            0,
            "{}: handoff delivery must create no resolution row",
            case.name
        );
    }
    Ok(())
}

#[test]
fn stdio_rejects_tampered_summaries_and_noncanonical_full_form_before_delivery(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-pending-form-fail-closed")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let before = user_action_side_effect_snapshot(&fixture)?;

    let mut mismatched_id = pending_response.clone();
    mismatched_id.response_value["user_action_request_summary"]["user_action_request_id"] =
        json!("uar_not_in_the_trusted_projection");
    let mut invalid_summary = pending_response.clone();
    invalid_summary.response_value["user_action_request_summary"]["next_actor"] = json!("agent");
    let mut noncanonical_full_form = pending_response.clone();
    noncanonical_full_form.response_value["inbox_item"] = json!({
        "form": {"question": "noncanonical model-visible form must not be trusted"}
    });

    for (case, response) in [
        ("mismatched_id", mismatched_id),
        ("invalid_summary", invalid_summary),
        ("noncanonical_full_form", noncanonical_full_form),
    ] {
        let error = crate::stdio::user_action_tool_output(&adapter(&fixture)?, response)
            .expect_err("untrusted public pending data must fail before delivery");
        assert!(matches!(
            error,
            McpAdapterError::Protocol(_) | McpAdapterError::Json(_)
        ));
        assert_eq!(
            user_action_side_effect_snapshot(&fixture)?,
            before,
            "{case} must not create a token, resolution, or project effect"
        );
    }
    Ok(())
}

#[test]
fn stdio_resume_replays_exact_origin_after_cli_inbox_resolution() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-user-action-cli-inbox-resume")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;

    let create_input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            product_action_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut create_output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(create_input),
        &mut create_output,
    )?;

    let create_values = stdio_responses(&create_output)?;
    assert_eq!(create_values.len(), 2);
    let created = volicord_response_from_tool(&create_values[1])?;
    assert_eq!(created["current_status"], "pending");
    assert_eq!(created["agent_workflow_result_replayed"], false);
    let exact_origin = created["agent_workflow_result"].clone();
    let exact_origin_bytes = serde_json::to_vec(&exact_origin)?;
    let origin_operation_result_ref =
        create_values[1]["result"]["structuredContent"]["operation_result_ref"].clone();
    let user_action_request_id = exact_origin["user_action_request_summary"]
        ["user_action_request_id"]
        .as_str()
        .ok_or("created response should identify the user-action request")?
        .to_owned();
    let after_create = fixture.counts()?;

    let core = CoreService::new(fixture.runtime_home_path());
    let resolved = core.resolve_user_action(
        fixture.resolve_user_action_request(ResolveUserActionFixture {
            request_id: "req_cli_inbox_resolution",
            task_id: &task_id,
            user_action_request_id: &user_action_request_id,
            channel_submission_id: "submission_cli_inbox_resolution",
            resolution: volicord_types::UserActionResolutionInput::Choice {
                selected_option_id: volicord_types::UserActionOptionId::new("keep"),
                note: Some("This private user note must not enter the MCP projection.".to_owned())
                    .into(),
            },
        }),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            volicord_types::VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        ),
    )?;
    assert_eq!(resolved.response_value["base"]["response_kind"], "result");
    let historical_derived_refs = resolved.response_value["derived_refs"].clone();
    assert!(historical_derived_refs
        .as_array()
        .is_some_and(|refs| !refs.is_empty()));
    let historical_resolution_ref = resolved.response_value["user_action_resolution_ref"].clone();
    let resolution_state_version = resolved.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("resolution should report its committed state version")?;
    let after_resolution = fixture.counts()?;
    assert_eq!(
        after_resolution.user_action_requests,
        after_create.user_action_requests
    );
    assert_eq!(
        after_resolution.user_action_resolutions,
        after_create.user_action_resolutions + 1
    );

    let unrelated = core.request_user_action(
        fixture.user_action_request(UserActionFixture {
            request_id: "req_mcp_cross_channel_unrelated_action",
            idempotency_key: "idem_mcp_cross_channel_unrelated_action",
            dry_run: false,
            expected_state_version: Some(resolution_state_version),
            task_id: &task_id,
            change_unit_id: None,
            judgment_kind: volicord_types::JudgmentKind::TechnicalDecision,
        }),
        test_agent_invocation(&fixture, OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(unrelated.response_value["base"]["response_kind"], "result");
    let before_resume = fixture.counts()?;
    assert_eq!(before_resume.state_version, resolution_state_version + 1);
    assert_eq!(
        before_resume.user_action_requests,
        after_resolution.user_action_requests + 1
    );

    let wrong_connection_id = "conn_mcp_cross_channel_wrong";
    let wrong_adapter = adapter_for_additional_connection(&fixture, wrong_connection_id)?;
    let wrong_error = wrong_adapter
        .call_tool(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            resume_user_action_args(&fixture, &user_action_request_id),
        )
        .expect_err("another Agent Connection must not resume the originating result");
    assert!(matches!(wrong_error, McpAdapterError::ToolExecution { .. }));
    assert_eq!(fixture.counts()?, before_resume);

    let resume_input = Cursor::new(json_lines(&[
        initialize_request(3, json!({})),
        initialized_notification(),
        tools_call(
            4,
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            resume_user_action_args(&fixture, &user_action_request_id),
        ),
    ])?);
    let mut resume_output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(resume_input),
        &mut resume_output,
    )?;

    let resume_values = stdio_responses(&resume_output)?;
    assert_eq!(resume_values.len(), 2);
    let resumed = volicord_response_from_tool(&resume_values[1])?;
    assert_eq!(
        serde_json::to_vec(&resumed["agent_workflow_result"])?,
        exact_origin_bytes
    );
    assert_eq!(resumed["agent_workflow_result_replayed"], true);
    assert_eq!(resumed["current_status"], "resolved");
    assert_eq!(
        resumed["current_projection_state_version"],
        before_resume.state_version
    );
    assert!(resumed["current_projection_observed_at"].is_string());
    assert_eq!(
        resumed["user_channel_resolution_ref"],
        historical_resolution_ref
    );
    assert_eq!(resumed["derived_refs"], historical_derived_refs);
    assert_eq!(
        resumed["user_channel_resolution"]["resolution_summary"]["selected_option_id"],
        "keep"
    );
    assert!(resumed["user_channel_resolution"]
        .to_string()
        .find("private user note")
        .is_none());
    assert_eq!(
        resume_values[1]["result"]["structuredContent"]["operation_result_ref"],
        origin_operation_result_ref
    );
    assert_eq!(fixture.counts()?, before_resume);
    Ok(())
}

#[test]
fn project_tool_rejects_missing_managed_session_coordinates() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-managed-agent-session-required")?;
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), context);

    let error = adapter
        .call_tool(
            AgentToolId::STATUS.wire_name(),
            json!({"detail": "workflow"}),
        )
        .expect_err("project tools require current managed session coordinates");
    assert!(error.to_string().contains("agent_session_missing"));
    Ok(())
}

#[test]
fn invented_session_coordinates_do_not_authorize_or_insert_a_project_session(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invented-session-not-authority")?;
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
    let before_sessions = read_only_table_count(&fixture, "host_sessions")?;

    let error = adapter
        .call_tool_for_session(
            AgentToolId::STATUS,
            json!({"detail": "workflow"}),
            Some(AgentSessionCoordinates {
                runtime_session_id: "mcp_invented_runtime",
                project_session_id: "agent_invented_session",
            }),
        )
        .expect_err("caller-invented coordinates must not establish session authority");

    assert!(error
        .to_string()
        .contains("agent_runtime_session_not_current"));
    assert_eq!(
        read_only_table_count(&fixture, "host_sessions")?,
        before_sessions
    );
    Ok(())
}
