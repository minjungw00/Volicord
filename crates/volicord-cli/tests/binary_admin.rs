#![forbid(unsafe_code)]

mod support;

use std::{error::Error, fs, path::Path, process::Command};

use serde_json::Value;
use support::binary_fixture::base_command;
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
        let mut command = base_command();
        command
            .arg("connection")
            .arg(operation)
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root)
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        if json {
            command.arg("--json");
        }
        Ok(command.output()?)
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
