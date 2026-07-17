#![forbid(unsafe_code)]

use std::{error::Error, path::Path, process::Command};

use serde_json::Value;

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
