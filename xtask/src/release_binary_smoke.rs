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

const COMMAND_TIMEOUT: Duration = Duration::from_secs(90);
const MCP_TIMEOUT: Duration = Duration::from_secs(15);
const PROCESS_CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const STDOUT_LIMIT_BYTES: usize = 1024 * 1024;
const STDERR_LIMIT_BYTES: usize = 8 * 1024;

const EXPECTED_PUBLIC_TOOLS: &[&str] = &[
    "volicord.status",
    "volicord.close_task",
    "volicord.request_user_action",
    "volicord.list_projects",
    "volicord.begin_integration_verification",
    "volicord.guard_probe",
    "volicord.get_integration_verification",
];
const USER_ONLY_TOOL: &str = "volicord.resolve_user_action";

/// Summary of one successful release-binary smoke run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBinarySmokeReport {
    binary: PathBuf,
    protocol_revision: String,
    tool_count: usize,
}

impl ReleaseBinarySmokeReport {
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
pub fn run_release_binary_smoke(binary: &Path) -> Result<ReleaseBinarySmokeReport> {
    let binary = resolve_binary(binary)?;
    let temporary = tempfile::Builder::new()
        .prefix("volicord-release-smoke-")
        .tempdir()?;
    let result = run_in_fixture(&binary, temporary.path());
    let cleanup = temporary.close().context("remove release smoke fixture");

    match (result, cleanup) {
        (Ok(report), Ok(())) => Ok(report),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(error.context(cleanup_error.to_string())),
    }
}

fn run_in_fixture(binary: &Path, root: &Path) -> Result<ReleaseBinarySmokeReport> {
    let repository = root.join("product-repository");
    let runtime_home = root.join("runtime-home");
    let codex_home = root.join("codex-home");
    let fake_bin = root.join("bin");
    fs::create_dir_all(&repository)?;
    fs::create_dir_all(&codex_home)?;
    fs::create_dir_all(&fake_bin)?;

    initialize_git_repository(&repository)?;
    create_fake_codex(binary, &fake_bin)?;
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

    Ok(ReleaseBinarySmokeReport {
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

fn create_fake_codex(binary: &Path, fake_bin: &Path) -> Result<()> {
    let name = if cfg!(windows) { "codex.exe" } else { "codex" };
    let destination = fake_bin.join(name);
    fs::copy(binary, &destination).with_context(|| {
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
        if !names.contains(expected) {
            bail!("MCP tools/list was missing expected public tool {expected}");
        }
    }
    if names.contains(USER_ONLY_TOOL) {
        bail!("MCP tools/list exposed user-only tool {USER_ONLY_TOOL}");
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

    const HANG_FIXTURE_ENV: &str = "VOLICORD_XTASK_HANG_FIXTURE";

    fn transcript(revision: &str, tools: &[&str]) -> BoundedCapture {
        let initialize = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"protocolVersion": revision}
        });
        let tools = tools
            .iter()
            .map(|name| serde_json::json!({"name": name}))
            .collect::<Vec<_>>();
        let tools_list = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"tools": tools}
        });
        BoundedCapture::from_bytes(format!("{initialize}\n{tools_list}\n"), STDOUT_LIMIT_BYTES)
    }

    #[test]
    fn missing_binary_is_rejected() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let error = run_release_binary_smoke(&temporary.path().join("missing-volicord"))
            .expect_err("missing binary must fail");
        assert!(error.to_string().contains("release binary does not exist"));
    }

    #[test]
    fn unlaunchable_binary_is_rejected() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let binary = temporary.path().join(if cfg!(windows) {
            "not-volicord.exe"
        } else {
            "not-volicord"
        });
        fs::write(&binary, b"not an executable").expect("write unlaunchable fixture");
        let error = run_release_binary_smoke(&binary).expect_err("unlaunchable binary must fail");
        assert!(error
            .to_string()
            .contains("failed to launch release binary --help"));
    }

    #[test]
    fn init_failure_is_reported_without_leaking_fixture_state() {
        let current_test = env::current_exe().expect("current test executable");
        let error = run_release_binary_smoke(&current_test)
            .expect_err("the Rust test harness is not the volicord CLI");
        assert!(error.to_string().contains("volicord init failed"));
    }

    #[test]
    fn nonzero_serve_exit_is_reported() {
        let current_test = env::current_exe().expect("current test executable");
        let error = execute_smoke_process(
            "nonzero serve fixture",
            smoke_command(current_test, Duration::from_secs(5))
                .arg("--unsupported-release-smoke-argument")
                .require_success(true),
        )
        .expect_err("nonzero serve status must fail");
        assert!(error.to_string().contains("nonzero serve fixture failed"));
    }

    #[test]
    fn malformed_init_json_is_rejected() {
        let error = parse_connection_id(&BoundedCapture::from_bytes(
            "{not-json\n",
            STDOUT_LIMIT_BYTES,
        ))
        .expect_err("malformed init JSON must fail");
        assert!(error.to_string().contains("parse volicord init JSON"));
    }

    #[test]
    fn malformed_initialize_response_is_rejected() {
        let revision = "preferred-test-revision";
        let tools = EXPECTED_PUBLIC_TOOLS;
        let mut transcript = transcript(revision, tools);
        let text = String::from_utf8(transcript.into_bytes()).expect("fixture UTF-8");
        transcript = BoundedCapture::from_bytes(
            text.replace(
                &format!("\"protocolVersion\":\"{revision}\""),
                "\"protocolVersion\":17",
            ),
            STDOUT_LIMIT_BYTES,
        );
        let error = validate_mcp_transcript(&transcript, revision)
            .expect_err("malformed initialize result must fail");
        assert!(error
            .to_string()
            .contains("did not contain a string protocolVersion"));
    }

    #[test]
    fn missing_expected_tool_is_rejected() {
        let revision = "preferred-test-revision";
        let tools = EXPECTED_PUBLIC_TOOLS
            .iter()
            .copied()
            .filter(|name| *name != "volicord.guard_probe")
            .collect::<Vec<_>>();
        let error = validate_mcp_transcript(&transcript(revision, &tools), revision)
            .expect_err("missing representative tool must fail");
        assert!(error.to_string().contains("volicord.guard_probe"));
    }

    #[test]
    fn unexpected_user_only_tool_is_rejected() {
        let revision = "preferred-test-revision";
        let mut tools = EXPECTED_PUBLIC_TOOLS.to_vec();
        tools.push(USER_ONLY_TOOL);
        let error = validate_mcp_transcript(&transcript(revision, &tools), revision)
            .expect_err("user-only resolution must not be exposed");
        assert!(error.to_string().contains(USER_ONLY_TOOL));
    }

    #[test]
    fn successful_transcript_reports_observed_inventory() {
        let revision = "preferred-test-revision";
        let tool_count =
            validate_mcp_transcript(&transcript(revision, EXPECTED_PUBLIC_TOOLS), revision)
                .expect("representative smoke transcript");
        assert_eq!(tool_count, EXPECTED_PUBLIC_TOOLS.len());
    }

    #[test]
    fn mcp_process_timeout_terminates_and_reaps_the_child() {
        let current_test = env::current_exe().expect("current test executable");
        let failure = smoke_command(current_test, Duration::from_millis(100))
            .args([
                "--ignored",
                "--exact",
                "release_binary_smoke::tests::bounded_process_hang_fixture",
            ])
            .env(HANG_FIXTURE_ENV, "1")
            .run()
            .expect_err("hanging child must time out");
        assert_eq!(failure.kind(), BoundedProcessFailureKind::Timeout);
        assert!(failure.status().is_some());
        assert!(failure.cleanup_detail().is_none());
    }

    #[test]
    #[ignore = "child-process fixture invoked by mcp_process_timeout_terminates_and_reaps_the_child"]
    fn bounded_process_hang_fixture() {
        if env::var_os(HANG_FIXTURE_ENV).is_some() {
            loop {
                std::thread::park();
            }
        }
    }
}
