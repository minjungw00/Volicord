#![forbid(unsafe_code)]

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use volicord_core::{CoreService, InvocationContext};
use volicord_store::agent_connections::{
    agent_connection_record, list_connection_projects, CONNECTION_MODE_READ_ONLY,
    CONNECTION_MODE_WORKFLOW, HOST_KIND_GENERIC, HOST_SCOPE_EXPORT,
    VERIFIED_STATUS_ACTION_REQUIRED,
};
use volicord_store::{
    bootstrap::{
        initialize_runtime_home, list_projects, register_project, write_installation_profile,
        InstallationProfileRegistration, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::CoreProjectStore,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    ActorSource, IdempotencyKey, InitialScope, JudgmentKind, JudgmentPresentation,
    JudgmentRequiredFor, OperationCategory, ProjectId, RequestId, RequestedMode, RequiredNullable,
    ResumePolicy, StateRecordKind, StateRecordRef, TaskId, ToolEnvelope, UserJudgmentContext,
    UserJudgmentOptionId, UserJudgmentOptionInput, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

#[test]
fn binary_help_uses_agent_connection_model() -> Result<(), Box<dyn Error>> {
    let help = run_without_home(["--help"])?;
    assert_success(&help);
    let text = stdout(&help);

    assert!(text.contains("volicord setup"));
    assert!(text.contains("volicord doctor"));
    assert!(text.contains("volicord export mcp-config"));
    assert!(text.contains("volicord connect [HOST]"));
    assert!(text.contains("volicord connections [--repo PATH]"));
    assert!(text.contains("volicord connection status [HOST]"));
    assert_removed_low_level_flags_absent(&text);
    assert!(!text.contains("volicord agent connect"));
    assert!(text.contains("volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID"));
    assert!(text.contains("User Channel"));

    let setup_help = run_without_home(["setup", "--help"])?;
    assert_success(&setup_help);
    let setup_text = stdout(&setup_help);
    assert!(setup_text.contains("volicord setup"));
    assert!(setup_text.contains("--mcp-command PATH"));

    let unknown_user = run_without_home(["user", "not-a-real-command", "--repo", "."])?;
    assert_eq!(unknown_user.status.code(), Some(2));
    assert!(stderr(&unknown_user).contains("unknown user command: not-a-real-command"));

    let connect_help = run_without_home(["connect", "--help"])?;
    assert_success(&connect_help);
    let connect_text = stdout(&connect_help);
    assert!(connect_text.contains("volicord connect [HOST]"));
    assert!(connect_text.contains("--repo PATH"));
    assert!(connect_text.contains("--shared|--global"));
    assert!(connect_text.contains("--read-only"));
    assert!(!connect_text.contains("--mcp-command"));
    assert_removed_low_level_flags_absent(&connect_text);

    let connection_help = run_without_home(["connection", "status", "--help"])?;
    assert_success(&connection_help);
    let connection_text = stdout(&connection_help);
    assert!(connection_text.contains("volicord connection status [HOST]"));
    assert!(connection_text.contains("volicord connection verify [HOST]"));
    assert_removed_low_level_flags_absent(&connection_text);

    let export_help = run_without_home(["export", "mcp-config", "--help"])?;
    assert_success(&export_help);
    let export_text = stdout(&export_help);
    assert!(export_text.contains("volicord export mcp-config"));
    assert!(export_text.contains("--output PATH"));
    assert_removed_low_level_flags_absent(&export_text);
    Ok(())
}

#[cfg(unix)]
#[test]
fn setup_and_doctor_report_installation_profile() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-doctor")?;
    let bin_dir = runtime_home.path().join("bin");
    let mcp = write_fake_mcp(&bin_dir)?;

    let setup = run_setup_json(runtime_home.path(), &mcp)?;
    assert_success(&setup);
    let setup_json = json_stdout(&setup)?;
    assert_eq!(setup_json["status"], "complete");
    assert_eq!(
        setup_json["installation_profile"]["volicord_mcp_command"],
        path_text(&mcp)
    );
    assert_eq!(
        setup_json["installation_profile"]["default_connection_mode"],
        "workflow"
    );

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let doctor_json = json_stdout(&doctor)?;
    assert_eq!(doctor_json["status"], "complete");
    assert!(doctor_json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "installation_profile" && check["status"] == "passed"));
    Ok(())
}

#[test]
fn doctor_without_setup_reports_action_required() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-doctor-missing")?;

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let value = json_stdout(&doctor)?;
    assert_eq!(value["status"], "action_required");
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "run_setup"));
    Ok(())
}

#[test]
fn ordinary_command_before_setup_instructs_setup() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-required")?;

    let output = run_with_home_env(runtime_home.path(), ["project", "list"], &[])?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("run `volicord setup`"));
    Ok(())
}

#[test]
fn project_commands_use_current_git_repository_without_user_ids() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-project-lifecycle")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_project_lifecycle", "{}")?;
    write_test_installation_profile(runtime_home.path())?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let nested = repo_root.join("src/nested");
    fs::create_dir_all(&nested)?;

    let current =
        run_with_home_env_in_dir(runtime_home.path(), ["project", "current"], &[], &nested)?;
    assert_success(&current);
    assert!(stdout(&current).contains("project not registered"));
    assert!(list_projects(runtime_home.path())?.is_empty());

    let use_output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["project", "use", "--json"],
        &[],
        &nested,
    )?;
    assert_success(&use_output);
    let use_json = json_stdout(&use_output)?;
    assert_eq!(use_json["status"], "registered");
    assert_eq!(use_json["project"]["project_name"], "product-repo");
    assert_eq!(use_json["project"]["repo_root"], path_text(&repo_root));
    let project_internal_id = use_json["project"]["project_internal_id"]
        .as_str()
        .expect("project_internal_id should be present")
        .to_owned();
    assert!(project_internal_id.starts_with("prj_"));

    let projects = list_projects(runtime_home.path())?;
    assert_eq!(projects.len(), 1);
    assert!(projects[0].state_db_path.exists());

    let text_current =
        run_with_home_env_in_dir(runtime_home.path(), ["project", "current"], &[], &nested)?;
    assert_success(&text_current);
    let text = stdout(&text_current);
    assert!(text.contains("project current"));
    assert!(text.contains("name: product-repo"));
    assert!(!text.contains(&project_internal_id));
    assert!(!text.contains("project_internal_id"));

    let rename = run_with_home_env(
        runtime_home.path(),
        [
            "project",
            "rename",
            "renamed-product",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[],
    )?;
    assert_success(&rename);
    let rename_json = json_stdout(&rename)?;
    assert_eq!(rename_json["status"], "renamed");
    assert_eq!(rename_json["project"]["project_name"], "renamed-product");
    assert_eq!(
        rename_json["project"]["project_internal_id"],
        project_internal_id
    );

    let renamed_current =
        run_with_home_env_in_dir(runtime_home.path(), ["project", "current"], &[], &nested)?;
    assert_success(&renamed_current);
    let renamed_text = stdout(&renamed_current);
    assert!(renamed_text.contains("name: renamed-product"));
    assert!(!renamed_text.contains("project_internal_id"));

    let forget = run_with_home_env_in_dir(
        runtime_home.path(),
        ["project", "forget", "renamed-product", "--json"],
        &[],
        &nested,
    )?;
    assert_success(&forget);
    let forget_json = json_stdout(&forget)?;
    assert_eq!(forget_json["status"], "forgotten");
    assert_eq!(forget_json["project_state_deleted"], false);
    assert_eq!(list_projects(runtime_home.path())?.len(), 0);

    let forgotten_current =
        run_with_home_env_in_dir(runtime_home.path(), ["project", "current"], &[], &nested)?;
    assert_success(&forgotten_current);
    assert!(stdout(&forgotten_current).contains("project not registered"));
    Ok(())
}

#[test]
fn project_list_disambiguates_same_basename_repositories() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-project-duplicates")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_project_duplicates", "{}")?;
    write_test_installation_profile(runtime_home.path())?;
    let repo_a = create_git_repo(&runtime_home, "left/repo")?;
    let repo_b = create_git_repo(&runtime_home, "right/repo")?;

    let first = run_with_home_env(
        runtime_home.path(),
        [
            "project",
            "use",
            repo_a.to_str().expect("repo path should be utf8"),
            "--json",
        ],
        &[],
    )?;
    assert_success(&first);
    let second = run_with_home_env(
        runtime_home.path(),
        [
            "project",
            "use",
            repo_b.to_str().expect("repo path should be utf8"),
            "--json",
        ],
        &[],
    )?;
    assert_success(&second);
    let first_id = json_stdout(&first)?["project"]["project_internal_id"]
        .as_str()
        .expect("first id should be present")
        .to_owned();
    let second_id = json_stdout(&second)?["project"]["project_internal_id"]
        .as_str()
        .expect("second id should be present")
        .to_owned();

    let list = run_with_home_env(runtime_home.path(), ["project", "list"], &[])?;
    assert_success(&list);
    let text = stdout(&list);
    assert!(text.contains(&format!("repo\t{}\tactive", path_text(&repo_a))));
    assert!(text.contains(&format!("repo\t{}\tactive", path_text(&repo_b))));
    assert!(!text.contains(&first_id));
    assert!(!text.contains(&second_id));

    let json_list = run_with_home_env(runtime_home.path(), ["project", "list", "--json"], &[])?;
    assert_success(&json_list);
    assert!(stdout(&json_list).contains("project_internal_id"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connect_respects_explicit_read_only_and_uses_same_dry_run_plan() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-read-only")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let dry_run = run_with_home_env(
        runtime_home.path(),
        [
            "connect",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--read-only",
            "--dry-run",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;
    assert_success(&dry_run);
    let dry_run_json = json_stdout(&dry_run)?;
    assert_eq!(dry_run_json["status"], "dry_run");
    assert_eq!(
        dry_run_json["connection"]["mode"],
        CONNECTION_MODE_READ_ONLY
    );
    assert_eq!(dry_run_json["planned_change"], "create");
    assert_eq!(list_projects(runtime_home.path())?.len(), 0);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connect",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--read-only",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    let connection = &value["connection"];
    let connection_id = connection["connection_id"]
        .as_str()
        .expect("connection_id should be present");

    assert_eq!(connection["mode"], CONNECTION_MODE_READ_ONLY);
    assert_eq!(connection["host_kind"], "codex");
    assert_eq!(connection["host_scope"], "project");
    assert_eq!(value["target"], dry_run_json["target"]);
    assert_eq!(value["planned_change"], dry_run_json["planned_change"]);
    assert_eq!(value["status"], "action_required");
    assert_eq!(
        connection["verification_status"],
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    assert_eq!(
        value["verification"]["status"],
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    assert_eq!(value["verification"]["preflight"]["status"], "passed");
    assert_eq!(value["verification"]["mcp_handshake"]["status"], "passed");
    assert_eq!(
        connection["verification_report"]["status"],
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    assert_eq!(
        connection["verification_report"]["preflight"]["status"],
        "passed"
    );
    assert_eq!(
        connection["verification_report"]["mcp_handshake"]["status"],
        "passed"
    );
    assert!(connection["verification_report"]["tools"]
        .as_array()
        .expect("stored verification tools should be an array")
        .iter()
        .any(|tool| tool == "volicord.check_close"));

    let record = agent_connection_record(runtime_home.path(), connection_id)?
        .expect("connection should be stored");
    assert_eq!(record.mode, CONNECTION_MODE_READ_ONLY);
    assert_eq!(
        record.last_verification_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    let stored_report: Value = serde_json::from_str(&record.last_verification_report_json)?;
    assert_eq!(stored_report["status"], VERIFIED_STATUS_ACTION_REQUIRED);
    assert_eq!(stored_report["preflight"]["status"], "passed");
    assert_eq!(stored_report["mcp_handshake"]["status"], "passed");
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project.repo_root, repo_root);

    let config = fs::read_to_string(repo_root.join(".codex").join("config.toml"))?;
    assert!(config.contains(&format!("args = [\"--connection\", \"{connection_id}\"]")));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connect_defaults_to_workflow_mode() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-workflow")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    let nested = repo_root.join("src/app");
    fs::create_dir_all(&nested)?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["connect", "codex", "--json"],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
        &nested,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");

    assert_eq!(value["connection"]["mode"], CONNECTION_MODE_WORKFLOW);
    assert_eq!(value["connection"]["host_kind"], "codex");
    assert_eq!(value["connection"]["host_scope"], "user");
    assert_eq!(value["status"], "complete");
    let record = agent_connection_record(runtime_home.path(), connection_id)?
        .expect("connection should be stored");
    assert_eq!(record.mode, CONNECTION_MODE_WORKFLOW);
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project.repo_root, repo_root);
    Ok(())
}

#[cfg(unix)]
#[test]
fn export_mcp_config_writes_default_path_and_connection_context() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-export-default")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["export", "mcp-config", "--json"],
        &[],
        &repo_root,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    let output_path = repo_root.join("volicord.mcp.json");
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");

    assert_eq!(value["output_path"], path_text(&output_path));
    assert_eq!(value["mode"], CONNECTION_MODE_WORKFLOW);
    assert_eq!(value["connection"]["status"], "created");
    assert_eq!(value["connection"]["host_kind"], HOST_KIND_GENERIC);
    assert_eq!(value["connection"]["host_scope"], HOST_SCOPE_EXPORT);

    let config: Value = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    let server = &config["mcpServers"]["volicord"];
    assert_eq!(server["command"], path_text(&mcp));
    assert_eq!(
        server["args"],
        serde_json::json!(["--connection", connection_id])
    );
    assert_eq!(
        server["env"]["VOLICORD_HOME"],
        path_text(runtime_home.path())
    );

    let record = agent_connection_record(runtime_home.path(), connection_id)?
        .expect("generic export connection should be stored");
    assert_eq!(record.host_kind, HOST_KIND_GENERIC);
    assert_eq!(record.host_scope, HOST_SCOPE_EXPORT);
    assert_eq!(record.mode, CONNECTION_MODE_WORKFLOW);
    assert_eq!(record.config_target, path_text(&output_path));
    assert_eq!(
        record.last_verification_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project.repo_root, repo_root);
    Ok(())
}

#[cfg(unix)]
#[test]
fn export_mcp_config_explicit_output_read_only_reuses_connection() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-export-explicit")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);
    let output_path = runtime_home.path().join("exports").join("custom.mcp.json");

    let first = run_with_home_env(
        runtime_home.path(),
        [
            "export",
            "mcp-config",
            "--repo",
            path_text(&repo_root).as_str(),
            "--output",
            path_text(&output_path).as_str(),
            "--read-only",
            "--json",
        ],
        &[],
    )?;
    assert_success(&first);
    let first_json = json_stdout(&first)?;
    let connection_id = first_json["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present")
        .to_owned();
    assert_eq!(first_json["output_path"], path_text(&output_path));
    assert_eq!(first_json["mode"], CONNECTION_MODE_READ_ONLY);
    assert_eq!(first_json["connection"]["status"], "created");

    let second = run_with_home_env(
        runtime_home.path(),
        [
            "export",
            "mcp-config",
            "--repo",
            path_text(&repo_root).as_str(),
            "--output",
            path_text(&output_path).as_str(),
            "--read-only",
            "--json",
        ],
        &[],
    )?;
    assert_success(&second);
    let second_json = json_stdout(&second)?;
    assert_eq!(
        second_json["connection"]["connection_id"],
        connection_id.as_str()
    );
    assert_eq!(second_json["connection"]["status"], "reused");
    assert_eq!(
        agent_connection_record(runtime_home.path(), &connection_id)?
            .expect("connection should remain")
            .mode,
        CONNECTION_MODE_READ_ONLY
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_mode_and_remove_use_natural_selectors() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-lifecycle")?;
    let repo_a = runtime_home.create_product_repo("product-a")?;
    let repo_b = runtime_home.create_product_repo("product-b")?;
    fs::create_dir_all(repo_a.join(".git"))?;
    fs::create_dir_all(repo_b.join(".git"))?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    let mcp = write_fake_mcp(&bin_dir)?;
    write_fake_codex(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let connect = run_with_home_env(
        runtime_home.path(),
        [
            "connect",
            "codex",
            "--repo",
            path_text(&repo_a).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&connect);
    let connect_json = json_stdout(&connect)?;
    let connection_id = connect_json["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present")
        .to_owned();
    assert_eq!(connect_json["status"], "complete");

    let connect_second = run_with_home_env(
        runtime_home.path(),
        [
            "connect",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&connect_second);
    assert_eq!(
        json_stdout(&connect_second)?["connection"]["connection_id"],
        connection_id
    );
    assert_eq!(
        list_connection_projects(runtime_home.path(), &connection_id)?.len(),
        2
    );

    let connections = run_with_home_env(
        runtime_home.path(),
        ["connections", "--repo", path_text(&repo_a).as_str()],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&connections);
    let connections_text = stdout(&connections);
    assert!(connections_text.contains("codex\tpersonal\tworkflow"));
    assert!(connections_text.contains(&path_text(&repo_a)));
    assert!(connections_text.contains(&path_text(&repo_b)));
    assert!(!connections_text.contains(&connection_id));

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_a).as_str(),
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    assert_eq!(
        json_stdout(&status)?["connection"]["connection_id"],
        connection_id
    );

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--repo",
            path_text(&repo_a).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&verify);
    let verify_json = json_stdout(&verify)?;
    assert_eq!(verify_json["status"], "complete");
    assert!(verify_json["verification"]["tools"]
        .as_array()
        .expect("verified tools should be an array")
        .iter()
        .any(|tool| tool == "volicord.check_close"));

    let mode = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "mode",
            "codex",
            "read-only",
            "--repo",
            path_text(&repo_a).as_str(),
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&mode);
    let mode_json = json_stdout(&mode)?;
    assert_eq!(mode_json["connection"]["mode"], CONNECTION_MODE_READ_ONLY);
    assert!(mode_json["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "reload_required"));
    assert_eq!(
        agent_connection_record(runtime_home.path(), &connection_id)?
            .expect("connection should remain")
            .mode,
        CONNECTION_MODE_READ_ONLY
    );

    let remove_dry_run = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "remove",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
            "--dry-run",
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&remove_dry_run);
    assert_eq!(
        json_stdout(&remove_dry_run)?["planned_change"],
        "membership"
    );

    let remove = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "remove",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&remove);
    assert_eq!(
        list_connection_projects(runtime_home.path(), &connection_id)?.len(),
        1
    );

    let remove_last = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "remove",
            "codex",
            "--repo",
            path_text(&repo_a).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&remove_last);
    assert!(agent_connection_record(runtime_home.path(), &connection_id)?.is_none());
    let config = fs::read_to_string(codex_home.join("config.toml"))?;
    assert!(!config.contains(&connection_id));
    Ok(())
}

#[cfg(unix)]
#[test]
fn ambiguous_connection_selector_reports_actionable_choices() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-ambiguous")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home_a = runtime_home.path().join("codex-a");
    let codex_home_b = runtime_home.path().join("codex-b");
    let mcp = write_fake_mcp(&bin_dir)?;
    write_fake_codex(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    for codex_home in [&codex_home_a, &codex_home_b] {
        let connect = run_with_home_env(
            runtime_home.path(),
            [
                "connect",
                "codex",
                "--repo",
                path_text(&repo_root).as_str(),
                "--json",
            ],
            &[
                ("PATH", path_env(&[bin_dir.as_path()])),
                ("CODEX_HOME", path_text(codex_home)),
                ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
            ],
        )?;
        assert_success(&connect);
    }

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &[],
    )?;

    assert_eq!(status.status.code(), Some(1));
    let diagnostic = stderr(&status);
    assert!(diagnostic.contains("connection selector is ambiguous"));
    assert!(diagnostic.contains("choices:"));
    assert!(diagnostic.contains(&path_text(&codex_home_a.join("config.toml"))));
    assert!(diagnostic.contains(&path_text(&codex_home_b.join("config.toml"))));
    Ok(())
}

#[test]
fn user_channel_records_pending_judgment_with_local_user_provenance() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-user-channel")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_user_channel", "{}")?;
    write_test_installation_profile(runtime_home.path())?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "project_user_channel".to_owned(),
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let service = CoreService::new(runtime_home.path());
    let intake = service.intake(
        intake_request("req_cli_user_intake", "idem_cli_user_intake", Some(0)),
        core_invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = record_id(&intake.response_value["task_ref"])?;
    let judgment = service.request_user_judgment(
        request_user_judgment_request(
            "req_cli_user_judgment",
            "idem_cli_user_judgment",
            Some(1),
            &task_id,
        ),
        core_invocation(OperationCategory::AgentWorkflow),
    )?;
    let judgment_id = record_id(&judgment.response_value["user_judgment_ref"])?;

    let status =
        run_with_home_env_in_dir(runtime_home.path(), ["user", "status"], &[], &repo_root)?;
    assert_success(&status);
    assert!(stdout(&status).contains("User Channel status"));
    assert!(stdout(&status).contains("pending judgments: 1"));

    let list =
        run_with_home_env_in_dir(runtime_home.path(), ["user", "judgments"], &[], &repo_root)?;
    assert_success(&list);
    let list_text = stdout(&list);
    assert!(list_text.contains("Pending judgments"));
    assert!(list_text.contains("1. Should the focused CLI user-channel choice be accepted?"));
    assert!(list_text.contains("1. Accept focused choice (accepted)"));
    assert!(!list_text.contains("project_user_channel"));
    assert!(!list_text.contains(judgment_id.as_str()));

    let list_json = run_with_home_env_in_dir(
        runtime_home.path(),
        ["user", "judgments", "--json"],
        &[],
        &repo_root,
    )?;
    assert_success(&list_json);
    let list_value = json_stdout(&list_json)?;
    let first = &list_value["pending_user_judgments"][0];
    assert_eq!(first["index"], 1);
    assert_eq!(first["project_internal_id"], "project_user_channel");
    assert_eq!(first["judgment_id"], judgment_id.as_str());
    assert_eq!(first["options"][0]["index"], 1);
    assert_eq!(first["options"][0]["option_id"], "accept");

    let show = run_with_home_env_in_dir(
        runtime_home.path(),
        ["user", "judgment", "show", "1"],
        &[],
        &repo_root,
    )?;
    assert_success(&show);
    let show_text = stdout(&show);
    assert!(show_text.contains("User judgment 1"));
    assert!(show_text.contains("1. Accept focused choice (accepted)"));
    assert!(!show_text.contains("project_user_channel"));
    assert!(!show_text.contains(judgment_id.as_str()));

    let record_note = "Recorded from numbered CLI";
    let record = run_with_home_env_in_dir(
        runtime_home.path(),
        [
            "user",
            "judgment",
            "answer",
            "1",
            "1",
            "--note",
            record_note,
        ],
        &[],
        &repo_root,
    )?;
    assert_success(&record);
    let text = stdout(&record);
    assert!(text.contains("User Channel judgment recorded"));
    assert!(text.contains("selected: Accept focused choice"));
    assert!(text.contains("outcome: accepted"));
    assert!(!text.contains("project_user_channel"));
    assert!(!text.contains(judgment_id.as_str()));
    assert!(!text.contains("operation_category"));

    let store =
        CoreProjectStore::open(runtime_home.path(), &ProjectId::new("project_user_channel"))?;
    let persisted = store
        .user_judgment_record(&judgment_id)?
        .expect("recorded judgment should be stored");
    assert_eq!(persisted.status, "resolved");
    assert_eq!(
        persisted.resolved_by_actor_source.as_deref(),
        Some("local_user")
    );
    assert_eq!(
        persisted.resolved_verification_basis.as_deref(),
        Some("cli_direct_user_channel")
    );
    assert_eq!(
        persisted.resolved_assurance_level.as_deref(),
        Some("local_user_channel")
    );
    let resolution_json: Value = serde_json::from_str(
        persisted
            .resolution_json
            .as_deref()
            .expect("resolution_json should be stored"),
    )?;
    assert_eq!(resolution_json["note"], record_note);
    Ok(())
}

fn run_without_home<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin()).args(args).output()?)
}

fn assert_removed_low_level_flags_absent(text: &str) {
    for removed in [
        "--export-path",
        "--export-dir",
        "--connection-id ID",
        "--mode read_only|workflow",
        "--repo-root PATH",
        "--server-name",
        "--allow-repository-write",
        "--replace-managed",
        "--project-id ID",
    ] {
        assert!(
            !text.contains(removed),
            "help output should not contain removed low-level flag {removed}:\n{text}"
        );
    }
}

fn run_with_home_env<const N: usize>(
    runtime_home: &Path,
    args: [&str; N],
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command.args(args).env("VOLICORD_HOME", runtime_home);
    for (name, value) in envs {
        command.env(name, value);
    }
    Ok(command.output()?)
}

fn run_with_home_env_in_dir<const N: usize>(
    runtime_home: &Path,
    args: [&str; N],
    envs: &[(&str, String)],
    current_dir: &Path,
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir);
    for (name, value) in envs {
        command.env(name, value);
    }
    Ok(command.output()?)
}

fn run_setup(runtime_home: &Path, mcp_command: &Path) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command
        .args(["setup", "--mcp-command"])
        .arg(mcp_command)
        .env("VOLICORD_HOME", runtime_home);
    Ok(command.output()?)
}

fn run_setup_json(runtime_home: &Path, mcp_command: &Path) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(volicord_bin());
    command
        .args(["setup", "--mcp-command"])
        .arg(mcp_command)
        .arg("--json")
        .env("VOLICORD_HOME", runtime_home);
    Ok(command.output()?)
}

fn volicord_bin() -> &'static str {
    env!("CARGO_BIN_EXE_volicord")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json_stdout(output: &Output) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&stdout(output))?)
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

fn write_test_installation_profile(runtime_home: &Path) -> Result<(), Box<dyn Error>> {
    write_installation_profile(
        runtime_home,
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: "volicord".to_owned(),
            volicord_mcp_command: "volicord-mcp".to_owned(),
            bin_dir: runtime_home.join("bin"),
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn create_git_repo(
    runtime_home: &TempRuntimeHome,
    name: impl AsRef<Path>,
) -> Result<PathBuf, Box<dyn Error>> {
    let repo_root = runtime_home.create_product_repo(name)?;
    fs::create_dir_all(repo_root.join(".git"))?;
    Ok(repo_root)
}

#[cfg(unix)]
fn path_env(path_dirs: &[&Path]) -> String {
    std::env::join_paths(path_dirs)
        .expect("test PATH should be valid")
        .to_string_lossy()
        .into_owned()
}

#[cfg(unix)]
fn write_fake_codex(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("codex");
    fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'codex 1.2.3-test\\n'; exit 0; fi\nprintf 'unexpected codex invocation\\n' >&2\nexit 2\n",
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn write_fake_mcp(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("volicord-mcp");
    fs::write(
        &path,
        "#!/bin/sh\n\
         mode=\"${VOLICORD_TEST_CONNECTION_MODE:-read_only}\"\n\
         if [ \"$1\" = \"--check\" ]; then\n\
         shift\n\
         if [ \"$1\" != \"--connection\" ]; then printf 'missing connection\\n' >&2; exit 2; fi\n\
         connection=\"$2\"\n\
         printf 'configuration: valid\\n'\n\
         printf 'transport: stdio\\n'\n\
         printf 'runtime_home: %s\\n' \"$VOLICORD_HOME\"\n\
         printf 'connection_id: %s\\n' \"$connection\"\n\
         printf 'mode: %s\\n' \"$mode\"\n\
         printf 'enabled: true\\n'\n\
         printf 'allowed_projects: 1\\n'\n\
         printf 'available_projects: 1\\n'\n\
         printf 'verification_scope: startup_check_only\\n'\n\
         exit 0\n\
         fi\n\
         if [ \"$1\" = \"--connection\" ]; then\n\
         while IFS= read -r line; do\n\
         case \"$line\" in\n\
         *'\"method\":\"initialize\"'*) printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{\"tools\":{}},\"serverInfo\":{\"name\":\"volicord-mcp\",\"version\":\"test\"},\"instructions\":\"Use Volicord.\"}}' ;;\n\
         *'\"method\":\"tools/list\"'*)\n\
         if [ \"$mode\" = \"workflow\" ]; then\n\
         printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"volicord.intake\"},{\"name\":\"volicord.update_scope\"},{\"name\":\"volicord.status\"},{\"name\":\"volicord.prepare_write\"},{\"name\":\"volicord.stage_artifact\"},{\"name\":\"volicord.record_run\"},{\"name\":\"volicord.request_user_judgment\"},{\"name\":\"volicord.check_close\"},{\"name\":\"volicord.close_task\"},{\"name\":\"volicord.list_projects\"}]}}'\n\
         else\n\
         printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"volicord.status\"},{\"name\":\"volicord.check_close\"},{\"name\":\"volicord.list_projects\"}]}}'\n\
         fi\n\
         exit 0 ;;\n\
         esac\n\
         done\n\
         exit 0\n\
         fi\n\
         printf 'unexpected invocation\\n' >&2\n\
         exit 2\n",
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn intake_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
) -> volicord_types::IntakeRequest {
    volicord_types::IntakeRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            expected_state_version,
            None,
        ),
        plain_language_request: "Create a focused CLI user-channel test task.".to_owned(),
        requested_mode: RequestedMode::Work,
        resume_policy: ResumePolicy::CreateNew,
        initial_scope: InitialScope {
            boundary: "Exercise the local User Channel.".to_owned(),
            non_goals: vec!["Changing unrelated CLI behavior.".to_owned()],
            acceptance_criteria: vec!["The pending judgment can be recorded locally.".to_owned()],
        },
        initial_context_refs: Vec::new(),
    }
}

fn request_user_judgment_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: &str,
) -> volicord_types::RequestUserJudgmentRequest {
    volicord_types::RequestUserJudgmentRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: RequiredNullable::null(),
        sensitive_action_scope: RequiredNullable::null(),
        judgment_kind: JudgmentKind::ProductDecision,
        presentation: JudgmentPresentation::Short,
        question: "Should the focused CLI user-channel choice be accepted?".to_owned(),
        options: Some(vec![UserJudgmentOptionInput {
            option_id: UserJudgmentOptionId::new("accept"),
            label: "Accept focused choice".to_owned(),
            description: "Record the focused user-owned choice.".to_owned(),
            consequence: "Only this judgment is resolved.".to_owned(),
            is_default: true,
        }])
        .into(),
        context: UserJudgmentContext {
            summary: "The CLI needs a pending judgment to record.".to_owned(),
            related_refs: Vec::new(),
            artifact_refs: Vec::new(),
            visible_risks: Vec::new(),
            constraints: vec!["This choice does not imply broader acceptance.".to_owned()],
        },
        affected_refs: vec![StateRecordRef {
            record_kind: StateRecordKind::Task,
            record_id: volicord_types::RecordId::new(task_id),
            project_id: ProjectId::new("project_user_channel"),
            task_id: Some(TaskId::new(task_id)).into(),
            state_version: expected_state_version.into(),
        }],
        required_for: vec![JudgmentRequiredFor::Informational],
        expires_at: RequiredNullable::null(),
    }
}

fn envelope(
    request_id: &str,
    idempotency_key: Option<&str>,
    expected_state_version: Option<u64>,
    task_id: Option<&str>,
) -> ToolEnvelope {
    ToolEnvelope {
        project_id: ProjectId::new("project_user_channel"),
        task_id: task_id.map(TaskId::new).into(),
        request_id: RequestId::new(request_id),
        idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
        expected_state_version: expected_state_version.into(),
        dry_run: false,
        locale: None.into(),
    }
}

fn core_invocation(operation_category: OperationCategory) -> InvocationContext {
    let actor_source = match operation_category {
        OperationCategory::Read | OperationCategory::AgentWorkflow => {
            ActorSource::agent_connection("connection_cli_user_channel")
        }
        OperationCategory::UserOnly | OperationCategory::AdminLocal => ActorSource::LocalUser,
    };
    InvocationContext::new(
        ProjectId::new("project_user_channel"),
        actor_source,
        operation_category,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    )
}

fn record_id(value: &Value) -> Result<String, Box<dyn Error>> {
    value["record_id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "record_id should be present".into())
}
