#![forbid(unsafe_code)]

mod support;

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    ffi::OsStr,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin as ProcessStdin, Command, Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use support::binary_fixture::{run_child, ChildStdin};
use volicord_store::inspection::{
    inspect_runtime_home, DatabaseInspection, RegistryInspectionSnapshot,
};
use volicord_store::operational_sessions::latest_current_managed_runtime_session;
use volicord_test_support::TempRuntimeHome;
use volicord_types::{guard_manifest_from_json, GuardHookPhase, GuardManifest};

const FUTURE_VERSION: &str = "999.0.0";
const NEXT_FUTURE_VERSION: &str = "1000.0.0";
const NATIVE_SESSION_999: &str = "future.session.999";
const NATIVE_SESSION_1000: &str = "future.session.1000";
const NATIVE_THREAD: &str = "future.thread.operational";
const MCP_FIXTURE_MODE: &str = "VOLICORD_TEST_MCP_FIXTURE";
const CODEX_VERSION_ENV: &str = "VOLICORD_TEST_CODEX_VERSION";

fn main() {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args == [OsStr::new("--version")] {
        println!(
            "codex-cli {}",
            env::var(CODEX_VERSION_ENV).unwrap_or_else(|_| FUTURE_VERSION.to_owned())
        );
        return;
    }
    if args.first().is_some_and(|arg| arg == "mcp")
        && env::var(MCP_FIXTURE_MODE).as_deref() == Ok("startup_failure")
    {
        eprintln!("deterministic MCP fixture startup failure");
        std::process::exit(70);
    }

    if let Err(error) = run_operational_regressions() {
        panic!("operational host end-to-end regression failed: {error}");
    }
}

fn run_operational_regressions() -> Result<(), Box<dyn Error>> {
    fresh_operation_version_transition_and_read_only_status()?;
    dry_run_has_no_mutation()?;
    protocol_failures_are_authoritative()?;
    local_process_and_configuration_failures_are_structured()?;
    guard_failures_are_current_and_structured()?;
    Ok(())
}

fn fresh_operation_version_transition_and_read_only_status() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-host-complete")?;
    let init = fixture.run_init(FUTURE_VERSION, None, false)?;
    let init_report = assert_connection_report(&init, 0, "init", "action_required")?;
    assert_eq!(init_report["setup_applied"], true);
    assert_check(&init_report, "managed_config", "passed", None);
    assert_check(&init_report, "host_executable", "passed", None);
    assert_check(&init_report, "mcp_server", "passed", None);
    assert_check(&init_report, "host_session", "pending", None);
    assert_check(&init_report, "required_tools", "pending", None);
    assert_check(&init_report, "tool_round_trip", "pending", None);
    assert_check(&init_report, "guard_observation", "pending", None);
    assert!(init_report["actions"]
        .as_array()
        .is_some_and(|actions| !actions.is_empty()));

    let snapshot = fixture.registry_snapshot();
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.agent_connections.len(), 1);
    assert_eq!(snapshot.connection_projects.len(), 1);
    assert_eq!(snapshot.guard_installations.len(), 1);
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let project_id = snapshot.projects[0].project_id.clone();
    let manifest = guard_manifest_from_json(&snapshot.guard_installations[0].manifest_json)?;
    assert_current_guard_projection(&fixture, &manifest)?;

    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        NATIVE_SESSION_999,
        &manifest,
    )?;

    let complete = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let complete_report = assert_connection_report(&complete, 0, "status", "complete")?;
    for check_id in [
        "guard_files",
        "guard_observation",
        "host_session",
        "required_tools",
        "tool_round_trip",
    ] {
        assert_check(&complete_report, check_id, "passed", None);
    }
    assert_eq!(complete_report["actions"], json!([]));
    assert_compact_public_shape(&complete_report);

    let before_status = fixture.content_snapshot()?;
    let repeated = fixture.run_connection("status", FUTURE_VERSION, true)?;
    assert_connection_report(&repeated, 0, "status", "complete")?;
    let after_status = fixture.content_snapshot()?;
    assert_eq!(after_status, before_status, "connection status wrote state");

    let human = fixture.run_connection("status", FUTURE_VERSION, false)?;
    assert_eq!(human.status.code(), Some(0));
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout)?;
    assert!(human.starts_with("Operation: status\nStatus: complete\n"));
    for check in complete_report["checks"].as_array().expect("checks") {
        assert!(human.contains(check["id"].as_str().expect("check id")));
        assert!(human.contains(check["summary"].as_str().expect("check summary")));
    }

    let changed_version = fixture.run_connection("verify", NEXT_FUTURE_VERSION, true)?;
    let changed_report =
        assert_connection_report(&changed_version, 0, "verify", "action_required")?;
    for (check_id, code) in [
        ("host_session", "host_version_observation_stale"),
        ("required_tools", "required_tools_observation_stale"),
        ("tool_round_trip", "tool_round_trip_observation_stale"),
    ] {
        assert_check(&changed_report, check_id, "pending", Some(code));
    }
    assert!(changed_report["actions"].as_array().is_some_and(|actions| {
        actions.iter().any(|action| {
            let instruction = action["instruction"].as_str().unwrap_or_default();
            instruction.contains("Codex") || instruction.contains("Volicord")
        })
    }));

    fixture.run_successful_managed_mcp(
        &connection_id,
        &project_id,
        NEXT_FUTURE_VERSION,
        NATIVE_SESSION_1000,
    )?;
    let completed_again = fixture.run_connection("status", NEXT_FUTURE_VERSION, true)?;
    assert_connection_report(&completed_again, 0, "status", "complete")?;

    let wrapper = fixture.repo_root.join(".codex/hooks/volicord-pre-tool.sh");
    fs::write(&wrapper, "malformed current wrapper\n")?;
    let tampered = fixture.run_connection("status", NEXT_FUTURE_VERSION, true)?;
    let tampered_report = assert_connection_report(&tampered, 1, "status", "failed")?;
    assert_check(
        &tampered_report,
        "guard_files",
        "failed",
        Some("guard_files_failed"),
    );
    Ok(())
}

fn dry_run_has_no_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::new("operational-host-dry-run")?;
    let repo_before = fixture.repository_snapshot()?;
    assert!(!fixture.runtime_home.exists());
    let output = fixture.run_init(FUTURE_VERSION, None, true)?;
    let report = assert_connection_report(&output, 0, "init", "action_required")?;
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["setup_applied"], false);
    assert!(report["planned_changes"].is_array());
    assert!(!fixture.runtime_home.exists());
    assert_eq!(fixture.repository_snapshot()?, repo_before);
    Ok(())
}

fn protocol_failures_are_authoritative() -> Result<(), Box<dyn Error>> {
    let initialize = OperationalFixture::initialized("operational-initialize-failure")?;
    initialize.run_managed_mcp_messages(
        &initialize.connection_id(),
        Some(&initialize.project_id()),
        json_lines(&[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "9999-unsupported",
                "capabilities": {},
                "clientInfo": {"name": "future-client", "version": FUTURE_VERSION}
            }
        })])?,
    )?;
    initialize.assert_failed_status("host_session", "host_session_initialize_failed")?;

    let tools_list = OperationalFixture::initialized("operational-tools-list-failure")?;
    tools_list.run_managed_mcp_messages(
        &tools_list.connection_id(),
        Some(&tools_list.project_id()),
        json_lines(&[
            initialize_request(FUTURE_VERSION),
            initialized_notification(),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": []
            }),
        ])?,
    )?;
    tools_list.assert_failed_status("required_tools", "required_tools_invalid")?;

    let safe_call = OperationalFixture::initialized("operational-safe-call-failure")?;
    safe_call.run_safe_tool_storage_failure()?;
    safe_call.assert_failed_status("tool_round_trip", "tool_round_trip_failed")?;

    let missing_tools = OperationalFixture::initialized("operational-missing-tools")?;
    let state_db = missing_tools.project_state_db_path();
    let displaced = state_db.with_extension("sqlite.displaced");
    fs::rename(&state_db, &displaced)?;
    let result = missing_tools.run_managed_mcp_messages(
        &missing_tools.connection_id(),
        None,
        json_lines(&[
            initialize_request(FUTURE_VERSION),
            initialized_notification(),
            tools_list_request(),
        ])?,
    );
    fs::rename(&displaced, &state_db)?;
    result?;
    missing_tools.assert_failed_status("required_tools", "required_tools_missing")?;
    Ok(())
}

fn local_process_and_configuration_failures_are_structured() -> Result<(), Box<dyn Error>> {
    let unavailable = OperationalFixture::initialized("operational-host-unavailable")?;
    let output =
        unavailable.run_connection_with_path("verify", FUTURE_VERSION, true, Path::new(""))?;
    let report = assert_connection_report(&output, 1, "verify", "failed")?;
    assert_check(
        &report,
        "host_executable",
        "failed",
        Some("host_executable_not_found"),
    );

    let malformed = OperationalFixture::initialized("operational-config-malformed")?;
    let snapshot = malformed.registry_snapshot();
    fs::write(
        &snapshot.agent_connections[0].config_target,
        "[mcp_servers.volicord\n",
    )?;
    let output = malformed.run_connection("status", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&output, 1, "status", "failed")?;
    assert_check(
        &report,
        "managed_config",
        "failed",
        Some("managed_config_malformed"),
    );

    let startup = OperationalFixture::new("operational-mcp-startup-failure")?;
    let fixture_executable = startup.install_mcp_fixture_executable()?;
    let output = startup.run_init(
        FUTURE_VERSION,
        Some((&fixture_executable, "startup_failure")),
        false,
    )?;
    let report = assert_connection_report(&output, 1, "init", "failed")?;
    assert_eq!(report["setup_applied"], true);
    assert_check(
        &report,
        "mcp_server",
        "failed",
        Some("mcp_server_preflight_failed"),
    );
    Ok(())
}

fn guard_failures_are_current_and_structured() -> Result<(), Box<dyn Error>> {
    let fixture = OperationalFixture::initialized("operational-guard-contract-failure")?;
    let snapshot = fixture.registry_snapshot();
    let connection_id = snapshot.agent_connections[0].connection_internal_id.clone();
    let project_id = snapshot.projects[0].project_id.clone();
    let manifest = guard_manifest_from_json(&snapshot.guard_installations[0].manifest_json)?;
    fixture.run_successful_managed_mcp_with_guard(
        &connection_id,
        &project_id,
        FUTURE_VERSION,
        "future.session.guard.failure",
        &manifest,
    )?;

    let command = manifest.runtime_commands.get(GuardHookPhase::PreTool);
    let malformed_event = json!({
        "session_id": "future.session.guard.failure",
        "turn_id": "future.turn.guard.malformed",
        "tool_name": "Read",
        "tool_input": {"path": fixture.repo_root.join("README.md")}
    });
    let failed_hook = fixture.run_guard_command(command, &malformed_event)?;
    assert!(!failed_hook.status.success());

    let status = fixture.run_connection("status", FUTURE_VERSION, true)?;
    let report = assert_connection_report(&status, 1, "status", "failed")?;
    assert_check(
        &report,
        "guard_observation",
        "failed",
        Some("guard_observation_failed"),
    );
    Ok(())
}

struct OperationalFixture {
    _temporary_root: TempRuntimeHome,
    runtime_home: PathBuf,
    codex_home: PathBuf,
    user_home: PathBuf,
    path_dir: PathBuf,
    repo_root: PathBuf,
}

impl OperationalFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let temporary_root = TempRuntimeHome::new(prefix)?;
        let runtime_home = temporary_root.path().join("runtime-home");
        let codex_home = temporary_root.path().join("codex-home");
        let user_home = temporary_root.path().join("user-home");
        let path_dir = temporary_root.path().join("path");
        let repo_root = temporary_root.path().join("product-repository");
        for directory in [&codex_home, &user_home, &path_dir, &repo_root] {
            fs::create_dir_all(directory)?;
        }
        fs::create_dir(repo_root.join(".git"))?;
        let codex_name = if cfg!(windows) { "codex.exe" } else { "codex" };
        fs::copy(env::current_exe()?, path_dir.join(codex_name))?;
        Ok(Self {
            _temporary_root: temporary_root,
            runtime_home,
            codex_home,
            user_home,
            path_dir,
            repo_root,
        })
    }

    fn initialized(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let fixture = Self::new(prefix)?;
        let output = fixture.run_init(FUTURE_VERSION, None, false)?;
        assert_connection_report(&output, 0, "init", "action_required")?;
        Ok(fixture)
    }

    fn install_mcp_fixture_executable(&self) -> Result<PathBuf, Box<dyn Error>> {
        let directory = self._temporary_root.path().join("mcp-fixture");
        fs::create_dir_all(&directory)?;
        let name = if cfg!(windows) {
            "volicord.exe"
        } else {
            "volicord"
        };
        let path = directory.join(name);
        fs::copy(env::current_exe()?, &path)?;
        Ok(path)
    }

    fn base_command(&self, program: impl AsRef<OsStr>, version: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env("PATH", &self.path_dir)
            .env(CODEX_VERSION_ENV, version)
            .current_dir(&self.repo_root);
        copy_required_platform_environment(&mut command);
        command
    }

    fn run_init(
        &self,
        version: &str,
        mcp_fixture: Option<(&Path, &str)>,
        dry_run: bool,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .arg("init")
            .arg("--host")
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--profile")
            .arg("record")
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--json");
        if let Some((path, mode)) = mcp_fixture {
            command
                .arg("--mcp-command")
                .arg(path)
                .env(MCP_FIXTURE_MODE, mode);
        }
        if dry_run {
            command.arg("--dry-run");
        }
        Ok(command.output()?)
    }

    fn run_connection(
        &self,
        operation: &str,
        version: &str,
        json: bool,
    ) -> Result<Output, Box<dyn Error>> {
        self.run_connection_with_path(operation, version, json, &self.path_dir)
    }

    fn run_connection_with_path(
        &self,
        operation: &str,
        version: &str,
        json: bool,
        path: &Path,
    ) -> Result<Output, Box<dyn Error>> {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), version);
        command
            .env("PATH", path)
            .arg("connection")
            .arg(operation)
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root);
        if json {
            command.arg("--json");
        }
        Ok(command.output()?)
    }

    fn run_successful_managed_mcp(
        &self,
        connection_id: &str,
        project_id: &str,
        version: &str,
        native_session: &str,
    ) -> Result<(), Box<dyn Error>> {
        let output = self.run_managed_mcp_messages(
            connection_id,
            Some(project_id),
            json_lines(&[
                initialize_request(version),
                initialized_notification(),
                tools_list_request(),
                managed_tool_call(3, "volicord.list_projects", json!({}), native_session),
            ])?,
        )?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let responses = json_rpc_responses(&output.stdout)?;
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[2]["result"]["isError"], false);
        Ok(())
    }

    fn run_successful_managed_mcp_with_guard(
        &self,
        connection_id: &str,
        project_id: &str,
        version: &str,
        native_session: &str,
        manifest: &GuardManifest,
    ) -> Result<(), Box<dyn Error>> {
        let mut command = self.managed_mcp_command(connection_id, Some(project_id));
        let mut child = LiveMcpChild::spawn(&mut command)?;
        child.write(&json_lines(&[
            initialize_request(version),
            initialized_notification(),
            tools_list_request(),
            managed_tool_call(3, "volicord.list_projects", json!({}), native_session),
        ])?)?;
        let started = Instant::now();
        loop {
            if latest_current_managed_runtime_session(&self.runtime_home, connection_id)?
                .is_some_and(|session| session.last_safe_read_only_tool_call_at.is_some())
            {
                break;
            }
            if started.elapsed() >= Duration::from_secs(10) {
                return Err("managed MCP safe round trip was not recorded before timeout".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.run_current_guard_phases(manifest, native_session)?;
        let output = child.finish()?;
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let responses = json_rpc_responses(&output.stdout)?;
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[2]["result"]["isError"], false);
        Ok(())
    }

    fn run_safe_tool_storage_failure(&self) -> Result<(), Box<dyn Error>> {
        let connection_id = self.connection_id();
        let project_id = self.project_id();
        let output = self.run_managed_mcp_messages(
            &connection_id,
            Some(&project_id),
            json_lines(&[
                initialize_request(FUTURE_VERSION),
                initialized_notification(),
                tools_list_request(),
                managed_tool_call(
                    3,
                    "volicord.status",
                    json!({"detail": "workflow", "task_id": "task_missing"}),
                    "future.session.safe.failure",
                ),
            ])?,
        )?;
        assert!(
            !output.status.success()
                || json_rpc_responses(&output.stdout)?.iter().any(|response| {
                    response.pointer("/result/isError").and_then(Value::as_bool) == Some(true)
                        || response.get("error").is_some()
                }),
            "safe call unexpectedly succeeded"
        );
        Ok(())
    }

    fn run_managed_mcp_messages(
        &self,
        connection_id: &str,
        project_id: Option<&str>,
        input: String,
    ) -> Result<support::binary_fixture::CapturedChildOutput, Box<dyn Error>> {
        let command = self.managed_mcp_command(connection_id, project_id);
        run_child(command, ChildStdin::WriteAndClose(input))
    }

    fn managed_mcp_command(&self, connection_id: &str, project_id: Option<&str>) -> Command {
        let mut command = self.base_command(env!("CARGO_BIN_EXE_volicord"), FUTURE_VERSION);
        command
            .env("VOLICORD_MCP_LAUNCH", "managed_host")
            .env("VOLICORD_MCP_HOST", "codex")
            .env("VOLICORD_MCP_CONNECTION_ID", connection_id)
            .arg("mcp")
            .arg("--stdio")
            .arg("--connection")
            .arg(connection_id);
        if let Some(project_id) = project_id {
            command
                .env("VOLICORD_MCP_PROJECT_ID", project_id)
                .arg("--project")
                .arg(project_id);
        }
        command
    }

    fn run_current_guard_phases(
        &self,
        manifest: &GuardManifest,
        native_session: &str,
    ) -> Result<(), Box<dyn Error>> {
        for (phase, event) in [
            (
                GuardHookPhase::PreTool,
                json!({
                    "session_id": native_session,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": "future.turn.pre",
                    "tool_name": "Read",
                    "tool_input": {"path": self.repo_root.join("README.md")}
                }),
            ),
            (
                GuardHookPhase::PostTool,
                json!({
                    "session_id": native_session,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": "future.turn.post",
                    "tool_name": "Read",
                    "tool_input": {"path": self.repo_root.join("README.md")},
                    "tool_response": {"success": true}
                }),
            ),
            (
                GuardHookPhase::PromptCapture,
                json!({
                    "session_id": native_session,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": "future.turn.prompt",
                    "prompt": "Observe current Guard behavior."
                }),
            ),
        ] {
            let output = self.run_guard_command(manifest.runtime_commands.get(phase), &event)?;
            assert!(
                output.status.success(),
                "Guard phase {} failed: {}",
                phase.as_str(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    fn run_guard_command(
        &self,
        command_spec: &volicord_types::GuardCommand,
        event: &Value,
    ) -> Result<support::binary_fixture::CapturedChildOutput, Box<dyn Error>> {
        let mut command = self.base_command(&command_spec.command, FUTURE_VERSION);
        command
            .env("VOLICORD_MANAGED_WRAPPER", "codex-record")
            .args(&command_spec.args);
        run_child(
            command,
            ChildStdin::WriteAndClose(format!("{}\n", serde_json::to_string(event)?)),
        )
    }

    fn assert_failed_status(&self, check_id: &str, code: &str) -> Result<(), Box<dyn Error>> {
        let output = self.run_connection("status", FUTURE_VERSION, true)?;
        let report = assert_connection_report(&output, 1, "status", "failed")?;
        assert_check(&report, check_id, "failed", Some(code));
        assert!(!serde_json::to_string(&report)?.contains("unsupported_artifact"));
        Ok(())
    }

    fn registry_snapshot(&self) -> RegistryInspectionSnapshot {
        match inspect_runtime_home(&self.runtime_home).registry {
            DatabaseInspection::Present(snapshot) => snapshot,
            other => panic!("expected registry snapshot, got {other:?}"),
        }
    }

    fn connection_id(&self) -> String {
        self.registry_snapshot().agent_connections[0]
            .connection_internal_id
            .clone()
    }

    fn project_id(&self) -> String {
        self.registry_snapshot().projects[0].project_id.clone()
    }

    fn project_state_db_path(&self) -> PathBuf {
        self.runtime_home
            .join("projects")
            .join(self.project_id())
            .join("state.sqlite")
    }

    fn repository_snapshot(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        directory_contents(&self.repo_root)
    }

    fn content_snapshot(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        let mut snapshot = BTreeMap::new();
        for (prefix, root) in [
            (Path::new("runtime"), &self.runtime_home),
            (Path::new("repository"), &self.repo_root),
            (Path::new("codex"), &self.codex_home),
        ] {
            for (path, bytes) in directory_contents(root)? {
                snapshot.insert(prefix.join(path), bytes);
            }
        }
        Ok(snapshot)
    }
}

struct LiveMcpChild {
    child: Child,
    stdin: Option<ProcessStdin>,
    stdout: JoinHandle<io::Result<Vec<u8>>>,
    stderr: JoinHandle<io::Result<Vec<u8>>>,
}

impl LiveMcpChild {
    fn spawn(command: &mut Command) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("managed MCP stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("managed MCP stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("managed MCP stderr was not piped"))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout: thread::spawn(move || read_to_end(stdout)),
            stderr: thread::spawn(move || read_to_end(stderr)),
        })
    }

    fn write(&mut self, input: &str) -> io::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::other("managed MCP stdin is closed"))?;
        stdin.write_all(input.as_bytes())?;
        stdin.flush()
    }

    fn finish(mut self) -> io::Result<support::binary_fixture::CapturedChildOutput> {
        self.stdin.take();
        let status = self.child.wait()?;
        Ok(support::binary_fixture::CapturedChildOutput {
            status,
            stdout: join_reader(self.stdout)?,
            stderr: join_reader(self.stderr)?,
        })
    }
}

fn read_to_end(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(reader: JoinHandle<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    reader
        .join()
        .map_err(|_| io::Error::other("managed MCP reader thread panicked"))?
}

fn assert_current_guard_projection(
    fixture: &OperationalFixture,
    manifest: &GuardManifest,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(manifest.required_hook_phases, GuardHookPhase::REQUIRED);
    for phase in GuardHookPhase::REQUIRED {
        let runtime = manifest.runtime_commands.get(phase);
        let hash_index = runtime
            .args
            .iter()
            .position(|arg| arg == "--policy-hash")
            .expect("runtime policy hash argument");
        assert_eq!(
            runtime.args.get(hash_index + 1).map(String::as_str),
            Some(manifest.policy_hash.as_str())
        );
    }
    let policy: Value =
        serde_json::from_slice(&fs::read(fixture.repo_root.join(".volicord/policy.json"))?)?;
    for command in policy["host_hook"]["commands"]
        .as_object()
        .expect("policy commands")
        .values()
    {
        assert!(command["args"]
            .as_array()
            .expect("policy args")
            .iter()
            .all(|arg| arg != "--policy-hash"));
    }
    for file in &manifest.managed_files {
        assert!(Path::new(&file.path).is_file(), "missing {}", file.path);
        if file.ownership == "managed_script" {
            assert_eq!(file.executable_required, Some(true));
        } else {
            assert_eq!(file.executable_required, None);
        }
    }
    assert_platform_script_permissions(manifest);
    Ok(())
}

#[cfg(unix)]
fn assert_platform_script_permissions(manifest: &GuardManifest) {
    use std::os::unix::fs::PermissionsExt;

    for file in manifest
        .managed_files
        .iter()
        .filter(|file| file.ownership == "managed_script")
    {
        let mode = fs::metadata(&file.path)
            .unwrap_or_else(|error| panic!("failed to inspect {}: {error}", file.path))
            .permissions()
            .mode();
        assert_ne!(mode & 0o100, 0, "script is not executable: {}", file.path);
    }
}

#[cfg(not(unix))]
fn assert_platform_script_permissions(_manifest: &GuardManifest) {}

fn assert_connection_report(
    output: &Output,
    expected_exit: i32,
    operation: &str,
    status: &str,
) -> Result<Value, Box<dyn Error>> {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "unexpected exit; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["operation"], operation);
    assert_eq!(report["status"], status);
    assert_compact_public_shape(&report);
    Ok(report)
}

fn assert_compact_public_shape(report: &Value) {
    let object = report.as_object().expect("connection report object");
    let mut expected = BTreeSet::from([
        "actions",
        "checks",
        "connection",
        "dry_run",
        "limits",
        "operation",
        "runtime_home",
        "status",
    ]);
    if report["operation"] == "init" {
        expected.insert("setup_applied");
    }
    if report["dry_run"] == true {
        expected.insert("planned_changes");
    }
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected
    );
    for forbidden in [
        "states",
        "verification_report",
        "host_hook",
        "summary_card",
        "receipt",
        "catalog",
        "primary_action",
    ] {
        assert!(
            !json_key_exists(report, forbidden),
            "unexpected {forbidden}"
        );
    }
    assert_eq!(report["limits"].as_array().map(Vec::len), Some(1));
}

fn assert_check(report: &Value, id: &str, status: &str, expected_code: Option<&str>) {
    let check = report["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == id)
        .unwrap_or_else(|| panic!("missing check {id}: {report}"));
    assert_eq!(check["status"], status, "unexpected check {id}: {check}");
    if let Some(expected) = expected_code {
        assert_eq!(
            check["code"], expected,
            "unexpected check code for {id}: {check}"
        );
    }
}

fn initialize_request(version: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "arbitrary-future-client", "version": version}
        }
    })
}

fn initialized_notification() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    })
}

fn tools_list_request() -> Value {
    json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
}

fn managed_tool_call(id: u64, name: &str, arguments: Value, session_id: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
            "_meta": {
                "threadId": NATIVE_THREAD,
                "x-codex-turn-metadata": {
                    "session_id": session_id,
                    "thread_id": NATIVE_THREAD,
                    "turn_id": format!("future.turn.{id}")
                }
            }
        }
    })
}

fn json_lines(messages: &[Value]) -> Result<String, serde_json::Error> {
    let mut input = String::new();
    for message in messages {
        input.push_str(&serde_json::to_string(message)?);
        input.push('\n');
    }
    Ok(input)
}

fn json_rpc_responses(bytes: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    String::from_utf8(bytes.to_vec())?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

fn directory_contents(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    fn visit(
        root: &Path,
        current: &Path,
        output: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        if !current.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                visit(root, &path, output)?;
            } else {
                output.insert(path.strip_prefix(root)?.to_path_buf(), fs::read(path)?);
            }
        }
        Ok(())
    }

    let mut output = BTreeMap::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn json_key_exists(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_key_exists(value, key))
        }
        Value::Array(values) => values.iter().any(|value| json_key_exists(value, key)),
        _ => false,
    }
}

fn copy_required_platform_environment(command: &mut Command) {
    #[cfg(target_os = "linux")]
    if fs::read_to_string("/proc/sys/kernel/osrelease").is_ok_and(|release| {
        let release = release.to_ascii_lowercase();
        release.contains("microsoft-standard") || release.contains("wsl2")
    }) {
        command.env(
            "WSL_DISTRO_NAME",
            volicord_types::PINNED_WSL2_DISTRIBUTION_NAME,
        );
    }
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR", "PATHEXT", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}
