#![forbid(unsafe_code)]

mod support;

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{json, Value};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::{
    agent_connections::{
        agent_connection_record_read_only, ensure_agent_connection, AgentConnectionRegistration,
        CONNECTION_MODE_READ_ONLY,
    },
    bootstrap::project_record_read_only,
    core_pipeline::StorageEffectCounts,
    sqlite::registry_db_path,
};
use volicord_test_support::{
    core_fixtures::CoreFixture, transition_test_connection_mode, TestRuntimeHomeSetup,
};
use volicord_types::ids::{AgentConnectionId, ProjectId};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::OperationCategory;

use support::{
    assertions::{
        assert_success, assert_success_captured, captured_stderr, captured_stdout, stderr, stdout,
    },
    binary_fixture::{base_command, run_child, run_without_binding, ChildStdin},
    json::{
        adapter_tool_response, initialize_request, initialized_notification,
        initialized_notification_with_params, json_lines, notification, request, responses_by_id,
        tools_call, tools_list_messages,
    },
};

const MAX_RUNTIME_TOOLS_LIST_BYTES: usize = 38_000;

#[cfg(unix)]
#[test]
fn mcp_preflight_succeeds_with_read_only_registry_and_project_databases(
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut fixture = McpFixture::new("mcp-bin-readonly-preflight")?;
    let project = project_record_read_only(fixture.runtime_home_path(), fixture.project_id())?
        .expect("fixture project");
    let registry = registry_db_path(fixture.runtime_home_path());
    let before = fixture.registry_observation_counts()?;
    let before_rows = fixture.store_row_counts()?;
    fs::set_permissions(&registry, fs::Permissions::from_mode(0o444))?;
    fs::set_permissions(&project.state_db_path, fs::Permissions::from_mode(0o444))?;
    let registry_modified = fs::metadata(&registry)?.modified()?;
    let project_modified = fs::metadata(&project.state_db_path)?.modified()?;
    fixture.fixture.release_mutation_admission();
    let setup = TestRuntimeHomeSetup::acquire(fixture.runtime_home_path())?;

    let output = run_child(
        fixture.connection_command([
            "preflight",
            "--connection",
            fixture.connection_id(),
            "--json",
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&output);
    let report: Value = serde_json::from_str(&captured_stdout(&output))?;
    assert_eq!(report["status"], "passed");
    assert_eq!(report["writeability"]["status"], "not_checked");
    assert_eq!(fixture.registry_observation_counts()?, before);
    assert_eq!(fixture.store_row_counts()?, before_rows);
    assert_eq!(fs::metadata(&registry)?.modified()?, registry_modified);
    assert_eq!(
        fs::metadata(&project.state_db_path)?.modified()?,
        project_modified
    );
    drop(setup);
    Ok(())
}

#[test]
fn mcp_preflight_rejects_noncanonical_managed_entry_without_observations(
) -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-drifted-preflight")?;
    let connection =
        agent_connection_record_read_only(fixture.runtime_home_path(), fixture.connection_id())?
            .expect("fixture connection");
    let before = fixture.registry_observation_counts()?;
    fs::write(
        connection.config_target,
        "[mcp_servers.\"volicord-test\"]\ncommand = \"changed\"\nargs = []\n",
    )?;
    let output = run_child(
        fixture.connection_command([
            "preflight",
            "--connection",
            fixture.connection_id(),
            "--json",
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(captured_stderr(&output).contains("canonical managed MCP entry validation failed"));
    assert_eq!(fixture.registry_observation_counts()?, before);
    Ok(())
}

#[test]
fn volicord_mcp_subcommands_report_effects_and_read_only_preflight() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-preflight")?;

    let help = run_without_binding(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("Usage: volicord mcp"));
    assert!(stdout(&help).contains("preflight"));
    assert!(stdout(&help).contains("serve"));
    let preflight_help = run_without_binding(["preflight", "--help"])?;
    assert_success(&preflight_help);
    assert!(stdout(&preflight_help).contains("Side effects: none"));
    assert!(stdout(&preflight_help).contains("Writeability is not checked"));
    let serve_help = run_without_binding(["serve", "--help"])?;
    assert_success(&serve_help);
    assert!(stdout(&serve_help).contains("manual_cli"));
    assert!(stdout(&serve_help).contains("never create a managed_host session"));
    let mut verify_help = base_command();
    verify_help.args(["connection", "verify", "--help"]);
    let verify_help = verify_help.output()?;
    assert_success(&verify_help);
    assert!(stdout(&verify_help).contains("rollback-only Store writeability probes"));
    assert!(stdout(&verify_help).contains("disposable protocol conformance"));

    let no_args = run_without_binding([])?;
    assert_eq!(no_args.status.code(), Some(2));
    assert!(stderr(&no_args).contains("Usage: volicord mcp"));

    let check_without_connection = run_without_binding(["preflight"])?;
    assert_eq!(check_without_connection.status.code(), Some(2));
    assert!(stderr(&check_without_connection).contains("required"));

    let before = fixture.counts()?;
    let before_rows = fixture.store_row_counts()?;
    let before_observations = fixture.registry_observation_counts()?;
    let connection_check = run_child(
        fixture.connection_command([
            "preflight",
            "--connection",
            fixture.connection_id(),
            "--json",
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&connection_check);
    let report: Value = serde_json::from_str(&captured_stdout(&connection_check))?;
    assert_eq!(report["operation"], "mcp_preflight");
    assert_eq!(report["status"], "passed");
    assert_eq!(report["side_effects"], json!([]));
    assert_eq!(report["evidence_class"], "read_only_preflight");
    assert_eq!(report["canonical_managed_entry"], "passed");
    assert_eq!(
        report["runtime_home"],
        fixture.runtime_home_path().display().to_string()
    );
    assert_eq!(report["connection_id"], fixture.connection_id());
    assert_eq!(report["registry_read"], "passed");
    assert_eq!(report["project_state_read"], "passed");
    assert_eq!(report["writeability"]["status"], "not_checked");
    assert_eq!(
        report["writeability"]["requirement"],
        "requires_active_verification"
    );
    assert_eq!(
        report["effective_tool_mode"],
        "requires_active_verification"
    );
    assert_eq!(report["tools_list_schema_validation"], "passed");
    assert_eq!(report["projects"][0]["state_write"], "not_checked");
    let concise = run_child(
        fixture.connection_command(["preflight", "--connection", fixture.connection_id()]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&concise);
    assert!(captured_stdout(&concise).contains("Operation: MCP preflight"));
    assert!(captured_stdout(&concise).contains("Side effects: none"));
    assert!(captured_stdout(&concise).contains("Evidence class: read_only_preflight"));
    let verbose = run_child(
        fixture.connection_command([
            "preflight",
            "--connection",
            fixture.connection_id(),
            "--verbose",
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&verbose);
    assert!(captured_stdout(&verbose).contains("Canonical managed entry: passed"));
    assert!(captured_stdout(&verbose).contains("Protocol profiles:"));
    assert!(captured_stdout(&verbose).contains("Host contracts:"));
    assert_eq!(fixture.counts()?, before);
    assert_eq!(fixture.store_row_counts()?, before_rows);
    assert_eq!(fixture.registry_observation_counts()?, before_observations);

    let project_check = run_child(
        fixture.connection_command([
            "preflight",
            "--connection",
            fixture.connection_id(),
            "--project",
            fixture.project_id(),
            "--json",
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&project_check);
    let project_report: Value = serde_json::from_str(&captured_stdout(&project_check))?;
    assert_eq!(project_report["allowed_projects"], 1);
    assert_eq!(
        project_report["projects"][0]["project_id"],
        fixture.project_id()
    );

    let missing_connection = run_child(
        fixture.connection_command(["preflight", "--connection", "missing_connection"]),
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
fn repository_discovery_stdio_starts_from_managed_configuration() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-repository-discovery")?;
    let repo_root = fixture.repo_root();
    fs::create_dir(repo_root.join(".git"))?;
    let before = fixture.counts()?;
    let mut command =
        fixture.connection_command(["serve", "--discover-repository", "--host", "codex"]);
    command.current_dir(&repo_root);

    let output = run_child(
        command,
        ChildStdin::WriteAndClose(tools_list_messages(1, 2)?),
    )?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let responses = responses_by_id(&output.stdout)?;
    assert_eq!(responses[&1]["result"]["protocolVersion"], "2025-11-25");
    assert!(responses[&2]["result"]["tools"].is_array());
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_keeps_protocol_and_rejects_core_without_managed_session(
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
        fixture.connection_command(["serve", "--connection", fixture.connection_id()]),
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

    assert_agent_session_missing(&responses[&4]);
    assert_agent_session_missing(&responses[&5]);

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
        fixture.connection_command(["serve", "--connection", fixture.connection_id()]),
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
        fixture.connection_command(["serve", "--connection", fixture.connection_id()]),
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
    assert_agent_session_missing(&reconnect_responses[&12]);
    assert_eq!(fixture.counts()?, before);

    Ok(())
}

#[test]
fn volicord_mcp_subcommand_stdio_rejects_user_action_without_managed_session_and_effects(
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

    let command = fixture.connection_command(["serve", "--connection", fixture.connection_id()]);
    let output = run_child(command, ChildStdin::WriteAndClose(messages))?;

    assert_success_captured(&output);
    assert_eq!(captured_stderr(&output), "");
    let responses = responses_by_id(&output.stdout)?;
    assert_eq!(responses.len(), 2);
    assert_agent_session_missing(&responses[&2]);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn volicord_mcp_subcommand_tools_list_respects_connection_mode_and_schema_boundary(
) -> Result<(), Box<dyn Error>> {
    let workflow = McpFixture::new("mcp-bin-tools-workflow")?;
    let workflow_output = run_child(
        workflow.connection_command(["serve", "--connection", workflow.connection_id()]),
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
        read_only.connection_command(["serve", "--connection", read_only.connection_id()]),
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
        AgentToolId::INTAKE.wire_name(),
        AgentToolId::PREPARE_WRITE.wire_name(),
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        AgentToolId::RECONCILE_CHANGES.wire_name(),
        AgentToolId::CLOSE_TASK.wire_name(),
    ] {
        assert!(!read_only_names.contains(&mutation_tool));
    }
    assert_public_tool_schemas_hide_internal_fields(read_only_tools);

    Ok(())
}

#[test]
fn volicord_mcp_subcommand_suppresses_notifications_and_rejects_core_without_managed_session(
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
        fixture.connection_command(["serve", "--connection", fixture.connection_id()]),
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
    assert_agent_session_missing(&responses[&4]);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

struct McpFixture {
    fixture: CoreFixture,
}

impl McpFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        let connection = agent_connection_record_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?
        .expect("fixture connection");
        let launch = volicord_mcp::ManagedMcpLaunchSpec::shared_repository(
            volicord_types::values::HostKind::Codex,
        )?;
        let fingerprint = launch.managed_fingerprint(&connection.server_name);
        ensure_agent_connection(
            &fixture.mutation_context()?,
            AgentConnectionRegistration {
                connection_internal_id: connection.connection_internal_id.clone(),
                host_kind: connection.host_kind.clone(),
                intent: connection.intent.clone(),
                host_scope: connection.host_scope.clone(),
                server_name: connection.server_name.clone(),
                config_target: connection.config_target.clone(),
                mode: connection.mode.clone(),
                enabled: connection.enabled,
                managed_fingerprint: fingerprint,
                metadata_json: connection.metadata_json.clone(),
            },
        )?;
        if let Some(parent) = std::path::Path::new(&connection.config_target).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &connection.config_target,
            format!(
                "[mcp_servers.\"{}\"]\ncommand = \"volicord\"\nargs = [\"_host-launch\", \"codex\", \"--discover-repository\"]\nenv_vars = [\"VOLICORD_HOME\"]\n",
                connection.server_name
            ),
        )?;
        Ok(Self { fixture })
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

    fn registry_observation_counts(&self) -> Result<(u64, u64), Box<dyn Error>> {
        let connection = rusqlite::Connection::open_with_flags(
            registry_db_path(self.runtime_home_path()),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let sessions =
            connection.query_row("SELECT COUNT(*) FROM mcp_runtime_sessions", [], |row| {
                row.get(0)
            })?;
        let findings =
            connection.query_row("SELECT COUNT(*) FROM diagnostic_findings", [], |row| {
                row.get(0)
            })?;
        Ok((sessions, findings))
    }

    fn store_row_counts(&self) -> Result<BTreeMap<String, u64>, Box<dyn Error>> {
        let project = project_record_read_only(self.runtime_home_path(), self.project_id())?
            .expect("fixture project");
        let mut counts =
            database_row_counts("registry", &registry_db_path(self.runtime_home_path()))?;
        counts.extend(database_row_counts("project", &project.state_db_path)?);
        Ok(counts)
    }

    fn set_connection_mode(&self, mode: &str) -> Result<(), Box<dyn Error>> {
        transition_test_connection_mode(
            self.runtime_home_path(),
            &self.repo_root(),
            self.project_id(),
            self.connection_id(),
            mode,
        )?;
        Ok(())
    }

    fn create_task(&self, suffix: &str) -> Result<(String, u64), Box<dyn Error>> {
        let context = self.fixture.mutation_context()?;
        let core = CoreService::for_mutation(&context);
        let session = volicord_test_support::seed_test_agent_session(
            self.runtime_home_path(),
            self.project_id(),
            self.connection_id(),
            None,
        )?;
        let validated = core.validate_agent_session(
            AgentConnectionId::new(self.connection_id()),
            ProjectId::new(self.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            OperationCategory::AgentWorkflow,
        )?;
        let response = core.intake(
            &context,
            self.fixture.intake_request(
                &format!("req_mcp_bin_{suffix}_task"),
                &format!("idem_mcp_bin_{suffix}_task"),
                false,
                Some(0),
            ),
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated),
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

fn database_row_counts(prefix: &str, path: &Path) -> Result<BTreeMap<String, u64>, Box<dyn Error>> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_master
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
         ORDER BY name",
    )?;
    let tables = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut counts = BTreeMap::new();
    for table in tables {
        let quoted = table.replace('"', "\"\"");
        let count =
            connection.query_row(&format!("SELECT COUNT(*) FROM \"{quoted}\""), [], |row| {
                row.get(0)
            })?;
        counts.insert(format!("{prefix}.{table}"), count);
    }
    Ok(counts)
}

fn assert_agent_session_missing(response: &Value) {
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["data"]
        .as_str()
        .is_some_and(|data| data.contains("agent_session_missing")));
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
    AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect()
}

fn expected_read_only_tools() -> Vec<&'static str> {
    AgentToolId::ALL
        .iter()
        .copied()
        .filter(|tool| tool.available_in(volicord_types::values::AgentConnectionMode::ReadOnly))
        .map(AgentToolId::wire_name)
        .collect()
}

fn public_method_tool_set() -> BTreeSet<&'static str> {
    AgentToolId::ALL
        .iter()
        .copied()
        .filter_map(|tool| tool.method().map(|_| tool.wire_name()))
        .collect()
}

fn assert_public_tool_schemas_hide_internal_fields(tools: &[Value]) {
    let expected_public = public_method_tool_set();
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        assert_eq!(
            tool["outputSchema"]["type"], "object",
            "{name} output schema should have an object root"
        );
        let identity = AgentToolId::from_wire_name(name).expect("advertised canonical tool");
        let read_only = matches!(
            identity.category(),
            volicord_types::tool_names::AgentToolCategory::ReadOnly
        );
        let destructive = matches!(
            identity.category(),
            volicord_types::tool_names::AgentToolCategory::DestructiveMutation
        );
        assert_eq!(tool["annotations"]["readOnlyHint"], read_only);
        assert_eq!(tool["annotations"]["destructiveHint"], destructive);
        assert_eq!(
            tool["annotations"]["idempotentHint"],
            identity.is_idempotent()
        );
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
