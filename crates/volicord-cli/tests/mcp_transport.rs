#![forbid(unsafe_code)]

mod support;

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf, process::Command};

use serde_json::{json, Value};
use volicord_core::{validate_host_verification_receipt, CoreService, InvocationContext};
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
use volicord_test_support::{core_fixtures::CoreFixture, test_host_receipt_fixture};
use volicord_types::{
    ActorSource, OperationCategory, ProjectId, CLOSE_TASK_TOOL_NAME, INTAKE_TOOL_NAME,
    PREPARE_WRITE_TOOL_NAME, RECONCILE_CHANGES_TOOL_NAME, RECORD_RUN_TOOL_NAME,
    REQUEST_USER_ACTION_TOOL_NAME, RESOLVE_USER_ACTION_TOOL_NAME, UPDATE_SCOPE_TOOL_NAME,
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
};

use support::{
    assertions::{
        assert_report_line, assert_report_line_names, assert_success, assert_success_captured,
        captured_stderr, captured_stdout, stderr, stdout,
    },
    binary_fixture::{base_command, run_child, run_without_binding, ChildStdin},
    json::{
        adapter_tool_response, initialize_request, initialized_notification,
        initialized_notification_with_params, json_lines, notification, request, responses_by_id,
        tools_call, tools_list_messages,
    },
};

const MAX_RUNTIME_TOOLS_LIST_BYTES: usize = 35_000;

#[test]
fn volicord_mcp_subcommand_reports_help_version_and_preflight() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-preflight")?;

    let help = run_without_binding(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("Usage: volicord mcp"));
    assert!(stdout(&help).contains("--discover-repository"));
    assert!(stdout(&help).contains("--connection <CONNECTION>"));

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
    assert!(stderr(&no_args).contains("required arguments were not provided"));

    let check_without_connection = run_without_binding(["--check"])?;
    assert_eq!(check_without_connection.status.code(), Some(2));
    assert!(stderr(&check_without_connection).contains("--connection"));

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
            "effective_tool_mode:",
            "tools_list_schema_validation:",
            "tool_naming_style:",
            "allowed_projects:",
            "available_projects:",
            "verification_scope:",
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
    assert_report_line(&report, "effective_tool_mode: workflow");
    assert_report_line(&report, "tools_list_schema_validation: passed");
    assert_report_line(&report, "tool_naming_style: dotted_namespace");
    assert_report_line(&report, "allowed_projects: 1");
    assert_report_line(&report, "available_projects: 1");
    assert_report_line(&report, "verification_scope: startup_check_only");
    assert_report_line(
        &report,
        &format!("project[0].project_id: {}", fixture.project_id()),
    );
    assert_report_line(&report, "project[0].available: true");
    assert_report_line(&report, "project[0].state_read: passed");
    assert_report_line(&report, "project[0].state_write: passed");
    assert_report_line(&report, "project[0].unavailable_reason: not_applicable");
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
    assert!(stderr(&unknown).contains("unexpected argument"));

    Ok(())
}

#[test]
fn repository_discovery_stdio_rejects_unverified_managed_host_without_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-repository-discovery")?;
    let repo_root = fixture.repo_root();
    fs::create_dir(repo_root.join(".git"))?;
    let before = fixture.counts()?;
    let mut command =
        fixture.connection_command(["--stdio", "--discover-repository", "--host", "codex"]);
    command.current_dir(&repo_root);

    let output = run_child(
        command,
        ChildStdin::WriteAndClose(tools_list_messages(1, 2)?),
    )?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(captured_stdout(&output), "");
    assert!(captured_stderr(&output).contains("managed_host_configuration_stale"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_keeps_protocol_and_rejects_core_without_host_receipt(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-stdio")?;
    let before = fixture.counts()?;
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

    assert_host_receipt_missing(&responses[&4]);
    assert_host_receipt_missing(&responses[&5]);

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
    assert_eq!(fixture.counts()?, before);

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
    assert_host_receipt_missing(&reconnect_responses[&12]);
    assert_eq!(fixture.counts()?, before);

    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_rejects_user_action_without_host_receipt_and_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-cli-inbox-recovery")?;
    let (task_id, state_version) = fixture.create_task("cli_inbox_recovery")?;
    let before = fixture.counts()?;
    let messages = json_lines(&[
        initialize_request(1),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            request_user_action_arguments(&fixture, &task_id, state_version),
        ),
    ])?;

    let command = fixture.connection_command(["--stdio", "--connection", fixture.connection_id()]);
    let output = run_child(command, ChildStdin::WriteAndClose(messages))?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let responses = responses_by_id(&output.stdout)?;
    assert_eq!(responses.len(), 2);
    assert_host_receipt_missing(&responses[&2]);
    assert_eq!(fixture.counts()?, before);
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
    assert_runtime_tools_list_result_is_compact(&workflow_responses[&2]["result"]);
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
    assert_runtime_tools_list_result_is_compact(&read_only_responses[&11]["result"]);
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
fn volicord_mcp_subcommand_suppresses_notifications_and_rejects_core_without_host_receipt(
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
    assert_host_receipt_missing(&responses[&4]);
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
        let host = test_host_receipt_fixture(self.project_id(), self.connection_id());
        let receipt =
            validate_host_verification_receipt(host.receipt, &host.current, &host.validation_time)
                .expect("the typed MCP host-receipt fixture must validate");
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
                VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
            )
            .with_validated_host_receipt(receipt),
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
}

fn assert_host_receipt_missing(response: &Value) {
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["data"]
        .as_str()
        .is_some_and(|data| data.contains("host_receipt_missing")));
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
        assert!(
            !json_member_exists(schema, "examples"),
            "{name} runtime input schema must not contain examples"
        );
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

fn assert_runtime_tools_list_result_is_compact(result: &Value) {
    let serialized = serde_json::to_vec(result).expect("tools/list result should serialize");
    assert!(
        serialized.len() <= MAX_RUNTIME_TOOLS_LIST_BYTES,
        "runtime tools/list result is {} bytes (limit {})",
        serialized.len(),
        MAX_RUNTIME_TOOLS_LIST_BYTES
    );
}

fn json_member_exists(value: &Value, member: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(member)
                || object
                    .values()
                    .any(|child| json_member_exists(child, member))
        }
        Value::Array(items) => items.iter().any(|child| json_member_exists(child, member)),
        _ => false,
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
