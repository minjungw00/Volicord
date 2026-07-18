#![forbid(unsafe_code)]

mod support;

use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;
use support::binary_fixture::base_command;
use volicord_store::inspection::{
    inspect_runtime_home, DatabaseInspection, RegistryInspectionSnapshot,
};
use volicord_test_support::TempRuntimeHome;

const GENERATED_SHAPE_ERROR: &str =
    "generated host-hook capability does not match the current exact shape";

const ROOT_HELP: &str = "Local Volicord administration and managed stdio MCP

Usage: volicord
       volicord <COMMAND>

Commands:
  init         Initialize a Codex Record connection
  status       Show current project workflow status
  doctor       Inspect the local installation and managed integrations
  diagnostics  Read bounded local diagnostic data
  policy       Manage the authoritative project workflow policy
  connection   Manage Codex Agent Connections
  project      Manage registered Product Repositories
  mcp          Run managed stdio MCP or its preflight check
  export       Export local authority records
  changes      Reconcile observed product changes
  inbox        List or resolve pending UserAction requests
  evidence     Fulfill an authorized evidence-capture intent
  help         Print this message or the help of the given subcommand(s)

Options:
  -V, --version  Print the exact build identity
  -h, --help     Print help
";

const INIT_HELP: &str = concat!(
    "Initialize a Codex Record connection\n",
    "\n",
    "Usage: volicord init [OPTIONS] --host <HOST> --repo <REPO>\n",
    "\n",
    "Options:\n",
    "      --host <HOST>                [possible values: codex]\n",
    "      --repo <REPO>                \n",
    "      --shared                     \n",
    "      --profile <PROFILE>          [default: record] [possible values: record]\n",
    "      --home <HOME>                \n",
    "      --mcp-command <MCP_COMMAND>  \n",
    "      --dry-run                    \n",
    "      --json                       \n",
    "  -h, --help                       Print help\n",
);

fn run(args: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(args)
        .output()?)
}

fn stdout(output: &std::process::Output) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(output.stdout.clone())?)
}

fn stderr(output: &std::process::Output) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(output.stderr.clone())?)
}

#[test]
fn binary_help_exposes_current_codex_record_workflows() -> Result<(), Box<dyn Error>> {
    let output = run(&["--help"])?;
    assert!(output.status.success());
    assert_eq!(stdout(&output)?, ROOT_HELP);
    assert_eq!(stderr(&output)?, "");
    Ok(())
}

#[test]
fn init_help_is_codex_record_only() -> Result<(), Box<dyn Error>> {
    let output = run(&["init", "--help"])?;
    assert!(output.status.success());
    assert_eq!(stdout(&output)?, INIT_HELP);
    assert_eq!(stderr(&output)?, "");
    Ok(())
}

#[test]
fn connection_and_evidence_help_are_available_without_setup() -> Result<(), Box<dyn Error>> {
    for args in [
        &["connection", "--help"][..],
        &["connection", "status", "--help"][..],
        &["connection", "verify", "--help"][..],
        &["evidence", "--help"][..],
    ] {
        let output = run(args)?;
        assert!(
            output.status.success(),
            "{}",
            stderr(&output).unwrap_or_default()
        );
    }
    Ok(())
}

#[test]
fn unsupported_host_is_rejected_by_current_init_parser() -> Result<(), Box<dyn Error>> {
    let output = run(&[
        "init",
        "--host",
        "unsupported",
        "--repo",
        ".",
        "--profile",
        "record",
        "--dry-run",
    ])?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output)?, "");
    assert!(stderr(&output)?.contains("invalid value 'unsupported'"));
    Ok(())
}

#[test]
fn usage_errors_are_exit_two_and_stderr_only() -> Result<(), Box<dyn Error>> {
    for args in [
        &["status", "--not-a-real-option"][..],
        &["policy", "validate", "--file"][..],
    ] {
        let output = run(args)?;
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(stdout(&output)?, "");
        assert!(stderr(&output)?.starts_with("error:"));
    }
    Ok(())
}

#[test]
fn nested_policy_dispatch_preserves_runtime_failure_channels() -> Result<(), Box<dyn Error>> {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/missing-policy.json");
    let output = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(["policy", "validate", "--file"])
        .arg(missing)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output)?, "");
    assert!(stderr(&output)?.starts_with("error: POLICY_FILE_ACCESS_FAILED:"));
    Ok(())
}

#[test]
fn json_machine_output_is_one_stdout_document() -> Result<(), Box<dyn Error>> {
    let output = run(&["doctor", "--privacy-footprint", "--json"])?;
    assert!(output.status.success());
    assert_eq!(stderr(&output)?, "");

    let text = stdout(&output)?;
    let value: Value = serde_json::from_str(&text)?;
    assert_eq!(value["status"], "complete");
    assert!(value["privacy_footprint"].is_object());
    Ok(())
}

#[test]
fn failed_init_is_one_stdout_document_and_exit_one() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-init-failed")?;
    let output = fixture.run(false)?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(!text.contains(GENERATED_SHAPE_ERROR));
    let value: Value = serde_json::from_str(&text)?;
    assert_eq!(value["status"], "failed");
    assert_eq!(value["operation"], "init");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["setup_applied"], true);
    assert!(value.get("planned_changes").is_none());
    assert!(value["checks"].is_array());
    assert!(value["actions"].is_array());
    assert_eq!(value["limits"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[test]
fn dry_run_init_is_one_stdout_document_and_exit_zero() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-init-dry-run")?;
    let output = fixture.run(true)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(!text.contains(GENERATED_SHAPE_ERROR));
    let value: Value = serde_json::from_str(&text)?;
    assert_eq!(value["operation"], "init");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["setup_applied"], false);
    assert!(value["planned_changes"].is_array());
    Ok(())
}

#[test]
fn fresh_init_without_host_observation_is_action_required_and_exit_zero(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-init-awaiting-observation")?;
    fixture.install_codex_executable()?;
    let output = fixture.run(false)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let value: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(value["operation"], "init");
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["setup_applied"], true);
    assert!(value["checks"].as_array().is_some_and(|checks| {
        checks
            .iter()
            .any(|check| check["id"] == "host_session" && check["status"] == "pending")
    }));
    Ok(())
}

#[test]
fn connection_remove_after_fresh_init_removes_last_connection_state() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-remove-fresh-init")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let before = fixture.registry_snapshot();
    let connection_id = before.agent_connections[0].connection_internal_id.clone();
    let config_target = PathBuf::from(&before.agent_connections[0].config_target);
    assert_eq!(before.connection_projects.len(), 1);
    assert_eq!(before.guard_installations.len(), 1);

    let output = fixture.run_connection("remove", true)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(report["action"], "removed");
    assert_eq!(report["membership_removed"], true);
    assert_eq!(report["connection_removed"], true);
    assert_eq!(report["remaining_project_count"], 0);
    let after = fixture.registry_snapshot();
    assert!(after.agent_connections.is_empty());
    assert!(after.connection_projects.is_empty());
    assert!(after.guard_installations.is_empty());
    assert_eq!(
        registry_connection_row_count(&after.path, "mcp_runtime_sessions", &connection_id)?,
        0
    );
    assert!(!fs::read_to_string(config_target)
        .unwrap_or_default()
        .contains("mcp_servers.volicord"));
    Ok(())
}

#[test]
fn connection_remove_human_output_reports_complete_connection_removal() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-remove-human-last")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));

    let output = fixture.run_connection("remove", false)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(text.starts_with("Agent Connection removed for Codex\n"));
    assert!(text.contains("Membership: removed\n"));
    assert!(text.contains("Agent Connection: removed\n"));
    assert!(text.contains("Remaining repositories: 0\n"));
    Ok(())
}

#[test]
fn connection_remove_dry_run_has_no_registry_host_or_repository_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-remove-dry-run")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let registry_before = fixture.registry_snapshot();
    let runtime_before = directory_contents(&fixture.runtime_home)?;
    let host_before = directory_contents(&fixture.codex_home)?;
    let repository_before = directory_contents(&fixture.repo_root)?;

    let output = fixture.run_connection_with_options("remove", &fixture.repo_root, true, true)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(fixture.registry_snapshot(), registry_before);
    assert_eq!(directory_contents(&fixture.runtime_home)?, runtime_before);
    assert_eq!(directory_contents(&fixture.codex_home)?, host_before);
    assert_eq!(directory_contents(&fixture.repo_root)?, repository_before);
    Ok(())
}

#[test]
fn host_removal_conflict_preserves_registry_membership_and_child_state(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-remove-host-conflict")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let before = fixture.registry_snapshot();
    let connection_id = before.agent_connections[0].connection_internal_id.clone();
    let runtime_sessions_before =
        registry_connection_row_count(&before.path, "mcp_runtime_sessions", &connection_id)?;
    assert!(runtime_sessions_before > 0);
    let config_target = PathBuf::from(&before.agent_connections[0].config_target);
    fs::write(
        &config_target,
        "[mcp_servers.volicord]\ncommand = \"changed-by-user\"\n",
    )?;

    let output = fixture.run_connection("remove", true)?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output)?, "");
    assert!(stderr(&output)?.contains("changed since Volicord last managed it"));
    assert_eq!(fixture.registry_snapshot(), before);
    assert_eq!(
        registry_connection_row_count(
            &fixture.registry_snapshot().path,
            "mcp_runtime_sessions",
            &connection_id,
        )?,
        runtime_sessions_before
    );
    Ok(())
}

#[test]
fn registry_failure_after_host_removal_preserves_selectable_connection_state(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-remove-registry-failure")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let initial = fixture.registry_snapshot();
    let connection_id = initial.agent_connections[0].connection_internal_id.clone();
    let config_target = PathBuf::from(&initial.agent_connections[0].config_target);
    let mut metadata: Value = serde_json::from_str(&initial.agent_connections[0].metadata_json)?;
    metadata
        .as_object_mut()
        .ok_or("connection metadata should be an object")?
        .insert(
            "pending_host_cleanup".to_owned(),
            Value::String("blocked-test-inventory".to_owned()),
        );
    let metadata_json = serde_json::to_string(&metadata)?;
    let registry = rusqlite::Connection::open(&initial.path)?;
    registry.execute(
        "UPDATE agent_connections SET metadata_json = ?2 WHERE connection_internal_id = ?1",
        [connection_id.as_str(), metadata_json.as_str()],
    )?;
    drop(registry);
    let before = fixture.registry_snapshot();
    let runtime_sessions_before =
        registry_connection_row_count(&before.path, "mcp_runtime_sessions", &connection_id)?;

    let output = fixture.run_connection("remove", true)?;

    assert_eq!(output.status.code(), Some(1));
    let remove_stderr = stderr(&output)?;
    assert!(
        remove_stderr.contains("pending host cleanup"),
        "unexpected remove stderr: {remove_stderr}"
    );
    assert_eq!(fixture.registry_snapshot(), before);
    assert_eq!(
        registry_connection_row_count(&before.path, "mcp_runtime_sessions", &connection_id)?,
        runtime_sessions_before
    );
    assert!(!fs::read_to_string(config_target)
        .unwrap_or_default()
        .contains("mcp_servers.volicord"));
    let selectable = fixture.run_connection("status", true)?;
    let report: Value = serde_json::from_str(&stdout(&selectable)?)?;
    assert_eq!(report["connection"]["id"], connection_id);
    Ok(())
}

#[test]
fn absent_owned_host_entry_can_retry_registry_cleanup() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-remove-host-already-absent")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let before = fixture.registry_snapshot();
    let config_target = PathBuf::from(&before.agent_connections[0].config_target);
    fs::remove_file(config_target)?;

    let output = fixture.run_connection("remove", true)?;

    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(report["connection_removed"], true);
    let after = fixture.registry_snapshot();
    assert!(after.agent_connections.is_empty());
    assert!(after.connection_projects.is_empty());
    assert!(after.guard_installations.is_empty());
    Ok(())
}

#[test]
fn membership_only_remove_keeps_shared_host_configuration_and_reports_retention(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-remove-one-membership")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let other_repo = fixture.create_repository("product-repository-two")?;
    assert_eq!(
        fixture
            .run_connection_for_repo("add", &other_repo, true)?
            .status
            .code(),
        Some(0)
    );
    let before = fixture.registry_snapshot();
    assert_eq!(before.agent_connections.len(), 1);
    assert_eq!(before.connection_projects.len(), 2);
    let guard_installation_count = before.guard_installations.len();
    assert!(guard_installation_count >= 1);
    let config_target = PathBuf::from(&before.agent_connections[0].config_target);
    let host_before = fs::read(&config_target)?;

    let output = fixture.run_connection("remove", true)?;

    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(report["membership_removed"], true);
    assert_eq!(report["connection_removed"], false);
    assert_eq!(report["remaining_project_count"], 1);
    let after = fixture.registry_snapshot();
    assert_eq!(after.agent_connections.len(), 1);
    assert_eq!(after.connection_projects.len(), 1);
    assert_eq!(
        after.guard_installations.len(),
        guard_installation_count - 1
    );
    assert_eq!(fs::read(config_target)?, host_before);
    Ok(())
}

#[test]
fn membership_only_remove_human_output_reports_connection_retention() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-remove-one-membership-human")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let other_repo = fixture.create_repository("product-repository-two")?;
    assert_eq!(
        fixture
            .run_connection_for_repo("add", &other_repo, true)?
            .status
            .code(),
        Some(0)
    );

    let output = fixture.run_connection("remove", false)?;

    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output)?;
    assert!(text.starts_with("Repository membership removed for Codex\n"));
    assert!(text.contains("Agent Connection: retained\n"));
    assert!(text.contains("Remaining repositories: 1\n"));
    Ok(())
}

#[test]
fn failed_verify_json_is_one_stdout_document_and_empty_stderr() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-verify-failed-json")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let output = fixture.run_connection("verify", true)?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output)?, "");
    let value: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(value["operation"], "verify");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["status"], "failed");
    assert!(value.get("setup_applied").is_none());
    assert!(value.get("planned_changes").is_none());
    Ok(())
}

#[test]
fn failed_verify_human_report_is_written_to_stdout() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-verify-failed-text")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let output = fixture.run_connection("verify", false)?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(text.starts_with("Operation: verify\nStatus: failed\n"));
    assert!(text.contains("Checks:\n"));
    assert!(text.contains("Actions:\n"));
    Ok(())
}

struct IsolatedInitFixture {
    _temporary_root: TempRuntimeHome,
    runtime_home: std::path::PathBuf,
    codex_home: std::path::PathBuf,
    user_home: std::path::PathBuf,
    empty_path: std::path::PathBuf,
    repo_root: std::path::PathBuf,
}

impl IsolatedInitFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let temporary_root = TempRuntimeHome::new(prefix)?;
        let runtime_home = temporary_root.path().join("volicord-home");
        let codex_home = temporary_root.path().join("codex-home");
        let user_home = temporary_root.path().join("user-home");
        let empty_path = temporary_root.path().join("empty-path");
        let repo_root = temporary_root.path().join("product-repository");
        for directory in [
            &runtime_home,
            &codex_home,
            &user_home,
            &empty_path,
            &repo_root,
        ] {
            fs::create_dir_all(directory)?;
        }
        fs::create_dir(repo_root.join(".git"))?;
        Ok(Self {
            _temporary_root: temporary_root,
            runtime_home,
            codex_home,
            user_home,
            empty_path,
            repo_root,
        })
    }

    fn run(&self, dry_run: bool) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
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
            .arg("--json")
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env_remove("VOLICORD_CODEX_NATIVE_EXECUTABLE")
            .current_dir(&self.repo_root);
        if dry_run {
            command.arg("--dry-run");
        }
        Ok(command.output()?)
    }

    fn run_connection(
        &self,
        operation: &str,
        json: bool,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        self.run_connection_for_repo(operation, &self.repo_root, json)
    }

    fn run_connection_for_repo(
        &self,
        operation: &str,
        repo_root: &Path,
        json: bool,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        self.run_connection_with_options(operation, repo_root, json, false)
    }

    fn run_connection_with_options(
        &self,
        operation: &str,
        repo_root: &Path,
        json: bool,
        dry_run: bool,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("connection")
            .arg(operation)
            .arg("codex")
            .arg("--repo")
            .arg(repo_root)
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(repo_root);
        if json {
            command.arg("--json");
        }
        if dry_run {
            command.arg("--dry-run");
        }
        Ok(command.output()?)
    }

    fn create_repository(&self, name: &str) -> Result<PathBuf, Box<dyn Error>> {
        let repo_root = self._temporary_root.path().join(name);
        fs::create_dir_all(repo_root.join(".git"))?;
        Ok(repo_root)
    }

    fn registry_snapshot(&self) -> RegistryInspectionSnapshot {
        match inspect_runtime_home(&self.runtime_home).registry {
            DatabaseInspection::Present(snapshot) => snapshot,
            other => panic!("expected registry snapshot, got {other:?}"),
        }
    }

    fn install_codex_executable(&self) -> Result<(), Box<dyn Error>> {
        let filename = if cfg!(windows) { "codex.exe" } else { "codex" };
        fs::copy(
            env!("CARGO_BIN_EXE_volicord"),
            self.empty_path.join(filename),
        )?;
        Ok(())
    }
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

fn registry_connection_row_count(
    registry_path: &Path,
    table: &str,
    connection_internal_id: &str,
) -> Result<i64, Box<dyn Error>> {
    let conn = rusqlite::Connection::open(registry_path)?;
    Ok(conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE connection_internal_id = ?1"),
        [connection_internal_id],
        |row| row.get(0),
    )?)
}
