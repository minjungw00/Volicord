use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::{BufReader, Cursor},
};

use crate::local_http::{validate_bearer_token_text, validate_local_http_server_config};
use crate::local_web_consent::{parse_urlencoded, single_param};
use crate::prelude::*;
use crate::stdio::{pending_judgment_from_response, percent_encode_query};
use volicord_core::CoreBoundary;
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_MODE_READ_ONLY,
};
use volicord_store::bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS};
use volicord_store::guards::{
    list_unresolved_unrecorded_changes, upsert_guard_installation, GuardInstallationUpsert,
};
use volicord_store::session_watch::{
    latest_watch_baseline_for_connection, latest_watch_baseline_for_session,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    AgentConnectionMode, OperationCategory, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

use super::*;

#[test]
fn mcp_boundary_wraps_core_boundary() {
    assert_eq!(
        McpAdapterBoundary::new(CoreBoundary::new()).label(),
        "mcp-adapter"
    );
}

#[test]
fn tool_sets_follow_connection_mode_and_exclude_user_only_recording() {
    let workflow = mcp_tools_for_mode(AgentConnectionMode::Workflow);
    let workflow_names = tool_names(&workflow);
    assert_eq!(
        &workflow_names[..PUBLIC_METHOD_TOOL_NAMES.len()],
        PUBLIC_METHOD_TOOL_NAMES
    );
    assert!(workflow_names.contains(&"volicord.request_user_judgment"));
    assert!(workflow_names.contains(&"volicord.reconcile_changes"));
    assert!(workflow_names.contains(&CHECK_CLOSE_TOOL_NAME));
    assert!(workflow_names.contains(&"volicord.close_task"));
    assert!(!workflow_names.contains(&"volicord.record_user_judgment"));
    assert_eq!(
        workflow_names.last().copied(),
        Some(LIST_PROJECTS_TOOL_NAME)
    );

    let read_only = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
    let read_only_names = tool_names(&read_only);
    assert_eq!(
        read_only_names,
        vec![
            "volicord.status",
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
}

#[test]
fn mcp_visible_schemas_hide_envelope_and_metadata() {
    for tool in public_method_tools() {
        let properties = root_properties(&tool.input_schema);
        let required = root_required_fields(&tool.input_schema);
        assert!(
            properties.contains(&"project_selector".to_owned()),
            "{} should expose the public project selector",
            tool.name
        );
        assert!(
            !required.contains(&"project_selector".to_owned()),
            "{} should not require project selection for single-project connections",
            tool.name
        );
        for forbidden in [
            "envelope",
            "project_id",
            "request_id",
            "idempotency_key",
            "expected_state_version",
            "dry_run",
            "locale",
            "actor_source",
            "operation_category",
            "mode",
            "connection_id",
        ] {
            assert!(
                !properties.contains(&forbidden.to_owned()),
                "{} should not expose MCP-internal field {forbidden}",
                tool.name
            );
        }
        assert!(
            !schema_has_definition(&tool.input_schema, "ToolEnvelope"),
            "{} should not include the internal ToolEnvelope schema",
            tool.name
        );
    }
}

#[test]
fn generated_bearer_token_is_visible_ascii_hex() -> Result<(), Box<dyn Error>> {
    let token = generate_bearer_token()?;

    assert_eq!(token.len(), 64);
    assert!(validate_bearer_token_text(&token).is_ok());
    assert!(token
        .chars()
        .all(|character| matches!(character, '0'..='9' | 'a'..='f')));
    assert!(!token.chars().any(char::is_whitespace));
    Ok(())
}

#[test]
fn generated_bearer_tokens_are_unique_in_small_sample() -> Result<(), Box<dyn Error>> {
    let mut tokens = BTreeSet::new();
    for _ in 0..8 {
        let token = generate_bearer_token()?;
        assert!(
            tokens.insert(token),
            "generated bearer token repeated in a small sanity sample"
        );
    }
    Ok(())
}

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
    assert!(report.contains(&format!("connection_id: {}", fixture.connection_id())));
    assert!(report.contains("mode: workflow"));
    assert!(report.contains("allowed_projects: 1"));
    assert!(report.contains("available_projects: 1"));
    assert!(report.contains("watcher_status: pending_mcp_start"));
    assert!(report.contains("watcher_coverage_basis: mcp_start"));
    Ok(())
}

#[test]
fn project_bound_stdio_startup_creates_baseline_before_tool_handling() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-stdio-startup-watch")?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    assert!(output.is_empty());
    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("stdio startup should create a watch baseline");
    assert_eq!(baseline.status, "active");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["coverage_basis"], "mcp_start");
    assert!(metadata.get("partial_coverage_warning").is_none());
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
fn multi_project_session_reports_pending_project_selection() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-pending")?;
    add_allowed_project(&fixture, "project_watch_pending_other")?;
    let adapter = adapter(&fixture)?;

    let result =
        adapter.call_adapter_tool(LIST_PROJECTS_TOOL_NAME, json!({}), Some("session_pending"))?;

    assert_eq!(result["watcher_status"], "pending_project_selection");
    assert!(result["watcher_baseline_created_at"].is_null());
    assert!(result["watcher_coverage_start_at"].is_null());
    assert!(result["watcher_coverage_basis"].is_null());
    assert!(result["watcher_partial_coverage_warning"]
        .as_str()
        .unwrap_or_default()
        .contains("project_selector"));
    Ok(())
}

#[test]
fn first_project_selection_creates_partial_coverage_baseline() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-first-selection")?;
    add_allowed_project(&fixture, "project_watch_first_selection_other")?;
    let adapter = adapter(&fixture)?;
    let session_id = "session_first_project_selection";

    let response = adapter.call_tool_for_session(
        "volicord.status",
        json!({ "project_selector": fixture.project_id() }),
        Some(session_id),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let baseline = latest_watch_baseline_for_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        session_id,
    )?
    .expect("first explicit project selection should create a baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["coverage_basis"], "first_project_selection");
    assert!(metadata["partial_coverage_warning"]
        .as_str()
        .unwrap_or_default()
        .contains("project selection"));
    Ok(())
}

#[test]
fn project_bound_early_edit_is_detected_on_first_check() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-early-edit")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&adapter)?;
    let session_id = "session_project_bound_early_edit";
    adapter.initialize_startup_session_watch(session_id)?;
    write_product_file(&fixture, "src/early.txt", "changed before first method\n")?;

    let response = adapter.call_tool_for_session(
        CHECK_CLOSE_TOOL_NAME,
        json!({ "task_id": task_id }),
        Some(session_id),
    )?;

    assert_eq!(
        response.response_value["guard_health"]["session_watch_coverage_basis"],
        "mcp_start"
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_partial_coverage_warning"],
        Value::Null
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    let changes = list_unresolved_unrecorded_changes(
        fixture.runtime_home_path(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?;
    assert_eq!(changes.len(), 1);
    assert!(!changes[0]
        .detection_json
        .contains("changed before first method"));
    Ok(())
}

#[test]
fn edit_before_project_selection_is_reported_outside_coverage() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-before-selection")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    add_allowed_project(&fixture, "project_watch_before_selection_other")?;
    let adapter = adapter(&fixture)?;
    let session_id = "session_before_project_selection";
    write_product_file(&fixture, "src/before-selection.txt", "before selection\n")?;

    let response = adapter.call_tool_for_session(
        CHECK_CLOSE_TOOL_NAME,
        json!({
            "project_selector": fixture.project_id(),
            "task_id": task_id
        }),
        Some(session_id),
    )?;

    assert_eq!(
        response.response_value["guard_health"]["session_watch_coverage_basis"],
        "first_project_selection"
    );
    assert!(
        response.response_value["guard_health"]["session_watch_partial_coverage_warning"]
            .as_str()
            .unwrap_or_default()
            .contains("project selection")
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    let changes = list_unresolved_unrecorded_changes(
        fixture.runtime_home_path(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?;
    assert!(changes.is_empty());
    Ok(())
}

#[test]
fn read_only_mode_rejects_agent_workflow_calls_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-read-only")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let error = adapter
        .call_tool(
            "volicord.intake",
            json!({
                "plain_language_request": "Exercise read-only rejection.",
                "requested_mode": "work",
                "resume_policy": "create_new",
                "initial_scope": {
                    "boundary": "Read-only rejection.",
                    "non_goals": [],
                    "acceptance_criteria": ["No Core mutation occurs."]
                },
                "initial_context_refs": []
            }),
        )
        .expect_err("read_only should reject agent workflow calls");

    assert!(error.to_string().contains("mode read_only"));
    assert!(error.to_string().contains("agent_workflow"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_lists_mode_filtered_tools() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-mode")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let names = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "volicord.status",
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
    Ok(())
}

#[test]
fn stdio_elicitation_accept_records_user_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-accept")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("keep", None),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 3);
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    assert_eq!(values[1]["id"], "elicit_user_judgment_1");
    assert_eq!(
        values[1]["params"]["requestedSchema"]["properties"]["selected_option_id"]["enum"][0],
        "keep"
    );
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["base"]["response_kind"], "result");
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["resolved_by_actor_source"],
        "local_user"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "keep"
    );
    assert_eq!(
        stored_resolution_basis(&fixture, &task_id, &response)?,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    Ok(())
}

#[test]
fn stdio_elicitation_decline_records_rejected_authority_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-decline")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            authority_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_action("decline"),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "reject"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["resolution_outcome"],
        "rejected"
    );
    assert_eq!(
        stored_resolution_basis(&fixture, &task_id, &response)?,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    Ok(())
}

#[test]
fn stdio_elicitation_accept_can_record_deferred_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-defer")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            authority_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("defer", Some("Not enough context yet.")),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "defer"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["resolution_outcome"],
        "deferred"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["note"],
        "Not enough context yet."
    );
    Ok(())
}

#[test]
fn stdio_elicitation_cancel_leaves_judgment_pending() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-cancel")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_action("cancel"),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert!(values[2]["result"]["content"][1]["text"]
        .as_str()
        .expect("extra text")
        .contains("remains pending"));
    let record = stored_judgment_record(&fixture, &task_id, &response)?;
    assert_eq!(record.status, "pending");
    assert!(record.resolved_verification_basis.is_none());
    Ok(())
}

#[test]
fn stdio_elicitation_invalid_response_leaves_judgment_pending() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-invalid")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("not_an_option", None),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert!(values[2]["result"]["content"][1]["text"]
        .as_str()
        .expect("extra text")
        .contains("unknown option_id"));
    let record = stored_judgment_record(&fixture, &task_id, &response)?;
    assert_eq!(record.status, "pending");
    Ok(())
}

#[test]
fn stdio_without_elicitation_capability_returns_cli_recovery_when_prompt_capture_unavailable(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-unavailable")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    let response = volicord_response_from_tool(&values[1])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert_eq!(
        response["inbox_item"]["preferred_capture_path"]["kind"],
        "cli"
    );
    assert!(response["inbox_item"]["preferred_capture_path"]["command"]
        .as_str()
        .expect("CLI fallback command should be present")
        .contains("volicord inbox answer"));
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("MCP elicitation is unavailable"));
    assert!(fallback.contains("local CLI recovery path"));
    assert!(fallback.contains("volicord inbox answer"));
    assert!(!fallback.contains("Volicord: answer J-1 1 #"));
    Ok(())
}

#[test]
fn stdio_without_elicitation_capability_returns_chat_capture_when_configured(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-chat-capture")?;
    install_prompt_capture_guard(&fixture)?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    let response = volicord_response_from_tool(&values[1])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert_eq!(
        response["inbox_item"]["preferred_capture_path"]["kind"],
        "prompt_capture"
    );
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("MCP elicitation is unavailable"));
    assert!(fallback.contains("Volicord: answer J-1 1 #"));
    assert!(fallback.contains("Volicord: note J-1 \"text\" #"));
    Ok(())
}

#[test]
fn stdio_without_elicitation_uses_local_web_consent_when_prompt_capture_unavailable(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-fallback")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter_with_local_web_consent(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    let response = volicord_response_from_tool(&values[1])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert_eq!(
        response["inbox_item"]["preferred_capture_path"]["kind"],
        "local_web_consent"
    );
    assert!(response["inbox_item"]["preferred_capture_path"]["url"]
        .as_str()
        .expect("local web URL should be present")
        .starts_with(&format!(
            "{}{}?project=",
            consent_base_url(),
            LOCAL_WEB_CONSENT_PATH
        )));
    assert!(response["inbox_item"]["fallbacks"]
        .as_array()
        .expect("inbox fallbacks should be an array")
        .iter()
        .any(|fallback| fallback["kind"] == "cli"
            && fallback["command"]
                .as_str()
                .is_some_and(|command| command.contains("volicord inbox answer"))));
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("local Volicord consent link"));
    assert!(!fallback.contains("volicord user judgment answer"));

    let state: Value = serde_json::from_str(
        values[1]["result"]["content"][2]["text"]
            .as_str()
            .expect("structured fallback text"),
    )?;
    let state = &state["volicord_fallback"];
    assert_eq!(state["kind"], "local_web_consent");
    assert_eq!(state["project_id"], fixture.project_id());
    assert_eq!(state["connection_id"], fixture.connection_id());
    assert_eq!(
        state["capture_basis"],
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
    );
    let url = state["url"].as_str().expect("fallback URL");
    assert!(url.starts_with(&format!(
        "{}{}?project=",
        consent_base_url(),
        LOCAL_WEB_CONSENT_PATH
    )));
    let token = token_from_consent_url(url)?;
    let now =
        local_web_consent_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let validation = validate_local_web_consent_token(
        fixture.runtime_home_path(),
        LocalWebConsentTokenCheck {
            token,
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now,
        },
    )?;
    assert!(matches!(
        validation,
        LocalWebConsentTokenValidation::Valid(_)
    ));
    Ok(())
}

#[test]
fn local_web_consent_get_renders_pending_judgment_page() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-get")?;
    let (_task_id, response) = create_pending_product_judgment(&fixture)?;
    let token = "1111111111111111111111111111111111111111111111111111111111111111";
    create_consent_token_for_response(&fixture, &response, token, 60)?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));

    assert_eq!(response.status, 200);
    let body = http_body_text(&response)?;
    assert!(body.contains("Volicord Consent"));
    assert!(body.contains("Choose the focused MCP elicitation test outcome."));
    assert!(body.contains("local_user_local_web"));
    assert!(!body.contains("Runtime Home"));
    Ok(())
}

#[test]
fn local_web_consent_post_records_user_owned_answer() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-post")?;
    let (task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "2222222222222222222222222222222222222222222222222222222222222222";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=keep&note=Browser+answer",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));

    assert_eq!(response.status, 200);
    let body = http_body_text(&response)?;
    assert!(body.contains("Answer recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_judgment_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, "resolved");
    assert_eq!(
        record.resolved_by_actor_source.as_deref(),
        Some("local_user")
    );
    assert_eq!(
        record.resolved_verification_basis.as_deref(),
        Some(VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB)
    );
    Ok(())
}

#[test]
fn local_web_consent_rejects_origin_mismatch_without_consuming_token() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-local-web-origin")?;
    let (task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "9999999999999999999999999999999999999999999999999999999999999999";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;
    let form_body = format!(
        "project={}&token={}&selected_option_id=keep",
        percent_encode_query(fixture.project_id()),
        token
    );

    let rejected = server.handle_request(consent_post_request(
        Some("http://example.invalid"),
        &form_body,
    ));

    assert_eq!(rejected.status, 403);
    assert!(http_body_text(&rejected)?.contains("ORIGIN_NOT_ALLOWED"));

    let valid = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));
    assert_eq!(valid.status, 200);
    assert!(http_body_text(&valid)?.contains("Answer recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_judgment_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, "resolved");
    Ok(())
}

#[test]
fn local_web_consent_validation_failure_leaves_token_reusable() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-validation-retry")?;
    let (task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "8888888888888888888888888888888888888888888888888888888888888888";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;

    let invalid = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=missing",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));
    assert_eq!(invalid.status, 400);
    assert!(http_body_text(&invalid)?.contains("INVALID_SELECTION"));

    let valid = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=keep",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));

    assert_eq!(valid.status, 200);
    assert!(http_body_text(&valid)?.contains("Answer recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_judgment_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, "resolved");
    Ok(())
}

#[test]
fn local_web_consent_rejects_invalid_token() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-invalid")?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        "invalid-token",
    )));

    assert_eq!(response.status, 404);
    assert!(http_body_text(&response)?.contains("INVALID_TOKEN"));
    Ok(())
}

#[test]
fn local_web_consent_rejects_expired_token() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-expired")?;
    let (_task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "3333333333333333333333333333333333333333333333333333333333333333";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    volicord_store::local_consent::expire_local_web_consent_tokens(
        fixture.runtime_home_path(),
        fixture.project_id(),
        "2999-01-01T00:00:00.000Z",
    )?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));

    assert_eq!(response.status, 410);
    assert!(http_body_text(&response)?.contains("TOKEN_EXPIRED"));
    Ok(())
}

#[test]
fn local_web_consent_rejects_replay() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-replay")?;
    let (_task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "4444444444444444444444444444444444444444444444444444444444444444";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;
    let form_body = format!(
        "project={}&token={}&selected_option_id=keep",
        percent_encode_query(fixture.project_id()),
        token
    );

    let first = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));
    let replay = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));

    assert_eq!(first.status, 200);
    assert_eq!(replay.status, 409);
    assert!(http_body_text(&replay)?.contains("TOKEN_CONSUMED"));
    Ok(())
}

#[test]
fn local_web_consent_rejects_wrong_project_and_connection() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-context")?;
    let (_task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "5555555555555555555555555555555555555555555555555555555555555555";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;

    let mut server = consent_server(&fixture)?;
    let wrong_project =
        server.handle_request(consent_get_request(&consent_target("project_other", token)));
    assert_eq!(wrong_project.status, 403);
    assert!(http_body_text(&wrong_project)?.contains("WRONG_PROJECT"));

    let mut wrong_connection_server =
        consent_server_for_connection(&fixture, "conn_mcp_local_web_other")?;
    let wrong_connection = wrong_connection_server.handle_request(consent_get_request(
        &consent_target(fixture.project_id(), token),
    ));
    assert_eq!(wrong_connection.status, 403);
    assert!(http_body_text(&wrong_connection)?.contains("WRONG_CONNECTION"));
    Ok(())
}

#[test]
fn local_http_rejects_missing_bearer_auth() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-auth")?;
    let mut server = http_server(&fixture, Vec::new(), Vec::new())?;

    let response = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        None,
        None,
        None,
        initialize_request(1, json!({})),
    )?);

    assert_eq!(response.status, 401);
    assert_eq!(http_json(&response)["error"]["code"], "AUTH_REQUIRED");
    assert_diagnostic_disclosure(&http_json(&response));
    assert_eq!(http_header(&response, "WWW-Authenticate"), Some("Bearer"));

    let unauthenticated_health = server.handle_request(http_request(
        "GET",
        "/healthz",
        None,
        None,
        None,
        Value::Null,
    )?);
    assert_eq!(unauthenticated_health.status, 401);
    assert_eq!(
        http_json(&unauthenticated_health)["error"]["code"],
        "AUTH_REQUIRED"
    );

    let health = server.handle_request(http_request(
        "GET",
        "/healthz",
        Some("test_token"),
        None,
        None,
        Value::Null,
    )?);
    assert_eq!(health.status, 200);
    assert_eq!(http_json(&health)["status"], "ok");
    assert_diagnostic_disclosure(&http_json(&health));
    let health_body = serde_json::to_string(&http_json(&health))?;
    assert!(!health_body.contains("test_token"));
    assert!(!health_body.contains(fixture.connection_id()));
    assert!(!health_body.contains(fixture.project_id()));
    assert!(!health_body.contains(&fixture.runtime_home_path().display().to_string()));
    Ok(())
}

#[test]
fn local_http_rejects_origin_unless_explicitly_allowed() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-origin")?;
    let mut server = http_server(&fixture, Vec::new(), Vec::new())?;

    let rejected = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        Some("https://example.invalid"),
        None,
        initialize_request(1, json!({})),
    )?);

    assert_eq!(rejected.status, 403);
    assert_eq!(http_json(&rejected)["error"]["code"], "ORIGIN_NOT_ALLOWED");
    assert_eq!(http_header(&rejected, "Access-Control-Allow-Origin"), None);

    let denied_preflight = server.handle_request(http_request(
        "OPTIONS",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        None,
        Some("https://example.invalid"),
        None,
        Value::Null,
    )?);
    assert_eq!(denied_preflight.status, 403);
    assert_eq!(
        http_json(&denied_preflight)["error"]["code"],
        "ORIGIN_NOT_ALLOWED"
    );

    let mut allowed_server = http_server(
        &fixture,
        Vec::new(),
        vec!["https://allowed.example".to_owned()],
    )?;
    let allowed = allowed_server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        Some("https://allowed.example"),
        None,
        initialize_request(2, json!({})),
    )?);
    assert_eq!(allowed.status, 200);
    assert_eq!(
        http_header(&allowed, "Access-Control-Allow-Origin"),
        Some("https://allowed.example")
    );
    Ok(())
}

#[test]
fn local_http_rejects_nonlocal_listen_addresses() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-listen")?;

    for listen_addr in ["0.0.0.0:8765", "[::]:8765", "192.0.2.10:8765"] {
        let mut config = http_config(&fixture, Vec::new(), Vec::new());
        config.listen_addr = listen_addr.parse()?;
        let error = validate_local_http_server_config(&config)
            .expect_err("nonlocal listen address should be rejected");
        assert!(
            error.to_string().contains("NONLOCAL_LISTEN_REJECTED"),
            "unexpected error for {listen_addr}: {error}"
        );
    }

    for listen_addr in ["127.0.0.1:0", "[::1]:0"] {
        let mut config = http_config(&fixture, Vec::new(), Vec::new());
        config.listen_addr = listen_addr.parse()?;
        validate_local_http_server_config(&config)?;
    }
    Ok(())
}

#[test]
fn project_bound_http_initialize_creates_baseline_before_tool_handling(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-startup-watch")?;
    let mut server = http_server(
        &fixture,
        vec![ProjectId::new(fixture.project_id())],
        Vec::new(),
    )?;

    let initialize = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        None,
        initialize_request(1, json!({})),
    )?);

    assert_eq!(initialize.status, 200);
    assert!(http_header(&initialize, "Mcp-Session-Id").is_some());
    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("HTTP initialize should create a watch baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["coverage_basis"], "mcp_start");
    Ok(())
}

#[test]
fn local_http_project_allowlist_narrows_connection_projects() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-project-allowlist")?;
    let outside_project_id = "project_http_allowed_by_connection";
    add_allowed_project(&fixture, outside_project_id)?;
    let mut server = http_server(
        &fixture,
        vec![ProjectId::new(fixture.project_id())],
        Vec::new(),
    )?;

    let initialize = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        None,
        initialize_request(1, json!({})),
    )?);
    assert_eq!(initialize.status, 200);
    let session_id = http_header(&initialize, "Mcp-Session-Id")
        .expect("initialize should create session")
        .to_owned();

    let initialized = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        Some(&session_id),
        initialized_notification(),
    )?);
    assert_eq!(initialized.status, 202);

    let listed = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        Some(&session_id),
        tools_call(2, LIST_PROJECTS_TOOL_NAME, json!({})),
    )?);
    assert_eq!(listed.status, 200);
    let listed_tool = volicord_response_from_tool(&http_json(&listed))?;
    let projects = listed_tool["projects"]
        .as_array()
        .expect("projects should be listed");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["project_selector"], fixture.project_id());

    let rejected = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        Some(&session_id),
        tools_call(
            3,
            "volicord.status",
            json!({
                "detail": "workflow",
                "project_selector": outside_project_id
            }),
        ),
    )?);
    assert_eq!(rejected.status, 200);
    assert_eq!(http_json(&rejected)["result"]["isError"], true);
    let error_text = http_json(&rejected)["result"]["content"][0]["text"]
        .as_str()
        .expect("tool error should be text")
        .to_owned();
    assert!(error_text.contains("outside this HTTP serve project allowlist"));
    Ok(())
}

fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

fn adapter_with_local_web_consent(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    Ok(
        adapter(fixture)?.with_local_web_consent(LocalWebConsentContext {
            base_url: consent_base_url().to_owned(),
        }),
    )
}

fn consent_base_url() -> &'static str {
    "http://127.0.0.1:39000"
}

fn install_prompt_capture_guard(fixture: &CoreFixture) -> Result<(), Box<dyn Error>> {
    upsert_guard_installation(
        fixture.runtime_home_path(),
        GuardInstallationUpsert {
            guard_installation_id: "guard_installation_mcp_prompt_capture".to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: Some(fixture.project_id().to_owned()),
            host_kind: "prompt_capture_test_host".to_owned(),
            guard_mode: IntegrationProfile::Observe.as_str().to_owned(),
            host_capability_json: json!({
                "schema": "volicord-guard-capability-v1",
                "policy_hash": "sha256:mcp-prompt-capture",
                "host_capabilities": {
                    "user_prompt_submit_hook": true
                },
                "required_guard_phases": [
                    "session_start_hook",
                    "pre_tool_hook",
                    "post_tool_hook",
                    "user_prompt_submit_hook",
                    "stop_hook"
                ],
                "missing_required_hooks": [],
                "prompt_capture": true
            })
            .to_string(),
            installation_status: "configured".to_owned(),
            installed_at: Some("2026-06-30T00:00:00Z".to_owned()),
            last_checked_at: "2026-06-30T00:00:00Z".to_owned(),
            first_seen_at: None,
            last_seen_at: None,
            last_seen_phase: None,
            observed_host_kind: None,
            observed_policy_hash: None,
            observed_binary_version: None,
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn set_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
    let existing = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
        .expect("fixture connection should exist");
    ensure_agent_connection(
        fixture.runtime_home_path(),
        AgentConnectionRegistration {
            connection_internal_id: existing.connection_internal_id,
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: existing.config_target,
            mode: mode.to_owned(),
            enabled: existing.enabled,
            managed_fingerprint: existing.managed_fingerprint,
            last_verification_status: existing.last_verification_status,
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    Ok(())
}

fn http_config(
    fixture: &CoreFixture,
    project_allowlist: Vec<ProjectId>,
    allowed_origins: Vec<String>,
) -> LocalHttpServerConfig {
    LocalHttpServerConfig {
        runtime_home: fixture.runtime_home_path().to_path_buf(),
        connection_id: fixture.connection_id().to_owned(),
        listen_addr: "127.0.0.1:0".parse().expect("valid test listen"),
        bearer_token: "test_token".to_owned(),
        token_source: LocalHttpTokenSource::Supplied,
        project_allowlist,
        allowed_origins,
    }
}

fn http_server(
    fixture: &CoreFixture,
    project_allowlist: Vec<ProjectId>,
    allowed_origins: Vec<String>,
) -> Result<LocalHttpServer, Box<dyn Error>> {
    let config = http_config(fixture, project_allowlist.clone(), allowed_origins);
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING)
            .with_project_allowlist(project_allowlist);
    Ok(LocalHttpServer::new(
        McpAdapter::new(fixture.runtime_home_path(), context),
        config,
    ))
}

fn consent_server(fixture: &CoreFixture) -> Result<LocalHttpServer, Box<dyn Error>> {
    consent_server_with_context(
        fixture,
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING),
    )
}

fn consent_server_for_connection(
    fixture: &CoreFixture,
    connection_id: &str,
) -> Result<LocalHttpServer, Box<dyn Error>> {
    let existing = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
        .expect("fixture connection should exist");
    ensure_agent_connection(
        fixture.runtime_home_path(),
        AgentConnectionRegistration {
            connection_internal_id: connection_id.to_owned(),
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: format!("{}_other", existing.config_target),
            mode: existing.mode,
            enabled: existing.enabled,
            managed_fingerprint: format!("{}_other", existing.managed_fingerprint),
            last_verification_status: existing.last_verification_status,
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    add_connection_project(
        fixture.runtime_home_path(),
        ConnectionProjectRegistration {
            connection_internal_id: connection_id.to_owned(),
            project_id: fixture.project_id().to_owned(),
        },
    )?;
    consent_server_with_context(
        fixture,
        McpConnectionContext::resolve(fixture.runtime_home_path(), connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING),
    )
}

fn consent_server_with_context(
    fixture: &CoreFixture,
    context: McpConnectionContext,
) -> Result<LocalHttpServer, Box<dyn Error>> {
    Ok(LocalHttpServer::new(
        McpAdapter::new(fixture.runtime_home_path(), context).with_local_web_consent(
            LocalWebConsentContext {
                base_url: consent_base_url().to_owned(),
            },
        ),
        http_config(fixture, Vec::new(), Vec::new()),
    ))
}

fn http_request(
    method: &str,
    target: &str,
    token: Option<&str>,
    origin: Option<&str>,
    session_id: Option<&str>,
    body: Value,
) -> Result<HttpRequest, serde_json::Error> {
    let mut headers = BTreeMap::new();
    headers.insert(
        "accept".to_owned(),
        "application/json, text/event-stream".to_owned(),
    );
    headers.insert("content-type".to_owned(), "application/json".to_owned());
    if let Some(token) = token {
        headers.insert("authorization".to_owned(), format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        headers.insert("origin".to_owned(), origin.to_owned());
    }
    if let Some(session_id) = session_id {
        headers.insert("mcp-session-id".to_owned(), session_id.to_owned());
    }
    Ok(HttpRequest {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
        body: serde_json::to_vec(&body)?,
    })
}

fn consent_get_request(target: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_owned(),
        target: target.to_owned(),
        headers: BTreeMap::new(),
        body: Vec::new(),
    }
}

fn consent_post_request(origin: Option<&str>, body: &str) -> HttpRequest {
    let mut headers = BTreeMap::new();
    headers.insert(
        "content-type".to_owned(),
        "application/x-www-form-urlencoded".to_owned(),
    );
    if let Some(origin) = origin {
        headers.insert("origin".to_owned(), origin.to_owned());
    }
    HttpRequest {
        method: "POST".to_owned(),
        target: LOCAL_WEB_CONSENT_PATH.to_owned(),
        headers,
        body: body.as_bytes().to_vec(),
    }
}

fn http_json(response: &HttpResponse) -> Value {
    serde_json::from_slice(&response.body).expect("HTTP body should be JSON")
}

fn assert_diagnostic_disclosure(value: &Value) {
    let disclosure = value
        .get("disclosure")
        .expect("HTTP status or error should include disclosure");
    assert_eq!(disclosure["guarantee_class"], "detective_observation");
    let values = disclosure["non_guarantees"]
        .as_array()
        .expect("disclosure should include non_guarantees");
    for expected in [
        "NotOsSandbox",
        "NotActorAttributionProof",
        "NotNetworkIsolation",
    ] {
        assert!(
            values.iter().any(|value| value.as_str() == Some(expected)),
            "missing non-guarantee {expected}: {disclosure}"
        );
    }
}

fn http_body_text(response: &HttpResponse) -> Result<String, Box<dyn Error>> {
    Ok(std::str::from_utf8(&response.body)?.to_owned())
}

fn http_header<'a>(response: &'a HttpResponse, name: &str) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn add_allowed_project(fixture: &CoreFixture, project_id: &str) -> Result<(), Box<dyn Error>> {
    let repo_root = fixture.create_product_repo(format!("repo-{project_id}"))?;
    register_project(
        fixture.runtime_home_path(),
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        fixture.runtime_home_path(),
        ConnectionProjectRegistration {
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: project_id.to_owned(),
        },
    )?;
    Ok(())
}

fn create_pending_product_judgment(
    fixture: &CoreFixture,
) -> Result<(String, PipelineResponse), Box<dyn Error>> {
    let setup_adapter = adapter(fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let response = setup_adapter.call_tool(
        "volicord.request_user_judgment",
        product_judgment_args(fixture, &task_id, state_version),
    )?;
    Ok((task_id, response))
}

fn create_consent_token_for_response(
    fixture: &CoreFixture,
    response: &PipelineResponse,
    token: &str,
    ttl_seconds: u64,
) -> Result<(), Box<dyn Error>> {
    let judgment = pending_judgment_from_response(response)
        .ok_or("response should include a pending user judgment")?;
    create_local_web_consent_token(
        fixture.runtime_home_path(),
        LocalWebConsentTokenCreate {
            token: token.to_owned(),
            project_id: judgment.project_id.as_str().to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            judgment_id: judgment.judgment_id.as_str().to_owned(),
            capture_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
            ttl_seconds,
            created_metadata_json: json!({ "test": "local_web_consent" }).to_string(),
        },
    )?;
    Ok(())
}

fn consent_target(project_id: &str, token: &str) -> String {
    format!(
        "{}?project={}&token={}",
        LOCAL_WEB_CONSENT_PATH,
        percent_encode_query(project_id),
        percent_encode_query(token)
    )
}

fn token_from_consent_url(url: &str) -> Result<String, Box<dyn Error>> {
    let query = url
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or("consent URL should include a query string")?;
    let fields = parse_urlencoded(query);
    Ok(single_param(&fields, "token")
        .ok_or("consent URL should include exactly one token")?
        .to_owned())
}

fn create_task(adapter: &McpAdapter) -> Result<(String, u64), Box<dyn Error>> {
    let response = adapter.call_tool(
        "volicord.intake",
        json!({
            "plain_language_request": "Create a task for MCP elicitation tests.",
            "requested_mode": "work",
            "resume_policy": "create_new",
            "initial_scope": {
                "boundary": "MCP elicitation test task.",
                "non_goals": ["Changing unrelated behavior."],
                "acceptance_criteria": ["A pending judgment can be requested."]
            },
            "initial_context_refs": []
        }),
    )?;
    let task_id = response.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version");
    Ok((task_id, state_version))
}

fn write_product_file(
    fixture: &CoreFixture,
    path: &str,
    contents: &str,
) -> Result<(), Box<dyn Error>> {
    let absolute = fixture.product_repo_path().join(path);
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(absolute, contents)?;
    Ok(())
}

fn initialize_request(id: u64, capabilities: Value) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
            "capabilities": capabilities,
            "clientInfo": {
                "name": "volicord-unit-test",
                "version": "0.0.0"
            }
        }),
    )
}

fn initialized_notification() -> Value {
    notification("notifications/initialized", json!({}))
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
}

fn tools_call(id: u64, name: &str, arguments: Value) -> Value {
    request(
        id,
        "tools/call",
        json!({
            "name": name,
            "arguments": arguments
        }),
    )
}

fn product_judgment_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    judgment_args(
        fixture,
        task_id,
        state_version,
        "product_decision",
        json!([
            {
                "option_id": "keep",
                "label": "Keep focused behavior",
                "description": "Record the user-owned product decision to keep the behavior.",
                "consequence": "Only this focused judgment is resolved.",
                "is_default": true
            },
            {
                "option_id": "change",
                "label": "Change focused behavior",
                "description": "Record the user-owned product decision to change the behavior.",
                "consequence": "Only this focused judgment is resolved with the alternate option.",
                "is_default": false
            }
        ]),
        json!(["close_complete"]),
    )
}

fn authority_judgment_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    judgment_args(
        fixture,
        task_id,
        state_version,
        "scope_decision",
        Value::Null,
        json!(["scope_update"]),
    )
}

fn judgment_args(
    fixture: &CoreFixture,
    task_id: &str,
    state_version: u64,
    judgment_kind: &str,
    options: Value,
    required_for: Value,
) -> Value {
    json!({
        "task_id": task_id,
        "change_unit_id": null,
        "judgment_kind": judgment_kind,
        "presentation": "short",
        "question": "Choose the focused MCP elicitation test outcome.",
        "options": options,
        "context": {
            "summary": "A focused test judgment needs a user-owned answer.",
            "related_refs": [],
            "artifact_refs": [],
            "visible_risks": [],
            "constraints": ["The answer covers only this pending judgment."]
        },
        "affected_refs": [
            {
                "record_kind": "task",
                "record_id": task_id,
                "project_id": fixture.project_id(),
                "task_id": task_id,
                "state_version": state_version
            }
        ],
        "required_for": required_for,
        "expires_at": null
    })
}

fn elicitation_accept(selected_option_id: &str, note: Option<&str>) -> Value {
    let mut content = json!({
        "selected_option_id": selected_option_id
    });
    if let Some(note) = note {
        content["note"] = json!(note);
    }
    json!({
        "jsonrpc": "2.0",
        "id": "elicit_user_judgment_1",
        "result": {
            "action": "accept",
            "content": content
        }
    })
}

fn elicitation_action(action: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "elicit_user_judgment_1",
        "result": {
            "action": action
        }
    })
}

fn json_lines(messages: &[Value]) -> Result<Vec<u8>, serde_json::Error> {
    let mut output = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut output, message)?;
        output.push(b'\n');
    }
    Ok(output)
}

fn volicord_response_from_tool(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(response["result"]["isError"], json!(false));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("tools/call response should include text content")?;
    Ok(serde_json::from_str(text)?)
}

fn stored_resolution_basis(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<String, Box<dyn Error>> {
    let record = stored_judgment_record(fixture, task_id, response)?;
    record
        .resolved_verification_basis
        .ok_or_else(|| "stored judgment should have a resolution basis".into())
}

fn stored_judgment_record(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<volicord_store::core_pipeline::UserJudgmentRecord, Box<dyn Error>> {
    let judgment_id = response["user_judgment_ref"]["record_id"]
        .as_str()
        .ok_or("response should include user_judgment_ref.record_id")?;
    let store = CoreProjectStore::open(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    let record = store
        .user_judgment_records_for_task(&volicord_types::TaskId::new(task_id))?
        .into_iter()
        .find(|record| record.judgment_id == judgment_id)
        .ok_or("stored judgment record should exist")?;
    Ok(record)
}

fn tool_names(tools: &[McpToolDefinition]) -> Vec<&'static str> {
    tools.iter().map(|tool| tool.name).collect::<Vec<_>>()
}

fn root_properties(schema: &Value) -> Vec<String> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

fn root_required_fields(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn schema_has_definition(schema: &Value, name: &str) -> bool {
    schema
        .get("definitions")
        .and_then(Value::as_object)
        .is_some_and(|definitions| definitions.contains_key(name))
}

fn stdio_responses(output: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    let text = std::str::from_utf8(output)?;
    let mut responses = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        responses.push(serde_json::from_str(line)?);
    }
    Ok(responses)
}

#[test]
fn workflow_public_tool_names_are_unique() {
    let unique = PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), PUBLIC_METHOD_TOOL_NAMES.len());
}
