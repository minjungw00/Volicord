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
use volicord_mcp::PUBLIC_METHOD_TOOL_NAMES;
use volicord_store::core_pipeline::StorageEffectCounts;
use volicord_test_support::core_fixtures::CoreFixture;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn volicord_mcp_binary_reports_help_version_and_preflight() -> Result<(), Box<dyn Error>> {
    let fixture = McpFixture::new("mcp-bin-preflight")?;

    let help = run_without_binding(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("--connection <connection_id>"));

    let version = run_without_binding(["--version"])?;
    assert_success(&version);
    assert!(stdout(&version).starts_with("volicord-mcp "));

    let no_args = run_without_binding([])?;
    assert_eq!(no_args.status.code(), Some(2));
    assert!(stderr(&no_args).contains("--connection is required"));

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
    assert_report_line(&report, "allowed_projects: 1");
    assert_report_line(&report, "available_projects: 1");
    assert_report_line(
        &report,
        &format!("project[0].project_id: {}", fixture.project_id()),
    );
    assert_report_line(&report, "project[0].available: true");
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
fn volicord_mcp_stdio_uses_line_delimited_json_and_reconnects_state() -> Result<(), Box<dyn Error>>
{
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
        fixture.connection_command(["--connection", fixture.connection_id()]),
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
        &tool_names[..PUBLIC_METHOD_TOOL_NAMES.len()],
        PUBLIC_METHOD_TOOL_NAMES
    );
    assert_eq!(
        tool_names[PUBLIC_METHOD_TOOL_NAMES.len()],
        "volicord.list_projects"
    );
    assert!(!tool_names.contains(&"volicord.record_user_judgment"));
    assert_eq!(
        tool_names.iter().copied().collect::<BTreeSet<_>>().len(),
        PUBLIC_METHOD_TOOL_NAMES.len() + 1
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
        fixture.connection_command(["--connection", fixture.connection_id()]),
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
        fixture.connection_command(["--connection", fixture.connection_id()]),
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
        fixture.connection_command(["--connection", fixture.connection_id()]),
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
        command.args(args);
        command
    }

    fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        Ok(self.fixture.counts()?)
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
