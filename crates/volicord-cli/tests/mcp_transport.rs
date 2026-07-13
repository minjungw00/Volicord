#![forbid(unsafe_code)]

mod support;

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf, process::Command};

use serde_json::{json, Value};
use volicord_core::{CoreService, InvocationContext};
use volicord_mcp::{
    ADAPTER_UTILITY_TOOL_NAMES, PUBLIC_METHOD_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES,
};
use volicord_store::{
    agent_connections::{
        agent_connection_record, ensure_agent_connection, AgentConnectionRegistration,
        CONNECTION_MODE_READ_ONLY,
    },
    core_pipeline::StorageEffectCounts,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    ActorSource, OperationCategory, ProjectId, CLOSE_TASK_TOOL_NAME, INTAKE_TOOL_NAME,
    PREPARE_WRITE_TOOL_NAME, RECONCILE_CHANGES_TOOL_NAME, RECORD_RUN_TOOL_NAME,
    REQUEST_USER_ACTION_TOOL_NAME, RESOLVE_USER_ACTION_TOOL_NAME, UPDATE_SCOPE_TOOL_NAME,
    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

use support::{
    assertions::{
        assert_report_line, assert_report_line_names, assert_success, assert_success_captured,
        captured_stderr, captured_stdout, stderr, stdout,
    },
    binary_fixture::{base_command, run_child, run_without_binding, ChildStdin},
    json::{
        adapter_tool_response, initialize_request, initialize_request_with_capabilities,
        initialized_notification, initialized_notification_with_params, json_lines,
        json_rpc_values, notification, request, responses_by_id, tools_call, tools_list_messages,
        volicord_response,
    },
};

#[test]
fn volicord_mcp_subcommand_reports_help_version_and_preflight() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-preflight")?;

    let help = run_without_binding(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("mcp --stdio --connection <connection_id>"));
    assert!(stdout(&help).contains("mcp --stdio --discover-repository --host codex|claude-code"));

    let version = run_without_binding(["--version"])?;
    assert_success(&version);
    assert_eq!(
        stdout(&version),
        format!(
            "volicord {} (build_id={})\n",
            env!("CARGO_PKG_VERSION"),
            volicord_mcp::build_id()
        )
    );

    let no_args = run_without_binding([])?;
    assert_eq!(no_args.status.code(), Some(2));
    assert!(stderr(&no_args).contains("MCP mode is required"));

    let check_without_connection = run_without_binding(["--check"])?;
    assert_eq!(check_without_connection.status.code(), Some(2));
    assert!(stderr(&check_without_connection).contains("--connection is required"));

    let before = fixture.counts()?;
    let connection_check = run_child(
        fixture.connection_command(["--check", "--connection", fixture.connection_id()]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&connection_check);
    let report = captured_stdout(&connection_check);
    assert_report_line_names(
        &report,
        &[
            "configuration:",
            "transport:",
            "Does not prove:",
            "runtime_home:",
            "connection_id:",
            "mode:",
            "enabled:",
            "registry_read:",
            "project_state_read:",
            "project_state_write:",
            "startup_observation:",
            "effective_tool_mode:",
            "tools_list_schema_validation:",
            "tool_naming_style:",
            "allowed_projects:",
            "available_projects:",
            "verification_scope:",
            "watcher_status:",
            "watcher_baseline_created_at:",
            "watcher_coverage_start_at:",
            "watcher_coverage_basis:",
            "watcher_partial_coverage_warning:",
            "project[0].project_id:",
            "project[0].available:",
            "project[0].state_read:",
            "project[0].state_write:",
            "project[0].unavailable_reason:",
            "project[0].repo_root:",
        ],
    );
    assert_report_line(&report, "configuration: valid");
    assert_report_line(&report, "transport: stdio");
    assert_report_line(
        &report,
        &format!("runtime_home: {}", fixture.runtime_home_path().display()),
    );
    assert_report_line(
        &report,
        &format!("connection_id: {}", fixture.connection_id()),
    );
    assert_report_line(&report, "mode: workflow");
    assert_report_line(&report, "enabled: true");
    assert_report_line(&report, "registry_read: passed");
    assert_report_line(&report, "project_state_read: passed");
    assert_report_line(&report, "project_state_write: passed");
    assert_report_line(&report, "startup_observation: recordable");
    assert_report_line(&report, "effective_tool_mode: workflow");
    assert_report_line(&report, "tools_list_schema_validation: passed");
    assert_report_line(&report, "tool_naming_style: dotted_namespace");
    assert_report_line(&report, "allowed_projects: 1");
    assert_report_line(&report, "available_projects: 1");
    assert_report_line(&report, "verification_scope: startup_check_only");
    assert_report_line(&report, "watcher_status: pending_mcp_start");
    assert_report_line(&report, "watcher_coverage_basis: mcp_start");
    assert_report_line(
        &report,
        &format!("project[0].project_id: {}", fixture.project_id()),
    );
    assert_report_line(&report, "project[0].available: true");
    assert_report_line(&report, "project[0].state_read: passed");
    assert_report_line(&report, "project[0].state_write: passed");
    assert_report_line(&report, "project[0].unavailable_reason: ");
    assert_eq!(fixture.counts()?, before);

    let project_check = run_child(
        fixture.connection_command([
            "--check",
            "--connection",
            fixture.connection_id(),
            "--project",
            fixture.project_id(),
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&project_check);
    let project_report = captured_stdout(&project_check);
    assert_report_line(&project_report, "allowed_projects: 1");
    assert_report_line(
        &project_report,
        &format!("project[0].project_id: {}", fixture.project_id()),
    );

    let missing_connection = run_child(
        fixture.connection_command(["--check", "--connection", "missing_connection"]),
        ChildStdin::KeepOpen,
    )?;
    assert_eq!(missing_connection.status.code(), Some(1));
    assert!(captured_stderr(&missing_connection).contains("not registered"));

    let unknown = run_without_binding(["--not-a-real-option"])?;
    assert_eq!(unknown.status.code(), Some(2));
    assert!(stderr(&unknown).contains("unknown option"));

    Ok(())
}

#[test]
fn repository_discovery_stdio_resolves_the_clone_local_binding() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-repository-discovery")?;
    let repo_root = fixture.repo_root();
    fs::create_dir(repo_root.join(".git"))?;
    let mut command =
        fixture.connection_command(["--stdio", "--discover-repository", "--host", "codex"]);
    command.current_dir(&repo_root);

    let output = run_child(
        command,
        ChildStdin::WriteAndClose(tools_list_messages(1, 2)?),
    )?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let responses = responses_by_id(&output.stdout)?;
    assert!(responses[&1]["result"]["serverInfo"]["name"].is_string());
    assert!(responses[&2]["result"]["tools"].is_array());
    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_uses_line_delimited_json_and_reconnects_state(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-stdio")?;
    let first_messages = json_lines(&[
        initialize_request(1),
        initialized_notification(),
        request(2, "ping", json!({})),
        request(3, "tools/list", json!({})),
        tools_call(30, "volicord.list_projects", json!({})),
        tools_call(4, "volicord.status", status_arguments(None)),
        tools_call(5, "volicord.intake", intake_arguments(None)),
        tools_call(
            6,
            "volicord.status",
            status_arguments_with_connection_id(None, "forged_connection"),
        ),
        tools_call(7, "volicord.status", json!({ "unexpected": true })),
    ])?;

    let first = run_child(
        fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]),
        ChildStdin::WriteAndClose(first_messages),
    )?;
    assert_success_captured(&first);
    assert_eq!(captured_stderr(&first), "");

    let responses = responses_by_id(&first.stdout)?;
    assert_eq!(
        responses.len(),
        8,
        "notifications must not produce responses"
    );

    assert_eq!(
        responses[&1]["result"]["serverInfo"]["name"],
        json!("volicord-mcp")
    );
    assert_eq!(
        responses[&1]["result"]["serverInfo"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        responses[&1]["result"]["serverInfo"]
            .as_object()
            .expect("serverInfo should be an object")
            .len(),
        2,
        "serverInfo should contain only standard Implementation fields"
    );
    assert_eq!(
        responses[&1]["result"]["_meta"]["io.volicord/build"],
        serde_json::to_value(volicord_mcp::build_info())?
    );
    let build_id = responses[&1]["result"]["_meta"]["io.volicord/build"]["build_id"]
        .as_str()
        .expect("build metadata build_id should be a string");
    for component in [
        ";git=",
        ";tree=",
        ";metadata_source=",
        ";target=",
        ";profile=",
        ";profile_class=",
        ";profile_exact=",
        ";opt=",
        ";debug=",
    ] {
        assert!(
            build_id.contains(component),
            "missing {component}: {build_id}"
        );
    }
    assert_eq!(
        responses[&1]["result"]["protocolVersion"],
        json!("2025-11-25")
    );

    let tool_names = responses[&3]["result"]["tools"]
        .as_array()
        .expect("tools/list result should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    let expected_tools = expected_workflow_tools();
    assert_eq!(tool_names, expected_tools);
    assert!(tool_names.contains(&"volicord.request_user_action"));
    assert!(!tool_names.contains(&"volicord.resolve_user_action"));
    assert_eq!(
        tool_names.iter().copied().collect::<BTreeSet<_>>().len(),
        expected_tools.len()
    );
    assert_public_tool_schemas_hide_internal_fields(
        responses[&3]["result"]["tools"]
            .as_array()
            .expect("tools/list result should be an array"),
    );

    assert_eq!(responses[&30]["result"]["isError"], json!(false));
    let project_list = adapter_tool_response(&responses[&30])?;
    assert_eq!(project_list["connection_id"], fixture.connection_id());
    assert_eq!(project_list["mode"], "workflow");
    assert_eq!(
        project_list["projects"][0]["project_selector"],
        fixture.project_id()
    );
    assert_eq!(project_list["projects"][0]["available"], true);

    assert_eq!(responses[&4]["result"]["isError"], json!(false));
    let status = volicord_response(&responses[&4])?;
    assert_eq!(status["base"]["response_kind"], "result");
    assert_eq!(status["base"]["state_version"], 0);
    assert_authority_disclosure(&status);

    let intake = volicord_response(&responses[&5])?;
    assert_eq!(intake["base"]["response_kind"], "result");
    assert_eq!(intake["base"]["state_version"], 1);
    let task_id = intake["task_ref"]["record_id"]
        .as_str()
        .expect("intake response should include a task ref")
        .to_owned();

    for (id, path) in [(6, "/connection_id"), (7, "/unexpected")] {
        assert!(responses[&id].get("error").is_none());
        assert_eq!(responses[&id]["result"]["isError"], json!(true));
        let structured = &responses[&id]["result"]["structuredContent"];
        let text = responses[&id]["result"]["content"][0]["text"]
            .as_str()
            .expect("invalid known-tool arguments should return JSON text content");
        assert_eq!(serde_json::from_str::<Value>(text)?, *structured);
        assert_eq!(structured["code"], "MCP_INVALID_ARGUMENTS");
        assert_eq!(structured["tool_name"], "volicord.status");
        assert_eq!(structured["retryable"], true);
        assert_eq!(structured["reached_core"], false);
        assert_eq!(structured["committed"], false);
        assert!(structured["issues"]
            .as_array()
            .expect("structured error should include issues")
            .iter()
            .any(|issue| issue["path"] == path && issue["code"] == "MCP_ARGUMENT_UNKNOWN"));
    }

    let reconnect_before_handshake = run_child(
        fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]),
        ChildStdin::WriteAndClose(json_lines(&[request(10, "tools/list", json!({}))])?),
    )?;
    assert_success_captured(&reconnect_before_handshake);
    let reconnect_before_handshake_responses = responses_by_id(&reconnect_before_handshake.stdout)?;
    assert_eq!(
        reconnect_before_handshake_responses[&10]["error"]["code"],
        -32600
    );

    let reconnect_messages = json_lines(&[
        initialize_request(11),
        initialized_notification(),
        tools_call(
            12,
            "volicord.status",
            status_arguments(Some(fixture.project_id())),
        ),
    ])?;
    let reconnect = run_child(
        fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]),
        ChildStdin::WriteAndClose(reconnect_messages),
    )?;
    assert_success_captured(&reconnect);
    assert_eq!(captured_stderr(&reconnect), "");

    let reconnect_responses = responses_by_id(&reconnect.stdout)?;
    assert_eq!(
        reconnect_responses[&11]["result"]["serverInfo"]["name"],
        "volicord-mcp"
    );
    assert_eq!(
        reconnect_responses[&11]["result"]["protocolVersion"],
        "2025-11-25"
    );
    let reconnect_status = volicord_response(&reconnect_responses[&12])?;
    assert_eq!(reconnect_status["base"]["response_kind"], "result");
    assert_eq!(reconnect_status["base"]["state_version"], 1);
    assert_eq!(
        reconnect_status["active_task"]["task_ref"]["record_id"],
        task_id
    );

    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_resolves_user_action_with_elicitation(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-elicitation")?;
    let (task_id, state_version) = fixture.create_task("elicitation")?;
    let messages = json_lines(&[
        initialize_request_with_capabilities(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            request_user_action_arguments(&fixture, &task_id, state_version),
        ),
        json!({
            "jsonrpc": "2.0",
            "id": "elicit_user_action_1",
            "result": {
                "action": "accept",
                "content": {
                    "selected_option_id": "keep"
                }
            }
        }),
    ])?;

    let output = run_child(
        fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]),
        ChildStdin::WriteAndClose(messages),
    )?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let values = json_rpc_values(&output.stdout)?;
    assert_eq!(values.len(), 3);
    assert_eq!(values[1]["method"], "elicitation/create");
    assert_eq!(values[1]["id"], "elicit_user_action_1");
    let projection = volicord_response(&values[2])?;
    let response = &projection["method_result"];
    assert_agent_safe_pending_user_action_summary(response);
    assert_model_visible_user_action_private_fields_absent(&values[2]["result"]);
    assert!(response["user_channel_resolution_ref"].is_object());
    let resolution = &response["user_channel_resolution"];
    assert_eq!(resolution["action_kind"], "product_decision");
    assert_eq!(resolution["channel_kind"], "mcp_elicitation");
    assert_eq!(
        resolution["resolution_summary"]["resolution_type"],
        "choice"
    );
    assert_eq!(
        resolution["resolution_summary"]["selected_option_id"],
        "keep"
    );
    assert_eq!(
        resolution["resolution_summary"]["selected_option_label"],
        "Keep focused behavior"
    );
    assert_eq!(
        resolution["resolution_summary"]["resolution_outcome"],
        "accepted"
    );
    assert!(resolution.get("note").is_none());
    assert!(resolution["resolution_summary"].get("summary").is_none());
    let record = fixture.stored_user_action(&task_id, response)?;
    assert_eq!(
        record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_by_actor_source.as_str()),
        Some("local_user")
    );
    assert_eq!(
        record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_verification_basis.as_str()),
        Some(VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL)
    );
    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_without_elicitation_returns_cli_recovery_fallback(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-elicitation-fallback")?;
    let (task_id, state_version) = fixture.create_task("elicitation_fallback")?;
    let messages = json_lines(&[
        initialize_request(1),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            request_user_action_arguments(&fixture, &task_id, state_version),
        ),
    ])?;

    let mut command =
        fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]);
    command.env("VOLICORD_LOCAL_WEB_CONSENT", "disabled");
    let output = run_child(command, ChildStdin::WriteAndClose(messages))?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let responses = responses_by_id(&output.stdout)?;
    assert_eq!(responses.len(), 2);
    let projection = volicord_response(&responses[&2])?;
    let response = &projection["method_result"];
    assert_agent_safe_pending_user_action_summary(response);
    assert_model_visible_user_action_private_fields_absent(&responses[&2]["result"]);
    assert!(response["user_channel_resolution_ref"].is_null());
    assert!(response["user_channel_resolution"].is_null());
    let fallback = responses[&2]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text should be present");
    assert!(fallback.contains("pending UserAction requires the user"));
    assert!(fallback.contains("`volicord inbox`"));
    assert!(!fallback.contains("volicord inbox resolve"));
    assert!(!fallback.contains("request.operation=resume"));
    assert!(!fallback.contains("Volicord: resolve A-1"));

    let record = fixture.stored_user_action(&task_id, response)?;
    assert_eq!(record.status, volicord_types::UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn volicord_mcp_subcommand_tools_list_respects_connection_mode_and_schema_boundary(
) -> Result<(), Box<dyn Error>> {
    let workflow = McpFixture::new("mcp-bin-tools-workflow")?;
    let workflow_output = run_child(
        workflow.connection_command(["--stdio", "--connection", workflow.connection_id()]),
        ChildStdin::WriteAndClose(tools_list_messages(1, 2)?),
    )?;
    assert_success_captured(&workflow_output);
    assert_eq!(captured_stderr(&workflow_output), "");
    let workflow_responses = responses_by_id(&workflow_output.stdout)?;
    let workflow_tools = tools_from_response(&workflow_responses[&2]);
    assert_eq!(
        tool_names_from_tools(workflow_tools),
        expected_workflow_tools()
    );
    assert_public_tool_schemas_hide_internal_fields(workflow_tools);

    let read_only = McpFixture::new("mcp-bin-tools-read-only")?;
    read_only.set_connection_mode(CONNECTION_MODE_READ_ONLY)?;
    let read_only_output = run_child(
        read_only.connection_command(["--stdio", "--connection", read_only.connection_id()]),
        ChildStdin::WriteAndClose(tools_list_messages(10, 11)?),
    )?;
    assert_success_captured(&read_only_output);
    assert_eq!(captured_stderr(&read_only_output), "");
    let read_only_responses = responses_by_id(&read_only_output.stdout)?;
    let read_only_tools = tools_from_response(&read_only_responses[&11]);
    let read_only_names = tool_names_from_tools(read_only_tools);
    assert_eq!(read_only_names, expected_read_only_tools());
    for mutation_tool in [
        INTAKE_TOOL_NAME,
        PREPARE_WRITE_TOOL_NAME,
        REQUEST_USER_ACTION_TOOL_NAME,
        RECONCILE_CHANGES_TOOL_NAME,
        CLOSE_TASK_TOOL_NAME,
    ] {
        assert!(!read_only_names.contains(&mutation_tool));
    }
    assert_public_tool_schemas_hide_internal_fields(read_only_tools);

    Ok(())
}

#[test]
fn volicord_mcp_subcommand_suppresses_malformed_notification_output_and_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-notification-suppression")?;
    let before = fixture.counts()?;
    let messages = json_lines(&[
        initialize_request(1),
        initialized_notification_with_params(json!([])),
        request(2, "tools/list", json!({})),
        notification("notifications/unknown", json!({ "ignored": true })),
        notification("tools/call", json!([])),
        notification(
            "tools/call",
            json!({
                "name": "volicord.intake",
                "arguments": intake_arguments(
                    Some(fixture.project_id()),
                )
            }),
        ),
        initialized_notification(),
        request(3, "tools/list", json!({})),
        tools_call(
            4,
            "volicord.status",
            status_arguments(Some(fixture.project_id())),
        ),
    ])?;

    let output = run_child(
        fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]),
        ChildStdin::WriteAndClose(messages),
    )?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let stdout = captured_stdout(&output);
    let stdout_lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(
        stdout_lines.len(),
        4,
        "notifications must not emit blank or placeholder output lines"
    );
    assert!(stdout_lines.iter().all(|line| !line.trim().is_empty()));
    let responses = responses_by_id(&output.stdout)?;
    assert_eq!(responses.len(), 4);
    assert_eq!(
        responses[&1]["result"]["protocolVersion"],
        json!("2025-11-25")
    );
    assert!(responses[&2]["result"]["tools"].is_array());
    assert!(responses[&3]["result"]["tools"].is_array());
    let status = volicord_response(&responses[&4])?;
    assert_eq!(status["base"]["response_kind"], "result");
    assert_eq!(status["base"]["state_version"], 0);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

struct McpFixture {
    fixture: CoreFixture,
}

impl McpFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            fixture: CoreFixture::new(prefix)?,
        })
    }

    fn runtime_home_path(&self) -> &std::path::Path {
        self.fixture.runtime_home_path()
    }

    fn project_id(&self) -> &str {
        self.fixture.project_id()
    }

    fn repo_root(&self) -> PathBuf {
        self.fixture.product_repo_path()
    }

    fn connection_id(&self) -> &str {
        self.fixture.connection_id()
    }

    fn connection_command<const N: usize>(&self, args: [&str; N]) -> Command {
        let mut command = base_command();
        command.env("VOLICORD_HOME", self.runtime_home_path());
        command.arg("mcp");
        command.args(args);
        command
    }

    fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        Ok(self.fixture.counts()?)
    }

    fn set_connection_mode(&self, mode: &str) -> Result<(), Box<dyn Error>> {
        let existing = agent_connection_record(self.runtime_home_path(), self.connection_id())?
            .expect("fixture connection should exist");
        ensure_agent_connection(
            self.runtime_home_path(),
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

    fn create_task(&self, suffix: &str) -> Result<(String, u64), Box<dyn Error>> {
        let response = CoreService::new(self.runtime_home_path()).intake(
            self.fixture.intake_request(
                &format!("req_mcp_bin_{suffix}_task"),
                &format!("idem_mcp_bin_{suffix}_task"),
                false,
                Some(0),
            ),
            InvocationContext::new(
                ProjectId::new(self.project_id()),
                ActorSource::agent_connection(self.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
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

    fn stored_user_action(
        &self,
        task_id: &str,
        response: &Value,
    ) -> Result<volicord_store::core_pipeline::EffectiveUserActionRecord, Box<dyn Error>> {
        let user_action_request_id = response
            .pointer("/user_action_request_summary/user_action_request_id")
            .or_else(|| {
                response.pointer(
                    "/agent_workflow_result/user_action_request_summary/user_action_request_id",
                )
            })
            .and_then(Value::as_str)
            .ok_or("response should include user_action_request_summary.user_action_request_id")?;
        let store = volicord_store::core_pipeline::CoreProjectStore::open(
            self.runtime_home_path(),
            &ProjectId::new(self.project_id()),
        )?;
        let record = store
            .user_action_records_for_task(
                &volicord_types::TaskId::new(task_id),
                &volicord_types::UtcTimestamp::parse("2026-12-01T00:00:00Z")?,
            )?
            .into_iter()
            .find(|record| record.request.user_action_request_id == user_action_request_id)
            .ok_or("stored user-action record should exist")?;
        Ok(record)
    }
}

fn assert_agent_safe_pending_user_action_summary(response: &Value) -> &str {
    let agent_result = response["agent_workflow_result"]
        .as_object()
        .expect("request_user_action result should include agent_workflow_result");
    let summary = agent_result
        .get("user_action_request_summary")
        .and_then(Value::as_object)
        .expect("agent_workflow_result should include user_action_request_summary");
    let actual_keys = summary.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected_keys = ["next_actor", "status", "user_action_request_id"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(summary.get("status"), Some(&json!("pending")));
    assert_eq!(summary.get("next_actor"), Some(&json!("user")));
    summary
        .get("user_action_request_id")
        .and_then(Value::as_str)
        .filter(|request_id| !request_id.is_empty())
        .expect("safe pending summary should include a non-empty request id")
}

fn assert_model_visible_user_action_private_fields_absent(tool_result: &Value) {
    let model_visible = json!({
        "content": tool_result["content"].clone(),
        "structuredContent": tool_result["structuredContent"].clone()
    });
    let mut forbidden_keys = BTreeSet::new();
    collect_forbidden_user_action_keys(&model_visible, &mut forbidden_keys);
    assert!(
        forbidden_keys.is_empty(),
        "model-visible UserAction projection exposed forbidden keys: {forbidden_keys:?}"
    );
    let rendered = serde_json::to_string(&model_visible)
        .expect("model-visible UserAction projection should serialize");
    let normalized = rendered.to_ascii_lowercase();
    for forbidden in [
        "http://",
        "https://",
        "/consent?",
        "token=",
        "choose the focused user channel outcome",
        "user_action_request_ref",
        "request_ref",
    ] {
        assert!(
            !normalized.contains(forbidden),
            "model-visible UserAction projection exposed forbidden text {forbidden:?}"
        );
    }
}

fn collect_forbidden_user_action_keys<'a>(value: &'a Value, found: &mut BTreeSet<&'a str>) {
    const FORBIDDEN: &[&str] = &[
        "user_action_request",
        "user_action_request_ref",
        "request_ref",
        "inbox_item",
        "question",
        "options",
        "form",
        "preferred_capture_path",
        "answer_path_availability",
        "user_channel_availability",
        "command",
        "url",
        "token",
    ];
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if FORBIDDEN.contains(&key.as_str()) {
                    found.insert(key.as_str());
                }
                collect_forbidden_user_action_keys(child, found);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_forbidden_user_action_keys(child, found);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn status_arguments(project_selector: Option<&str>) -> Value {
    let mut arguments = json!({
        "detail": "workflow"
    });
    if let Some(project_selector) = project_selector {
        arguments["project_selector"] = json!(project_selector);
    }
    arguments
}

fn intake_arguments(project_selector: Option<&str>) -> Value {
    let mut arguments = json!({
        "detail": "full",
        "plain_language_request": "Exercise the compiled MCP stdio binary.",
        "requested_mode": "work",
        "resume_policy": "create_new",
        "acceptance_policy": null,
        "lineage": null,
        "initial_scope": {
            "boundary": "Compiled MCP stdio process behavior.",
            "non_goals": ["Changing Core method semantics."],
            "acceptance_criteria": [{
                "statement": "The stdio process records one task.",
                "evidence_requirement": "required"
            }]
        },
        "initial_context_refs": []
    });
    if let Some(project_selector) = project_selector {
        arguments["project_selector"] = json!(project_selector);
    }
    arguments
}

fn status_arguments_with_connection_id(
    project_selector: Option<&str>,
    connection_id: &str,
) -> Value {
    let mut arguments = status_arguments(project_selector);
    arguments["connection_id"] = json!(connection_id);
    arguments
}

fn request_user_action_arguments(fixture: &McpFixture, task_id: &str, state_version: u64) -> Value {
    json!({
        "detail": "full",
        "project_selector": fixture.project_id(),
        "request": {
            "operation": "create",
            "task_id": task_id,
            "change_unit_id": null,
            "action": {
                "action_type": "choice",
                "judgment_kind": "product_decision",
                "presentation": "short",
                "question": "Choose the focused User Channel outcome.",
                "options": [
                    {
                        "option_id": "keep",
                        "label": "Keep focused behavior",
                        "description": "Record the user-owned product decision to keep the behavior.",
                        "consequence": "Only this focused user action is resolved.",
                        "is_default": true
                    },
                    {
                        "option_id": "change",
                        "label": "Change focused behavior",
                        "description": "Record the user-owned product decision to change the behavior.",
                        "consequence": "Only this focused user action is resolved with the alternate option.",
                        "is_default": false
                    }
                ],
                "context": {
                    "summary": "A compiled MCP process test user action needs a user-owned resolution.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": ["The resolution covers only this pending user action."]
                },
                "affected_refs": [
                    {
                        "record_kind": "task",
                        "record_id": task_id,
                        "project_id": fixture.project_id(),
                        "task_id": task_id,
                        "produced_at_state_version": state_version
                    }
                ],
                "sensitive_action_scope": null
            },
            "required_for": ["close_complete"],
            "expires_at": null
        }
    })
}

fn tools_from_response(response: &Value) -> &[Value] {
    response["result"]["tools"]
        .as_array()
        .expect("tools/list result should be an array")
}

fn tool_names_from_tools(tools: &[Value]) -> Vec<&str> {
    tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect()
}

fn expected_workflow_tools() -> Vec<&'static str> {
    PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
        .collect()
}

fn expected_read_only_tools() -> Vec<&'static str> {
    READ_ONLY_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
        .collect()
}

fn public_method_tool_set() -> BTreeSet<&'static str> {
    PUBLIC_METHOD_TOOL_NAMES.iter().copied().collect()
}

fn assert_public_tool_schemas_hide_internal_fields(tools: &[Value]) {
    let expected_public = public_method_tool_set();
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        assert_eq!(
            tool["outputSchema"]["type"], "object",
            "{name} output schema should have an object root"
        );
        let read_only = READ_ONLY_METHOD_TOOL_NAMES.contains(&name)
            || ADAPTER_UTILITY_TOOL_NAMES.contains(&name);
        let destructive = matches!(
            name,
            INTAKE_TOOL_NAME
                | UPDATE_SCOPE_TOOL_NAME
                | RECORD_RUN_TOOL_NAME
                | REQUEST_USER_ACTION_TOOL_NAME
                | RESOLVE_USER_ACTION_TOOL_NAME
                | RECONCILE_CHANGES_TOOL_NAME
                | CLOSE_TASK_TOOL_NAME
        );
        assert_eq!(tool["annotations"]["readOnlyHint"], read_only);
        assert_eq!(tool["annotations"]["destructiveHint"], destructive);
        assert_eq!(tool["annotations"]["idempotentHint"], read_only);
        assert_eq!(tool["annotations"]["openWorldHint"], false);

        if !expected_public.contains(name) {
            continue;
        }
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object", "{name} schema should be object");
        let properties = schema["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} should expose root properties"));
        assert!(
            properties.contains_key("project_selector"),
            "{name} should expose project_selector"
        );
        for forbidden in [
            "envelope",
            "project_id",
            "connection_id",
            "request_id",
            "idempotency_key",
            "expected_state_version",
            "dry_run",
            "locale",
            "actor_source",
            "operation_category",
            "mode",
            "verification_basis",
            "invocation_binding_basis",
        ] {
            assert!(
                !properties.contains_key(forbidden),
                "{name} should not expose internal argument {forbidden}"
            );
        }
        assert!(
            !schema_definitions_contain(schema, "ToolEnvelope"),
            "{name} should not include ToolEnvelope in public schema definitions"
        );
    }
}

fn schema_definitions_contain(schema: &Value, name: &str) -> bool {
    ["definitions", "$defs"].iter().any(|definitions_key| {
        schema
            .get(*definitions_key)
            .and_then(Value::as_object)
            .is_some_and(|definitions| definitions.contains_key(name))
    })
}

fn assert_authority_disclosure(value: &Value) {
    let disclosure = &value["base"]["disclosure"];
    assert_eq!(disclosure["guarantee_class"], "authority_record");
    let values = disclosure["non_guarantees"]
        .as_array()
        .expect("authority disclosure should include non_guarantees");
    for expected in [
        "NotCorrectnessProof",
        "NotTestSufficiencyProof",
        "NotHumanReviewReplacement",
    ] {
        assert!(
            values.iter().any(|value| value.as_str() == Some(expected)),
            "missing non-guarantee {expected}: {disclosure}"
        );
    }
}
