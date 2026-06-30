#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{self, Read, Write},
    process::{Child, Command, ExitStatus, Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::{
    agent_connections::{
        agent_connection_record, ensure_agent_connection, AgentConnectionRegistration,
        CONNECTION_MODE_READ_ONLY,
    },
    core_pipeline::StorageEffectCounts,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    ActorSource, OperationCategory, ProjectId, VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const EXPECTED_WORKFLOW_METHOD_TOOLS: [&str; 9] = [
    "volicord.intake",
    "volicord.update_scope",
    "volicord.status",
    "volicord.prepare_write",
    "volicord.stage_artifact",
    "volicord.record_run",
    "volicord.request_user_judgment",
    "volicord.check_close",
    "volicord.close_task",
];
const EXPECTED_READ_ONLY_TOOLS: [&str; 3] = [
    "volicord.status",
    "volicord.check_close",
    "volicord.list_projects",
];
const LIST_PROJECTS_TOOL_NAME: &str = "volicord.list_projects";

#[test]
fn volicord_mcp_subcommand_reports_help_version_and_preflight() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-preflight")?;

    let help = run_without_binding(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("mcp --stdio --connection <connection_id>"));

    let version = run_without_binding(["--version"])?;
    assert_success(&version);
    assert!(stdout(&version).starts_with("volicord "));

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
            "runtime_home:",
            "connection_id:",
            "mode:",
            "enabled:",
            "allowed_projects:",
            "available_projects:",
            "verification_scope:",
            "project[0].project_id:",
            "project[0].available:",
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
    assert_report_line(&report, "allowed_projects: 1");
    assert_report_line(&report, "available_projects: 1");
    assert_report_line(&report, "verification_scope: startup_check_only");
    assert_report_line(
        &report,
        &format!("project[0].project_id: {}", fixture.project_id()),
    );
    assert_report_line(&report, "project[0].available: true");
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
        responses[&1]["result"]["protocolVersion"],
        json!("2025-11-25")
    );

    let tool_names = responses[&3]["result"]["tools"]
        .as_array()
        .expect("tools/list result should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(
        &tool_names[..EXPECTED_WORKFLOW_METHOD_TOOLS.len()],
        EXPECTED_WORKFLOW_METHOD_TOOLS
    );
    assert_eq!(
        tool_names[EXPECTED_WORKFLOW_METHOD_TOOLS.len()],
        LIST_PROJECTS_TOOL_NAME
    );
    assert!(!tool_names.contains(&"volicord.record_user_judgment"));
    assert_eq!(
        tool_names.iter().copied().collect::<BTreeSet<_>>().len(),
        EXPECTED_WORKFLOW_METHOD_TOOLS.len() + 1
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

    let intake = volicord_response(&responses[&5])?;
    assert_eq!(intake["base"]["response_kind"], "result");
    assert_eq!(intake["base"]["state_version"], 1);
    let task_id = intake["task_ref"]["record_id"]
        .as_str()
        .expect("intake response should include a task ref")
        .to_owned();

    assert_eq!(responses[&6]["result"]["isError"], json!(true));
    let connection_rejection = responses[&6]["result"]["content"][0]["text"]
        .as_str()
        .expect("connection rejection should be text");
    assert!(connection_rejection.contains("connection_id"));

    assert!(responses[&7].get("error").is_none());
    assert_eq!(responses[&7]["result"]["isError"], json!(true));
    let tool_error = responses[&7]["result"]["content"][0]["text"]
        .as_str()
        .expect("invalid known-tool arguments should return text content");
    assert!(tool_error.contains("unknown field"));

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
fn volicord_mcp_subcommand_stdio_records_judgment_with_elicitation() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-elicitation")?;
    let (task_id, state_version) = fixture.create_task("elicitation")?;
    let messages = json_lines(&[
        initialize_request_with_capabilities(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            request_user_judgment_arguments(&fixture, &task_id, state_version),
        ),
        json!({
            "jsonrpc": "2.0",
            "id": "elicit_user_judgment_1",
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
    assert_eq!(values[1]["id"], "elicit_user_judgment_1");
    let response = volicord_response(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["resolved_by_actor_source"],
        "local_user"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "keep"
    );
    let record = fixture.stored_judgment(&task_id, &response)?;
    assert_eq!(
        record.resolved_verification_basis.as_deref(),
        Some(VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL)
    );
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
        vec![
            "volicord.intake",
            "volicord.update_scope",
            "volicord.status",
            "volicord.prepare_write",
            "volicord.stage_artifact",
            "volicord.record_run",
            "volicord.request_user_judgment",
            "volicord.check_close",
            "volicord.close_task",
            "volicord.list_projects",
        ]
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
    assert_eq!(read_only_names.as_slice(), EXPECTED_READ_ONLY_TOOLS);
    assert!(!read_only_names.contains(&"volicord.intake"));
    assert!(!read_only_names.contains(&"volicord.close_task"));
    assert!(!read_only_names.contains(&"volicord.prepare_write"));
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
    assert_eq!(responses[&2]["error"]["code"], -32600);
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

    fn stored_judgment(
        &self,
        task_id: &str,
        response: &Value,
    ) -> Result<volicord_store::core_pipeline::UserJudgmentRecord, Box<dyn Error>> {
        let judgment_id = response["user_judgment_ref"]["record_id"]
            .as_str()
            .ok_or("response should include user_judgment_ref.record_id")?;
        let store = volicord_store::core_pipeline::CoreProjectStore::open(
            self.runtime_home_path(),
            &ProjectId::new(self.project_id()),
        )?;
        let record = store
            .user_judgment_records_for_task(&volicord_types::TaskId::new(task_id))?
            .into_iter()
            .find(|record| record.judgment_id == judgment_id)
            .ok_or("stored judgment record should exist")?;
        Ok(record)
    }
}

fn run_without_binding<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    command.arg("mcp");
    command.args(args);
    Ok(command.output()?)
}

fn base_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_volicord"));
    command.env_clear();
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

fn initialize_request(id: u64) -> Value {
    initialize_request_with_capabilities(id, json!({}))
}

fn initialize_request_with_capabilities(id: u64, capabilities: Value) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": capabilities,
            "clientInfo": {
                "name": "volicord-binary-test",
                "version": "0.0.0"
            }
        }),
    )
}

fn initialized_notification() -> Value {
    initialized_notification_with_params(json!({}))
}

fn initialized_notification_with_params(params: Value) -> Value {
    notification("notifications/initialized", params)
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

fn tools_list_messages(
    initialize_id: u64,
    tools_list_id: u64,
) -> Result<String, serde_json::Error> {
    json_lines(&[
        initialize_request(initialize_id),
        initialized_notification(),
        request(tools_list_id, "tools/list", json!({})),
    ])
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
        "plain_language_request": "Exercise the compiled MCP stdio binary.",
        "requested_mode": "work",
        "resume_policy": "create_new",
        "initial_scope": {
            "boundary": "Compiled MCP stdio process behavior.",
            "non_goals": ["Changing Core method semantics."],
            "acceptance_criteria": ["The stdio process records one task."]
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

fn request_user_judgment_arguments(
    fixture: &McpFixture,
    task_id: &str,
    state_version: u64,
) -> Value {
    json!({
        "task_id": task_id,
        "change_unit_id": null,
        "judgment_kind": "product_decision",
        "presentation": "short",
        "question": "Choose the focused compiled MCP elicitation outcome.",
        "options": [
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
        ],
        "context": {
            "summary": "A compiled MCP process test judgment needs a user-owned answer.",
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
        "required_for": ["close_complete"],
        "expires_at": null
    })
}

fn json_lines(messages: &[Value]) -> Result<String, serde_json::Error> {
    let mut output = String::new();
    for message in messages {
        output.push_str(&serde_json::to_string(message)?);
        output.push('\n');
    }
    Ok(output)
}

fn json_rpc_values(output: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    let text = std::str::from_utf8(output)?;
    let mut values = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSON on output line {}: {error}", line_number + 1))?;
        assert_eq!(value["jsonrpc"], "2.0");
        values.push(value);
    }
    Ok(values)
}

fn responses_by_id(output: &[u8]) -> Result<BTreeMap<u64, Value>, Box<dyn Error>> {
    let text = std::str::from_utf8(output)?;
    let mut responses = BTreeMap::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSON on output line {}: {error}", line_number + 1))?;
        assert_eq!(value["jsonrpc"], "2.0");
        let id = value["id"]
            .as_u64()
            .ok_or_else(|| format!("missing numeric id on output line {}", line_number + 1))?;
        assert!(
            responses.insert(id, value).is_none(),
            "duplicate JSON-RPC response id {id}"
        );
    }
    Ok(responses)
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

fn assert_public_tool_schemas_hide_internal_fields(tools: &[Value]) {
    let expected_public = EXPECTED_WORKFLOW_METHOD_TOOLS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
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

fn volicord_response(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(response["result"]["isError"], json!(false));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("tools/call response should contain text content")?;
    Ok(serde_json::from_str(text)?)
}

fn adapter_tool_response(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(response["result"]["isError"], json!(false));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("adapter tools/call response should contain text content")?;
    Ok(serde_json::from_str(text)?)
}

enum ChildStdin {
    KeepOpen,
    WriteAndClose(String),
}

struct CapturedChildOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct RunningChild {
    child: Option<Child>,
    stdout: Option<JoinHandle<io::Result<Vec<u8>>>>,
    stderr: Option<JoinHandle<io::Result<Vec<u8>>>>,
}

impl RunningChild {
    fn spawn(mut command: Command, stdin: ChildStdin) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("stderr was not piped"))?;
        let stdout = thread::spawn(move || read_to_end(stdout));
        let stderr = thread::spawn(move || read_to_end(stderr));

        match stdin {
            ChildStdin::KeepOpen => {}
            ChildStdin::WriteAndClose(input) => {
                let mut child_stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| io::Error::other("stdin was not piped"))?;
                child_stdin.write_all(input.as_bytes())?;
            }
        }

        Ok(Self {
            child: Some(child),
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }

    fn wait(mut self, timeout: Duration) -> io::Result<CapturedChildOutput> {
        let started = Instant::now();
        loop {
            let child = self
                .child
                .as_mut()
                .ok_or_else(|| io::Error::other("child already reaped"))?;
            if let Some(status) = child.try_wait()? {
                self.child.take();
                return Ok(CapturedChildOutput {
                    status,
                    stdout: join_reader(self.stdout.take())?,
                    stderr: join_reader(self.stderr.take())?,
                });
            }
            if started.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = join_reader(self.stdout.take()).unwrap_or_default();
                let stderr = join_reader(self.stderr.take()).unwrap_or_default();
                self.child.take();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "child process timed out after {:?}\nstdout:\n{}\nstderr:\n{}",
                        timeout,
                        String::from_utf8_lossy(&stdout),
                        String::from_utf8_lossy(&stderr)
                    ),
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for RunningChild {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(stdout) = self.stdout.take() {
            let _ = stdout.join();
        }
        if let Some(stderr) = self.stderr.take() {
            let _ = stderr.join();
        }
    }
}

fn read_to_end(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}

fn join_reader(handle: Option<JoinHandle<io::Result<Vec<u8>>>>) -> io::Result<Vec<u8>> {
    let handle = handle.ok_or_else(|| io::Error::other("missing reader"))?;
    handle
        .join()
        .map_err(|_| io::Error::other("reader thread panicked"))?
}

fn run_child(command: Command, stdin: ChildStdin) -> Result<CapturedChildOutput, Box<dyn Error>> {
    Ok(RunningChild::spawn(command, stdin)?.wait(PROCESS_TIMEOUT)?)
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
}

fn assert_success_captured(output: &CapturedChildOutput) {
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        captured_stdout(output),
        captured_stderr(output)
    );
}

fn assert_report_line(report: &str, expected: &str) {
    assert!(
        report.lines().any(|line| line == expected),
        "missing report line `{expected}` in:\n{report}"
    );
}

fn assert_report_line_names(report: &str, expected: &[&str]) {
    let actual = report
        .lines()
        .map(|line| {
            let separator = line
                .find(':')
                .unwrap_or_else(|| panic!("report line missing `:` separator: {line}"));
            &line[..=separator]
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "unexpected preflight report line names");
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn captured_stdout(output: &CapturedChildOutput) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn captured_stderr(output: &CapturedChildOutput) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
