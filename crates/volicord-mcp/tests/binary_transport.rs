#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use volicord_mcp::PUBLIC_METHOD_TOOL_NAMES;
use volicord_store::{
    agent_integrations::{
        add_integration_project, register_agent_integration, AgentIntegrationRegistration,
        IntegrationProjectRegistration,
    },
    bootstrap::{
        initialize_runtime_home, register_project, register_surface, ProjectRegistration,
        SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts},
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    ProjectId, SurfaceInteractionRole, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
};

const PROJECT_ID: &str = "project_binary_mcp";
const INTEGRATION_ID: &str = "agent_binary_mcp";
const AGENT_SURFACE_ID: &str = "surface_binary_agent";
const AGENT_INSTANCE_ID: &str = "surface_instance_binary_agent";
const USER_SURFACE_ID: &str = "surface_binary_user";
const USER_INSTANCE_ID: &str = "surface_instance_binary_user";
const BASELINE_ACCESS_CLASSES: [&str; 5] = [
    "read_status",
    "core_mutation",
    "write_authorization",
    "artifact_registration",
    "run_recording",
];
const USER_ACCESS_CLASSES: [&str; 2] = ["read_status", "core_mutation"];
const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn volicord_mcp_binary_reports_help_version_and_preflight() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-preflight")?;

    let help = run_without_binding(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("--integration <integration_id>"));
    assert!(!stdout(&help).contains("HARNESS_PROJECT_ID"));

    let version = run_without_binding(["--version"])?;
    assert_success(&version);
    assert!(stdout(&version).starts_with("harness-mcp "));

    let no_args = run_without_binding([])?;
    assert_eq!(no_args.status.code(), Some(2));
    assert!(stderr(&no_args).contains("--integration is required"));

    let check_without_integration = run_without_binding(["--check"])?;
    assert_eq!(check_without_integration.status.code(), Some(2));
    assert!(stderr(&check_without_integration).contains("--integration is required"));

    let fixed_project_env_no_args =
        run_child(fixture.fixed_project_env_command([]), ChildStdin::KeepOpen)?;
    assert_eq!(fixed_project_env_no_args.status.code(), Some(2));
    assert!(captured_stderr(&fixed_project_env_no_args).contains("--integration is required"));

    let fixed_project_env_check = run_child(
        fixture.fixed_project_env_command(["--check"]),
        ChildStdin::KeepOpen,
    )?;
    assert_eq!(fixed_project_env_check.status.code(), Some(2));
    assert!(captured_stderr(&fixed_project_env_check).contains("--integration is required"));

    let before = fixture.counts()?;
    let agent_check = run_child(
        fixture.integration_command(["--check", "--integration", INTEGRATION_ID]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&agent_check);
    let report = captured_stdout(&agent_check);
    assert_report_line(&report, "configuration: valid");
    assert_report_line(&report, "transport: stdio");
    assert_report_line(
        &report,
        &format!("runtime_home: {}", fixture.runtime_home_path.display()),
    );
    assert_report_line(&report, &format!("integration_id: {INTEGRATION_ID}"));
    assert_report_line(&report, &format!("surface_id: {AGENT_SURFACE_ID}"));
    assert_report_line(
        &report,
        &format!("surface_instance_id: {AGENT_INSTANCE_ID}"),
    );
    assert_report_line(&report, "interaction_role: agent");
    assert_report_line(&report, "allowed_projects: 1");
    assert_report_line(&report, "available_projects: 1");
    assert_report_line(&report, "default_project_id: ");
    assert_report_line(&report, "project[0].project_id: project_binary_mcp");
    assert_report_line(&report, "project[0].available: true");
    assert_report_line(&report, "project[0].baseline_workflow_access: full");
    assert_eq!(fixture.counts()?, before);

    let project_check = run_child(
        fixture.integration_command([
            "--check",
            "--integration",
            INTEGRATION_ID,
            "--project",
            PROJECT_ID,
        ]),
        ChildStdin::KeepOpen,
    )?;
    assert_success_captured(&project_check);
    let project_report = captured_stdout(&project_check);
    assert_report_line(&project_report, "allowed_projects: 1");
    assert_report_line(&project_report, "project[0].project_id: project_binary_mcp");

    let missing_integration = run_child(
        fixture.integration_command(["--check", "--integration", "missing_agent"]),
        ChildStdin::KeepOpen,
    )?;
    assert_eq!(missing_integration.status.code(), Some(1));
    assert!(captured_stderr(&missing_integration).contains("not registered"));

    let unknown = run_without_binding(["--not-a-real-option"])?;
    assert_eq!(unknown.status.code(), Some(2));
    assert!(stderr(&unknown).contains("unknown option"));

    Ok(())
}

#[test]
fn volicord_mcp_stdio_uses_line_delimited_json_and_reconnects_state() -> Result<(), Box<dyn Error>>
{
    let fixture = McpFixture::new("mcp-bin-stdio")?;
    let first_messages = json_lines(&[
        initialize_request(1),
        initialized_notification(),
        request(2, "ping", json!({})),
        request(3, "tools/list", json!({})),
        tools_call(30, "harness.list_projects", json!({})),
        tools_call(
            4,
            "harness.status",
            status_arguments(PROJECT_ID, "req_binary_status"),
        ),
        tools_call(
            5,
            "harness.intake",
            intake_arguments(PROJECT_ID, "req_binary_intake", "idem_binary_intake"),
        ),
        tools_call(
            6,
            "harness.status",
            status_arguments_with_surface_id(PROJECT_ID, AGENT_SURFACE_ID, "req_binary_rejected"),
        ),
        tools_call(7, "harness.status", json!({ "unexpected": true })),
    ])?;

    let first = run_child(
        fixture.integration_command(["--integration", INTEGRATION_ID]),
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
        json!("harness-mcp")
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
        &tool_names[..PUBLIC_METHOD_TOOL_NAMES.len()],
        PUBLIC_METHOD_TOOL_NAMES
    );
    assert_eq!(
        tool_names[PUBLIC_METHOD_TOOL_NAMES.len()],
        "harness.list_projects"
    );
    assert_eq!(
        tool_names.iter().copied().collect::<BTreeSet<_>>().len(),
        10
    );

    assert_eq!(responses[&30]["result"]["isError"], json!(false));
    let project_list = adapter_tool_response(&responses[&30])?;
    assert_eq!(project_list["integration_id"], INTEGRATION_ID);
    assert_eq!(project_list["projects"][0]["project_id"], PROJECT_ID);
    assert_eq!(project_list["projects"][0]["available"], true);

    assert_eq!(responses[&4]["result"]["isError"], json!(false));
    let status = harness_response(&responses[&4])?;
    assert_eq!(status["base"]["response_kind"], "result");
    assert_eq!(status["base"]["state_version"], 0);

    let intake = harness_response(&responses[&5])?;
    assert_eq!(intake["base"]["response_kind"], "result");
    assert_eq!(intake["base"]["state_version"], 1);
    let task_id = intake["task_ref"]["record_id"]
        .as_str()
        .expect("intake response should include a task ref")
        .to_owned();

    assert_eq!(responses[&6]["result"]["isError"], json!(true));
    let surface_rejection = responses[&6]["result"]["content"][0]["text"]
        .as_str()
        .expect("surface rejection should be text");
    assert!(surface_rejection.contains("envelope.surface_id"));

    assert!(responses[&7].get("error").is_none());
    assert_eq!(responses[&7]["result"]["isError"], json!(true));
    let tool_error = responses[&7]["result"]["content"][0]["text"]
        .as_str()
        .expect("invalid known-tool arguments should return text content");
    assert!(tool_error.contains("envelope object"));

    let reconnect_before_handshake = run_child(
        fixture.integration_command(["--integration", INTEGRATION_ID]),
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
            "harness.status",
            status_arguments(PROJECT_ID, "req_binary_reconnect_status"),
        ),
    ])?;
    let reconnect = run_child(
        fixture.integration_command(["--integration", INTEGRATION_ID]),
        ChildStdin::WriteAndClose(reconnect_messages),
    )?;
    assert_success_captured(&reconnect);
    assert_eq!(captured_stderr(&reconnect), "");

    let reconnect_responses = responses_by_id(&reconnect.stdout)?;
    assert_eq!(
        reconnect_responses[&11]["result"]["serverInfo"]["name"],
        "harness-mcp"
    );
    assert_eq!(
        reconnect_responses[&11]["result"]["protocolVersion"],
        "2025-11-25"
    );
    let reconnect_status = harness_response(&reconnect_responses[&12])?;
    assert_eq!(reconnect_status["base"]["response_kind"], "result");
    assert_eq!(reconnect_status["base"]["state_version"], 1);
    assert_eq!(
        reconnect_status["active_task"]["task_ref"]["record_id"],
        task_id
    );

    Ok(())
}

#[test]
fn volicord_mcp_binary_suppresses_malformed_notification_output_and_effects(
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
                "name": "harness.intake",
                "arguments": intake_arguments(
                    PROJECT_ID,
                    "req_binary_notification_intake",
                    "idem_binary_notification_intake",
                )
            }),
        ),
        initialized_notification(),
        request(3, "tools/list", json!({})),
        tools_call(
            4,
            "harness.status",
            status_arguments(PROJECT_ID, "req_binary_notification_status"),
        ),
    ])?;

    let output = run_child(
        fixture.integration_command(["--integration", INTEGRATION_ID]),
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
    let status = harness_response(&responses[&4])?;
    assert_eq!(status["base"]["response_kind"], "result");
    assert_eq!(status["base"]["state_version"], 0);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

struct McpFixture {
    _runtime_home: TempRuntimeHome,
    runtime_home_path: PathBuf,
}

impl McpFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let repo_root = runtime_home.create_product_repo("product-repo")?;

        initialize_runtime_home(runtime_home.path(), "runtime_home_binary_mcp", "{}")?;
        register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: PROJECT_ID.to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        register_surface(
            runtime_home.path(),
            surface_registration(
                AGENT_SURFACE_ID,
                AGENT_INSTANCE_ID,
                SurfaceInteractionRole::Agent,
                &BASELINE_ACCESS_CLASSES,
            ),
        )?;
        register_surface(
            runtime_home.path(),
            surface_registration(
                USER_SURFACE_ID,
                USER_INSTANCE_ID,
                SurfaceInteractionRole::UserInteraction,
                &USER_ACCESS_CLASSES,
            ),
        )?;
        register_agent_integration(
            runtime_home.path(),
            AgentIntegrationRegistration {
                integration_id: INTEGRATION_ID.to_owned(),
                interaction_role: "agent".to_owned(),
                surface_id: AGENT_SURFACE_ID.to_owned(),
                surface_instance_id: AGENT_INSTANCE_ID.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_integration_project(
            runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: INTEGRATION_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;

        Ok(Self {
            runtime_home_path: runtime_home.path().to_path_buf(),
            _runtime_home: runtime_home,
        })
    }

    fn integration_command<const N: usize>(&self, args: [&str; N]) -> Command {
        let mut command = base_command();
        command.env("HARNESS_HOME", &self.runtime_home_path);
        command.args(args);
        command
    }

    fn fixed_project_env_command<const N: usize>(&self, args: [&str; N]) -> Command {
        let mut command = base_command();
        command.env("HARNESS_HOME", &self.runtime_home_path);
        command.env("HARNESS_PROJECT_ID", PROJECT_ID);
        command.env("HARNESS_SURFACE_ID", AGENT_SURFACE_ID);
        command.env("HARNESS_SURFACE_INSTANCE_ID", AGENT_INSTANCE_ID);
        command.args(args);
        command
    }

    fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        Ok(
            CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?
                .effect_counts()?,
        )
    }
}

fn surface_registration(
    surface_id: &str,
    surface_instance_id: &str,
    interaction_role: SurfaceInteractionRole,
    access_classes: &[&str],
) -> SurfaceRegistration {
    surface_registration_for_project(
        PROJECT_ID,
        surface_id,
        surface_instance_id,
        interaction_role,
        access_classes,
    )
}

fn surface_registration_for_project(
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
    interaction_role: SurfaceInteractionRole,
    access_classes: &[&str],
) -> SurfaceRegistration {
    SurfaceRegistration {
        project_id: project_id.to_owned(),
        surface_id: surface_id.to_owned(),
        surface_instance_id: surface_instance_id.to_owned(),
        surface_kind: "mcp".to_owned(),
        interaction_role,
        display_name: Some(format!("{surface_id} test surface")),
        capability_profile_json: json!({
            "supported_access_classes": access_classes,
            "write_authorization": access_classes.contains(&"write_authorization"),
            "manual_artifact_attachment_supported": access_classes.contains(&"artifact_registration")
        })
        .to_string(),
        local_access_json: json!({
            "authorized_access_classes": access_classes,
            "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
        })
        .to_string(),
        metadata_json: "{}".to_owned(),
    }
}

fn run_without_binding<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    command.args(args);
    Ok(command.output()?)
}

fn base_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_volicord-mcp"));
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
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "harness-binary-test",
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

fn status_arguments(project_id: &str, request_id: &str) -> Value {
    json!({
        "envelope": envelope(
            project_id,
            request_id,
            Value::Null,
            Value::Null
        ),
        "include": {
            "task": true,
            "pending_user_judgments": true,
            "write_authority": false,
            "evidence": false,
            "close": true,
            "guarantees": true
        }
    })
}

fn intake_arguments(project_id: &str, request_id: &str, idempotency_key: &str) -> Value {
    json!({
        "envelope": envelope(project_id, request_id, json!(idempotency_key), json!(0)),
        "plain_language_request": "Exercise the compiled MCP stdio binary.",
        "requested_mode": "work",
        "resume_policy": "create_new",
        "initial_scope": {
            "boundary": "Compiled MCP stdio process behavior.",
            "non_goals": ["Changing Core method semantics."],
            "acceptance_criteria": ["The stdio process records one task."]
        },
        "initial_context_refs": []
    })
}

fn status_arguments_with_surface_id(project_id: &str, surface_id: &str, request_id: &str) -> Value {
    let mut arguments = status_arguments(project_id, request_id);
    arguments["envelope"]["surface_id"] = json!(surface_id);
    arguments
}

fn envelope(
    project_id: &str,
    request_id: &str,
    idempotency_key: Value,
    expected_state_version: Value,
) -> Value {
    json!({
        "project_id": project_id,
        "task_id": null,
        "actor_kind": "agent",
        "request_id": request_id,
        "idempotency_key": idempotency_key,
        "expected_state_version": expected_state_version,
        "dry_run": false,
        "locale": "en-US"
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

fn harness_response(response: &Value) -> Result<Value, Box<dyn Error>> {
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
