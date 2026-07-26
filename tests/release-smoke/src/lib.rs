//! Cross-platform smoke validation for one externally supplied Volicord binary.

#![deny(unsafe_code)]

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};
use volicord_mcp_protocol::ProtocolRegistry;
use volicord_test_process::{
    BoundedCapture, BoundedCommand, BoundedProcessFailureKind, BoundedProcessOutput,
    ProcessDeadline,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::MethodName;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(90);
const MCP_TIMEOUT: Duration = Duration::from_secs(15);
const PROCESS_CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const STDOUT_LIMIT_BYTES: usize = 1024 * 1024;
const STDERR_LIMIT_BYTES: usize = 8 * 1024;

const EXPECTED_PUBLIC_TOOLS: [AgentToolId; 7] = [
    AgentToolId::STATUS,
    AgentToolId::CLOSE_TASK,
    AgentToolId::REQUEST_USER_ACTION,
    AgentToolId::LIST_PROJECTS,
    AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
    AgentToolId::GUARD_PROBE,
    AgentToolId::GET_INTEGRATION_VERIFICATION,
];

/// Stable version reported by the test-owned Codex executable fixture.
pub const CODEX_FIXTURE_VERSION: &str = "codex-fixture 0.145.0-test";

/// Summary of one successful actual-binary smoke run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSmokeReport {
    binary: PathBuf,
    protocol_revision: String,
    tool_count: usize,
}

impl ReleaseSmokeReport {
    /// Returns the exact executable file exercised by the harness.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Returns the preferred protocol revision exercised by the harness.
    pub fn protocol_revision(&self) -> &str {
        &self.protocol_revision
    }

    /// Returns the number of MCP tools observed in the smoke inventory.
    pub const fn tool_count(&self) -> usize {
        self.tool_count
    }
}

/// Exercises one already-built `volicord` executable through its public CLI and
/// manual stdio MCP boundary.
pub fn run_release_smoke(binary: &Path, fixture_executable: &Path) -> Result<ReleaseSmokeReport> {
    let binary = resolve_binary(binary)?;
    let fixture_executable = resolve_fixture_executable(fixture_executable)?;
    let temporary = tempfile::Builder::new()
        .prefix("volicord-release-smoke-")
        .tempdir()?;
    let result = run_in_fixture(&binary, &fixture_executable, temporary.path());
    let cleanup = temporary.close().context("remove release smoke fixture");

    match (result, cleanup) {
        (Ok(report), Ok(())) => Ok(report),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(error.context(cleanup_error.to_string())),
    }
}

/// Returns whether an executable path has the private Codex fixture identity.
pub fn is_codex_fixture_executable(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == codex_executable_name())
}

/// Returns the platform-native filename used for the private Codex fixture.
pub fn codex_executable_name() -> &'static str {
    if cfg!(windows) {
        "codex.exe"
    } else {
        "codex"
    }
}

fn run_in_fixture(
    binary: &Path,
    fixture_executable: &Path,
    root: &Path,
) -> Result<ReleaseSmokeReport> {
    let repository = root.join("product-repository");
    let runtime_home = root.join("runtime-home");
    let codex_home = root.join("codex-home");
    let fake_bin = root.join("bin");
    fs::create_dir_all(&repository)?;
    fs::create_dir_all(&codex_home)?;
    fs::create_dir_all(&fake_bin)?;

    initialize_git_repository(&repository)?;
    create_fake_codex(fixture_executable, &fake_bin)?;
    let fixture_path = prepend_path(&fake_bin)?;

    execute_smoke_process(
        "release binary --help",
        smoke_command(binary, COMMAND_TIMEOUT)
            .arg("--help")
            .require_success(true),
    )?;

    let init = smoke_command(binary, COMMAND_TIMEOUT)
        .args(["init", "--host", "codex", "--repo"])
        .arg(&repository)
        .args(["--profile", "record", "--home"])
        .arg(&runtime_home)
        .arg("--mcp-command")
        .arg(binary)
        .arg("--json");
    let init =
        configure_fixture_command(init, &repository, &runtime_home, &codex_home, &fixture_path);
    let init_output = execute_smoke_process("volicord init", init.require_success(true))?;
    let connection_id = parse_connection_id(init_output.stdout())?;

    let revision = ProtocolRegistry::production()
        .preferred_server_profile()
        .revision()
        .as_str();
    let input = mcp_input(revision)?;
    let serve = smoke_command(binary, MCP_TIMEOUT)
        .args(["mcp", "serve", "--connection"])
        .arg(&connection_id)
        .stdin(input);
    let serve = configure_fixture_command(
        serve,
        &repository,
        &runtime_home,
        &codex_home,
        &fixture_path,
    );
    let serve_output = execute_smoke_process("volicord mcp serve", serve.require_success(true))?;
    let tool_count = validate_mcp_transcript(serve_output.stdout(), revision)
        .with_context(|| format!("validate MCP stdout\n{}", process_context(&serve_output)))?;

    Ok(ReleaseSmokeReport {
        binary: binary.to_path_buf(),
        protocol_revision: revision.to_owned(),
        tool_count,
    })
}

fn resolve_binary(binary: &Path) -> Result<PathBuf> {
    let metadata = fs::metadata(binary)
        .with_context(|| format!("release binary does not exist: {}", binary.display()))?;
    if !metadata.is_file() {
        bail!("release binary is not a file: {}", binary.display());
    }
    fs::canonicalize(binary)
        .with_context(|| format!("resolve release binary path: {}", binary.display()))
}

fn resolve_fixture_executable(executable: &Path) -> Result<PathBuf> {
    let metadata = fs::metadata(executable).with_context(|| {
        format!(
            "release-smoke fixture executable does not exist: {}",
            executable.display()
        )
    })?;
    if !metadata.is_file() {
        bail!(
            "release-smoke fixture executable is not a file: {}",
            executable.display()
        );
    }
    fs::canonicalize(executable).with_context(|| {
        format!(
            "resolve release-smoke fixture executable path: {}",
            executable.display()
        )
    })
}

fn initialize_git_repository(repository: &Path) -> Result<()> {
    execute_smoke_process(
        "git init for disposable Product Repository",
        smoke_command("git", COMMAND_TIMEOUT)
            .arg("init")
            .arg("--quiet")
            .arg(repository)
            .require_success(true),
    )
    .map(|_| ())
}

fn create_fake_codex(fixture_executable: &Path, fake_bin: &Path) -> Result<()> {
    let destination = fake_bin.join(codex_executable_name());
    fs::copy(fixture_executable, &destination).with_context(|| {
        format!(
            "create disposable codex executable at {}",
            destination.display()
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&destination)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions)?;
    }
    Ok(())
}

fn prepend_path(directory: &Path) -> Result<std::ffi::OsString> {
    let mut paths = vec![directory.to_path_buf()];
    if let Some(current) = env::var_os("PATH") {
        paths.extend(env::split_paths(&current));
    }
    env::join_paths(paths).context("construct disposable smoke PATH")
}

fn configure_fixture_command(
    command: BoundedCommand,
    repository: &Path,
    runtime_home: &Path,
    codex_home: &Path,
    path: &std::ffi::OsStr,
) -> BoundedCommand {
    command
        .current_dir(repository)
        .env("VOLICORD_HOME", runtime_home)
        .env("CODEX_HOME", codex_home)
        .env("PATH", path)
}

fn parse_connection_id(stdout: &BoundedCapture) -> Result<String> {
    require_complete(stdout, "volicord init stdout")?;
    let document: Value = serde_json::from_slice(stdout.bytes())
        .with_context(|| format!("parse volicord init JSON\n{}", stdout.render_lossy()))?;
    document
        .pointer("/connection/connection_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("volicord init JSON did not contain connection.connection_id"))
}

fn mcp_input(revision: &str) -> Result<Vec<u8>> {
    let requests = [
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": revision,
                "capabilities": {},
                "clientInfo": {
                    "name": "volicord-release-smoke",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    ];
    let mut input = Vec::new();
    for request in requests {
        serde_json::to_writer(&mut input, &request)?;
        input.push(b'\n');
    }
    Ok(input)
}

fn validate_mcp_transcript(stdout: &BoundedCapture, requested_revision: &str) -> Result<usize> {
    require_complete(stdout, "MCP stdout")?;
    let text = std::str::from_utf8(stdout.bytes()).context("MCP stdout was not UTF-8")?;
    let mut initialize = None;
    let mut tools_list = None;

    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = serde_json::from_str(line)
            .with_context(|| format!("MCP stdout line {} was not JSON", index + 1))?;
        match message.get("id").and_then(Value::as_u64) {
            Some(1) => insert_response(&mut initialize, message, "initialize")?,
            Some(2) => insert_response(&mut tools_list, message, "tools/list")?,
            _ => {}
        }
    }

    let initialize = initialize.ok_or_else(|| anyhow!("MCP initialize response was missing"))?;
    if let Some(error) = initialize.get("error") {
        bail!("MCP initialize returned an error: {error}");
    }
    let actual_revision = initialize
        .pointer("/result/protocolVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow!("MCP initialize response did not contain a string protocolVersion")
        })?;
    if actual_revision != requested_revision {
        bail!(
            "MCP initialize selected {actual_revision}, expected requested revision {requested_revision}"
        );
    }

    let tools_list = tools_list
        .ok_or_else(|| anyhow!("MCP tools/list response was missing after initialized"))?;
    if let Some(error) = tools_list.get("error") {
        bail!("MCP tools/list returned an error after initialized: {error}");
    }
    let tools = tools_list
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("MCP tools/list response did not contain a tools array"))?;
    let names = tools
        .iter()
        .map(|tool| {
            tool.get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("MCP tools/list contained a tool without a string name"))
        })
        .collect::<Result<BTreeSet<_>>>()?;

    for expected in EXPECTED_PUBLIC_TOOLS {
        if !names.contains(expected.wire_name()) {
            bail!(
                "MCP tools/list was missing expected public tool {}",
                expected.wire_name()
            );
        }
    }
    let user_only_tool = MethodName::ResolveUserAction.as_str();
    if names.contains(user_only_tool) {
        bail!("MCP tools/list exposed user-only tool {user_only_tool}");
    }
    Ok(names.len())
}

fn insert_response(slot: &mut Option<Value>, message: Value, label: &str) -> Result<()> {
    if slot.replace(message).is_some() {
        bail!("MCP stdout contained duplicate {label} responses");
    }
    Ok(())
}

fn smoke_command(program: impl AsRef<std::ffi::OsStr>, timeout: Duration) -> BoundedCommand {
    BoundedCommand::new(
        program,
        ProcessDeadline::new(timeout, PROCESS_CLEANUP_TIMEOUT),
        STDOUT_LIMIT_BYTES,
        STDERR_LIMIT_BYTES,
    )
}

fn execute_smoke_process(label: &str, command: BoundedCommand) -> Result<BoundedProcessOutput> {
    command.run().map_err(|failure| {
        if failure.kind() == BoundedProcessFailureKind::Spawn {
            anyhow!("failed to launch {label}: {failure}")
        } else {
            anyhow!("{label} failed: {failure}")
        }
    })
}

fn require_complete(capture: &BoundedCapture, label: &str) -> Result<()> {
    if capture.is_truncated() {
        bail!(
            "{label} exceeded the capture limit by {} bytes",
            capture.omitted_bytes()
        );
    }
    Ok(())
}

fn process_context(output: &BoundedProcessOutput) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status(),
        output.stdout().render_lossy(),
        output.stderr().render_lossy()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcript(
        initialize: Value,
        tools: impl IntoIterator<Item = &'static str>,
    ) -> BoundedCapture {
        let tools = tools
            .into_iter()
            .map(|name| serde_json::json!({"name": name}))
            .collect::<Vec<_>>();
        let tools_list = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"tools": tools}
        });
        BoundedCapture::from_bytes(format!("{initialize}\n{tools_list}\n"), STDOUT_LIMIT_BYTES)
    }

    fn successful_initialize(revision: &str) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"protocolVersion": revision}
        })
    }

    fn expected_tool_names() -> Vec<&'static str> {
        EXPECTED_PUBLIC_TOOLS
            .iter()
            .map(|tool| tool.wire_name())
            .collect()
    }

    #[test]
    fn initialize_failure_is_reported() {
        let revision = "preferred-test-revision";
        let initialize = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32602, "message": "fixture initialize failure"}
        });
        let error =
            validate_mcp_transcript(&transcript(initialize, expected_tool_names()), revision)
                .expect_err("initialize failure must fail the smoke");
        assert!(error
            .to_string()
            .contains("MCP initialize returned an error"));
    }

    #[test]
    fn missing_expected_tool_is_rejected() {
        let revision = "preferred-test-revision";
        let tools = EXPECTED_PUBLIC_TOOLS
            .iter()
            .copied()
            .filter(|tool| *tool != AgentToolId::GUARD_PROBE)
            .map(AgentToolId::wire_name);
        let error = validate_mcp_transcript(
            &transcript(successful_initialize(revision), tools),
            revision,
        )
        .expect_err("missing representative tool must fail");
        assert!(error
            .to_string()
            .contains(AgentToolId::GUARD_PROBE.wire_name()));
    }

    #[test]
    fn unexpected_user_only_tool_is_rejected() {
        let revision = "preferred-test-revision";
        let mut tools = expected_tool_names();
        tools.push(MethodName::ResolveUserAction.as_str());
        let error = validate_mcp_transcript(
            &transcript(successful_initialize(revision), tools),
            revision,
        )
        .expect_err("user-only resolution must not be exposed");
        assert!(error
            .to_string()
            .contains(MethodName::ResolveUserAction.as_str()));
    }

    #[test]
    fn successful_transcript_reports_observed_inventory() {
        let revision = "preferred-test-revision";
        let tool_count = validate_mcp_transcript(
            &transcript(successful_initialize(revision), expected_tool_names()),
            revision,
        )
        .expect("representative smoke transcript");
        assert_eq!(tool_count, EXPECTED_PUBLIC_TOOLS.len());
    }
}
