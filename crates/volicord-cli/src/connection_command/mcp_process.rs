use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use volicord_store::agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW};
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES, WORKFLOW_METHOD_TOOL_NAMES,
};

use crate::host_integration::{HostPlan, HostScope};

use super::verification::{McpPreflightDiagnostics, VerificationStep};

const VOLICORD_HOME: &str = "VOLICORD_HOME";
const VOLICORD_MCP_VERIFICATION: &str = "VOLICORD_MCP_VERIFICATION";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProcessOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpLaunch {
    command: PathBuf,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<PathBuf>,
}

pub trait ConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        project_id: Option<&str>,
    ) -> Result<ConnectionProcessOutput, String>;
    fn verify_mcp_stdio(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, String>;
}

pub struct ProductionConnectionProcess;

impl ConnectionProcess for ProductionConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }

    fn run_preflight(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        project_id: Option<&str>,
    ) -> Result<ConnectionProcessOutput, String> {
        let mut child = Command::new(&launch.command);
        child
            .arg("mcp")
            .arg("--check")
            .arg("--connection")
            .arg(connection_id);
        if let Some(project_id) = project_id {
            child.arg("--project").arg(project_id);
        }
        apply_mcp_launch_context(&mut child, launch, runtime_home);
        child.stdin(Stdio::null());
        let output = child.output().map_err(|error| {
            format!(
                "failed to run {} mcp --check --connection {}: {error}",
                launch.command.display(),
                connection_id
            )
        })?;
        Ok(ConnectionProcessOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn verify_mcp_stdio(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, String> {
        verify_mcp_stdio_process(launch, runtime_home, connection_id, mode, DEFAULT_TIMEOUT)
    }
}

#[derive(Debug, Clone)]
pub struct McpVerification {
    pub(super) step: VerificationStep,
    pub(super) tools: Vec<String>,
}

impl McpVerification {
    pub(super) fn passed(tools: Vec<String>) -> Self {
        Self {
            step: VerificationStep::passed(format!("tools/list returned {} tools", tools.len())),
            tools,
        }
    }

    pub(super) fn failed(details: impl Into<String>) -> Self {
        Self {
            step: VerificationStep::failed(details),
            tools: Vec::new(),
        }
    }
}

pub(super) fn run_connection_preflight(
    process: &mut impl ConnectionProcess,
    launch: &McpLaunch,
    runtime_home: &Path,
    connection_id: &str,
    project_id: Option<&str>,
    mode: &str,
) -> VerificationStep {
    match process.run_preflight(launch, runtime_home, connection_id, project_id) {
        Ok(output) if output.success => {
            match validate_connection_preflight_report(&output.stdout, connection_id, mode) {
                Ok(diagnostics) => VerificationStep::passed("volicord mcp preflight passed")
                    .with_preflight_diagnostics(diagnostics),
                Err(message) => VerificationStep::failed(message),
            }
        }
        Ok(output) => VerificationStep::failed(format!(
            "volicord mcp preflight failed with status {}; stderr: {}",
            status_text(output.status_code),
            compact_stream(&output.stderr)
        )),
        Err(message) => VerificationStep::failed(message),
    }
}

fn validate_connection_preflight_report(
    stdout: &str,
    connection_id: &str,
    mode: &str,
) -> Result<Option<McpPreflightDiagnostics>, String> {
    let report = parse_colon_report(stdout)?;
    expect_report_field(&report, "configuration", "valid")?;
    expect_report_field(&report, "transport", "stdio")?;
    expect_report_field(&report, "connection_id", connection_id)?;
    expect_report_field(&report, "mode", mode)?;
    expect_report_field(&report, "enabled", "true")?;
    Ok(McpPreflightDiagnostics::from_preflight_report(&report))
}

fn parse_colon_report(stdout: &str) -> Result<BTreeMap<String, String>, String> {
    let mut report = BTreeMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once(':') {
            report.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    if report.is_empty() {
        Err("preflight did not return a key-value report".to_owned())
    } else {
        Ok(report)
    }
}

fn expect_report_field(
    report: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    match report.get(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!(
            "preflight field {key} was {actual}, expected {expected}"
        )),
        None => Err(format!("preflight field {key} was missing")),
    }
}

pub(super) fn mcp_launch_from_host_plan(plan: &HostPlan, repo_root: Option<&Path>) -> McpLaunch {
    let cwd = match plan.host_scope {
        HostScope::Project | HostScope::Local => repo_root.map(Path::to_path_buf),
        HostScope::User | HostScope::Export => None,
    };
    McpLaunch {
        command: PathBuf::from(&plan.entry.command),
        args: plan.entry.args.clone(),
        env: plan.entry.env.clone(),
        cwd,
    }
}

fn apply_mcp_launch_context(command: &mut Command, launch: &McpLaunch, runtime_home: &Path) {
    for (key, value) in &launch.env {
        command.env(key, value);
    }
    // Host-native repository projections may contain a portable environment
    // reference. Verification always binds the selected local Runtime Home
    // explicitly and must not launch with that unevaluated host placeholder.
    command.env(VOLICORD_HOME, runtime_home);
    if let Some(cwd) = &launch.cwd {
        command.current_dir(cwd);
    }
}

fn verify_mcp_stdio_process(
    launch: &McpLaunch,
    runtime_home: &Path,
    connection_id: &str,
    mode: &str,
    timeout: Duration,
) -> Result<McpVerification, String> {
    let mut child = Command::new(&launch.command);
    child.args(&launch.args);
    apply_mcp_launch_context(&mut child, launch, runtime_home);
    child.env(VOLICORD_MCP_VERIFICATION, "1");
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn().map_err(|error| {
        format!(
            "failed to launch {} for MCP handshake with connection {}: {error}",
            launch.command.display(),
            connection_id
        )
    })?;
    let deadline = Instant::now() + timeout;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture MCP stdout".to_owned())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open MCP stdin".to_owned())?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Ok(None));
                    break;
                }
                Ok(_) => {
                    let _ = tx.send(Ok(Some(line)));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.to_string()));
                    break;
                }
            }
        }
    });

    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "volicord-cli", "version": env!("CARGO_PKG_VERSION")}
            }
        }),
    )?;
    let initialize = read_json_response(&rx, deadline)?;
    validate_initialize_response(&initialize)?;
    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    )?;
    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )?;
    let tools = validate_tools_response(&read_json_response(&rx, deadline)?)?;
    validate_tools_for_mode(mode, &tools)?;
    drop(stdin);
    terminate_child(&mut child, deadline)?;
    Ok(McpVerification::passed(tools))
}

fn write_json_line(writer: &mut impl Write, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, &value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn read_json_response(
    rx: &mpsc::Receiver<Result<Option<String>, String>>,
    deadline: Instant,
) -> Result<Value, String> {
    let now = Instant::now();
    if now >= deadline {
        return Err("MCP handshake timed out".to_owned());
    }
    match rx.recv_timeout(deadline.saturating_duration_since(now)) {
        Ok(Ok(Some(line))) => serde_json::from_str::<Value>(&line)
            .map_err(|error| format!("invalid MCP JSON response: {error}; line: {line}")),
        Ok(Ok(None)) => Err("MCP process exited before response".to_owned()),
        Ok(Err(error)) => Err(format!("failed reading MCP response: {error}")),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("MCP handshake timed out".to_owned()),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("MCP response reader disconnected".to_owned())
        }
    }
}

fn validate_initialize_response(value: &Value) -> Result<(), String> {
    if value.get("error").is_some() {
        return Err(format!("MCP initialize returned error: {value}"));
    }
    let result = value
        .get("result")
        .ok_or_else(|| "MCP initialize response missing result".to_owned())?;
    if result
        .get("instructions")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("MCP initialize response missing instructions".to_owned());
    }
    Ok(())
}

fn validate_tools_response(value: &Value) -> Result<Vec<String>, String> {
    if value.get("error").is_some() {
        return Err(format!("MCP tools/list returned error: {value}"));
    }
    let tools = value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .ok_or_else(|| "MCP tools/list response missing result.tools".to_owned())?;
    let mut names = Vec::new();
    for tool in tools {
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "MCP tool entry missing name".to_owned())?;
        names.push(name.to_owned());
    }
    Ok(names)
}

pub(super) fn validate_tools_for_mode(mode: &str, tools: &[String]) -> Result<(), String> {
    match mode {
        CONNECTION_MODE_READ_ONLY => {
            validate_required_tools(tools, read_only_required_tool_names())
        }
        CONNECTION_MODE_WORKFLOW => validate_required_tools(tools, workflow_required_tool_names()),
        other => Err(format!(
            "unsupported connection mode for tool validation: {other}"
        )),
    }
}

fn validate_required_tools(
    tools: &[String],
    expected: impl IntoIterator<Item = &'static str>,
) -> Result<(), String> {
    for name in expected {
        if !tools.iter().any(|tool| tool == name) {
            return Err(format!("MCP tools/list missing required tool: {name}"));
        }
    }
    Ok(())
}

pub(super) fn workflow_required_tool_names() -> impl Iterator<Item = &'static str> {
    WORKFLOW_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

pub(super) fn read_only_required_tool_names() -> impl Iterator<Item = &'static str> {
    READ_ONLY_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

fn terminate_child(child: &mut Child, deadline: Instant) -> Result<(), String> {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
            Err(error) => return Err(format!("failed to wait for MCP process: {error}")),
        }
    }
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn compact_stream(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_launch_overrides_host_placeholder_with_selected_runtime_home() {
        let launch = McpLaunch {
            command: PathBuf::from("volicord"),
            args: Vec::new(),
            env: BTreeMap::from([(VOLICORD_HOME.to_owned(), "${VOLICORD_HOME}".to_owned())]),
            cwd: None,
        };
        let mut command = Command::new("volicord");

        apply_mcp_launch_context(&mut command, &launch, Path::new("/selected/runtime-home"));

        assert_eq!(
            command
                .get_envs()
                .find(|(name, _)| *name == VOLICORD_HOME)
                .and_then(|(_, value)| value),
            Some(std::ffi::OsStr::new("/selected/runtime-home"))
        );
    }
}
