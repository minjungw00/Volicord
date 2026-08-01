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
        initialize_runtime_home, register_project, write_installation_profile,
        InstallationProfileRegistration, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    diagnostic_findings::insert_occurrence_finding,
    inspection::{inspect_runtime_home, DatabaseInspection, RegistryInspectionSnapshot},
    schema::current_storage_manifest,
    sqlite::registry_db_path,
};
use volicord_test_support::{
    core_fixtures::CoreFixture, with_test_runtime_home_setup, TempRuntimeHome,
    TestRuntimeHomeMutation,
};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::connection_verification::ConnectionVerificationReport;
use volicord_types::diagnostics::{
    DiagnosticCode, DiagnosticDomain, DiagnosticFacts, DiagnosticFindingData, DiagnosticSeverity,
    DiagnosticSource, DiagnosticStage, DiagnosticSubject, OccurrenceDiagnosticFinding,
};
use volicord_types::values::UtcTimestamp;
use volicord_types::workflow_policy::{
    ManagedPolicyFileStatus, PolicyShowActionCommand, PolicyShowReport, PolicyShowReportSchema,
    PolicyShowStatus, PolicyValidationReport, PolicyValidationStatus, ProjectWorkflowPolicy,
    WorkflowPolicySchema,
};

const GENERATED_SHAPE_ERROR: &str =
    "generated host-hook capability does not match the current exact shape";

type SqliteMasterRow = (String, String, Option<String>);

fn assert_current_activation_step_shape(step: &Value) {
    let step = step.as_object().expect("typed activation step object");
    assert_eq!(
        step.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "agent_sequence",
            "completes_checks",
            "diagnostic_only",
            "execution_channel",
            "executor",
            "id",
            "initiator",
            "instruction",
            "prerequisites",
            "root_finding_ids",
        ])
    );
    assert!(matches!(
        step["id"].as_str(),
        Some(
            "reload_codex"
                | "review_project_hooks"
                | "request_integration_verification"
                | "read_connection_status"
                | "run_optional_active_diagnostics"
                | "repair_hook_contract"
                | "repair_managed_configuration"
        )
    ));
}

const ROOT_HELP: &str = "Local Volicord administration and managed stdio MCP

Usage: volicord
       volicord <COMMAND>

Commands:
  version      Show the Volicord version and build provenance
  init         Initialize a Codex Record connection
  status       Show current project workflow status
  doctor       Inspect the local installation and managed integrations
  diagnostics  Read bounded local diagnostic data
  policy       Manage the authoritative project workflow policy
  connection   Manage Codex Agent Connections
  project      Manage registered Product Repositories
  mcp          Inspect or manually serve the local stdio MCP adapter
  export       Export local authority records
  changes      Reconcile Unrecorded Changes in the Product Repository
  inbox        List or resolve pending UserAction requests
  evidence     Fulfill an authorized evidence-capture intent
  help         Print this message or the help of the given subcommand(s)

Options:
  -V, --version  Print the Volicord version
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

fn run_diagnostics_show(
    runtime_home: &Path,
    finding_id: &str,
) -> Result<std::process::Output, Box<dyn Error>> {
    let mut command = base_command();
    command
        .args(["diagnostics", "show", finding_id, "--json"])
        .env("VOLICORD_HOME", runtime_home);
    Ok(command.output()?)
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
fn root_version_options_print_exact_concise_product_identity() -> Result<(), Box<dyn Error>> {
    let long = run(&["--version"])?;
    let short = run(&["-V"])?;
    let expected = format!("volicord {}\n", env!("CARGO_PKG_VERSION"));

    assert!(long.status.success());
    assert!(short.status.success());
    assert_eq!(stdout(&long)?, expected);
    assert_eq!(stdout(&short)?, expected);
    assert_eq!(stderr(&long)?, "");
    assert_eq!(stderr(&short)?, "");
    Ok(())
}

#[test]
fn explicit_version_command_supports_concise_verbose_and_json_reports() -> Result<(), Box<dyn Error>>
{
    let concise = run(&["version"])?;
    assert!(concise.status.success());
    assert_eq!(
        stdout(&concise)?,
        format!("volicord {}\n", env!("CARGO_PKG_VERSION"))
    );

    let verbose = run(&["version", "--verbose"])?;
    assert!(verbose.status.success());
    let verbose_text = stdout(&verbose)?;
    assert!(verbose_text.starts_with(&format!(
        "Volicord {}\n\nSource\n",
        env!("CARGO_PKG_VERSION")
    )));
    for field in [
        "  Commit: ",
        "  Tree: ",
        "  Metadata source: ",
        "\nBuild\n",
        "  Target: ",
        "  Profile class: ",
        "  Profile precision: ",
        "  Exact Cargo profile: ",
        "  Optimization: ",
        "  Debug assertions: ",
    ] {
        assert!(
            verbose_text.contains(field),
            "missing verbose field {field}"
        );
    }

    let json_output = run(&["version", "--json"])?;
    assert!(json_output.status.success());
    let report: Value = serde_json::from_slice(&json_output.stdout)?;
    assert_eq!(report["product_name"], "Volicord");
    assert_eq!(report["package_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        report["build"]["package_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(report["build"]["build_id"].is_string());
    assert!(matches!(
        report["build"]["profile_precision"].as_str(),
        Some("exact" | "class_only")
    ));
    assert_eq!(
        report
            .as_object()
            .expect("version report object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["build", "package_version", "product_name"])
    );
    Ok(())
}

#[test]
fn doctor_and_version_json_share_one_build_projection() -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("binary-build-projection")?;
    let version = run(&["version", "--json"])?;
    let version: Value = serde_json::from_slice(&version.stdout)?;

    let mut doctor_command = base_command();
    let doctor = doctor_command
        .args(["doctor", "--json"])
        .env("VOLICORD_HOME", fixture.path())
        .output()?;
    let doctor: Value = serde_json::from_slice(&doctor.stdout)?;

    assert_eq!(doctor["build"], version["build"]);
    assert!(doctor["build"]["build_id"].is_string());
    assert!(!doctor
        .as_object()
        .expect("doctor report object")
        .contains_key("build_id"));
    assert!(doctor["checks"].is_array());
    assert!(!doctor["states"]
        .as_object()
        .expect("doctor states object")
        .contains_key("build_id"));
    Ok(())
}

#[test]
fn doctor_storage_profile_is_structured_read_only_and_terminal_clean() -> Result<(), Box<dyn Error>>
{
    let fixture = TempRuntimeHome::new("binary-doctor-storage-profile")?;
    let runtime_home = fixture.root_path().join("runtime-home");
    prepare_runtime_home(&runtime_home, Path::new(env!("CARGO_BIN_EXE_volicord")))?;
    let state_before = directory_state(fixture.root_path())?;
    let run_doctor = |mode: Option<&str>| -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command.arg("doctor").env("VOLICORD_HOME", &runtime_home);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        Ok(command.output()?)
    };

    let compact_output = run_doctor(None)?;
    assert!(matches!(compact_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&compact_output)?, "");

    let json_output = run_doctor(Some("--json"))?;
    assert!(matches!(json_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&json_output)?, "");
    let report: Value = serde_json::from_slice(&json_output.stdout)?;
    let registry = report["checks"]
        .as_array()
        .expect("Doctor checks")
        .iter()
        .find(|check| check["id"] == "registry_schema")
        .expect("registry schema check");
    let storage_profile = &registry["details"]["storage_profile"];
    assert!(storage_profile.is_object());
    assert!(!storage_profile.is_string());
    assert_eq!(
        storage_profile,
        &serde_json::to_value(current_storage_manifest()?)?
    );

    let verbose_output = run_doctor(Some("--verbose"))?;
    assert!(matches!(verbose_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&verbose_output)?, "");
    let verbose = stdout(&verbose_output)?;
    let manifest = current_storage_manifest()?;
    let mut previous_group = 0;
    for group in [
        "Runtime and build",
        "Integration control",
        "Guard and Hook state",
        "Project integration",
        "Command availability",
        "Inventory and optional diagnostics",
    ] {
        assert_eq!(
            verbose.lines().filter(|line| line.trim() == group).count(),
            1,
            "Doctor verbose output must render one {group} group: {verbose}"
        );
        let position = verbose.find(group).expect("Doctor semantic group");
        assert!(position >= previous_group, "{verbose}");
        previous_group = position;
    }
    for raw_check_id in [
        "build_identity",
        "guard_files",
        "volicord_command",
        "volicord_mcp_command",
        "path_or_shim",
        "host_detection",
    ] {
        assert!(
            !verbose.lines().any(|line| line.trim() == raw_check_id),
            "registered raw check ID must not be a heading: {verbose}"
        );
    }
    for expected in [
        "Path: ",
        "Storage contract: volicord.sqlite.canonical",
        "Canonical DDL digest: sha256:",
        "Enabled capabilities",
        "Integrity constraints digest: sha256:",
    ] {
        assert!(verbose.contains(expected), "{verbose}");
    }
    for capability in &manifest.enabled_capabilities {
        assert!(verbose.contains(&format!("- {capability}")), "{verbose}");
    }
    assert!(!verbose.contains("storage_profile: {"));
    assert!(!verbose.contains("\\\"contract_id\\\""));

    for output in [&compact_output, &json_output, &verbose_output] {
        let text = std::str::from_utf8(&output.stdout)?;
        assert_eq!(
            text.len() - text.trim_end_matches('\n').len(),
            1,
            "Doctor output must have exactly one trailing newline"
        );
    }
    assert_eq!(directory_state(fixture.root_path())?, state_before);

    rusqlite::Connection::open(registry_db_path(&runtime_home))?.execute(
        "UPDATE runtime_home SET storage_profile = 'not-json' WHERE singleton_id = 1",
        [],
    )?;
    let malformed_state_before = directory_state(fixture.root_path())?;
    let malformed_output = run_doctor(Some("--json"))?;
    assert!(matches!(malformed_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&malformed_output)?, "");
    let malformed_text = stdout(&malformed_output)?;
    let malformed: Value = serde_json::from_str(&malformed_text)?;
    assert!(malformed["checks"]
        .as_array()
        .expect("malformed Doctor checks")
        .iter()
        .any(|check| check["id"] == "registry" && check["status"] == "failed"));
    assert!(!malformed["checks"]
        .as_array()
        .expect("malformed Doctor checks")
        .iter()
        .any(|check| check["id"] == "registry_schema"));
    assert!(!json_value_contains_key(&malformed, "storage_profile"));
    assert!(!malformed_text.contains("not-json"));
    assert_eq!(
        malformed_text.len() - malformed_text.trim_end_matches('\n').len(),
        1
    );
    assert_eq!(
        directory_state(fixture.root_path())?,
        malformed_state_before
    );
    Ok(())
}

#[test]
fn doctor_remediation_projections_share_one_finalized_plan() -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("binary-doctor-remediation-plan")?;
    let run_doctor = |mode: Option<&str>| -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command.arg("doctor").env("VOLICORD_HOME", fixture.path());
        if let Some(mode) = mode {
            command.arg(mode);
        }
        Ok(command.output()?)
    };

    let json_output = run_doctor(Some("--json"))?;
    assert!(matches!(json_output.status.code(), Some(0 | 1)));
    assert!(json_output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&json_output.stdout)?;
    let codes = |field: &str| {
        report[field]
            .as_array()
            .expect("Doctor action collection")
            .iter()
            .map(|action| {
                action["code"]
                    .as_str()
                    .expect("Doctor action code")
                    .to_owned()
            })
            .collect::<BTreeSet<_>>()
    };
    let actions = codes("actions");
    let required = codes("actions_required");
    let recommended = codes("actions_recommended");
    assert_eq!(
        report["actions"].as_array().expect("Doctor actions").len(),
        actions.len()
    );
    assert_eq!(
        report["actions_required"]
            .as_array()
            .expect("required Doctor actions")
            .len(),
        required.len()
    );
    assert_eq!(
        report["actions_recommended"]
            .as_array()
            .expect("recommended Doctor actions")
            .len(),
        recommended.len()
    );
    assert_eq!(
        actions,
        required
            .union(&recommended)
            .cloned()
            .collect::<BTreeSet<_>>()
    );
    assert!(required.is_disjoint(&recommended));

    let finding_actions = report["findings"]
        .as_array()
        .expect("Doctor findings")
        .iter()
        .flat_map(|finding| {
            finding["actions"]
                .as_array()
                .expect("finding actions")
                .iter()
        })
        .map(|action| {
            action["code"]
                .as_str()
                .expect("finding action code")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert!(finding_actions.is_subset(&actions));
    assert!(finding_actions.contains("action.runtime_home.initialize_registry"));
    assert!(!actions.contains("action.setup.initialize_connection"));

    let primary = report["primary_next_action"]
        .as_object()
        .expect("primary Doctor action");
    let primary_code = primary["code"].as_str().expect("primary action code");
    let primary_summary = primary["summary"].as_str().expect("primary action summary");
    assert!(actions.contains(primary_code));
    assert_eq!(
        report["summary_card"]["next"],
        format!(
            "{}: {primary_summary}",
            primary["urgency"].as_str().unwrap()
        )
    );

    let compact_output = run_doctor(None)?;
    assert!(matches!(compact_output.status.code(), Some(0 | 1)));
    assert!(compact_output.stderr.is_empty());
    let compact = String::from_utf8(compact_output.stdout)?;
    assert!(compact.contains(&format!("Next action: {primary_summary}")));
    assert!(compact.contains(&format!("Action: {primary_code}")));
    for code in actions.iter().filter(|code| code.as_str() != primary_code) {
        assert!(!compact.contains(code), "{compact}");
    }

    let verbose_output = run_doctor(Some("--verbose"))?;
    assert!(matches!(verbose_output.status.code(), Some(0 | 1)));
    assert!(verbose_output.stderr.is_empty());
    let verbose = String::from_utf8(verbose_output.stdout)?;
    assert!(verbose.contains("\nPrimary action\n"));
    for code in &actions {
        assert!(verbose.contains(code), "{verbose}");
    }
    if !required.is_empty() {
        assert!(verbose.contains("\nRequired actions\n"));
    }
    if !recommended.is_empty() {
        assert!(verbose.contains("\nRecommended actions\n"));
    }
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
fn project_current_and_list_use_structured_human_output_in_canonical_order(
) -> Result<(), Box<dyn Error>> {
    let fixture = TempRuntimeHome::new("binary-project-presentation")?;
    let registrations = [
        ("Zulu_Project", fixture.create_product_repo("z")?),
        ("A", fixture.create_product_repo("a")?),
        (
            "Medium_Project",
            fixture.create_product_repo("medium-project")?,
        ),
    ];
    for (_, repo_root) in &registrations {
        fs::create_dir(repo_root.join(".git"))?;
    }
    prepare_runtime_home(fixture.path(), Path::new(env!("CARGO_BIN_EXE_volicord")))?;
    let mutation = TestRuntimeHomeMutation::acquire(fixture.path())?;
    let context = mutation.context()?;
    for (project_id, repo_root) in &registrations {
        register_project(
            &context,
            ProjectRegistration {
                project_id: (*project_id).to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
    }
    drop(context);
    drop(mutation);

    let mut current_command = base_command();
    let current = current_command
        .args(["project", "current"])
        .env("VOLICORD_HOME", fixture.path())
        .current_dir(&registrations[0].1)
        .output()?;
    assert!(current.status.success(), "{}", stderr(&current)?);
    assert_eq!(
        stdout(&current)?,
        format!(
            "Current project\n\nName: Zulu_Project\nRepository: {}\nStatus: active\n",
            registrations[0].1.display()
        )
    );

    let mut list_command = base_command();
    let list = list_command
        .args(["project", "list"])
        .env("VOLICORD_HOME", fixture.path())
        .output()?;
    assert!(list.status.success(), "{}", stderr(&list)?);
    let list = stdout(&list)?;
    assert!(list.starts_with("Projects (3)\n\nA\n"));
    let a = list.find("\nA\n").expect("A project");
    let medium = list.find("\nMedium_Project\n").expect("medium project");
    let zulu = list.find("\nZulu_Project\n").expect("Zulu project");
    assert!(a < medium && medium < zulu);
    assert!(!list.contains('\t'));
    assert!(list.ends_with('\n'));
    assert!(!list.ends_with("\n\n"));
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
        &["doctor", "--verbose", "--json"][..],
        &["doctor", "--privacy-footprint", "--verbose"][..],
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
fn privacy_footprint_direct_bytes_match_one_canonical_read_only_report(
) -> Result<(), Box<dyn Error>> {
    const CANONICAL_DIAGNOSTICS_CLAIM: &str = "bounded diagnostics.sqlite session, connection, project, transport, host, build, tool, categorical outcome, counter, byte-size, and latency observations when diagnostics are present";
    const CANONICAL_OUTPUT_SCOPE: &str =
        "Category and count summary only; stored row bodies are not printed.";

    let fixture = IsolatedInitFixture::new("binary-privacy-footprint-bytes")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    assert!(fixture
        .registry_snapshot()
        .agent_connections
        .iter()
        .all(|connection| connection.verification_report_json.is_some()));
    let state_before = directory_state(fixture._temporary_root.root_path())?;

    let human_output = fixture.run_privacy_footprint(false)?;
    assert!(human_output.status.success(), "{}", stderr(&human_output)?);
    assert_eq!(stderr(&human_output)?, "");
    assert_eq!(
        directory_state(fixture._temporary_root.root_path())?,
        state_before
    );

    let json_output = fixture.run_privacy_footprint(true)?;
    assert!(json_output.status.success(), "{}", stderr(&json_output)?);
    assert_eq!(stderr(&json_output)?, "");
    assert_eq!(
        directory_state(fixture._temporary_root.root_path())?,
        state_before
    );

    let human = std::str::from_utf8(&human_output.stdout)?;
    let json_text = std::str::from_utf8(&json_output.stdout)?;
    let json: Value = serde_json::from_str(json_text)?;
    let footprint = json["privacy_footprint"]
        .as_object()
        .expect("typed privacy_footprint object");
    let stores = json_string_array(&footprint["stores"]);
    let does_not_store = json_string_array(&footprint["does_not_store"]);
    let does_not_prove = json_string_array(&footprint["does_not_prove"]);
    let output_scope = footprint["doctor_output_scope"]
        .as_str()
        .expect("typed doctor_output_scope string");
    let unique_claims = stores
        .iter()
        .chain(does_not_store.iter())
        .chain(does_not_prove.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unique_claims.len(),
        stores.len() + does_not_store.len() + does_not_prove.len()
    );
    assert_eq!(output_scope, CANONICAL_OUTPUT_SCOPE);
    assert!(!unique_claims.contains(output_scope));

    let stores_section = human_section(human, "Stores", "Does not store");
    let does_not_store_section = human_section(human, "Does not store", "Does not prove");
    let does_not_prove_section = human_section(human, "Does not prove", "Output scope");
    let output_scope_section = human
        .split_once("\nOutput scope\n")
        .expect("Output scope section")
        .1;
    for (claims, section) in [
        (&stores, stores_section),
        (&does_not_store, does_not_store_section),
        (&does_not_prove, does_not_prove_section),
    ] {
        for claim in claims {
            assert!(section.contains(claim), "missing canonical claim: {claim}");
            assert_eq!(human.matches(claim).count(), 1, "duplicate claim: {claim}");
        }
    }
    assert!(output_scope_section.contains(output_scope));
    assert!(!does_not_store_section.contains(output_scope));
    assert_eq!(human.matches(output_scope).count(), 1);
    assert!(stores.contains(&CANONICAL_DIAGNOSTICS_CLAIM));
    assert!(human.contains(CANONICAL_DIAGNOSTICS_CLAIM));
    assert!(json_output
        .stdout
        .windows(CANONICAL_DIAGNOSTICS_CLAIM.len())
        .any(|bytes| bytes == CANONICAL_DIAGNOSTICS_CLAIM.as_bytes()));

    assert_eq!(
        human.len() - human.trim_end_matches('\n').len(),
        1,
        "human output must have exactly one trailing newline"
    );
    assert!(!human.contains('\t'));
    assert!(human
        .chars()
        .all(|character| character == '\n' || !character.is_control()));
    Ok(())
}

#[test]
fn contextual_read_only_reports_preserve_authority_state() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("binary-contextual-read-only")?;
    fs::create_dir_all(fixture.product_repo_path().join(".git"))?;
    let counts_before = fixture.counts()?;
    let authority_before = fixture.authority_snapshot()?;

    for json in [false, true] {
        let mut command = base_command();
        command
            .arg("status")
            .env("VOLICORD_HOME", fixture.runtime_home_path())
            .current_dir(fixture.product_repo_path());
        if json {
            command.arg("--json");
        }
        let output = command.output()?;
        assert!(output.status.success(), "{}", stderr(&output)?);
        assert_eq!(stderr(&output)?, "");
        if json {
            let report: Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(report["summary_card"]["task"], "none");
            assert_eq!(report["summary_card"]["profile"], "record");
            assert_eq!(report["active_task"], Value::Null);
        } else {
            let text = stdout(&output)?;
            assert!(text.starts_with("No active Task.\n\n"), "{text}");
            assert!(text.contains("Profile: record"), "{text}");
            assert!(text.contains("Pending user actions: none"), "{text}");
            assert!(text.contains("Next action: none"), "{text}");
            assert!(!text.contains("not shown in this view"), "{text}");
        }
    }

    for mode in [None, Some("--verbose"), Some("--json")] {
        let mut command = base_command();
        command
            .arg("doctor")
            .env("VOLICORD_HOME", fixture.runtime_home_path())
            .current_dir(fixture.product_repo_path());
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let output = command.output()?;
        assert!(matches!(output.status.code(), Some(0 | 1)));
        assert_eq!(stderr(&output)?, "");
        if mode == Some("--json") {
            let report: Value = serde_json::from_slice(&output.stdout)?;
            assert!(report["checks"].is_array());
            assert!(report["summary_card"].is_object());
        } else {
            let text = stdout(&output)?;
            assert!(text.starts_with("Volicord "), "{text}");
            assert!(!text.contains("not shown in this view"), "{text}");
            if mode == Some("--verbose") {
                assert!(text.contains("\nRuntime and build\n"), "{text}");
                assert!(text.contains("\nCommand availability\n"), "{text}");
                assert!(text.contains("\nBuild provenance\n"), "{text}");
                assert!(text.contains("\nOutput scope\n"), "{text}");
            }
        }
    }

    for json in [false, true] {
        let mut command = base_command();
        command
            .args(["doctor", "--privacy-footprint"])
            .env("VOLICORD_HOME", fixture.runtime_home_path());
        if json {
            command.arg("--json");
        }
        let output = command.output()?;
        assert!(output.status.success(), "{}", stderr(&output)?);
        assert_eq!(stderr(&output)?, "");
        if json {
            let report: Value = serde_json::from_slice(&output.stdout)?;
            assert!(report["privacy_footprint"]["stores"].is_array());
            assert!(report["privacy_footprint"]["does_not_store"].is_array());
            assert!(report["privacy_footprint"]["does_not_prove"].is_array());
        } else {
            let text = stdout(&output)?;
            for section in [
                "Runtime Home",
                "Record counts",
                "Stores",
                "Does not store",
                "Does not prove",
                "Output scope",
            ] {
                assert!(text.contains(section), "{text}");
            }
        }
    }

    assert_eq!(fixture.counts()?, counts_before);
    assert_eq!(fixture.authority_snapshot()?, authority_before);
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
    assert_eq!(value["operation_details"]["dry_run"], false);
    assert_eq!(value["operation_details"]["result"]["kind"], "setup");
    assert_eq!(
        value["operation_details"]["result"]["disposition"],
        "committed"
    );
    assert!(value["operation_details"].get("planned_changes").is_none());
    assert!(value["checks"].is_array());
    assert!(value["activation_plan"].is_object());
    assert_eq!(value["limits"].as_array().map(Vec::len), Some(3));
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
    assert_eq!(value["operation_details"]["dry_run"], true);
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["operation_details"]["result"]["kind"], "setup");
    assert_eq!(
        value["operation_details"]["result"]["disposition"],
        "planned"
    );
    assert_eq!(value["connection"]["mode"], "workflow");
    let planned_changes = value["operation_details"]["planned_changes"]
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
            "hook_definition",
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
    let step_ids = value["activation_plan"]["required_steps"]
        .as_array()
        .expect("typed activation steps")
        .iter()
        .map(|step| step["id"].as_str().expect("activation step id"))
        .collect::<Vec<_>>();
    assert!(step_ids.contains(&"reload_codex"));
    assert!(step_ids.contains(&"review_project_hooks"));
    assert!(step_ids.contains(&"request_integration_verification"));
    assert!(!step_ids.contains(&"guard_probe"));
    assert!(!fixture.runtime_home.join("registry.sqlite").exists());
    assert!(!fixture.codex_home.join("config.toml").exists());
    assert!(directory_contents(&fixture.repo_root)?.is_empty());
    Ok(())
}

#[test]
fn init_dry_run_is_read_only_for_every_initial_registry_state() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-init-dry-run-registry-states")?;
    let missing_home = fixture
        ._temporary_root
        .root_path()
        .join("missing-runtime-home");
    let absent_registry_home = fixture.runtime_home.clone();
    fs::create_dir(&absent_registry_home)?;
    let zero_byte_home = fixture._temporary_root.root_path().join("zero-byte-home");
    fs::create_dir(&zero_byte_home)?;
    let zero_byte_registry = registry_db_path(&zero_byte_home);
    fs::write(&zero_byte_registry, [])?;
    let schema_less_home = fixture._temporary_root.root_path().join("schema-less-home");
    fs::create_dir(&schema_less_home)?;
    let schema_less_registry = registry_db_path(&schema_less_home);
    let schema_less = rusqlite::Connection::open(&schema_less_registry)?;
    schema_less.execute_batch("VACUUM")?;
    drop(schema_less);
    let schema_less_master_before = sqlite_master_rows(&schema_less_registry)?;
    assert!(schema_less_master_before.is_empty());
    let no_profile_home = fixture._temporary_root.root_path().join("no-profile-home");
    with_test_runtime_home_setup(&no_profile_home, |context| {
        initialize_runtime_home(context, "runtime_home_without_profile", "{}")?;
        Ok(())
    })?;
    let no_profile_registry = registry_db_path(&no_profile_home);
    let current_profile_home = fixture
        ._temporary_root
        .root_path()
        .join("current-profile-home");
    let current_binary = fs::canonicalize(env!("CARGO_BIN_EXE_volicord"))?;
    let current_bin_dir = current_binary
        .parent()
        .ok_or("test binary path has no parent")?
        .to_path_buf();
    with_test_runtime_home_setup(&current_profile_home, |context| {
        initialize_runtime_home(context, "runtime_home_with_current_profile", "{}")?;
        write_installation_profile(
            context,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: path_text(&current_binary),
                volicord_mcp_command: path_text(&current_binary),
                bin_dir: current_bin_dir.clone(),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: r#"{"source":"binary-test"}"#.to_owned(),
            },
        )?;
        Ok(())
    })?;
    let current_profile_registry = registry_db_path(&current_profile_home);
    let fallback_home = fixture._temporary_root.root_path().join("fallback-home");

    let files_before = directory_contents(fixture._temporary_root.root_path())?;
    let entries_before = directory_entries(fixture._temporary_root.root_path())?;
    let zero_byte_modified_before = fs::metadata(&zero_byte_registry)?.modified()?;
    let schema_less_modified_before = fs::metadata(&schema_less_registry)?.modified()?;
    let no_profile_modified_before = fs::metadata(&no_profile_registry)?.modified()?;
    let current_profile_modified_before = fs::metadata(&current_profile_registry)?.modified()?;

    let output = fixture.run_init_dry_run_against_home(&missing_home, &fallback_home)?;
    let report = successful_init_dry_run_report(&output)?;
    assert!(report["operation_details"]["planned_changes"]
        .as_array()
        .expect("planned changes")
        .iter()
        .any(|change| change["kind"] == "runtime_home_initialization"));
    assert_eq!(
        directory_contents(fixture._temporary_root.root_path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.root_path())?,
        entries_before
    );
    assert!(!missing_home.exists());
    assert!(!registry_db_path(&absent_registry_home).exists());

    for runtime_home in [&absent_registry_home, &zero_byte_home, &schema_less_home] {
        let output = fixture.run_init_dry_run_against_home(runtime_home, &fallback_home)?;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout(&output)?, "");
        let diagnostic = stderr(&output)?;
        assert!(
            diagnostic.contains("existing state preserved"),
            "unexpected invalid Registry diagnostic: {diagnostic}"
        );
        assert!(!diagnostic.contains("missing canonical SQLite relation"));
        assert_eq!(
            directory_contents(fixture._temporary_root.root_path())?,
            files_before
        );
        assert_eq!(
            directory_entries(fixture._temporary_root.root_path())?,
            entries_before
        );
    }

    let no_profile = fixture.run_init_dry_run_against_home(&no_profile_home, &fallback_home)?;
    let no_profile_report = successful_init_dry_run_report(&no_profile)?;
    assert!(no_profile_report["operation_details"]["planned_changes"]
        .as_array()
        .expect("planned changes")
        .iter()
        .any(|change| change["kind"] == "runtime_home_initialization"));
    assert_eq!(
        directory_contents(fixture._temporary_root.root_path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.root_path())?,
        entries_before
    );

    let current_profile =
        fixture.run_init_dry_run_against_home(&current_profile_home, &fallback_home)?;
    let current_profile_report = successful_init_dry_run_report(&current_profile)?;
    assert!(
        !current_profile_report["operation_details"]["planned_changes"]
            .as_array()
            .expect("planned changes")
            .iter()
            .any(|change| change["kind"] == "runtime_home_initialization")
    );
    assert_eq!(
        directory_contents(fixture._temporary_root.root_path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.root_path())?,
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
    assert_eq!(
        init_report["operation_details"]["result"]["disposition"],
        "committed"
    );
    let files_before = directory_contents(fixture._temporary_root.root_path())?;
    let entries_before = directory_entries(fixture._temporary_root.root_path())?;

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
        std::collections::BTreeSet::from(["connections", "generated_at", "limits"])
    );
    assert_eq!(report["limits"], init_report["limits"]);
    assert_eq!(report["limits"].as_array().map(Vec::len), Some(3));

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
            "connection_id",
            "connection_intent",
            "enabled",
            "host_kind",
            "host_scope",
            "issues",
            "memberships",
            "mode",
            "server_name",
        ])
    );
    assert_eq!(entry["issues"], serde_json::json!([]));
    let membership = &entry["memberships"][0];
    assert_eq!(
        membership
            .as_object()
            .expect("membership object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["current_state", "project_id", "project_name", "repository"])
    );
    assert_eq!(membership["repository"], path_text(&fixture.repo_root));
    assert_eq!(membership["current_state"]["state"], "available");
    assert_eq!(
        membership["current_state"]["evaluated_at"],
        report["generated_at"]
    );
    let status = fixture.run_connection("status", true)?;
    let status_report: Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(status_report["operation"], "status");
    assert_eq!(
        status_report["connection"]["runtime_home"],
        path_text(&fixture.runtime_home)
    );
    assert_eq!(
        membership["current_state"]["status"],
        status_report["status"]
    );
    assert_eq!(
        membership["current_state"]["activation"],
        status_report["activation_state"]
    );
    assert_eq!(
        membership["current_state"]["hook_activation"],
        status_report["hook_activation_state"]
    );

    assert_eq!(
        directory_contents(fixture._temporary_root.root_path())?,
        files_before
    );
    assert_eq!(
        directory_entries(fixture._temporary_root.root_path())?,
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
    let connection_a = init_a["connection"]["connection_id"]
        .as_str()
        .expect("home A connection id")
        .to_owned();
    let connection_b = init_b["connection"]["connection_id"]
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
        assert_eq!(
            status["connection"]["runtime_home"],
            path_text(&selected.runtime_home)
        );
        assert_eq!(status["connection"]["connection_id"], expected_id);
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
        assert_eq!(
            add["connection"]["runtime_home"],
            path_text(&selected.runtime_home)
        );
        assert_eq!(add["connection"]["connection_id"], expected_id);
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
        assert_eq!(
            verify["connection"]["runtime_home"],
            path_text(&selected.runtime_home)
        );
        assert_eq!(verify["connection"]["connection_id"], expected_id);
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
        assert_eq!(
            mode["connection"]["runtime_home"],
            path_text(&selected.runtime_home)
        );
        assert_eq!(mode["connection"]["connection_id"], expected_id);
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
        assert_eq!(
            remove["connection"]["runtime_home"],
            path_text(&selected.runtime_home)
        );
        assert_eq!(remove["connection"]["connection_id"], expected_id);
        assert_eq!(
            remove["operation_details"]["result"]["connection_removed"],
            true
        );
        assert_eq!(decoy.all_contents()?, decoy_before);
    }
    Ok(())
}

#[test]
fn custom_home_lifecycle_needs_no_environment_binding() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-custom-home-lifecycle")?;
    let init: Value = serde_json::from_slice(&fixture.run(false)?.stdout)?;
    let connection_id = init["connection"]["connection_id"]
        .as_str()
        .expect("custom-home connection id");
    assert_eq!(
        init["connection"]["runtime_home"],
        path_text(&fixture.runtime_home)
    );

    let list: Value = serde_json::from_slice(&fixture.run_connection_list(None, true)?.stdout)?;
    assert_eq!(list["connections"].as_array().map(Vec::len), Some(1));
    assert_eq!(list["connections"][0]["connection_id"], connection_id);

    for operation in ["status", "verify", "add"] {
        let report: Value =
            serde_json::from_slice(&fixture.run_connection(operation, true)?.stdout)?;
        assert_eq!(report["operation"], operation);
        assert_eq!(
            report["connection"]["runtime_home"],
            path_text(&fixture.runtime_home)
        );
        assert_eq!(report["connection"]["connection_id"], connection_id);
    }

    let mode: Value = serde_json::from_slice(&fixture.run_connection_mode("read-only")?.stdout)?;
    assert_eq!(
        mode["connection"]["runtime_home"],
        path_text(&fixture.runtime_home)
    );
    assert_eq!(mode["connection"]["connection_id"], connection_id);
    assert_eq!(mode["connection"]["mode"], "read_only");

    let remove = fixture.run_connection("remove", true)?;
    assert_eq!(remove.status.code(), Some(0), "{}", stderr(&remove)?);
    let remove: Value = serde_json::from_slice(&remove.stdout)?;
    assert_eq!(
        remove["connection"]["runtime_home"],
        path_text(&fixture.runtime_home)
    );
    assert_eq!(remove["connection"]["connection_id"], connection_id);
    assert_eq!(
        remove["operation_details"]["result"]["connection_removed"],
        true
    );
    Ok(())
}

#[test]
fn relative_explicit_home_is_reported_as_the_selected_absolute_path() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-relative-explicit-home")?;
    fixture.run(false)?;
    let relative_home = fixture
        .runtime_home
        .strip_prefix(fixture._temporary_root.root_path())?;
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
        .current_dir(fixture._temporary_root.root_path())
        .output()?;
    assert!(
        !output.stdout.is_empty(),
        "relative Runtime Home status produced no report: {}",
        stderr(&output)?
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        report["connection"]["runtime_home"],
        path_text(&fixture.runtime_home)
    );
    Ok(())
}

#[test]
fn every_connection_command_rejects_unusable_explicit_home_without_mutation_or_fallback(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-unusable-explicit-home")?;
    let init: Value = serde_json::from_slice(&fixture.run(false)?.stdout)?;
    let connection_id = init["connection"]["connection_id"]
        .as_str()
        .expect("custom-home connection id");
    let missing_home = fixture
        ._temporary_root
        .root_path()
        .join("missing explicit home");
    let missing_registry_home = fixture
        ._temporary_root
        .root_path()
        .join("missing-registry-explicit-home");
    fs::create_dir(&missing_registry_home)?;
    let zero_byte_home = fixture
        ._temporary_root
        .root_path()
        .join("zero-byte-explicit-home");
    fs::create_dir(&zero_byte_home)?;
    let zero_byte_registry = registry_db_path(&zero_byte_home);
    fs::write(&zero_byte_registry, [])?;
    let empty_sqlite_home = fixture
        ._temporary_root
        .root_path()
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
        .root_path()
        .join("no-profile-explicit-home");
    with_test_runtime_home_setup(&no_profile_home, |context| {
        initialize_runtime_home(context, "runtime_home_without_profile", "{}")?;
        Ok(())
    })?;

    let zero_byte_modified = fs::metadata(&zero_byte_registry)?.modified()?;
    let empty_sqlite_modified = fs::metadata(&empty_sqlite_registry)?.modified()?;
    let no_profile_registry = registry_db_path(&no_profile_home);
    let no_profile_modified = fs::metadata(&no_profile_registry)?.modified()?;
    let files_before = directory_contents(fixture._temporary_root.root_path())?;
    let entries_before = directory_entries(fixture._temporary_root.root_path())?;

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
                directory_contents(fixture._temporary_root.root_path())?,
                files_before,
                "{operation} changed filesystem bytes for {}",
                unusable_home.display()
            );
            assert_eq!(
                directory_entries(fixture._temporary_root.root_path())?,
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
        .root_path()
        .join("Volicord Runtime Home's");
    fixture.repo_root = fixture
        ._temporary_root
        .root_path()
        .join("Product Repository's");
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
    let connection_id = init["connection"]["connection_id"]
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
fn connection_list_evaluates_current_state_without_persisting_a_missing_report(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-missing-report")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();
    set_verification_report(&fixture, &connection_id, None)?;

    let output = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let entry = &report["connections"][0];
    assert_eq!(
        entry["memberships"][0]["current_state"]["state"],
        "available"
    );
    assert_eq!(
        entry["memberships"][0]["current_state"]["evaluated_at"],
        report["generated_at"]
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
    assert_eq!(
        entry["memberships"][0]["current_state"]["state"],
        "unavailable"
    );
    assert_eq!(
        entry["memberships"][0]["current_state"]["reason"],
        "registration_metadata_corrupt"
    );
    assert!(entry["memberships"][0]["current_state"]
        .get("status")
        .is_none());
    assert_eq!(
        entry["issues"],
        serde_json::json!([{
            "kind": "registration_metadata_corrupt",
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
fn connection_list_reports_malformed_active_evidence_as_membership_unavailable(
) -> Result<(), Box<dyn Error>> {
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
    assert_eq!(entry["issues"], serde_json::json!([]));
    assert_eq!(
        entry["memberships"][0]["current_state"]["state"],
        "unavailable"
    );
    assert_eq!(
        entry["memberships"][0]["current_state"]["reason"],
        "persisted_active_verification_evidence_corrupt"
    );
    assert!(entry["memberships"][0]["current_state"]
        .get("status")
        .is_none());
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
    invalid["activation_plan"]["required_steps"]
        .as_array_mut()
        .and_then(|steps| steps.first_mut())
        .and_then(Value::as_object_mut)
        .expect("failed init report has an activation step")
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
    for step in generated["activation_plan"]["required_steps"]
        .as_array()
        .expect("generated activation steps")
    {
        assert_current_activation_step_shape(step);
    }
    assert!(generated["activation_plan"]["optional_diagnostics"]
        .as_array()
        .is_some_and(|steps| steps
            .iter()
            .any(|step| step["id"] == "run_optional_active_diagnostics")));

    let stored = stored_verification_report(&fixture, &connection_id)?
        .expect("active verification replaced the report");
    serde_json::from_str::<ConnectionVerificationReport>(&stored)?;

    let after_connection = fixture.registry_snapshot().agent_connections[0].clone();
    let mut expected_connection = before_connection;
    expected_connection.verification_report_json =
        after_connection.verification_report_json.clone();
    expected_connection.updated_at = after_connection.updated_at.clone();
    assert_eq!(after_connection, expected_connection);
    Ok(())
}

#[test]
fn connection_list_human_output_is_structured_tab_free_and_verbose_on_request(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-text")?;
    fixture.run(false)?;
    let connection_id = fixture.only_connection_id();

    let current = fixture.run_connection_list(Some(&fixture.repo_root), false)?;
    assert_eq!(current.status.code(), Some(0));
    let current_text = stdout(&current)?;
    assert!(current_text.starts_with("Connections (1)\n\ncodex\n"));
    assert!(current_text.contains(&format!("Repository: {}", path_text(&fixture.repo_root))));
    assert!(current_text.contains("Status: "));
    assert!(current_text.contains("Checks: "));
    assert!(!current_text.contains('\t'));
    assert!(!current_text.contains("Connection ID: "));
    assert!(!current_text.contains("Project ID: "));

    let verbose = fixture.run_connection_list_verbose(Some(&fixture.repo_root))?;
    assert_eq!(verbose.status.code(), Some(0));
    let verbose_text = stdout(&verbose)?;
    assert!(verbose_text.contains(&format!("Connection ID: {connection_id}")));
    assert!(verbose_text.contains("Project ID: "));
    assert!(verbose_text.contains("Configuration target: "));
    assert!(verbose_text.contains("Integration revision: "));
    assert!(verbose_text.contains("Evaluated at: "));
    assert!(verbose_text.contains("Not applicable checks: "));
    assert!(!verbose_text.contains('\t'));

    set_connection_metadata(&fixture, &connection_id, "{")?;
    let corrupt = fixture.run_connection_list(Some(&fixture.repo_root), false)?;
    assert_eq!(corrupt.status.code(), Some(0));
    let corrupt_text = stdout(&corrupt)?;
    assert!(corrupt_text.contains("Current state: unavailable"));
    assert!(corrupt_text.contains("Reason: registration metadata corrupt"));
    assert!(corrupt_text.contains("Registration issue: "));
    assert!(!corrupt_text.contains('\t'));
    Ok(())
}

#[test]
fn connection_list_filters_by_repository() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-connection-list-filter")?;
    fixture.run(false)?;
    let other_repo = fixture.create_repository("other-list-repository")?;
    let shared = fixture.run_shared_connection_add(&other_repo)?;
    let shared_report: Value = serde_json::from_slice(&shared.stdout)?;
    assert_eq!(
        shared_report["operation_details"]["result"]["disposition"],
        "committed"
    );
    let shared_config_target = PathBuf::from(
        shared_report["connection"]["config_target"]
            .as_str()
            .expect("shared config target"),
    );
    fs::remove_file(&shared_config_target)?;
    fs::create_dir(&shared_config_target)?;

    let first = fixture.run_connection_list(Some(&fixture.repo_root), true)?;
    assert_eq!(first.status.code(), Some(0));
    let first: Value = serde_json::from_slice(&first.stdout)?;
    assert_eq!(first["connections"].as_array().map(Vec::len), Some(1));
    assert_eq!(first["connections"][0]["connection_intent"], "personal");
    assert_eq!(
        first["connections"][0]["memberships"][0]["current_state"]["state"], "available",
        "filtering must avoid evaluation of the unreadable shared configuration"
    );

    let second = fixture.run_connection_list(Some(&other_repo), true)?;
    assert_eq!(second.status.code(), Some(0));
    let second: Value = serde_json::from_slice(&second.stdout)?;
    assert_eq!(second["connections"].as_array().map(Vec::len), Some(1));
    assert_eq!(second["connections"][0]["connection_intent"], "shared");
    assert_eq!(
        second["connections"][0]["memberships"][0]["current_state"]["state"],
        "unavailable"
    );
    assert_eq!(
        second["connections"][0]["memberships"][0]["current_state"]["reason"],
        "managed_configuration_unavailable"
    );

    let all = fixture.run_connection_list(None, true)?;
    assert_eq!(all.status.code(), Some(0));
    let all: Value = serde_json::from_slice(&all.stdout)?;
    assert_eq!(all["connections"].as_array().map(Vec::len), Some(2));
    assert!(all["connections"]
        .as_array()
        .expect("connections")
        .iter()
        .any(|connection| connection["memberships"][0]["current_state"]["state"] == "available"));
    assert!(all["connections"]
        .as_array()
        .expect("connections")
        .iter()
        .any(
            |connection| connection["memberships"][0]["current_state"]["reason"]
                == "managed_configuration_unavailable"
        ));
    Ok(())
}

#[test]
fn connection_list_empty_inventory_and_store_failure_use_owned_channels(
) -> Result<(), Box<dyn Error>> {
    let temporary_root = TempRuntimeHome::new("binary-connection-list-channels")?;
    let repo_root = temporary_root.root_path().join("repo");
    fs::create_dir_all(repo_root.join(".git"))?;
    let empty_home = temporary_root.root_path().join("empty-home");
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
    assert_eq!(empty_report["limits"].as_array().map(Vec::len), Some(3));

    let corrupt_home = temporary_root.root_path().join("corrupt-home");
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
fn diagnostics_show_exit_status_depends_on_lookup_not_finding_severity(
) -> Result<(), Box<dyn Error>> {
    let temporary_root = TempRuntimeHome::new("binary-diagnostics-show-exits")?;
    let runtime_home = temporary_root.root_path().join("runtime-home");
    prepare_runtime_home(&runtime_home, Path::new(env!("CARGO_BIN_EXE_volicord")))?;

    let finding = OccurrenceDiagnosticFinding::try_new(
        DiagnosticFindingData::try_new(
            DiagnosticCode::parse("diagnostics.process_error")?,
            DiagnosticDomain::parse("diagnostics")?,
            DiagnosticStage::parse("lookup")?,
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("binary_admin_test")?,
            DiagnosticSubject::try_new("test_record", "process-exit")?,
            DiagnosticFacts::empty(),
            UtcTimestamp::parse("2026-07-22T08:09:10Z")?,
        )?,
        None,
    )?;
    let finding_id = finding.id().to_string();
    let mutation = TestRuntimeHomeMutation::acquire(&runtime_home)?;
    let context = mutation.context()?;
    insert_occurrence_finding(&context, &finding)?;

    let found = run_diagnostics_show(&runtime_home, &finding_id)?;
    assert_eq!(found.status.code(), Some(0), "{}", stderr(&found)?);
    assert_eq!(stderr(&found)?, "");
    let found_report: Value = serde_json::from_slice(&found.stdout)?;
    assert_eq!(found_report["lookup_status"], "found");
    assert_eq!(found_report["root"]["lifecycle"], "occurrence");
    assert_eq!(found_report["root"]["finding"]["severity"], "error");

    let missing_id = "finding.does_not_exist";
    let missing = run_diagnostics_show(&runtime_home, missing_id)?;
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(stderr(&missing)?, "");
    let missing_report: Value = serde_json::from_slice(&missing.stdout)?;
    assert_eq!(missing_report["lookup_status"], "not_found");
    assert_eq!(missing_report["requested_id"], missing_id);
    assert!(missing_report["root"].is_null());

    let invalid = run_diagnostics_show(&runtime_home, "invalid finding id")?;
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(stdout(&invalid)?, "");
    assert!(!stderr(&invalid)?.is_empty());

    fs::write(registry_db_path(&runtime_home), b"not a sqlite database")?;
    let store_failure = run_diagnostics_show(&runtime_home, &finding_id)?;
    assert_eq!(store_failure.status.code(), Some(1));
    assert_eq!(stdout(&store_failure)?, "");
    assert!(stderr(&store_failure)?.starts_with("error:"));
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
    assert_eq!(value["operation_details"]["result"]["kind"], "setup");
    assert_eq!(
        value["operation_details"]["result"]["disposition"],
        "committed"
    );
    assert!(value["checks"].as_array().is_some_and(|checks| {
        checks
            .iter()
            .any(|check| check["id"] == "host_reload" && check["status"] == "pending")
    }));
    Ok(())
}

#[test]
fn doctor_reads_current_hook_path_safety_evidence_without_mutation() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-doctor-hook-path-safety")?;
    fixture.install_codex_executable()?;
    let init = fixture.run(false)?;
    assert_eq!(init.status.code(), Some(0), "{}", stderr(&init)?);

    let state_before = directory_state(fixture._temporary_root.root_path())?;
    let run_doctor = |mode: Option<&str>| -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("doctor")
            .env("VOLICORD_HOME", &fixture.runtime_home)
            .env("PATH", &fixture.empty_path)
            .env("CODEX_HOME", &fixture.codex_home)
            .env("HOME", &fixture.user_home)
            .env("USERPROFILE", &fixture.user_home)
            .current_dir(&fixture.repo_root);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        Ok(command.output()?)
    };

    let json_output = run_doctor(Some("--json"))?;
    assert!(matches!(json_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&json_output)?, "");
    let report: Value = serde_json::from_slice(&json_output.stdout)?;
    let assessment = &report["states"]["hook_path_safety"];
    assert_eq!(
        assessment
            .as_object()
            .expect("typed Hook path-safety assessment")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "cwd_independence",
            "evidence",
            "state",
            "subdirectory_safety",
        ])
    );
    assert_eq!(assessment["state"], "verified");
    assert_eq!(assessment["cwd_independence"], "verified");
    assert_eq!(assessment["subdirectory_safety"], "verified");
    assert!(!assessment["evidence"]
        .as_array()
        .expect("bounded Hook path-safety evidence")
        .is_empty());
    let guard_files = report["checks"]
        .as_array()
        .expect("Doctor checks")
        .iter()
        .find(|check| check["id"] == "guard_files")
        .expect("guard_files check");
    assert_eq!(guard_files["status"], "passed");
    assert_eq!(guard_files["details"]["hook_path_safety"], *assessment);
    for removed_key in [
        "hook_commands_cwd_independent",
        "hook_commands_subdirectory_safe",
        "hook_path_safety_details",
    ] {
        assert!(!json_value_contains_key(&report, removed_key));
    }

    let verbose_output = run_doctor(Some("--verbose"))?;
    assert!(matches!(verbose_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&verbose_output)?, "");
    let verbose = stdout(&verbose_output)?;
    assert!(verbose.contains("Hook path safety: verified"));
    assert!(verbose.contains("CWD independence: verified"));
    assert!(verbose.contains("Subdirectory safety: verified"));
    assert!(verbose.contains("Evidence: 6 current managed artifacts verified"));
    assert!(!verbose.contains("Evidence 1"));

    let compact_output = run_doctor(None)?;
    assert!(matches!(compact_output.status.code(), Some(0 | 1)));
    assert_eq!(stderr(&compact_output)?, "");
    assert!(!stdout(&compact_output)?.contains("Hook path safety"));
    assert_eq!(
        directory_state(fixture._temporary_root.root_path())?,
        state_before
    );
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
    assert!(text.starts_with("Setup committed; 4 host-owned activation steps remain.\n\n"));
    assert!(text.contains(&format!("Repository: {}\n", fixture.repo_root.display())));
    assert!(text.contains("Mode: workflow\nActivation: "));
    assert!(text.contains("\nHook activation: "));
    assert!(text.contains("\nChecks\n"));
    assert!(text.contains("\n  Passed: "));
    assert!(text.contains("\n  Blocked: "));
    assert!(text.contains("\n  Pending: "));
    assert!(text.contains("\n  Failed: "));
    assert!(text.contains("Waiting\n"));
    assert_eq!(text.matches("Required next steps\n").count(), 1);
    assert!(!text.contains("\nNext\n"));
    assert!(text.contains("Optional active diagnostics\n"));
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
    assert!(
        stdout(&init)?.starts_with("Setup committed; 4 host-owned activation steps remain.\n\n")
    );

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
    assert_eq!(report["operation_details"]["result"]["kind"], "removal");
    assert_eq!(
        report["operation_details"]["result"]["membership_removed"],
        true
    );
    assert_eq!(
        report["operation_details"]["result"]["connection_removed"],
        true
    );
    assert_eq!(
        report["operation_details"]["result"]["remaining_project_count"],
        0
    );
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
    assert!(text.contains(
        "Mode: workflow\nActivation: configured\nHook activation: unknown\n\nChecks\n  Passed: 1\n  Blocked: 0\n  Pending: 0\n  Failed: 0\n"
    ));
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
    assert_eq!(
        report["activation_plan"]["required_steps"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(fixture.registry_snapshot(), registry_before);
    assert_eq!(directory_contents(&fixture.runtime_home)?, runtime_before);
    assert_eq!(directory_contents(&fixture.codex_home)?, host_before);
    assert_eq!(directory_contents(&fixture.repo_root)?, repository_before);
    Ok(())
}

#[test]
fn connection_add_dry_run_preserves_setup_check_and_activation_plan() -> Result<(), Box<dyn Error>>
{
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
    assert!(report["activation_plan"]["required_steps"]
        .as_array()
        .is_some_and(|steps| steps
            .iter()
            .any(|step| step["id"] == "review_project_hooks")));
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
    assert_eq!(report["operation_details"]["dry_run"], false);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["operation_details"]["result"]["kind"], "setup");
    assert_eq!(
        report["operation_details"]["result"]["disposition"],
        "committed"
    );
    assert!(report["checks"].is_array());
    assert!(report["activation_plan"].is_object());
    Ok(())
}

#[test]
fn connection_mode_preserves_transition_check_and_typed_reload_action() -> Result<(), Box<dyn Error>>
{
    let fixture = IsolatedInitFixture::new("binary-mode-kinds")?;
    fixture.install_codex_executable()?;
    assert_eq!(fixture.run(false)?.status.code(), Some(0));

    let output = fixture.run_connection_mode("read-only")?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output)?, "");
    let report: Value = serde_json::from_str(&stdout(&output)?)?;
    assert_eq!(report["checks"][0]["id"], "mode_transition");
    assert_eq!(
        report["activation_plan"]["required_steps"][0]["id"],
        "reload_codex"
    );
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
    assert!(text.contains("Restart or reload Codex, then use the current Volicord integration"));
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
    assert_eq!(
        runtime_sessions_before, 0,
        "setup verification must keep conformance sessions out of the selected Runtime Home"
    );
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
    assert_eq!(report["connection"]["connection_id"], connection_id);
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
    assert_eq!(
        report["operation_details"]["result"]["connection_removed"],
        true
    );
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
    assert_eq!(
        report["operation_details"]["result"]["membership_removed"],
        true
    );
    assert_eq!(
        report["operation_details"]["result"]["connection_removed"],
        false
    );
    assert_eq!(
        report["operation_details"]["result"]["remaining_project_count"],
        1
    );
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
    assert!(text.contains(
        "Mode: workflow\nActivation: configured\nHook activation: unknown\n\nChecks\n  Passed: 1\n  Blocked: 0\n  Pending: 0\n  Failed: 0\n"
    ));
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
    assert_eq!(value["operation_details"]["dry_run"], false);
    assert_eq!(value["status"], "failed");
    assert!(value["operation_details"].get("result").is_none());
    assert!(value["operation_details"].get("planned_changes").is_none());
    for step in value["activation_plan"]["required_steps"]
        .as_array()
        .expect("verification activation steps")
    {
        assert_current_activation_step_shape(step);
    }

    let connection_id = fixture.only_connection_id();
    let stored = stored_verification_report(&fixture, &connection_id)?
        .expect("verification report was persisted");
    serde_json::from_str::<ConnectionVerificationReport>(&stored)?;
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
    assert!(text.contains("Required next steps\n"));
    assert!(!text.contains("\nNext\n"));
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
    assert!(text.contains("Operation: active verification"));
    assert!(text.contains("Side effects: rollback-only Store writeability probes"));
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
    assert!(text.contains("\n\nRequired next steps\n"));
    assert!(text.contains("\n\nOptional active diagnostics\n"));
    assert!(text.contains("\n\nReport limits\n"));
    assert!(!text.contains("Command:"));
    assert!(text.contains("volicord connection verify"));
    assert!(!text.contains("Details: {"));
    assert!(!text.contains("\":["));
    assert!(text.ends_with('\n'));
    assert!(!text.ends_with("\n\n"));
    Ok(())
}

#[test]
fn policy_show_projects_complete_authority_in_every_output_without_mutation(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new_with_repo_directory(
        "binary-policy-show",
        "product-repository-with-a-deliberately-long-name-that-keeps-the-full-policy-path-readable",
    )?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let managed_path = fixture.repo_root.join(".volicord").join("policy.json");
    let candidate_path = fixture
        ._temporary_root
        .root_path()
        .join("policy-candidate.json");
    let mut candidate: Value = serde_json::from_slice(&fs::read(&managed_path)?)?;
    candidate["workflow"]["default_direct_control"] = Value::String("light".to_owned());
    candidate["workflow"]["default_work_control"] = Value::String("sensitive".to_owned());
    candidate["workflow"]["light"]["enabled"] = Value::Bool(true);
    candidate["workflow"]["light"]["max_intended_paths"] = Value::from(7_u64);
    candidate["workflow"]["light"]["allowed_path_patterns"] = serde_json::json!(["src", "tests"]);
    candidate["workflow"]["light"]["denied_path_patterns"] = serde_json::json!(["generated"]);
    candidate["workflow"]["light"]["final_acceptance"] =
        Value::String("policy_dependent".to_owned());
    candidate["workflow"]["write_ticket"]["idle_timeout_minutes"] = Value::from(45_u64);
    fs::write(&candidate_path, serde_json::to_vec_pretty(&candidate)?)?;

    let apply = fixture.run_policy_apply(&candidate_path)?;
    assert!(
        apply.status.success(),
        "policy apply failed: {}",
        stderr(&apply)?
    );

    let before = fixture.all_contents()?;
    let concise = fixture.run_policy_show(None)?;
    let verbose = fixture.run_policy_show(Some("--verbose"))?;
    let json = fixture.run_policy_show(Some("--json"))?;
    let validate_human = fixture.run_policy_validate(&managed_path, false)?;
    let validate_json = fixture.run_policy_validate(&managed_path, true)?;
    assert_eq!(fixture.all_contents()?, before);

    assert!(concise.status.success(), "{}", stderr(&concise)?);
    assert_eq!(stderr(&concise)?, "");
    let concise_text = stdout(&concise)?;
    assert!(concise_text.starts_with("Workflow policy is active.\n\n"));
    assert!(concise_text.contains(&format!("Repository: {}", fixture.repo_root.display())));
    assert!(concise_text.contains("Authority: project database"));
    assert!(concise_text.contains("Managed file: matches authority"));
    assert!(concise_text.contains("Direct tasks: light"));
    assert!(concise_text.contains("Work tasks: sensitive"));
    assert!(concise_text.contains("Enabled: yes"));
    assert!(concise_text.contains("Maximum intended paths: 7"));
    assert!(concise_text.contains("Allowed path patterns: 2"));
    assert!(concise_text.contains("Denied path patterns: 1"));
    assert!(concise_text.contains("Final acceptance: policy dependent"));
    assert!(concise_text.contains("Idle timeout: 45 minutes"));
    assert!(concise_text.contains("Active Task escalation required: no"));

    assert!(json.status.success(), "{}", stderr(&json)?);
    assert_eq!(stderr(&json)?, "");
    let report: PolicyShowReport = serde_json::from_slice(&json.stdout)?;
    assert_eq!(report.schema, PolicyShowReportSchema::Current);
    assert_eq!(report.status, PolicyShowStatus::Active);
    assert_eq!(report.repository, fixture.repo_root.display().to_string());
    assert_eq!(
        report.authority.policy.schema,
        WorkflowPolicySchema::Current
    );
    assert_eq!(
        report
            .authority
            .policy
            .workflow
            .default_direct_control
            .as_str(),
        "light"
    );
    assert_eq!(
        report
            .authority
            .policy
            .workflow
            .default_work_control
            .as_str(),
        "sensitive"
    );
    assert_eq!(
        report
            .authority
            .policy
            .workflow
            .light
            .allowed_path_patterns
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        ["src", "tests"]
    );
    assert_eq!(
        report.authority.policy.workflow.light.denied_path_patterns[0].as_str(),
        "generated"
    );
    assert_eq!(
        report
            .authority
            .policy
            .workflow
            .write_ticket
            .idle_timeout_minutes,
        Some(45)
    );
    assert_eq!(report.managed_file.status, ManagedPolicyFileStatus::Matches);
    assert!(report.managed_file.matches_authority);
    assert!(!report.active_task_requires_escalation);
    assert!(report.actions.is_empty());

    let nested_fingerprint = canonical_json_sha256(&report.authority.policy)?;
    assert_eq!(
        report.authority.policy_fingerprint,
        nested_fingerprint.as_str()
    );
    assert_eq!(
        report.managed_file.fingerprint.as_deref(),
        Some(nested_fingerprint.as_str())
    );
    let managed_policy: ProjectWorkflowPolicy = serde_json::from_slice(&fs::read(&managed_path)?)?;
    assert_eq!(managed_policy, report.authority.policy);
    assert_eq!(
        canonical_json_sha256(&managed_policy)?.as_str(),
        report.authority.policy_fingerprint
    );

    assert!(verbose.status.success(), "{}", stderr(&verbose)?);
    assert_eq!(stderr(&verbose)?, "");
    let verbose_text = stdout(&verbose)?;
    for expected in [
        "Policy fingerprint:",
        "Managed-file fingerprint:",
        "Managed-file path:",
        "Connection intent:",
        "Host:",
        "Selected profile:",
        "Connection ID:",
        "Guard installation ID:",
        "MCP launch",
        "Static environment",
        "Host hooks",
        "Pre-tool",
        "Post-tool",
        "Prompt capture",
        "Path patterns",
        "Allowed: [\"src\",\"tests\"]",
        "Denied: [\"generated\"]",
        "Idle timeout: 45 minutes",
    ] {
        assert!(
            verbose_text.contains(expected),
            "missing `{expected}` in:\n{verbose_text}"
        );
    }
    assert!(verbose_text.contains(&report.authority.policy.connection_id));
    assert!(verbose_text.contains(&report.authority.policy.guard_installation_id));
    assert!(verbose_text.contains(&report.authority.policy.mcp.command));
    assert!(verbose_text.contains(&serde_json::to_string(&report.authority.policy.mcp.args)?));
    for (name, value) in &report.authority.policy.mcp.env {
        assert!(verbose_text.contains(name));
        assert!(verbose_text.contains(value));
    }
    for command in [
        &report.authority.policy.host_hook.commands.pre_tool,
        &report.authority.policy.host_hook.commands.post_tool,
        &report.authority.policy.host_hook.commands.prompt_capture,
    ] {
        assert!(verbose_text.contains(&command.command));
        assert!(verbose_text.contains(&serde_json::to_string(&command.args)?));
    }

    assert!(!concise_text.contains(&report.authority.policy.connection_id));
    assert!(!concise_text.contains(&report.authority.policy.guard_installation_id));
    assert!(!concise_text.contains(&report.authority.policy.mcp.command));
    assert!(!concise_text.contains(&report.authority.policy_fingerprint));
    assert!(!concise_text.contains("MCP launch"));
    assert!(!concise_text.contains("Host hooks"));

    assert!(validate_human.status.success());
    assert_eq!(stderr(&validate_human)?, "");
    let validation_text = stdout(&validate_human)?;
    assert!(validation_text.starts_with("Policy is valid.\n\n"));
    assert!(validation_text.contains(&format!("File: {}", managed_path.display())));
    assert!(validation_text.contains("Schema: volicord.workflow_policy"));
    assert!(validation_text.contains(&format!(
        "Fingerprint: {}",
        report.authority.policy_fingerprint
    )));

    assert!(validate_json.status.success());
    assert_eq!(stderr(&validate_json)?, "");
    let validation: PolicyValidationReport = serde_json::from_slice(&validate_json.stdout)?;
    assert_eq!(validation.status, PolicyValidationStatus::Valid);
    assert_eq!(validation.file, managed_path.display().to_string());
    assert_eq!(validation.policy_schema, WorkflowPolicySchema::Current);
    assert_eq!(
        validation.policy_fingerprint,
        report.authority.policy_fingerprint
    );

    fixture.insert_escalating_active_task()?;
    let active_before = fixture.all_contents()?;
    let active_json = fixture.run_policy_show(Some("--json"))?;
    assert_eq!(fixture.all_contents()?, active_before);
    assert!(active_json.status.success(), "{}", stderr(&active_json)?);
    let active_report: PolicyShowReport = serde_json::from_slice(&active_json.stdout)?;
    assert!(active_report.active_task_requires_escalation);
    Ok(())
}

#[test]
fn policy_show_reports_managed_file_states_without_repairing_them() -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-policy-managed-file-states")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let managed_path = fixture.repo_root.join(".volicord").join("policy.json");

    let matching = fixture.run_policy_show(Some("--json"))?;
    assert!(matching.status.success(), "{}", stderr(&matching)?);
    let matching: PolicyShowReport = serde_json::from_slice(&matching.stdout)?;
    assert_eq!(
        matching.managed_file.status,
        ManagedPolicyFileStatus::Matches
    );
    assert!(matching.actions.is_empty());

    let mut drifted: Value = serde_json::from_slice(&fs::read(&managed_path)?)?;
    drifted["workflow"]["default_work_control"] = Value::String("sensitive".to_owned());
    fs::write(&managed_path, serde_json::to_vec_pretty(&drifted)?)?;
    let drifted_bytes = fs::read(&managed_path)?;
    let mismatch = fixture.run_policy_show(Some("--json"))?;
    assert_eq!(fs::read(&managed_path)?, drifted_bytes);
    assert!(mismatch.status.success(), "{}", stderr(&mismatch)?);
    let mismatch: PolicyShowReport = serde_json::from_slice(&mismatch.stdout)?;
    assert_eq!(
        mismatch.managed_file.status,
        ManagedPolicyFileStatus::FingerprintMismatch
    );
    assert!(!mismatch.managed_file.matches_authority);
    assert_eq!(mismatch.actions.len(), 1);
    assert_eq!(
        mismatch.actions[0].command,
        PolicyShowActionCommand::PolicyApply
    );
    assert_ne!(
        mismatch.managed_file.fingerprint.as_deref(),
        Some(mismatch.authority.policy_fingerprint.as_str())
    );
    assert_eq!(
        canonical_json_sha256(&mismatch.authority.policy)?.as_str(),
        mismatch.authority.policy_fingerprint
    );

    fs::write(&managed_path, b"{")?;
    let malformed_bytes = fs::read(&managed_path)?;
    let malformed = fixture.run_policy_show(Some("--json"))?;
    assert_eq!(fs::read(&managed_path)?, malformed_bytes);
    assert!(malformed.status.success(), "{}", stderr(&malformed)?);
    let malformed: PolicyShowReport = serde_json::from_slice(&malformed.stdout)?;
    assert_eq!(
        malformed.managed_file.status,
        ManagedPolicyFileStatus::Malformed
    );
    assert!(!malformed.managed_file.matches_authority);
    assert_eq!(malformed.actions.len(), 1);

    fs::remove_file(&managed_path)?;
    let missing = fixture.run_policy_show(Some("--json"))?;
    assert!(!managed_path.exists());
    assert!(missing.status.success(), "{}", stderr(&missing)?);
    let missing: PolicyShowReport = serde_json::from_slice(&missing.stdout)?;
    assert_eq!(
        missing.managed_file.status,
        ManagedPolicyFileStatus::Missing
    );
    assert!(!missing.managed_file.matches_authority);
    assert_eq!(missing.actions.len(), 1);
    Ok(())
}

#[test]
fn policy_show_fails_on_corrupt_store_authority_without_managed_file_fallback(
) -> Result<(), Box<dyn Error>> {
    let fixture = IsolatedInitFixture::new("binary-policy-corrupt-authority")?;
    assert_eq!(fixture.run(false)?.status.code(), Some(1));
    let managed_path = fixture.repo_root.join(".volicord").join("policy.json");
    let managed_before = fs::read(&managed_path)?;
    let project = fixture.only_project();
    rusqlite::Connection::open(&project.state_db_path)?.execute(
        "UPDATE project_workflow_policies SET policy_json = '{}' WHERE project_id = ?1",
        [&project.project_id],
    )?;

    let before = fixture.all_contents()?;
    let output = fixture.run_policy_show(Some("--json"))?;
    assert_eq!(fixture.all_contents()?, before);
    assert!(!output.status.success());
    assert_eq!(stdout(&output)?, "");
    let failure = stderr(&output)?;
    assert!(failure.contains("project_workflow_policies"));
    assert!(failure.contains("policy_json"));
    assert_eq!(fs::read(&managed_path)?, managed_before);
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
        Self::new_with_repo_directory(prefix, "product-repository")
    }

    fn new_with_repo_directory(prefix: &str, repo_directory: &str) -> Result<Self, Box<dyn Error>> {
        let temporary_root = TempRuntimeHome::new(prefix)?;
        let runtime_home = temporary_root.root_path().join("volicord-home");
        let codex_home = temporary_root.root_path().join("codex-home");
        let user_home = temporary_root.root_path().join("user-home");
        let empty_path = temporary_root.root_path().join("empty-path");
        let repo_root = temporary_root.root_path().join(repo_directory);
        for directory in [&codex_home, &user_home, &empty_path, &repo_root] {
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

    fn run_privacy_footprint(&self, json: bool) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .args(["doctor", "--privacy-footprint"])
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

    fn run_policy_show(
        &self,
        output_flag: Option<&str>,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("policy")
            .arg("show")
            .arg("--repo")
            .arg(&self.repo_root)
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        if let Some(output_flag) = output_flag {
            command.arg(output_flag);
        }
        Ok(command.output()?)
    }

    fn run_policy_apply(&self, file: &Path) -> Result<std::process::Output, Box<dyn Error>> {
        Ok(base_command()
            .arg("policy")
            .arg("apply")
            .arg("--repo")
            .arg(&self.repo_root)
            .arg("--file")
            .arg(file)
            .arg("--json")
            .env("VOLICORD_HOME", &self.runtime_home)
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root)
            .output()?)
    }

    fn run_policy_validate(
        &self,
        file: &Path,
        json: bool,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("policy")
            .arg("validate")
            .arg("--file")
            .arg(file)
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

    fn insert_escalating_active_task(&self) -> Result<(), Box<dyn Error>> {
        let project = self.only_project();
        let connection = rusqlite::Connection::open(&project.state_db_path)?;
        connection.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_policy_show_active', 'system', 'direct', 'auto', 'observe',
                       'Initial observe control.', 'implementation', 'not_required',
                       'Observe control needs no acceptance.', 'executing',
                       '2026-07-31T00:00:00Z', '2026-07-31T00:00:00Z')",
            [&project.project_id],
        )?;
        connection.execute(
            "UPDATE project_state
                SET active_task_id = 'task_policy_show_active'
              WHERE project_id = ?1",
            [&project.project_id],
        )?;
        Ok(())
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

    fn run_connection_list_verbose(
        &self,
        repo_root: Option<&Path>,
    ) -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = base_command();
        command
            .arg("connection")
            .arg("list")
            .arg("--home")
            .arg(&self.runtime_home)
            .arg("--verbose")
            .env("PATH", &self.empty_path)
            .env("CODEX_HOME", &self.codex_home)
            .env("HOME", &self.user_home)
            .env("USERPROFILE", &self.user_home)
            .current_dir(&self.repo_root);
        if let Some(repo_root) = repo_root {
            command.arg("--repo").arg(repo_root);
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
        directory_contents(self._temporary_root.root_path())
    }

    fn only_connection_id(&self) -> String {
        let snapshot = self.registry_snapshot();
        assert_eq!(snapshot.agent_connections.len(), 1);
        snapshot.agent_connections[0].connection_internal_id.clone()
    }

    fn create_repository(&self, name: &str) -> Result<PathBuf, Box<dyn Error>> {
        let repo_root = self._temporary_root.root_path().join(name);
        fs::create_dir_all(repo_root.join(".git"))?;
        Ok(repo_root)
    }

    fn registry_snapshot(&self) -> RegistryInspectionSnapshot {
        match inspect_runtime_home(&self.runtime_home).registry {
            DatabaseInspection::Present(snapshot) => snapshot,
            other => panic!("expected registry snapshot, got {other:?}"),
        }
    }

    fn only_project(&self) -> volicord_store::inspection::ProjectInspectionRecord {
        let snapshot = self.registry_snapshot();
        assert_eq!(snapshot.projects.len(), 1);
        snapshot.projects[0].clone()
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

fn json_value_contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(key)
                || object
                    .values()
                    .any(|nested| json_value_contains_key(nested, key))
        }
        Value::Array(values) => values
            .iter()
            .any(|nested| json_value_contains_key(nested, key)),
        _ => false,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DirectoryState {
    entries: BTreeSet<PathBuf>,
    contents: BTreeMap<PathBuf, Vec<u8>>,
    modified: BTreeMap<PathBuf, std::time::SystemTime>,
}

fn directory_state(root: &Path) -> Result<DirectoryState, Box<dyn Error>> {
    fn visit(
        root: &Path,
        current: &Path,
        modified: &mut BTreeMap<PathBuf, std::time::SystemTime>,
    ) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(root)?.to_path_buf();
            modified.insert(relative, fs::symlink_metadata(&path)?.modified()?);
            if entry.file_type()?.is_dir() {
                visit(root, &path, modified)?;
            }
        }
        Ok(())
    }

    let mut modified = BTreeMap::new();
    visit(root, root, &mut modified)?;
    Ok(DirectoryState {
        entries: directory_entries(root)?,
        contents: directory_contents(root)?,
        modified,
    })
}

fn json_string_array(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .expect("typed privacy claim array")
        .iter()
        .map(|claim| claim.as_str().expect("typed privacy claim string"))
        .collect()
}

fn human_section<'a>(text: &'a str, heading: &str, next_heading: &str) -> &'a str {
    let start = format!("\n{heading}\n");
    let end = format!("\n{next_heading}\n");
    text.split_once(&start)
        .unwrap_or_else(|| panic!("missing human section {heading}"))
        .1
        .split_once(&end)
        .unwrap_or_else(|| panic!("missing human section {next_heading}"))
        .0
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
    assert_eq!(report["operation_details"]["dry_run"], true);
    assert_eq!(
        report["operation_details"]["result"]["disposition"],
        "planned"
    );
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
