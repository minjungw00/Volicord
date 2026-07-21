#![forbid(unsafe_code)]

mod support;

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;
use support::binary_fixture::{base_command, prepare_runtime_home};
use volicord_store::{
    bootstrap::{
        initialize_runtime_home, write_installation_profile, InstallationProfileRegistration,
    },
    inspection::{inspect_runtime_home, DatabaseInspection, RegistryInspectionSnapshot},
    sqlite::registry_db_path,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::ConnectionVerificationReport;

const GENERATED_SHAPE_ERROR: &str =
    "generated host-hook capability does not match the current exact shape";
const CONNECTION_LIST_TEXT_HEADER: &str =
    "host\tintent\tmode\tenabled\tconnected_repositories\tverification_status\tissues\ttarget";

type SqliteMasterRow = (String, String, Option<String>);

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
    "      --verbose                    \n",
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
fn every_connection_subcommand_help_exposes_runtime_home_selection() -> Result<(), Box<dyn Error>> {
    for subcommand in ["add", "list", "status", "verify", "mode", "remove"] {
        let output = run(&["connection", subcommand, "--help"])?;
        assert!(output.status.success(), "{}", stderr(&output)?);
        assert!(
            stdout(&output)?.contains("--home <HOME>"),
            "{subcommand} help omitted --home"
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
    assert_eq!(value["result"]["kind"], "setup");
    assert_eq!(value["result"]["applied"], true);
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
    assert_eq!(value["result"]["kind"], "setup");
    assert_eq!(value["result"]["applied"], false);
    assert_eq!(value["connection"]["mode"], "workflow");
    let planned_changes = value["planned_changes"]
        .as_array()
        .expect("typed planned changes");
    assert!(!planned_changes.is_empty());
    for change in planned_changes {
        let change = change.as_object().expect("planned change object");
        assert_eq!(change.len(), 3);
        assert!(change["kind"].is_string());
        assert!(change["operation"].is_string());
        assert!(change["target"].is_string());
        assert_ne!(change["operation"], "noop");
        assert!(!change.contains_key("change"));
    }
    let mut kinds = planned_changes
        .iter()
        .map(|change| change["kind"].as_str().expect("planned kind"))
        .collect::<Vec<_>>();
    kinds.dedup();
    assert_eq!(
        kinds,
        vec![
            "connection_membership",
            "guard_managed_file",
            "guard_registry_setup",
            "managed_host_configuration",
            "project_registration",
            "runtime_home_initialization",
        ]
    );
    let triples = planned_changes
        .iter()
        .map(|change| {
            (
                change["kind"].as_str().unwrap().to_owned(),
                change["operation"].as_str().unwrap().to_owned(),
                change["target"].as_str().unwrap().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let mut canonical = triples.clone();
    canonical.sort();
    canonical.dedup();
    assert_eq!(triples, canonical);
    let action_ids = value["actions"]
        .as_array()
        .expect("typed actions")
        .iter()
        .map(|action| action["id"].as_str().expect("action kind"))
        .collect::<Vec<_>>();
    assert!(action_ids.contains(&"apply_setup"));
    assert!(action_ids.contains(&"observe_codex"));
    assert!(!fixture.runtime_home.join("registry.sqlite").exists());
    assert!(!fixture.codex_home.join("config.toml").exists());
    assert!(directory_contents(&fixture.repo_root)?.is_empty());
    Ok(())
}

#[test]
fn init_dry_run_is_read_only_for_every_initial_registry_state() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-init-dry-run-registry-states")?;
    let missing_home = fixture._temporary_root.path().join("missing-runtime-home");
    let absent_registry_home = fixture.runtime_home.clone();
    let zero_byte_home = fixture._temporary_root.path().join("zero-byte-home");
    fs::create_dir(&zero_byte_home)?;
    let zero_byte_registry = registry_db_path(&zero_byte_home);
    fs::write(&zero_byte_registry, [])?;
    let schema_less_home = fixture._temporary_root.path().join("schema-less-home");
    fs::create_dir(&schema_less_home)?;
    let schema_less_registry = registry_db_path(&schema_less_home);
    let schema_less = rusqlite::Connection::open(&schema_less_registry)?;
    schema_less.execute_batch("VACUUM")?;
    drop(schema_less);
    let schema_less_master_before = sqlite_master_rows(&schema_less_registry)?;
    assert!(schema_less_master_before.is_empty());
    let no_profile_home = fixture._temporary_root.path().join("no-profile-home");
    initialize_runtime_home(&no_profile_home, "runtime_home_without_profile", "{}")?;
    let no_profile_registry = registry_db_path(&no_profile_home);
    let current_profile_home = fixture._temporary_root.path().join("current-profile-home");
    initialize_runtime_home(
        &current_profile_home,
        "runtime_home_with_current_profile",
        "{}",
    )?;
    let current_binary = fs::canonicalize(env!("CARGO_BIN_EXE_volicord"))?;
    write_installation_profile(
        &current_profile_home,
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: path_text(&current_binary),
            volicord_mcp_command: path_text(&current_binary),
            bin_dir: current_binary
                .parent()
                .ok_or("test binary path has no parent")?
                .to_path_buf(),
            default_connection_mode: "workflow".to_owned(),
            metadata_json: r#"{"source":"binary-test"}"#.to_owned(),
        },
    )?;
    let current_profile_registry = registry_db_path(&current_profile_home);
    let fallback_home = fixture._temporary_root.path().join("fallback-home");

    let files_before = directory_contents(fixture._temporary_root.path())?;
    let entries_before = directory_entries(fixture._temporary_root.path())?;
    let zero_byte_modified_before = fs::metadata(&zero_byte_registry)?.modified()?;
    let schema_less_modified_before = fs::metadata(&schema_less_registry)?.modified()?;
    let no_profile_modified_before = fs::metadata(&no_profile_registry)?.modified()?;
    let current_profile_modified_before = fs::metadata(&current_profile_registry)?.modified()?;

    for runtime_home in [&missing_home, &absent_registry_home] {
        let output = fixture.run_init_dry_run_against_home(runtime_home, &fallback_home)?;
        let report = successful_init_dry_run_report(&output)?;
        assert!(report["planned_changes"]
            .as_array()
            .expect("planned changes")
            .iter()
            .any(|change| change["kind"] == "runtime_home_initialization"));
        assert_eq!(
            directory_contents(fixture._temporary_root.path())?,
            files_before
        );
        assert_eq!(
            directory_entries(fixture._temporary_root.path())?,
            entries_before
        );
    }
    assert!(!missing_home.exists());
    assert!(!registry_db_path(&absent_registry_home).exists());

    for runtime_home in [&zero_byte_home, &schema_less_home] {
        let output = fixture.run_init_dry_run_against_home(runtime_home, &fallback_home)?;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout(&output)?, "");
        let diagnostic = stderr(&output)?;
        assert!(
            diagnostic.contains("schema invariant failed for registry")
                || diagnostic.contains("sqlite error: no such table: runtime_home"),
            "unexpected invalid Registry diagnostic: {diagnostic}"
        );
        assert_eq!(
            directory_contents(fixture._temporary_root.path())?,
            files_before
        );
        assert_eq!(
            directory_entries(fixture._temporary_root.path())?,
            entries_before
        );
    }

    let no_profile = fixture.run_init_dry_run_against_home(&no_profile_home, &fallback_home)?;
    let no_profile_report = successful_init_dry_run_report(&no_profile)?;
    assert!(no_profile_report["planned_changes"]
        .as_array()
        .expect("planned changes")
        .iter()
        .any(|change| change["kind"] == "runtime_home_initialization"));
    assert_eq!(
        directory_contents(fixture._temporary_root.path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.path())?,
        entries_before
    );

    let current_profile =
        fixture.run_init_dry_run_against_home(&current_profile_home, &fallback_home)?;
    let current_profile_report = successful_init_dry_run_report(&current_profile)?;
    assert!(!current_profile_report["planned_changes"]
        .as_array()
        .expect("planned changes")
        .iter()
        .any(|change| change["kind"] == "runtime_home_initialization"));
    assert_eq!(
        directory_contents(fixture._temporary_root.path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.path())?,
        entries_before
    );

    assert_eq!(fs::read(&zero_byte_registry)?, Vec::<u8>::new());
    assert_eq!(fs::metadata(&zero_byte_registry)?.len(), 0);
    assert_eq!(
        fs::metadata(&zero_byte_registry)?.modified()?,
        zero_byte_modified_before
    );
    assert_eq!(
        sqlite_master_rows(&schema_less_registry)?,
        schema_less_master_before
    );
    assert_eq!(
        fs::metadata(&schema_less_registry)?.modified()?,
        schema_less_modified_before
    );
    assert_eq!(
        fs::metadata(&no_profile_registry)?.modified()?,
        no_profile_modified_before
    );
    assert_eq!(
        fs::metadata(&current_profile_registry)?.modified()?,
        current_profile_modified_before
    );
    assert!(!fallback_home.exists());
    assert!(!fixture.codex_home.join("config.toml").exists());
    assert!(directory_contents(&fixture.repo_root)?.is_empty());
    Ok(())
}

#[test]
fn connection_list_json_is_a_read_only_typed_inventory() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-json")?;
    let init = fixture.run(false)?;
    let init_report: Value = serde_json::from_slice(&init.stdout)?;
    assert_eq!(init_report["result"]["applied"], true);
    let files_before = directory_contents(fixture._temporary_root.path())?;
    let entries_before = directory_entries(fixture._temporary_root.path())?;

    let output = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        report
            .as_object()
            .expect("list report object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["connections", "limits"])
    );
    assert_eq!(report["limits"], init_report["limits"]);
    assert_eq!(report["limits"].as_array().map(Vec::len), Some(1));

    let connections = report["connections"].as_array().expect("connections");
    assert_eq!(connections.len(), 1);
    let entry = connections[0].as_object().expect("connection list entry");
    assert_eq!(
        entry
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "config_target",
            "connected_projects",
            "connected_repositories",
            "connection_id",
            "connection_intent",
            "enabled",
            "host_kind",
            "host_scope",
            "issues",
            "mode",
            "server_name",
            "verification_report",
        ])
    );
    serde_json::from_value::<ConnectionVerificationReport>(entry["verification_report"].clone())?;
    assert_eq!(entry["issues"], serde_json::json!([]));
    assert!(!json_key_exists(&report, "metadata_state"));
    assert!(!json_string_value_exists(&report, "current"));
    assert!(!json_string_value_exists(&report, "degraded"));

    let status = fixture.run_connection("status", true)?;
    let status_report: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(status_report["operation"], "status");
    assert_eq!(
        status_report["runtime_home"],
        path_text(&fixture.runtime_home)
    );

    assert_eq!(
        directory_contents(fixture._temporary_root.path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.path())?,
        entries_before
    );
    Ok(())
}

#[test]
fn explicit_runtime_homes_isolate_every_connection_command() -> Result<(), Box<dyn Error>> {
    let home_a = IsolatedInitFixture::new("binary-explicit-home-a")?;
    let home_b = IsolatedInitFixture::new("binary-explicit-home-b")?;
    let init_a: Value = serde_json::from_slice(&home_a.run(false)?.stdout)?;
    let init_b: Value = serde_json::from_slice(&home_b.run(false)?.stdout)?;
    let connection_a = init_a["connection"]["id"]
        .as_str()
        .expect("home A connection id")
        .to_owned();
    let connection_b = init_b["connection"]["id"]
        .as_str()
        .expect("home B connection id")
        .to_owned();
    assert_ne!(connection_a, connection_b);

    let a_before_reads = home_a.all_contents()?;
    let b_before_reads = home_b.all_contents()?;
    for (selected, decoy, expected_id) in [
        (&home_a, &home_b, connection_a.as_str()),
        (&home_b, &home_a, connection_b.as_str()),
    ] {
        let list = selected.run_connection_with_decoy_home("list", &[], &decoy.runtime_home)?;
        assert_eq!(list.status.code(), Some(0), "{}", stderr(&list)?);
        let list: Value = serde_json::from_slice(&list.stdout)?;
        assert_eq!(list["connections"].as_array().map(Vec::len), Some(1));
        assert_eq!(list["connections"][0]["connection_id"], expected_id);

        let status =
            selected.run_connection_with_decoy_home("status", &["codex"], &decoy.runtime_home)?;
        let status: Value = serde_json::from_slice(&status.stdout)?;
        assert_eq!(status["operation"], "status");
        assert_eq!(status["runtime_home"], path_text(&selected.runtime_home));
        assert_eq!(status["connection"]["id"], expected_id);
    }
    assert_eq!(home_a.all_contents()?, a_before_reads);
    assert_eq!(home_b.all_contents()?, b_before_reads);

    for (selected, decoy, expected_id) in [
        (&home_a, &home_b, connection_a.as_str()),
        (&home_b, &home_a, connection_b.as_str()),
    ] {
        let decoy_before = decoy.all_contents()?;
        let add =
            selected.run_connection_with_decoy_home("add", &["codex"], &decoy.runtime_home)?;
        let add: Value = serde_json::from_slice(&add.stdout)?;
        assert_eq!(add["operation"], "add");
        assert_eq!(add["runtime_home"], path_text(&selected.runtime_home));
        assert_eq!(add["connection"]["id"], expected_id);
        assert_eq!(decoy.all_contents()?, decoy_before);
    }

    for (selected, decoy, expected_id) in [
        (&home_a, &home_b, connection_a.as_str()),
        (&home_b, &home_a, connection_b.as_str()),
    ] {
        let decoy_before = decoy.all_contents()?;
        let verify =
            selected.run_connection_with_decoy_home("verify", &["codex"], &decoy.runtime_home)?;
        let verify: Value = serde_json::from_slice(&verify.stdout)?;
        assert_eq!(verify["operation"], "verify");
        assert_eq!(verify["runtime_home"], path_text(&selected.runtime_home));
        assert_eq!(verify["connection"]["id"], expected_id);
        assert_eq!(decoy.all_contents()?, decoy_before);
    }

    for (selected, decoy, expected_id) in [
        (&home_a, &home_b, connection_a.as_str()),
        (&home_b, &home_a, connection_b.as_str()),
    ] {
        let decoy_before = decoy.all_contents()?;
        let mode = selected.run_connection_with_decoy_home(
            "mode",
            &["codex", "read-only"],
            &decoy.runtime_home,
        )?;
        assert_eq!(mode.status.code(), Some(0), "{}", stderr(&mode)?);
        let mode: Value = serde_json::from_slice(&mode.stdout)?;
        assert_eq!(mode["operation"], "mode");
        assert_eq!(mode["runtime_home"], path_text(&selected.runtime_home));
        assert_eq!(mode["connection"]["id"], expected_id);
        assert_eq!(mode["connection"]["mode"], "read_only");
        assert_eq!(decoy.all_contents()?, decoy_before);
    }

    for (selected, decoy, expected_id) in [
        (&home_a, &home_b, connection_a.as_str()),
        (&home_b, &home_a, connection_b.as_str()),
    ] {
        let decoy_before = decoy.all_contents()?;
        let remove =
            selected.run_connection_with_decoy_home("remove", &["codex"], &decoy.runtime_home)?;
        assert_eq!(remove.status.code(), Some(0), "{}", stderr(&remove)?);
        let remove: Value = serde_json::from_slice(&remove.stdout)?;
        assert_eq!(remove["operation"], "remove");
        assert_eq!(remove["runtime_home"], path_text(&selected.runtime_home));
        assert_eq!(remove["connection"]["id"], expected_id);
        assert_eq!(remove["result"]["connection_removed"], true);
        assert_eq!(decoy.all_contents()?, decoy_before);
    }
    Ok(())
}

#[test]
fn custom_home_lifecycle_needs_no_environment_binding() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-custom-home-lifecycle")?;
    let init: Value = serde_json::from_slice(&fixture.run(false)?.stdout)?;
    let connection_id = init["connection"]["id"]
        .as_str()
        .expect("custom-home connection id");
    assert_eq!(init["runtime_home"], path_text(&fixture.runtime_home));

    let list: Value = serde_json::from_slice(&fixture.run_connection_list(None, true)?.stdout)?;
    assert_eq!(list["connections"].as_array().map(Vec::len), Some(1));
    assert_eq!(list["connections"][0]["connection_id"], connection_id);

    for operation in ["status", "verify", "add"] {
        let report: Value =
            serde_json::from_slice(&fixture.run_connection(operation, true)?.stdout)?;
        assert_eq!(report["operation"], operation);
        assert_eq!(report["runtime_home"], path_text(&fixture.runtime_home));
        assert_eq!(report["connection"]["id"], connection_id);
    }

    let mode: Value = serde_json::from_slice(&fixture.run_connection_mode("read-only")?.stdout)?;
    assert_eq!(mode["runtime_home"], path_text(&fixture.runtime_home));
    assert_eq!(mode["connection"]["id"], connection_id);
    assert_eq!(mode["connection"]["mode"], "read_only");

    let remove = fixture.run_connection("remove", true)?;
    assert_eq!(remove.status.code(), Some(0), "{}", stderr(&remove)?);
    let remove: Value = serde_json::from_slice(&remove.stdout)?;
    assert_eq!(remove["runtime_home"], path_text(&fixture.runtime_home));
    assert_eq!(remove["connection"]["id"], connection_id);
    assert_eq!(remove["result"]["connection_removed"], true);
    Ok(())
}

#[test]
fn relative_explicit_home_is_reported_as_the_selected_absolute_path() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-relative-explicit-home")?;
    fixture.run(false)?;
    let relative_home = fixture
        .runtime_home
        .strip_prefix(fixture._temporary_root.path())?;
    let output = base_command()
        .arg("connection")
        .arg("status")
        .arg("codex")
        .arg("--repo")
        .arg(&fixture.repo_root)
        .arg("--home")
        .arg(relative_home)
        .arg("--json")
        .env("PATH", &fixture.empty_path)
        .env("CODEX_HOME", &fixture.codex_home)
        .env("HOME", &fixture.user_home)
        .env("USERPROFILE", &fixture.user_home)
        .current_dir(fixture._temporary_root.path())
        .output()?;
    assert!(
        !output.stdout.is_empty(),
        "relative Runtime Home status produced no report: {}",
        stderr(&output)?
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["runtime_home"], path_text(&fixture.runtime_home));
    Ok(())
}

#[test]
fn every_connection_command_rejects_unusable_explicit_home_without_mutation_or_fallback(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-unusable-explicit-home")?;
    let init: Value = serde_json::from_slice(&fixture.run(false)?.stdout)?;
    let connection_id = init["connection"]["id"]
        .as_str()
        .expect("custom-home connection id");
    let missing_home = fixture._temporary_root.path().join("missing explicit home");
    let missing_registry_home = fixture
        ._temporary_root
        .path()
        .join("missing-registry-explicit-home");
    fs::create_dir(&missing_registry_home)?;
    let zero_byte_home = fixture
        ._temporary_root
        .path()
        .join("zero-byte-explicit-home");
    fs::create_dir(&zero_byte_home)?;
    let zero_byte_registry = registry_db_path(&zero_byte_home);
    fs::write(&zero_byte_registry, [])?;
    let empty_sqlite_home = fixture
        ._temporary_root
        .path()
        .join("empty-sqlite-explicit-home");
    fs::create_dir(&empty_sqlite_home)?;
    let empty_sqlite_registry = registry_db_path(&empty_sqlite_home);
    let empty_sqlite = rusqlite::Connection::open(&empty_sqlite_registry)?;
    empty_sqlite.execute_batch("VACUUM")?;
    drop(empty_sqlite);
    assert!(
        !fs::read(&empty_sqlite_registry)?.is_empty(),
        "VACUUM should materialize an empty valid SQLite database"
    );
    let no_profile_home = fixture
        ._temporary_root
        .path()
        .join("no-profile-explicit-home");
    initialize_runtime_home(&no_profile_home, "runtime_home_without_profile", "{}")?;

    let zero_byte_modified = fs::metadata(&zero_byte_registry)?.modified()?;
    let empty_sqlite_modified = fs::metadata(&empty_sqlite_registry)?.modified()?;
    let no_profile_registry = registry_db_path(&no_profile_home);
    let no_profile_modified = fs::metadata(&no_profile_registry)?.modified()?;
    let files_before = directory_contents(fixture._temporary_root.path())?;
    let entries_before = directory_entries(fixture._temporary_root.path())?;

    for (unusable_home, expected_code) in [
        (&missing_home, "RUNTIME_HOME_MISSING"),
        (&missing_registry_home, "SETUP_REQUIRED"),
        (&zero_byte_home, "SETUP_REQUIRED"),
        (&empty_sqlite_home, "SETUP_REQUIRED"),
        (&no_profile_home, "SETUP_REQUIRED"),
    ] {
        for operation in ["add", "list", "status", "verify", "mode", "remove"] {
            let output = fixture.run_connection_against_home(operation, unusable_home)?;

            assert_eq!(
                output.status.code(),
                Some(1),
                "{operation} unexpectedly succeeded for {}",
                unusable_home.display()
            );
            assert_eq!(stdout(&output)?, "");
            let diagnostic = stderr(&output)?;
            assert!(
                diagnostic.contains(expected_code),
                "{operation} diagnostic did not contain {expected_code}: {diagnostic}"
            );
            assert!(diagnostic.contains(&path_text(unusable_home)));
            assert!(diagnostic.contains("with `volicord init` using:"));
            assert!(
                diagnostic.contains("Select the host and repository when running `volicord init`.")
            );
            assert!(!diagnostic.contains("<host>"));
            assert!(!diagnostic.contains("<path>"));
            assert!(!diagnostic.contains("'\\''"));
            assert!(!diagnostic.contains(&format!("--home '{}'", unusable_home.display())));
            assert!(!diagnostic.contains(connection_id));
            assert_eq!(
                directory_contents(fixture._temporary_root.path())?,
                files_before,
                "{operation} changed filesystem bytes for {}",
                unusable_home.display()
            );
            assert_eq!(
                directory_entries(fixture._temporary_root.path())?,
                entries_before,
                "{operation} changed directory entries for {}",
                unusable_home.display()
            );
        }
    }
    assert!(!missing_home.exists());
    assert!(missing_registry_home.is_dir());
    assert_eq!(fs::read(&zero_byte_registry)?, Vec::<u8>::new());
    assert_eq!(fs::metadata(&zero_byte_registry)?.len(), 0);
    assert_eq!(
        fs::metadata(&zero_byte_registry)?.modified()?,
        zero_byte_modified
    );
    assert_eq!(
        fs::metadata(&empty_sqlite_registry)?.modified()?,
        empty_sqlite_modified
    );
    assert_eq!(
        fs::metadata(&no_profile_registry)?.modified()?,
        no_profile_modified
    );
    Ok(())
}

#[test]
fn selection_failure_names_the_selected_runtime_home_and_repository() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-explicit-home-selection-error")?;
    fixture.run(false)?;
    let unregistered_repo = fixture.create_repository("unregistered-product-repository")?;

    let output = base_command()
        .arg("connection")
        .arg("status")
        .arg("codex")
        .arg("--repo")
        .arg(&unregistered_repo)
        .arg("--home")
        .arg(&fixture.runtime_home)
        .arg("--json")
        .env("HOME", &fixture.user_home)
        .env("USERPROFILE", &fixture.user_home)
        .current_dir(&fixture.repo_root)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output)?, "");
    let diagnostic = stderr(&output)?;
    assert!(diagnostic.contains("PROJECT_NOT_REGISTERED"));
    assert!(diagnostic.contains(&path_text(&fixture.runtime_home)));
    assert!(diagnostic.contains(&path_text(&unregistered_repo)));
    Ok(())
}

#[test]
fn shell_neutral_connection_guidance_preserves_one_unsafe_custom_home() -> Result<(), Box<dyn Error>>
{
    let mut fixture = IsolatedInitFixture::new("binary-shell-neutral-guidance")?;
    fixture.runtime_home = fixture
        ._temporary_root
        .path()
        .join("Volicord Runtime Home's");
    fixture.repo_root = fixture._temporary_root.path().join("Product Repository's");
    fs::create_dir_all(&fixture.runtime_home)?;
    fs::create_dir_all(fixture.repo_root.join(".git"))?;

    let initialized = fixture.run_init_with_output(false, None)?;
    assert_eq!(initialized.status.code(), Some(1));
    assert_eq!(stderr(&initialized)?, "");
    let diagnostic_guidance = stdout(&initialized)?;
    assert!(
        diagnostic_guidance.contains(&format!("  Repository: {}\n", fixture.repo_root.display()))
    );
    assert!(diagnostic_guidance.contains(&format!(
        "  Runtime home: {}\n",
        fixture.runtime_home.display()
    )));
    assert!(diagnostic_guidance.contains("  Verbose output: required."));
    assert_shell_neutral_guidance(&diagnostic_guidance, &fixture.runtime_home);

    let direct_status = fixture.run_connection_verbose("status")?;
    assert_eq!(direct_status.status.code(), Some(1));
    assert_eq!(stderr(&direct_status)?, "");
    assert!(stdout(&direct_status)?.contains(&format!(
        "  Runtime home: {}\n",
        fixture.runtime_home.display()
    )));

    let unregistered_repo = fixture.create_repository("Unregistered Product Repository's")?;
    let selection = fixture.run_connection_for_repo("status", &unregistered_repo, false)?;
    assert_eq!(selection.status.code(), Some(1));
    assert_eq!(stdout(&selection)?, "");
    let selection_guidance = stderr(&selection)?;
    assert!(selection_guidance.contains("PROJECT_NOT_REGISTERED"));
    assert!(
        selection_guidance.contains(&format!("  Repository: {}\n", unregistered_repo.display()))
    );
    assert!(selection_guidance.contains(&format!(
        "  Runtime home: {}\n",
        fixture.runtime_home.display()
    )));
    assert_shell_neutral_guidance(&selection_guidance, &fixture.runtime_home);

    let registry = rusqlite::Connection::open(registry_db_path(&fixture.runtime_home))?;
    registry.execute("DELETE FROM guard_installations", [])?;
    drop(registry);
    let before_failure = fixture.registry_snapshot();
    let mode = fixture.run_connection_mode_human("read-only")?;
    assert_eq!(mode.status.code(), Some(1));
    assert_eq!(stdout(&mode)?, "");
    let mode_guidance = stderr(&mode)?;
    assert!(mode_guidance.contains("exactly one current Guard Installation"));
    assert!(mode_guidance.contains(&format!("  Repository: {}\n", fixture.repo_root.display())));
    assert!(mode_guidance.contains(&format!(
        "  Runtime home: {}\n",
        fixture.runtime_home.display()
    )));
    assert!(mode_guidance.contains("  Connection intent: personal\n"));
    assert!(mode_guidance.ends_with("  Profile: record.\n"));
    assert_shell_neutral_guidance(&mode_guidance, &fixture.runtime_home);
    let after_failure = fixture.registry_snapshot();
    assert_eq!(
        after_failure.agent_connections,
        before_failure.agent_connections
    );
    assert_eq!(
        after_failure.guard_installations,
        before_failure.guard_installations
    );
    Ok(())
}

fn assert_shell_neutral_guidance(output: &str, runtime_home: &Path) {
    assert!(output.contains(&path_text(runtime_home)));
    assert!(!output.contains("'\\''"));
    assert!(!output.contains(&format!("--home '{}';", runtime_home.display())));
    assert!(!output.contains(&format!("--home '{}'", runtime_home.display())));
}

#[test]
fn platform_default_does_not_discover_an_explicit_custom_home() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-custom-home-default-isolation")?;
    let init: Value = serde_json::from_slice(&fixture.run(false)?.stdout)?;
    let connection_id = init["connection"]["id"]
        .as_str()
        .expect("custom-home connection id");
    let default_home = fixture.user_home.join(".volicord");
    let custom_before = fixture.all_contents()?;

    let output = base_command()
        .arg("connection")
        .arg("list")
        .arg("--json")
        .env("HOME", &fixture.user_home)
        .env("USERPROFILE", &fixture.user_home)
        .current_dir(&fixture.repo_root)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output)?.contains(&path_text(&default_home)));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(connection_id));
    assert!(!default_home.exists());
    assert_eq!(fixture.all_contents()?, custom_before);
    Ok(())
}

#[test]
fn connection_list_synthesizes_a_missing_report_without_an_issue() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-missing-report")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();
    set_verification_report(&fixture, &connection_id, None)?;

    let output = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let entry = &report["connections"][0];
    assert_eq!(entry["verification_report"]["status"], "action_required");
    assert_eq!(
        entry["verification_report"]["checks"][0]["id"],
        "verification_not_run"
    );
    assert_eq!(entry["issues"], serde_json::json!([]));
    let stored = stored_verification_report(&fixture, &connection_id)?;
    assert!(
        stored.is_none(),
        "list must not persist the synthesized report"
    );
    Ok(())
}

#[test]
fn connection_list_reports_malformed_metadata_as_a_row_issue() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-metadata-issue")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();
    set_connection_metadata(&fixture, &connection_id, "{")?;

    let output = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let entry = &report["connections"][0];
    assert!(entry["verification_report"].is_object());
    assert_eq!(
        entry["issues"],
        serde_json::json!([{
            "kind": "metadata_corrupt",
            "summary": "Persisted Agent Connection registration metadata is corrupt."
        }])
    );
    assert!(!json_key_exists(&report, "metadata_state"));

    let verify = fixture.run_connection("verify", true)?;
    assert_eq!(verify.status.code(), Some(1));
    assert_eq!(stdout(&verify)?, "");
    assert!(stderr(&verify)?.contains("persisted_connection_metadata_corrupt"));

    let mode = fixture.run_connection_mode("read-only")?;
    assert_eq!(mode.status.code(), Some(1));
    assert_eq!(stdout(&mode)?, "");
    assert!(stderr(&mode)?.contains("metadata_json"));
    Ok(())
}

#[test]
fn connection_list_reports_malformed_verification_as_a_row_issue() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-verification-issue")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();
    set_verification_report(
        &fixture,
        &connection_id,
        Some(r#"{"status":"not_verified"}"#),
    )?;

    let output = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let entry = &report["connections"][0];
    assert!(entry["verification_report"].is_null());
    assert_eq!(
        entry["issues"],
        serde_json::json!([{
            "kind": "verification_report_corrupt",
            "summary": "Persisted Agent Connection verification report is corrupt."
        }])
    );
    Ok(())
}

#[test]
fn connection_verify_replaces_a_command_bearing_report_without_changing_connection_owners(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-verify-replaces-command-report")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let connection_id = fixture.only_connection_id();
    let before_connection = fixture.registry_snapshot().agent_connections[0].clone();

    let mut invalid: Value = serde_json::from_str(
        stored_verification_report(&fixture, &connection_id)?
            .as_deref()
            .expect("init persisted a verification report"),
    )?;
    invalid["actions"]
        .as_array_mut()
        .and_then(|actions| actions.first_mut())
        .and_then(Value::as_object_mut)
        .expect("failed init report has an action")
        .insert(
            "command".to_owned(),
            Value::String("volicord connection verify".to_owned()),
        );
    assert!(
        serde_json::from_value::<ConnectionVerificationReport>(invalid.clone()).is_err(),
        "the command-bearing stored shape must not decode as current"
    );
    let invalid = serde_json::to_string(&invalid)?;
    set_verification_report(&fixture, &connection_id, Some(&invalid))?;

    let output = fixture.run_connection("verify", true)?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output)?, "");
    let generated: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(generated["operation"], "verify");
    for action in generated["actions"].as_array().expect("generated actions") {
        assert_eq!(
            action
                .as_object()
                .expect("action object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["id", "instruction"])
        );
    }
    assert!(!serde_json::to_string(&generated)?.contains("volicord connection verify"));

    let stored = stored_verification_report(&fixture, &connection_id)?
        .expect("active verification replaced the report");
    serde_json::from_str::<ConnectionVerificationReport>(&stored)?;
    assert!(!stored.contains("volicord connection verify"));

    let after_connection = fixture.registry_snapshot().agent_connections[0].clone();
    let mut expected_connection = before_connection;
    expected_connection.verification_report_json =
        after_connection.verification_report_json.clone();
    expected_connection.updated_at = after_connection.updated_at.clone();
    assert_eq!(after_connection, expected_connection);
    Ok(())
}

#[test]
fn connection_list_orders_multiple_row_issues() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-multiple-issues")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();
    set_connection_metadata(&fixture, &connection_id, "[]")?;
    set_verification_report(&fixture, &connection_id, Some("{}"))?;

    let output = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let kinds = report["connections"][0]["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .map(|issue| issue["kind"].as_str().expect("issue kind"))
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec!["metadata_corrupt", "verification_report_corrupt"]
    );
    Ok(())
}

#[test]
fn connection_list_text_uses_issues_and_a_neutral_report_placeholder() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-connection-list-text")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();

    let current = fixture.run_connection_list(Some(&fixture.repo_root), false)?;
    assert_eq!(current.status.code(), Some(0));
    let current_text = stdout(&current)?;
    let mut current_lines = current_text.lines();
    assert_eq!(current_lines.next(), Some(CONNECTION_LIST_TEXT_HEADER));
    let current_columns = current_lines
        .next()
        .expect("connection row")
        .split('\t')
        .collect::<Vec<_>>();
    assert!(matches!(
        current_columns[5],
        "complete" | "action_required" | "failed"
    ));
    assert_eq!(current_columns[6], "-");

    set_connection_metadata(&fixture, &connection_id, "{")?;
    set_verification_report(&fixture, &connection_id, Some("{"))?;
    let corrupt = fixture.run_connection_list(Some(&fixture.repo_root), false)?;
    assert_eq!(corrupt.status.code(), Some(0));
    let corrupt_text = stdout(&corrupt)?;
    let mut corrupt_lines = corrupt_text.lines();
    assert_eq!(corrupt_lines.next(), Some(CONNECTION_LIST_TEXT_HEADER));
    let corrupt_columns = corrupt_lines
        .next()
        .expect("connection row")
        .split('\t')
        .collect::<Vec<_>>();
    assert_eq!(corrupt_columns[5], "-");
    assert_eq!(
        corrupt_columns[6],
        "metadata_corrupt,verification_report_corrupt"
    );
    Ok(())
}

#[test]
fn connection_list_filters_by_repository() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-filter")?;
    fixture.run(false)?;
    let other_repo = fixture.create_repository("other-list-repository")?;
    let shared = fixture.run_shared_connection_add(&other_repo)?;
    let shared_report: Value = serde_json::from_slice(&shared.stdout)?;
    assert_eq!(shared_report["result"]["applied"], true);

    let first = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(first.status.code(), Some(0));
    let first: Value = serde_json::from_slice(&first.stdout)?;
    assert_eq!(first["connections"].as_array().map(Vec::len), Some(1));
    assert_eq!(first["connections"][0]["connection_intent"], "personal");

    let second = fixture.run_connection_list(Some(&other_repo), true)?;
    assert_eq!(second.status.code(), Some(0));
    let second: Value = serde_json::from_slice(&second.stdout)?;
    assert_eq!(second["connections"].as_array().map(Vec::len), Some(1));
    assert_eq!(second["connections"][0]["connection_intent"], "shared");
    Ok(())
}

#[test]
fn connection_list_empty_inventory_and_store_failure_use_owned_channels(
) -> Result<(), Box<dyn Error>> {
    let temporary_root = TempRuntimeHome::new("binary-connection-list-channels")?;
    let repo_root = temporary_root.path().join("repo");
    fs::create_dir_all(repo_root.join(".git"))?;
    let empty_home = temporary_root.path().join("empty-home");
    prepare_runtime_home(&empty_home, Path::new(env!("CARGO_BIN_EXE_volicord")))?;
    let empty = run_connection_list_at(&empty_home, &repo_root)?;
    assert_eq!(
        empty.status.code(),
        Some(0),
        "unexpected empty-list stderr: {}",
        stderr(&empty)?
    );
    assert_eq!(stderr(&empty)?, "");
    let empty_report: Value = serde_json::from_slice(&empty.stdout)?;
    assert_eq!(empty_report["connections"], serde_json::json!([]));
    assert_eq!(empty_report["limits"].as_array().map(Vec::len), Some(1));

    let corrupt_home = temporary_root.path().join("corrupt-home");
    prepare_runtime_home(&corrupt_home, Path::new(env!("CARGO_BIN_EXE_volicord")))?;
    fs::write(
        volicord_store::sqlite::registry_db_path(&corrupt_home),
        b"not a sqlite database",
    )?;
    let failed = run_connection_list_at(&corrupt_home, &repo_root)?;
    assert_eq!(failed.status.code(), Some(1));
    assert_eq!(stdout(&failed)?, "");
    assert!(stderr(&failed)?.starts_with("error:"));
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
    assert_eq!(value["result"]["kind"], "setup");
    assert_eq!(value["result"]["applied"], true);
    assert!(value["checks"].as_array().is_some_and(|checks| {
        checks
            .iter()
            .any(|check| check["id"] == "host_session" && check["status"] == "pending")
    }));
    Ok(())
}

#[test]
fn default_init_uses_concise_human_output() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-init-concise")?;
    fixture.install_codex_executable()?;
    let output = fixture.run_init_with_output(false, None)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(text.starts_with("Volicord setup was applied and needs one more step.\n\n"));
    assert!(text.contains(&format!("Repository: {}\n", fixture.repo_root.display())));
    assert!(text.contains("Mode: workflow\nChecks: "));
    assert!(text.contains("Waiting\n"));
    assert!(text.contains("Next\n"));
    assert!(text.contains("volicord connection status codex --repo"));
    assert!(text.ends_with("for detailed current Connection diagnostics.\n"));
    for hidden in [
        "Operation:",
        "Runtime home:",
        "Config target:",
        "Details: {",
    ] {
        assert!(!text.contains(hidden));
    }
    Ok(())
}

#[test]
fn concise_follow_up_argument_vector_preserves_custom_runtime_home() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-concise-follow-up-custom-home")?;
    fixture.install_codex_executable()?;
    let init = fixture.run_init_with_output(false, None)?;
    assert_eq!(init.status.code(), Some(0), "{}", stderr(&init)?);
    assert!(stdout(&init)?.starts_with("Volicord setup was applied and needs one more step.\n\n"));

    let default_runtime_home = fixture.user_home.join(".volicord");
    assert!(!default_runtime_home.exists());
    let follow_up_arguments = vec![
        OsString::from("connection"),
        OsString::from("status"),
        OsString::from("codex"),
        OsString::from("--repo"),
        fixture.repo_root.as_os_str().to_owned(),
        OsString::from("--home"),
        fixture.runtime_home.as_os_str().to_owned(),
        OsString::from("--verbose"),
    ];
    let follow_up = base_command()
        .args(&follow_up_arguments)
        .env("VOLICORD_HOME", &default_runtime_home)
        .env("PATH", &fixture.empty_path)
        .env("CODEX_HOME", &fixture.codex_home)
        .env("HOME", &fixture.user_home)
        .env("USERPROFILE", &fixture.user_home)
        .current_dir(&fixture.repo_root)
        .output()?;

    assert_eq!(follow_up.status.code(), Some(0), "{}", stderr(&follow_up)?);
    assert_eq!(stderr(&follow_up)?, "");
    let text = stdout(&follow_up)?;
    assert!(text.contains("Connection\n"));
    assert!(text.contains(&format!("  Repository: {}\n", fixture.repo_root.display())));
    assert!(text.contains(&format!(
        "  Runtime home: {}\n",
        fixture.runtime_home.display()
    )));
    assert!(text.contains("Summary\n"));
    assert!(text.contains("Checks\n"));
    assert!(!default_runtime_home.exists());
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
    assert_eq!(report["operation"], "remove");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["result"]["kind"], "removal");
    assert_eq!(report["result"]["membership_removed"], true);
    assert_eq!(report["result"]["connection_removed"], true);
    assert_eq!(report["result"]["remaining_project_count"], 0);
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
    assert!(text.starts_with("Connection membership and Connection record were removed.\n\n"));
    assert!(text.contains("Mode: workflow\nChecks: 1 ready, 0 blocked, 0 waiting, 0 failed\n"));
    assert!(!text.contains("Result:"));
    assert!(!text.contains("Connection removed:"));
    assert!(!text.contains("--verbose"));
    assert!(!text.contains("connection status"));
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
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert!(report["checks"].as_array().is_some_and(|checks| checks
        .iter()
        .any(|check| check["id"] == "connection_removal")));
    assert!(report["actions"]
        .as_array()
        .is_some_and(|actions| actions.iter().any(|action| action["id"] == "apply_removal")));
    assert_eq!(fixture.registry_snapshot(), registry_before);
    assert_eq!(directory_contents(&fixture.runtime_home)?, runtime_before);
    assert_eq!(directory_contents(&fixture.codex_home)?, host_before);
    assert_eq!(directory_contents(&fixture.repo_root)?, repository_before);
    Ok(())
}

#[test]
fn connection_add_dry_run_preserves_setup_check_and_action_kinds() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-add-dry-run-kinds")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));
    let other_repo = fixture.create_repository("add-dry-run-repository")?;

    let output = fixture.run_connection_with_options("add", &other_repo, true, true)?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert!(report["checks"]
        .as_array()
        .is_some_and(|checks| checks.iter().any(|check| check["id"] == "setup_plan")));
    assert!(report["actions"]
        .as_array()
        .is_some_and(|actions| actions.iter().any(|action| action["id"] == "apply_setup")));
    Ok(())
}

#[test]
fn connection_add_operational_failure_is_one_stdout_document_and_exit_one(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-add-operational-failure")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let other_repo = fixture.create_repository("add-failed-repository")?;

    let output = fixture.run_connection_for_repo("add", &other_repo, true)?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(report["operation"], "add");
    assert_eq!(report["dry_run"], false);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["result"]["kind"], "setup");
    assert_eq!(report["result"]["applied"], true);
    assert!(report["checks"].is_array());
    assert!(report["actions"].is_array());
    Ok(())
}

#[test]
fn connection_mode_preserves_transition_check_and_reload_action_kinds() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-mode-kinds")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));

    let output = fixture.run_connection_mode("read-only")?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(report["checks"][0]["id"], "mode_transition");
    assert_eq!(report["actions"][0]["id"], "reload_host");
    Ok(())
}

#[test]
fn connection_mode_human_output_does_not_offer_to_replay_the_transition(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-mode-human-no-replay")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));

    let output = fixture.run_connection_mode_human("read-only")?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(text.starts_with("Connection mode changed from workflow to read_only.\n\n"));
    assert!(text.contains("Restart or reload Codex, then use the current Volicord integration\n"));
    assert!(!text.contains("--verbose"));
    assert!(!text.contains("connection status"));
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
    assert_eq!(report["result"]["connection_removed"], true);
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
    assert_eq!(report["result"]["membership_removed"], true);
    assert_eq!(report["result"]["connection_removed"], false);
    assert_eq!(report["result"]["remaining_project_count"], 1);
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
    assert!(text.starts_with(
        "Connection membership was removed; the shared Connection remains in use.\n\n"
    ));
    assert!(text.contains("Mode: workflow\nChecks: 1 ready, 0 blocked, 0 waiting, 0 failed\n"));
    assert!(!text.contains("Result:"));
    assert!(!text.contains("Remaining project count:"));
    assert!(!text.contains("--verbose"));
    assert!(!text.contains("connection status"));
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
    assert!(value.get("result").is_none());
    assert!(value.get("planned_changes").is_none());
    for action in value["actions"].as_array().expect("verification actions") {
        assert_eq!(
            action
                .as_object()
                .expect("action object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["id", "instruction"])
        );
    }
    assert!(!stdout(&output)?.contains("volicord connection verify"));

    let connection_id = fixture.only_connection_id();
    let stored = stored_verification_report(&fixture, &connection_id)?
        .expect("verification report was persisted");
    serde_json::from_str::<ConnectionVerificationReport>(&stored)?;
    assert!(!stored.contains("volicord connection verify"));
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
    assert!(text.starts_with("Verification completed:"));
    assert!(text.contains(" failed.\n\n"));
    assert!(text.contains("Problems\n"));
    assert!(text.contains("Next\n"));
    if text.contains("`volicord connection verify") {
        assert!(text.contains(" codex --repo "));
        assert!(text.contains(&format!(
            " --home {} --verbose`",
            fixture.runtime_home.display()
        )));
    } else {
        assert!(text.contains("Host: codex"));
        assert!(text.contains(&format!("Repository: {}", fixture.repo_root.display())));
        assert!(text.contains(&format!("Runtime home: {}", fixture.runtime_home.display())));
        assert!(text.contains("Verbose output: required."));
    }
    assert!(!text.contains("Operation:"));
    assert!(!text.contains("Details: {"));
    Ok(())
}

#[test]
fn verbose_connection_report_retains_the_full_diagnostic_renderer() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-verify-verbose")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let output = fixture.run_connection_verbose("verify")?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output)?, "");
    let text = stdout(&output)?;
    assert!(text.starts_with("Verification completed:"));
    assert!(text.contains("\n\nConnection\n  ID:"));
    assert!(text.contains("  Runtime home:"));
    assert!(text.contains("\n\nSummary\n  Status: failed\n"));
    assert!(text.contains("\n\nChecks\n"));
    assert!(text.contains("\n\nActions\n"));
    assert!(text.contains("\n\nAssurance\n"));
    assert!(!text.contains("Command:"));
    assert!(!text.contains("volicord connection verify"));
    assert!(!text.contains("Details: {"));
    assert!(!text.contains("\":["));
    assert!(text.ends_with('\n'));
    assert!(!text.ends_with("\n\n"));
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
        self.run_init_with_output(dry_run, Some("--json"))
    }

    fn run_init_with_output(
        &self,
        dry_run: bool,
        output_flag: Option<&str>,
    ) -> Result<std::process::Output, Box<dyn Error>> {
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
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env_remove("VOLICORD_CODEX_NATIVE_EXECUTABLE")
            .current_dir(&self.repo_root);
        if let Some(output_flag) = output_flag {
            command.arg(output_flag);
        }
        if dry_run {
            command.arg("--dry-run");
        }
        Ok(command.output()?)
    }

    fn run_init_dry_run_against_home(
        &self,
        runtime_home: &Path,
        fallback_home: &Path,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        Ok(base_command()
            .arg("init")
            .arg("--host")
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--profile")
            .arg("record")
            .arg("--home")
            .arg(runtime_home)
            .arg("--dry-run")
            .arg("--json")
            .env("VOLICORD_HOME", fallback_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .env_remove("VOLICORD_CODEX_NATIVE_EXECUTABLE")
            .current_dir(&self.repo_root)
            .output()?)
    }

    fn run_connection(
        &self,
        operation: &str,
        json: bool,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        self.run_connection_for_repo(operation, &self.repo_root, json)
    }

    fn run_connection_verbose(
        &self,
        operation: &str,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("connection")
            .arg(operation)
            .arg("codex")
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--verbose")
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        Ok(command.output()?)
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
            .arg("--home")
            .arg(&self.runtime_home)
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

    fn run_connection_mode(&self, mode: &str) -> Result<std::process::Output, Box<dyn Error>> {
        Ok(base_command()
            .arg("connection")
            .arg("mode")
            .arg("codex")
            .arg(mode)
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--json")
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root)
            .output()?)
    }

    fn run_connection_mode_human(
        &self,
        mode: &str,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        Ok(base_command()
            .arg("connection")
            .arg("mode")
            .arg("codex")
            .arg(mode)
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--home")
            .arg(&self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root)
            .output()?)
    }

    fn run_connection_list(
        &self,
        repo_root: Option<&Path>,
        json: bool,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("connection")
            .arg("list")
            .arg("--home")
            .arg(&self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        if let Some(repo_root) = repo_root {
            command.arg("--repo").arg(repo_root);
        }
        if json {
            command.arg("--json");
        }
        Ok(command.output()?)
    }

    fn run_shared_connection_add(
        &self,
        repo_root: &Path,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        Ok(base_command()
            .arg("connection")
            .arg("add")
            .arg("codex")
            .arg("--repo")
            .arg(repo_root)
            .arg("--shared")
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--json")
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(repo_root)
            .output()?)
    }

    fn run_connection_with_decoy_home(
        &self,
        operation: &str,
        positionals: &[&str],
        decoy_runtime_home: &Path,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("connection")
            .arg(operation)
            .args(positionals)
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--json")
            .env("VOLICORD_HOME", decoy_runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        Ok(command.output()?)
    }

    fn run_connection_against_home(
        &self,
        operation: &str,
        runtime_home: &Path,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command.arg("connection").arg(operation);
        match operation {
            "list" => {}
            "mode" => {
                command.arg("codex").arg("read-only");
            }
            _ => {
                command.arg("codex");
            }
        }
        command
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--home")
            .arg(runtime_home)
            .arg("--json")
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        Ok(command.output()?)
    }

    fn all_contents(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        directory_contents(self._temporary_root.path())
    }

    fn only_connection_id(&self) -> String {
        let snapshot = self.registry_snapshot();
        assert_eq!(snapshot.agent_connections.len(), 1);
        snapshot.agent_connections[0].connection_internal_id.clone()
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

fn directory_entries(root: &Path) -> Result<BTreeSet<PathBuf>, Box<dyn Error>> {
    fn visit(
        root: &Path,
        current: &Path,
        output: &mut BTreeSet<PathBuf>,
    ) -> Result<(), Box<dyn Error>> {
        if !current.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            output.insert(path.strip_prefix(root)?.to_path_buf());
            if entry.file_type()?.is_dir() {
                visit(root, &path, output)?;
            }
        }
        Ok(())
    }

    let mut output = BTreeSet::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn sqlite_master_rows(path: &Path) -> Result<Vec<SqliteMasterRow>, Box<dyn Error>> {
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let mut stmt = conn.prepare(
        "SELECT type, name, sql
           FROM sqlite_master
          ORDER BY type, name",
    )?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn successful_init_dry_run_report(output: &std::process::Output) -> Result<Value, Box<dyn Error>> {
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(output)?, "");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["operation"], "init");
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["result"]["applied"], false);
    Ok(report)
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
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

fn set_connection_metadata(
    fixture: &IsolatedInitFixture,
    connection_id: &str,
    metadata_json: &str,
) -> Result<(), Box<dyn Error>> {
    rusqlite::Connection::open(volicord_store::sqlite::registry_db_path(
        &fixture.runtime_home,
    ))?
    .execute(
        "UPDATE agent_connections SET metadata_json = ?2 WHERE connection_internal_id = ?1",
        [connection_id, metadata_json],
    )?;
    Ok(())
}

fn set_verification_report(
    fixture: &IsolatedInitFixture,
    connection_id: &str,
    verification_report_json: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    rusqlite::Connection::open(volicord_store::sqlite::registry_db_path(&fixture.runtime_home))?
        .execute(
            "UPDATE agent_connections SET verification_report_json = ?2 WHERE connection_internal_id = ?1",
            rusqlite::params![connection_id, verification_report_json],
        )?;
    Ok(())
}

fn stored_verification_report(
    fixture: &IsolatedInitFixture,
    connection_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let connection = rusqlite::Connection::open(volicord_store::sqlite::registry_db_path(
        &fixture.runtime_home,
    ))?;
    Ok(connection.query_row(
        "SELECT verification_report_json FROM agent_connections WHERE connection_internal_id = ?1",
        [connection_id],
        |row| row.get(0),
    )?)
}

fn run_connection_list_at(
    runtime_home: &Path,
    current_dir: &Path,
) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(base_command()
        .arg("connection")
        .arg("list")
        .arg("--json")
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .output()?)
}

fn json_key_exists(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_key_exists(value, key))
        }
        Value::Array(values) => values.iter().any(|value| json_key_exists(value, key)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

fn json_string_value_exists(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(value) => value == expected,
        Value::Array(values) => values
            .iter()
            .any(|value| json_string_value_exists(value, expected)),
        Value::Object(object) => object
            .values()
            .any(|value| json_string_value_exists(value, expected)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}
