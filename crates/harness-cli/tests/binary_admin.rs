#![forbid(unsafe_code)]

use std::{
    error::Error,
    ffi::OsString,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use harness_cli::{
    registration::{baseline_workflow_access_classes, capability_profile_json, local_access_json},
    setup::{
        AGENT_SURFACE_ID as SETUP_AGENT_SURFACE_ID,
        AGENT_SURFACE_INSTANCE_ID as SETUP_AGENT_SURFACE_INSTANCE_ID,
        USER_INTERACTION_SURFACE_ID as SETUP_USER_SURFACE_ID,
        USER_INTERACTION_SURFACE_INSTANCE_ID as SETUP_USER_INSTANCE_ID,
    },
};
use harness_store::{
    agent_integrations::{
        agent_integration_record, list_host_installations_for_integration,
        VERIFIED_STATUS_ACTION_REQUIRED, VERIFIED_STATUS_COMPLETE, VERIFIED_STATUS_PARTIAL_FAILURE,
    },
    bootstrap::{
        initialize_runtime_home, list_projects, list_surfaces, register_project, ProjectRecord,
        ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    migrations::{
        test_support::{
            create_project_state_fixture_version, create_registry_fixture_version,
            RegistryFixtureProject,
        },
        PROJECT_STATE_DATABASE_KIND, PROJECT_STATE_SCHEMA_VERSION, REGISTRY_DATABASE_KIND,
        REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE,
    },
    sqlite::{open_read_only_database, project_state_db_path, registry_db_path},
};
use harness_test_support::TempRuntimeHome;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

const PROJECT_ID: &str = "project_binary_admin";
const AGENT_SURFACE_ID: &str = "surface_binary_agent";
const AGENT_INSTANCE_ID: &str = "surface_instance_binary_agent";
const USER_SURFACE_ID: &str = "surface_binary_user";
const USER_INSTANCE_ID: &str = "surface_instance_binary_user";
const LEGACY_PROJECT_STATE_SCHEMA_VERSION: i64 = 5;
const GUIDANCE_BEGIN_MARKER: &str = "<!-- BEGIN HARNESS MANAGED GUIDANCE v1 -->";

#[test]
fn harness_binary_runs_administrative_initialization_and_registration() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-admin")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let repo_root_text = path_text(&repo_root);

    let help = run_without_home(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("harness init"));
    assert!(stdout(&help).contains("harness setup local-mcp"));

    let version = run_without_home(["--version"])?;
    assert_success(&version);
    assert!(stdout(&version).starts_with("harness "));

    let init = run_with_home(
        runtime_home.path(),
        ["init", "--runtime-home-id", "runtime_home_binary_admin"],
    )?;
    assert_success(&init);
    assert!(stdout(&init).contains("runtime_home initialized"));

    let project = run_with_home(
        runtime_home.path(),
        [
            "project",
            "register",
            "--project-id",
            PROJECT_ID,
            "--repo-root",
            repo_root_text.as_str(),
        ],
    )?;
    assert_success(&project);
    assert!(stdout(&project).contains("project registered"));

    let projects = run_with_home(runtime_home.path(), ["project", "list"])?;
    assert_success(&projects);
    assert!(stdout(&projects).contains(PROJECT_ID));

    let agent_surface = run_with_home(
        runtime_home.path(),
        [
            "surface",
            "register",
            "--project-id",
            PROJECT_ID,
            "--surface-id",
            AGENT_SURFACE_ID,
            "--surface-instance-id",
            AGENT_INSTANCE_ID,
            "--kind",
            "mcp",
            "--interaction-role",
            "agent",
            "--profile",
            "baseline-workflow",
        ],
    )?;
    assert_success(&agent_surface);
    assert!(stdout(&agent_surface).contains(AGENT_INSTANCE_ID));

    let user_surface = run_with_home(
        runtime_home.path(),
        [
            "surface",
            "register",
            "--project-id",
            PROJECT_ID,
            "--surface-id",
            USER_SURFACE_ID,
            "--surface-instance-id",
            USER_INSTANCE_ID,
            "--kind",
            "mcp",
            "--interaction-role",
            "user_interaction",
            "--access-class",
            "read_status",
            "--access-class",
            "core_mutation",
        ],
    )?;
    assert_success(&user_surface);
    assert!(stdout(&user_surface).contains(USER_INSTANCE_ID));

    let surfaces = run_with_home(
        runtime_home.path(),
        ["surface", "list", "--project-id", PROJECT_ID],
    )?;
    assert_success(&surfaces);
    assert!(stdout(&surfaces).contains(AGENT_SURFACE_ID));
    assert!(stdout(&surfaces).contains(USER_SURFACE_ID));

    assert!(registry_db_path(runtime_home.path()).exists());
    assert!(project_state_db_path(runtime_home.path(), PROJECT_ID).exists());

    let project_records = list_projects(runtime_home.path())?;
    assert_eq!(project_records.len(), 1);
    assert_eq!(project_records[0].project_id, PROJECT_ID);
    assert_eq!(project_records[0].repo_root, fs::canonicalize(&repo_root)?);
    assert_eq!(project_records[0].status, "active");

    let surface_records = list_surfaces(runtime_home.path(), PROJECT_ID)?;
    assert_eq!(surface_records.len(), 2);

    let agent = surface_records
        .iter()
        .find(|surface| surface.surface_instance_id == AGENT_INSTANCE_ID)
        .expect("agent surface should be registered");
    assert_eq!(agent.surface_id, AGENT_SURFACE_ID);
    assert_eq!(agent.surface_kind, "mcp");
    assert_eq!(agent.interaction_role, "agent");
    assert_eq!(
        access_classes(&agent.local_access_json)?,
        json!([
            "read_status",
            "core_mutation",
            "write_authorization",
            "artifact_registration",
            "run_recording"
        ])
    );

    let user = surface_records
        .iter()
        .find(|surface| surface.surface_instance_id == USER_INSTANCE_ID)
        .expect("user-interaction surface should be registered");
    assert_eq!(user.surface_id, USER_SURFACE_ID);
    assert_eq!(user.surface_kind, "mcp");
    assert_eq!(user.interaction_role, "user_interaction");
    assert_eq!(
        access_classes(&user.local_access_json)?,
        json!(["read_status", "core_mutation"])
    );

    let invalid = run_without_home(["init", "--not-a-real-option", "value"])?;
    assert_eq!(invalid.status.code(), Some(2));
    assert!(stderr(&invalid).contains("unknown option"));

    let blocked_runtime_home = runtime_home.path().join("runtime-home-file");
    fs::write(&blocked_runtime_home, "not a directory")?;
    let runtime_failure = run_with_home(&blocked_runtime_home, ["init"])?;
    assert_eq!(runtime_failure.status.code(), Some(1));
    assert!(stderr(&runtime_failure).starts_with("error:"));

    Ok(())
}

#[test]
fn harness_binary_setup_help_and_usage_errors() -> Result<(), Box<dyn Error>> {
    let setup_help = run_without_home(["setup", "--help"])?;
    assert_success(&setup_help);
    assert!(stdout(&setup_help).contains("harness setup local-mcp"));

    let local_help = run_without_home(["setup", "local-mcp", "--help"])?;
    assert_success(&local_help);
    assert!(stdout(&local_help).contains("--interactive"));
    assert!(stdout(&local_help).contains("--runtime-home PATH"));
    assert!(stdout(&local_help).contains("--repo-root PATH is required"));
    assert!(stdout(&local_help).contains("--repo-root ."));
    assert!(stdout(&local_help).contains("must be absolute"));
    assert!(stdout(&local_help).contains("--dry-run"));

    let invalid = run_without_home(["setup", "local-mcp", "--not-real"])?;
    assert_eq!(invalid.status.code(), Some(2));
    assert!(stderr(&invalid).contains("unknown option"));

    Ok(())
}

#[test]
fn harness_binary_setup_local_mcp_requires_repo_root_before_work() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-requires-repo")?;
    let selected_runtime_home = runtime_home.path().join("runtime-home");

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(&selected_runtime_home).as_str(),
        "--dry-run",
    ])?;

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("--repo-root is required"));
    assert!(!stderr(&output).contains("harness-mcp"));
    assert!(!selected_runtime_home.exists());
    assert!(!registry_db_path(&selected_runtime_home).exists());
    Ok(())
}

#[test]
fn harness_binary_project_register_rejects_invalid_project_id() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-invalid-project-id")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let init = run_with_home(
        runtime_home.path(),
        ["init", "--runtime-home-id", "runtime_home_invalid_project"],
    )?;
    assert_success(&init);

    let output = run_with_home(
        runtime_home.path(),
        [
            "project",
            "register",
            "--project-id",
            "a/b",
            "--repo-root",
            path_text(&repo_root).as_str(),
        ],
    )?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("project_id must be a single path component"));
    assert!(list_projects(runtime_home.path())?.is_empty());
    assert!(!runtime_home.path().join("projects").exists());
    Ok(())
}

#[test]
fn harness_binary_surface_commands_reject_invalid_legacy_project_paths(
) -> Result<(), Box<dyn Error>> {
    for relationship in InvalidProjectRelationship::ALL {
        let project_id = relationship.project_id("current");
        let runtime_home = initialized_project(
            &relationship.prefix("current"),
            &project_id,
            "runtime_home_cli_surface_current",
        )?;
        let initial_surface = run_with_home(
            runtime_home.path(),
            [
                "surface",
                "register",
                "--project-id",
                project_id.as_str(),
                "--surface-id",
                "surface_existing",
                "--surface-instance-id",
                "surface_instance_existing",
                "--kind",
                "mcp",
                "--profile",
                "baseline-workflow",
            ],
        )?;
        assert_success(&initial_surface);

        relationship.replace_repo_root(&runtime_home, &project_id)?;
        let registry_before = single_project_record(runtime_home.path(), &project_id)?;
        let state_path = project_state_db_path(runtime_home.path(), &project_id);
        let migrations_before = migration_count(&state_path)?;
        let surfaces_before = surface_rows(&state_path)?;

        let register = run_with_home(
            runtime_home.path(),
            [
                "surface",
                "register",
                "--project-id",
                project_id.as_str(),
                "--surface-id",
                "surface_new",
                "--surface-instance-id",
                "surface_instance_new",
            ],
        )?;
        assert_invalid_project_path_error(&register, relationship.expected_error());
        assert_eq!(migration_count(&state_path)?, migrations_before);
        assert_eq!(surface_rows(&state_path)?, surfaces_before);
        assert_registry_unchanged_and_cli_visible(
            runtime_home.path(),
            &project_id,
            &registry_before,
        )?;

        let list = run_with_home(
            runtime_home.path(),
            ["surface", "list", "--project-id", project_id.as_str()],
        )?;
        assert_invalid_project_path_error(&list, relationship.expected_error());
        assert_eq!(migration_count(&state_path)?, migrations_before);
        assert_eq!(surface_rows(&state_path)?, surfaces_before);
        assert_registry_unchanged_and_cli_visible(
            runtime_home.path(),
            &project_id,
            &registry_before,
        )?;

        let missing_project_id = relationship.project_id("missing_db");
        let missing_runtime_home = initialized_project(
            &relationship.prefix("missing-db"),
            &missing_project_id,
            "runtime_home_cli_surface_missing",
        )?;
        relationship.replace_repo_root(&missing_runtime_home, &missing_project_id)?;
        let missing_registry_before =
            single_project_record(missing_runtime_home.path(), &missing_project_id)?;
        let missing_state_path =
            project_state_db_path(missing_runtime_home.path(), &missing_project_id);
        fs::remove_file(&missing_state_path)?;
        assert!(!missing_state_path.exists());

        let missing_list = run_with_home(
            missing_runtime_home.path(),
            [
                "surface",
                "list",
                "--project-id",
                missing_project_id.as_str(),
            ],
        )?;
        assert_invalid_project_path_error(&missing_list, relationship.expected_error());
        assert!(!missing_state_path.exists());
        assert_registry_unchanged_and_cli_visible(
            missing_runtime_home.path(),
            &missing_project_id,
            &missing_registry_before,
        )?;

        let missing_register = run_with_home(
            missing_runtime_home.path(),
            [
                "surface",
                "register",
                "--project-id",
                missing_project_id.as_str(),
                "--surface-id",
                "surface_missing",
                "--surface-instance-id",
                "surface_instance_missing",
            ],
        )?;
        assert_invalid_project_path_error(&missing_register, relationship.expected_error());
        assert!(!missing_state_path.exists());
        assert_registry_unchanged_and_cli_visible(
            missing_runtime_home.path(),
            &missing_project_id,
            &missing_registry_before,
        )?;

        let historical_project_id = relationship.project_id("historical");
        let historical_runtime_home = initialized_project(
            &relationship.prefix("historical"),
            &historical_project_id,
            "runtime_home_cli_surface_historical",
        )?;
        relationship.replace_repo_root(&historical_runtime_home, &historical_project_id)?;
        let historical_registry_before =
            single_project_record(historical_runtime_home.path(), &historical_project_id)?;
        let historical_state_path =
            project_state_db_path(historical_runtime_home.path(), &historical_project_id);
        fs::remove_file(&historical_state_path)?;
        let mut historical = Connection::open(&historical_state_path)?;
        create_project_state_fixture_version(
            &mut historical,
            &historical_project_id,
            LEGACY_PROJECT_STATE_SCHEMA_VERSION,
        )?;
        drop(historical);
        let historical_migrations_before = migration_count(&historical_state_path)?;
        let historical_surface_count_before = surface_count(&historical_state_path)?;
        assert!(!column_exists(
            &historical_state_path,
            "project_state",
            "enforcement_profile_json"
        )?);
        assert!(!column_exists(
            &historical_state_path,
            "surfaces",
            "interaction_role"
        )?);

        let historical_list = run_with_home(
            historical_runtime_home.path(),
            [
                "surface",
                "list",
                "--project-id",
                historical_project_id.as_str(),
            ],
        )?;
        assert_invalid_project_path_error(&historical_list, relationship.expected_error());
        assert_historical_project_state_unchanged(
            &historical_state_path,
            historical_migrations_before,
            historical_surface_count_before,
        )?;

        let historical_register = run_with_home(
            historical_runtime_home.path(),
            [
                "surface",
                "register",
                "--project-id",
                historical_project_id.as_str(),
                "--surface-id",
                "surface_historical",
                "--surface-instance-id",
                "surface_instance_historical",
            ],
        )?;
        assert_invalid_project_path_error(&historical_register, relationship.expected_error());
        assert_historical_project_state_unchanged(
            &historical_state_path,
            historical_migrations_before,
            historical_surface_count_before,
        )?;
        assert_registry_unchanged_and_cli_visible(
            historical_runtime_home.path(),
            &historical_project_id,
            &historical_registry_before,
        )?;
    }

    Ok(())
}

#[test]
fn harness_binary_interactive_rejects_non_terminal_input() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(["setup", "local-mcp", "--interactive"])?;

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("requires terminal input"));
    Ok(())
}

#[test]
fn harness_binary_json_dry_run_is_parseable_and_does_not_register() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-dry-run")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let mcp_command = runtime_home.path().join("harness-mcp");
    fs::write(&mcp_command, "not executed during dry run")?;

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--dry-run",
        "--output",
        "json",
    ])?;
    assert_success(&output);
    let value: Value = serde_json::from_str(&stdout(&output))?;

    assert_eq!(value["status"], "dry_run");
    assert_eq!(value["preflight"][0]["status"], "planned");
    assert!(!registry_db_path(runtime_home.path()).exists());
    Ok(())
}

#[test]
fn harness_binary_setup_config_file_ancestor_fails_before_runtime_home_creation(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-config-ancestor")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let mcp_command = runtime_home.path().join("harness-mcp");
    let output_root = runtime_home.path().join("output-root");
    fs::write(&mcp_command, "not executed")?;
    fs::write(&output_root, "not a directory")?;

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--config-dir",
        path_text(&output_root.join("mcp")).as_str(),
    ])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("configuration ancestor is not a directory"));
    assert!(!registry_db_path(runtime_home.path()).exists());
    assert!(!output_root.join("mcp").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_historical_setup_dry_run_and_real_execution() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-historical")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let marker = runtime_home.path().join("preflight-marker.txt");
    initialize_historical_setup(runtime_home.path(), &repo_root, "product-repo")?;
    let state_path = project_state_db_path(runtime_home.path(), "product-repo");
    let before_migrations = migration_count(&state_path)?;
    let before_hash = file_hash(&state_path)?;
    let mcp_command = write_test_mcp(runtime_home.path(), &marker)?;

    let dry_run = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--dry-run",
    ])?;
    assert_success(&dry_run);
    assert!(stdout(&dry_run).contains("setup: dry_run"));
    assert_eq!(migration_count(&state_path)?, before_migrations);
    assert_eq!(file_hash(&state_path)?, before_hash);
    assert!(!marker.exists());

    let first = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
    ])?;
    assert_success(&first);
    assert!(stdout(&first).contains("setup: complete"));
    assert_eq!(migration_count(&state_path)?, PROJECT_STATE_SCHEMA_VERSION);
    assert_eq!(fs::read_to_string(&marker)?.lines().count(), 1);

    let repeated = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
    ])?;
    assert_success(&repeated);
    assert!(stdout(&repeated).contains("project: reused"));
    assert!(stdout(&repeated).contains("agent_surface: reused"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_local_mcp_setup_flow() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-real")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let config_dir = runtime_home.path().join("configs");
    let marker = runtime_home.path().join("preflight-marker.txt");
    let mcp_command = write_test_mcp(runtime_home.path(), &marker)?;

    let first = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
    ])?;
    assert_success(&first);
    assert!(stdout(&first).contains("setup: complete"));
    assert!(stdout(&first).contains("preflight: passed"));
    assert!(stdout(&first).contains("agent_surface: created"));
    assert_eq!(fs::read_to_string(&marker)?.lines().count(), 1);

    let state_version_before = state_version(runtime_home.path(), "product-repo")?;
    let repeated = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
    ])?;
    assert_success(&repeated);
    assert!(stdout(&repeated).contains("project: reused"));
    assert!(stdout(&repeated).contains("agent_surface: reused"));
    assert_eq!(
        state_version(runtime_home.path(), "product-repo")?,
        state_version_before
    );

    let with_user_and_files = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--with-user-interaction",
        "--config-dir",
        path_text(&config_dir).as_str(),
    ])?;
    assert_success(&with_user_and_files);
    assert!(stdout(&with_user_and_files).contains("user_interaction_preflight: passed"));
    let agent_config = config_dir.join("harness-agent.mcp.json");
    let user_config = config_dir.join("harness-user-interaction.mcp.json");
    assert!(agent_config.exists());
    assert!(user_config.exists());
    assert!(temporary_files(&config_dir)?.is_empty());
    let agent_json: Value = serde_json::from_str(&fs::read_to_string(&agent_config)?)?;
    let user_json: Value = serde_json::from_str(&fs::read_to_string(&user_config)?)?;
    assert_eq!(
        agent_json["mcpServers"]["harness-agent"]["env"]["HARNESS_SURFACE_ID"],
        SETUP_AGENT_SURFACE_ID
    );
    assert_eq!(
        user_json["mcpServers"]["harness-user-interaction"]["env"]["HARNESS_SURFACE_ID"],
        SETUP_USER_SURFACE_ID
    );
    assert!(agent_json["mcpServers"]
        .as_object()
        .expect("servers object")
        .get("harness-user-interaction")
        .is_none());
    assert!(user_json["mcpServers"]
        .as_object()
        .expect("servers object")
        .get("harness-agent")
        .is_none());

    let collision = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--with-user-interaction",
        "--config-dir",
        path_text(&config_dir).as_str(),
    ])?;
    assert_eq!(collision.status.code(), Some(1));
    assert!(stderr(&collision).contains("already exists"));

    let overwrite = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--with-user-interaction",
        "--config-dir",
        path_text(&config_dir).as_str(),
        "--overwrite-config",
    ])?;
    assert_success(&overwrite);
    assert!(temporary_files(&config_dir)?.is_empty());

    let surfaces = list_surfaces(runtime_home.path(), "product-repo")?;
    assert!(surfaces.iter().any(|surface| {
        surface.surface_id == SETUP_AGENT_SURFACE_ID
            && surface.surface_instance_id == SETUP_AGENT_SURFACE_INSTANCE_ID
    }));
    assert!(surfaces.iter().any(|surface| {
        surface.surface_id == SETUP_USER_SURFACE_ID
            && surface.surface_instance_id == SETUP_USER_INSTANCE_ID
    }));

    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_preflight_failure_writes_no_configuration() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-preflight-fail")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let config_dir = runtime_home.path().join("configs");
    let mcp_command = write_failing_mcp(runtime_home.path())?;

    let failed = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--config-dir",
        path_text(&config_dir).as_str(),
    ])?;

    assert_eq!(failed.status.code(), Some(1));
    assert!(stderr(&failed).contains("preflight failed for agent"));
    assert!(stderr(&failed).contains("completed registration actions"));
    assert!(!config_dir.join("harness-agent.mcp.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_setup_rejects_existing_alternate_state_db_path_before_preflight_and_config(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-state-db-existing")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let marker = runtime_home.path().join("preflight-marker.txt");
    let config_dir = runtime_home.path().join("configs");
    let mcp_command = write_test_mcp(runtime_home.path(), &marker)?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_state_db_existing", "{}")?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "product-repo".to_owned(),
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let original = single_project_record(runtime_home.path(), "product-repo")?;
    let alternate_state_path = runtime_home
        .path()
        .join("alternate/historical-state.sqlite");
    fs::create_dir_all(
        alternate_state_path
            .parent()
            .expect("alternate state path has parent"),
    )?;
    let mut alternate = Connection::open(&alternate_state_path)?;
    create_project_state_fixture_version(
        &mut alternate,
        "alternate_project_state",
        LEGACY_PROJECT_STATE_SCHEMA_VERSION,
    )?;
    drop(alternate);
    let migrations_before = migration_count(&alternate_state_path)?;
    let surface_count_before = surface_count(&alternate_state_path)?;
    replace_project_state_db_path(runtime_home.path(), "product-repo", &alternate_state_path)?;
    let damaged = single_project_record(runtime_home.path(), "product-repo")?;
    assert_only_state_db_path_changed(&original, &damaged, &alternate_state_path);

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--config-dir",
        path_text(&config_dir).as_str(),
    ])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert_state_db_path_mismatch_stderr(&stderr(&output));
    assert!(!marker.exists());
    assert!(!config_dir.join("harness-agent.mcp.json").exists());
    assert!(!config_dir.exists());
    assert_historical_project_state_unchanged(
        &alternate_state_path,
        migrations_before,
        surface_count_before,
    )?;
    assert_registry_unchanged_and_cli_visible(runtime_home.path(), "product-repo", &damaged)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_setup_rejects_missing_alternate_state_db_path_without_creating_it(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-state-db-missing")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let marker = runtime_home.path().join("preflight-marker.txt");
    let config_dir = runtime_home.path().join("configs");
    let mcp_command = write_test_mcp(runtime_home.path(), &marker)?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_state_db_missing", "{}")?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "product-repo".to_owned(),
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let original = single_project_record(runtime_home.path(), "product-repo")?;
    let alternate_state_path = runtime_home.path().join("missing/alternate-state.sqlite");
    let alternate_parent = alternate_state_path
        .parent()
        .expect("alternate state path has parent");
    assert!(!alternate_state_path.exists());
    assert!(!alternate_parent.exists());
    replace_project_state_db_path(runtime_home.path(), "product-repo", &alternate_state_path)?;
    let damaged = single_project_record(runtime_home.path(), "product-repo")?;
    assert_only_state_db_path_changed(&original, &damaged, &alternate_state_path);

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--config-dir",
        path_text(&config_dir).as_str(),
    ])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert_state_db_path_mismatch_stderr(&stderr(&output));
    assert!(!alternate_state_path.exists());
    assert!(!alternate_parent.exists());
    assert!(!marker.exists());
    assert!(!config_dir.join("harness-agent.mcp.json").exists());
    assert!(!config_dir.exists());
    assert_registry_unchanged_and_cli_visible(runtime_home.path(), "product-repo", &damaged)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_setup_reuses_valid_custom_project_home() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-custom-home")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let marker = runtime_home.path().join("preflight-marker.txt");
    let mcp_command = write_test_mcp(runtime_home.path(), &marker)?;
    let custom_project_home = runtime_home.path().join("custom-projects/product-repo");
    initialize_runtime_home(
        runtime_home.path(),
        "runtime_home_custom_project_home",
        "{}",
    )?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "product-repo".to_owned(),
            repo_root: repo_root.clone(),
            project_home: Some(custom_project_home.clone()),
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let custom_state_path = custom_project_home.join("state.sqlite");
    assert!(custom_state_path.exists());
    assert!(!project_state_db_path(runtime_home.path(), "product-repo").exists());

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
    ])?;

    assert_success(&output);
    assert!(stdout(&output).contains("project: reused"));
    assert!(stdout(&output).contains("agent_surface: created"));
    assert!(stdout(&output).contains("preflight: passed"));
    assert_eq!(fs::read_to_string(&marker)?.lines().count(), 1);
    let record = single_project_record(runtime_home.path(), "product-repo")?;
    assert_eq!(record.project_home, custom_project_home);
    assert_eq!(record.state_db_path, custom_state_path);
    assert!(list_surfaces(runtime_home.path(), "product-repo")?
        .iter()
        .any(|surface| surface.surface_id == SETUP_AGENT_SURFACE_ID
            && surface.surface_instance_id == SETUP_AGENT_SURFACE_INSTANCE_ID));
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_setup_rejects_invalid_existing_project_before_preflight_and_config(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-invalid-existing")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let marker = runtime_home.path().join("preflight-marker.txt");
    let config_dir = runtime_home.path().join("configs");
    let mcp_command = write_test_mcp(runtime_home.path(), &marker)?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_invalid_existing", "{}")?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "product-repo".to_owned(),
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    replace_project_home(
        runtime_home.path(),
        "product-repo",
        &repo_root.join(".harness-project"),
    )?;

    let output = run_without_home([
        "setup",
        "local-mcp",
        "--runtime-home",
        path_text(runtime_home.path()).as_str(),
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--config-dir",
        path_text(&config_dir).as_str(),
    ])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("project_home_overlaps_product_repository"));
    assert!(!marker.exists());
    assert!(!config_dir.join("harness-agent.mcp.json").exists());
    let projects = list_projects(runtime_home.path())?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project_home, repo_root.join(".harness-project"));
    Ok(())
}

#[test]
fn harness_binary_agent_help_covers_nested_commands() -> Result<(), Box<dyn Error>> {
    assert_agent_help(["agent", "--help"])?;
    assert_agent_help(["agent", "install", "--help"])?;
    assert_agent_help(["agent", "project", "--help"])?;
    assert_agent_help(["agent", "project", "add", "--help"])?;
    assert_agent_help(["agent", "project", "remove", "--help"])?;
    assert_agent_help(["agent", "status", "--help"])?;
    assert_agent_help(["agent", "verify", "--help"])?;
    assert_agent_help(["agent", "uninstall", "--help"])?;
    assert_agent_help(["agent", "guidance", "--help"])?;
    assert_agent_help(["agent", "guidance", "apply", "--help"])?;
    assert_agent_help(["agent", "guidance", "status", "--help"])?;
    assert_agent_help(["agent", "guidance", "remove", "--help"])?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_codex_user_install_verify_and_uninstall() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_codex_user",
            "--server-name",
            "harness-test",
            "--project-id",
            "project_agent_codex",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);
    assert!(stdout(&install).contains("status: complete"));
    assert!(stdout(&install).contains("verification: complete"));

    let config = fs::read_to_string(codex_home.join("config.toml"))?;
    assert!(config.contains("[mcp_servers.harness-test]"));
    assert!(config.contains("args = [\"--integration\", \"agent_codex_user\"]"));
    assert!(!config.contains("HARNESS_PROJECT_ID"));

    let integration = agent_integration_record(runtime_home.path(), "agent_codex_user")?
        .expect("integration should be stored");
    assert!(integration.enabled);
    let installations =
        list_host_installations_for_integration(runtime_home.path(), "agent_codex_user")?;
    assert_eq!(installations.len(), 1);
    assert_eq!(
        installations[0].last_verified_status,
        VERIFIED_STATUS_COMPLETE
    );

    let status = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "status", "--integration-id", "agent_codex_user"],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    assert!(stdout(&status).contains("host_state"));

    let verify = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "verify", "--integration-id", "agent_codex_user"],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&verify);
    assert!(stdout(&verify).contains("status: complete"));

    let uninstall = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "uninstall", "--integration-id", "agent_codex_user"],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&uninstall);
    assert!(stdout(&uninstall).contains("status: complete"));
    let config_after = fs::read_to_string(codex_home.join("config.toml"))?;
    assert!(!config_after.contains("[mcp_servers.harness-test]"));
    assert!(
        list_host_installations_for_integration(runtime_home.path(), "agent_codex_user")?
            .is_empty()
    );
    assert!(
        !agent_integration_record(runtime_home.path(), "agent_codex_user")?
            .expect("integration remains as disabled inventory")
            .enabled
    );

    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_dry_run_writes_nothing_and_rejects_invalid_scope(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-dry-run")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = runtime_home.path().join("harness-mcp-dry");
    fs::write(&mcp_command, "not executed")?;

    let dry_run = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_dry_run",
            "--project-id",
            "project_agent_dry",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&dry_run);
    let value: Value = serde_json::from_str(&stdout(&dry_run))?;
    assert_eq!(value["status"], "dry_run");
    assert_eq!(
        value["runtime"]["runtime_home"],
        path_text(runtime_home.path())
    );
    assert_eq!(
        value["project"]["allowed_project_ids"],
        json!(["project_agent_dry"])
    );
    assert!(value["integration"]["integration_id"].is_string());
    assert_eq!(value["host"]["host_kind"], "codex");
    assert_eq!(value["guidance"]["status"], "not_managed");
    assert!(value["guidance"]["items"]
        .as_array()
        .expect("guidance items array")
        .is_empty());
    assert_eq!(value["verification"]["status"], "not_verified");
    assert!(value["action_required"].as_array().is_some());
    assert!(value["effects"].as_array().is_some());
    assert!(!registry_db_path(runtime_home.path()).exists());
    assert!(!codex_home.join("config.toml").exists());
    assert!(!repo_root.join("AGENTS.md").exists());
    assert!(!repo_root.join(".claude").exists());

    let invalid = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "local",
            "--project-id",
            "project_invalid",
        ],
    )?;
    assert_eq!(invalid.status.code(), Some(2));
    assert!(stderr(&invalid).contains("host and scope"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_dry_run_missing_runtime_home_creates_nothing() -> Result<(), Box<dyn Error>>
{
    let scratch = TempRuntimeHome::new("cli-bin-agent-dry-run-missing-home")?;
    let selected_runtime_home = scratch.path().join("missing-runtime-home");
    let repo_root = scratch.create_product_repo("product-repo")?;
    let codex_home = scratch.path().join("codex-home");
    let mcp_command = scratch.path().join("harness-mcp-dry-missing");
    fs::write(&mcp_command, "not executed")?;

    let dry_run = run_without_home_and_env(
        [
            "agent",
            "install",
            "--runtime-home",
            path_text(&selected_runtime_home).as_str(),
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_missing_runtime_dry",
            "--project-id",
            "project_missing_runtime_dry",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;

    assert_success(&dry_run);
    let value: Value = serde_json::from_str(&stdout(&dry_run))?;
    assert_eq!(value["status"], "dry_run");
    assert!(value["runtime"]["registry_schema_version"].is_null());
    assert_eq!(value["runtime"]["registry_migration_planned"], false);
    assert!(!selected_runtime_home.exists());
    assert!(!registry_db_path(&selected_runtime_home).exists());
    assert!(!codex_home.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_generic_export_dry_run_creates_no_export_files(
) -> Result<(), Box<dyn Error>> {
    let scratch = TempRuntimeHome::new("cli-bin-agent-generic-export-dry")?;
    let selected_runtime_home = scratch.path().join("missing-runtime-home");
    let repo_root = scratch.create_product_repo("product-repo")?;
    let export_dir = scratch.path().join("missing-export-dir").join("nested");
    let mcp_command = scratch.path().join("harness-mcp-generic-dry");
    fs::write(&mcp_command, "not executed")?;

    let dry_run = run_without_home([
        "agent",
        "install",
        "--runtime-home",
        path_text(&selected_runtime_home).as_str(),
        "--host",
        "generic",
        "--scope",
        "export",
        "--integration-id",
        "agent_generic_export_dry",
        "--project-id",
        "project_generic_export_dry",
        "--repo-root",
        path_text(&repo_root).as_str(),
        "--mcp-command",
        path_text(&mcp_command).as_str(),
        "--export-dir",
        path_text(&export_dir).as_str(),
        "--dry-run",
        "--output",
        "json",
    ])?;

    assert_success(&dry_run);
    let value: Value = serde_json::from_str(&stdout(&dry_run))?;
    assert_eq!(value["status"], "dry_run");
    assert_eq!(value["host"]["host_kind"], "generic");
    assert!(!selected_runtime_home.exists());
    assert!(!export_dir.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_dry_run_historical_registry_is_read_only_and_reports_migration(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-registry-v1-dry")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    create_registry_fixture_with_project(
        runtime_home.path(),
        &repo_root,
        "project_registry_v1_dry",
        1,
    )?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = runtime_home.path().join("harness-mcp-v1-dry");
    fs::write(&mcp_command, "not executed")?;
    let registry_path = registry_db_path(runtime_home.path());
    let hash_before = file_hash(&registry_path)?;
    let migrations_before = migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?;
    let sidecars_before = existing_sidecars(std::slice::from_ref(&registry_path));

    let dry_run = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_registry_v1_dry",
            "--project-id",
            "project_registry_v1_dry",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;

    assert_success(&dry_run);
    let value: Value = serde_json::from_str(&stdout(&dry_run))?;
    assert_eq!(value["runtime"]["registry_schema_version"], 1);
    assert_eq!(
        value["runtime"]["registry_latest_supported_schema_version"],
        REGISTRY_SCHEMA_VERSION
    );
    assert_eq!(value["runtime"]["registry_migration_planned"], true);
    assert!(value["actions"]
        .as_array()
        .expect("actions array")
        .iter()
        .any(|action| action["target"] == "registry_migration" && action["action"] == "planned"));
    assert_eq!(file_hash(&registry_path)?, hash_before);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before
    );
    assert_eq!(registry_schema_version(&registry_path)?, 1);
    assert!(!table_exists(&registry_path, "agent_integrations")?);
    assert_eq!(existing_sidecars(&[registry_path]), sidecars_before);
    assert!(!codex_home.join("config.toml").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_dry_run_current_registry_is_byte_identical() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-registry-v2-dry")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_registry_v2_dry",
            "--server-name",
            "harness-registry-v2-dry",
            "--project-id",
            "project_registry_v2_dry",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);

    let registry_path = registry_db_path(runtime_home.path());
    let hash_before = file_hash(&registry_path)?;
    let migrations_before = migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?;
    let sidecars_before = existing_sidecars(std::slice::from_ref(&registry_path));
    let codex_config_before = fs::read(codex_home.join("config.toml"))?;

    let dry_run = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_registry_v2_dry",
            "--server-name",
            "harness-registry-v2-dry",
            "--project-id",
            "project_registry_v2_dry",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;

    assert_success(&dry_run);
    let value: Value = serde_json::from_str(&stdout(&dry_run))?;
    assert_eq!(
        value["runtime"]["registry_schema_version"],
        REGISTRY_SCHEMA_VERSION
    );
    assert_eq!(value["runtime"]["registry_migration_planned"], false);
    assert_eq!(file_hash(&registry_path)?, hash_before);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before
    );
    assert!(table_exists(&registry_path, "agent_integrations")?);
    assert_eq!(existing_sidecars(&[registry_path]), sidecars_before);
    assert_eq!(
        fs::read(codex_home.join("config.toml"))?,
        codex_config_before
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_dry_run_unsupported_future_registry_is_read_only(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-registry-future-dry")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    create_registry_fixture_with_project(
        runtime_home.path(),
        &repo_root,
        "project_registry_future_dry",
        REGISTRY_SCHEMA_VERSION,
    )?;
    insert_future_registry_migration(runtime_home.path())?;
    let registry_path = registry_db_path(runtime_home.path());
    let hash_before = file_hash(&registry_path)?;
    let migrations_before = migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?;

    let dry_run = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_registry_future_dry",
            "--project-id",
            "project_registry_future_dry",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--dry-run",
        ],
    )?;

    assert_eq!(dry_run.status.code(), Some(1));
    assert!(stdout(&dry_run).is_empty());
    assert!(stderr(&dry_run).contains("not supported"));
    assert_eq!(file_hash(&registry_path)?, hash_before);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_guidance_apply_status_and_remove_flow() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-flow")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_guidance_flow",
            "--server-name",
            "harness-guidance-flow",
            "--project-id",
            "project_guidance_flow",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);
    assert!(!repo_root.join("AGENTS.md").exists());
    assert!(!repo_root.join(".claude").exists());

    let claude_dry_run = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "apply",
            "--integration-id",
            "agent_guidance_flow",
            "--project-id",
            "project_guidance_flow",
            "--host",
            "claude-code",
            "--dry-run",
            "--output",
            "json",
        ],
    )?;
    assert_success(&claude_dry_run);
    let claude_dry_json: Value = serde_json::from_str(&stdout(&claude_dry_run))?;
    assert_eq!(claude_dry_json["status"], "dry_run");
    assert_eq!(claude_dry_json["guidance"]["status"], "absent");
    assert!(claude_dry_json["guidance"]["items"][0]["planned_content"]
        .as_str()
        .expect("planned content")
        .contains(GUIDANCE_BEGIN_MARKER));
    assert!(!repo_root.join(".claude").exists());

    let missing_authorization = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "apply",
            "--integration-id",
            "agent_guidance_flow",
            "--project-id",
            "project_guidance_flow",
            "--host",
            "codex",
        ],
    )?;
    assert_eq!(missing_authorization.status.code(), Some(2));
    assert!(stderr(&missing_authorization).contains("--allow-repository-write"));

    let apply = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "apply",
            "--integration-id",
            "agent_guidance_flow",
            "--project-id",
            "project_guidance_flow",
            "--host",
            "codex",
            "--allow-repository-write",
            "--output",
            "json",
        ],
    )?;
    assert_success(&apply);
    let apply_json: Value = serde_json::from_str(&stdout(&apply))?;
    assert_eq!(apply_json["guidance"]["status"], "present");
    assert_guidance_item_state(&apply_json, "codex", "present");
    let agents_path = repo_root.join("AGENTS.md");
    let agents = fs::read_to_string(&agents_path)?;
    assert!(agents.contains(GUIDANCE_BEGIN_MARKER));
    assert!(agents.contains("harness.list_projects"));

    let status = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "status",
            "--integration-id",
            "agent_guidance_flow",
            "--project-id",
            "project_guidance_flow",
            "--output",
            "json",
        ],
    )?;
    assert_success(&status);
    let status_json: Value = serde_json::from_str(&stdout(&status))?;
    assert_eq!(status_json["guidance"]["status"], "mixed");
    assert_guidance_item_state(&status_json, "codex", "present");
    assert_guidance_item_state(&status_json, "claude_code", "absent");

    let remove_dry_run = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "remove",
            "--integration-id",
            "agent_guidance_flow",
            "--project-id",
            "project_guidance_flow",
            "--host",
            "codex",
            "--dry-run",
            "--remove-managed",
        ],
    )?;
    assert_success(&remove_dry_run);
    assert!(agents_path.exists());

    let remove = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "remove",
            "--integration-id",
            "agent_guidance_flow",
            "--project-id",
            "project_guidance_flow",
            "--host",
            "codex",
            "--allow-repository-write",
            "--remove-managed",
        ],
    )?;
    assert_success(&remove);
    assert!(!agents_path.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_install_guidance_both_and_uninstall_managed() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-install")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_guidance_install",
            "--server-name",
            "harness-guidance-install",
            "--project-id",
            "project_guidance_install",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--guidance",
            "both",
            "--allow-repository-write",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);
    assert!(stdout(&install).contains("guidance:"));
    assert!(stdout(&install).contains("codex: present"));
    assert!(stdout(&install).contains("claude_code: present"));

    let agents_path = repo_root.join("AGENTS.md");
    let claude_path = repo_root.join(".claude").join("rules").join("harness.md");
    assert!(fs::read_to_string(&agents_path)?.contains(GUIDANCE_BEGIN_MARKER));
    assert!(fs::read_to_string(&claude_path)?.contains(GUIDANCE_BEGIN_MARKER));

    let status = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "status",
            "--integration-id",
            "agent_guidance_install",
            "--output",
            "json",
        ],
    )?;
    assert_success(&status);
    let status_json: Value = serde_json::from_str(&stdout(&status))?;
    assert_eq!(status_json["guidance"]["status"], "present");
    assert_guidance_item_state(&status_json, "codex", "present");
    assert_guidance_item_state(&status_json, "claude_code", "present");

    let uninstall = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "uninstall",
            "--integration-id",
            "agent_guidance_install",
            "--allow-repository-write",
            "--remove-managed",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&uninstall);
    assert!(stdout(&uninstall).contains("status: complete"));
    assert!(!agents_path.exists());
    assert!(!repo_root.join(".claude").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_uninstall_preserves_changed_guidance() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-conflict")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_guidance_conflict",
            "--server-name",
            "harness-guidance-conflict",
            "--project-id",
            "project_guidance_conflict",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--guidance",
            "codex",
            "--allow-repository-write",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);

    let agents_path = repo_root.join("AGENTS.md");
    let original = fs::read_to_string(&agents_path)?;
    fs::write(
        &agents_path,
        original.replace("do not guess `project_id`", "guess project_id"),
    )?;

    let guidance_status = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "guidance",
            "status",
            "--integration-id",
            "agent_guidance_conflict",
            "--project-id",
            "project_guidance_conflict",
        ],
    )?;
    assert_eq!(guidance_status.status.code(), Some(1));
    assert!(stdout(&guidance_status).contains("status: failed"));
    assert!(stdout(&guidance_status).contains("codex: changed"));

    let uninstall = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "uninstall",
            "--integration-id",
            "agent_guidance_conflict",
            "--allow-repository-write",
            "--remove-managed",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_eq!(uninstall.status.code(), Some(1));
    assert!(stdout(&uninstall).contains("status: partial_failure"));
    assert!(stdout(&uninstall).contains("residual guidance preserved"));
    assert!(fs::read_to_string(&agents_path)?.contains("guess project_id"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_install_compensates_new_guidance_after_verification_failure(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-compensate")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::MissingUtilityTool)?;
    let agents_path = repo_root.join("AGENTS.md");
    fs::write(&agents_path, "# Existing instructions\nKeep this.\n")?;

    let output = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_guidance_compensate",
            "--server-name",
            "harness-guidance-compensate",
            "--project-id",
            "project_guidance_compensate",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--guidance",
            "codex",
            "--allow-repository-write",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("status: partial_failure"));
    assert!(stdout(&output).contains("harness.list_projects"));
    assert!(stdout(&output).contains("compensated newly-created guidance"));
    let agents = fs::read_to_string(&agents_path)?;
    assert!(agents.contains("# Existing instructions\nKeep this.\n"));
    assert!(!agents.contains(GUIDANCE_BEGIN_MARKER));
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_user_scope_project_membership_is_single_host_entry(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-project-add")?;
    let repo_a = runtime_home.create_product_repo("repo-a")?;
    let repo_b = runtime_home.create_product_repo("repo-b")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_multi_project",
            "--server-name",
            "harness-multi",
            "--project-id",
            "project_a",
            "--repo-root",
            path_text(&repo_a).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);

    let add = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "add",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_b",
            "--repo-root",
            path_text(&repo_b).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&add);
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_multi_project")?.len(),
        1
    );

    let remove_default = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "remove",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_a",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_eq!(remove_default.status.code(), Some(1));
    assert!(stderr(&remove_default).contains("default project"));

    let remove_b = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "remove",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_b",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&remove_b);
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_project_and_uninstall_dry_runs_are_read_only() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-project-uninstall-dry")?;
    let repo_a = runtime_home.create_product_repo("repo-a")?;
    let repo_b = runtime_home.create_product_repo("repo-b")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_project_uninstall_dry",
            "--server-name",
            "harness-project-uninstall-dry",
            "--project-id",
            "project_dry_a",
            "--repo-root",
            path_text(&repo_a).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&install);

    let registry_path = registry_db_path(runtime_home.path());
    let hash_before_add = file_hash(&registry_path)?;
    let migrations_before_add = migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?;
    let config_before_add = fs::read(codex_home.join("config.toml"))?;

    let add_dry_run = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "add",
            "--integration-id",
            "agent_project_uninstall_dry",
            "--project-id",
            "project_dry_b",
            "--repo-root",
            path_text(&repo_b).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&add_dry_run);
    let add_json: Value = serde_json::from_str(&stdout(&add_dry_run))?;
    assert_eq!(add_json["status"], "dry_run");
    assert_eq!(file_hash(&registry_path)?, hash_before_add);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before_add
    );
    assert_eq!(fs::read(codex_home.join("config.toml"))?, config_before_add);
    assert_eq!(list_projects(runtime_home.path())?.len(), 1);

    let add = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "add",
            "--integration-id",
            "agent_project_uninstall_dry",
            "--project-id",
            "project_dry_b",
            "--repo-root",
            path_text(&repo_b).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&add);
    let hash_before_remove = file_hash(&registry_path)?;
    let migrations_before_remove = migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?;
    let config_before_remove = fs::read(codex_home.join("config.toml"))?;

    let remove_dry_run = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "remove",
            "--integration-id",
            "agent_project_uninstall_dry",
            "--project-id",
            "project_dry_b",
            "--dry-run",
            "--output",
            "json",
        ],
    )?;
    assert_success(&remove_dry_run);
    let remove_json: Value = serde_json::from_str(&stdout(&remove_dry_run))?;
    assert_eq!(remove_json["status"], "dry_run");
    assert_eq!(file_hash(&registry_path)?, hash_before_remove);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before_remove
    );
    assert_eq!(
        fs::read(codex_home.join("config.toml"))?,
        config_before_remove
    );

    let uninstall_dry_run = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "uninstall",
            "--integration-id",
            "agent_project_uninstall_dry",
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&uninstall_dry_run);
    let uninstall_json: Value = serde_json::from_str(&stdout(&uninstall_dry_run))?;
    assert_eq!(uninstall_json["status"], "dry_run");
    assert_eq!(file_hash(&registry_path)?, hash_before_remove);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before_remove
    );
    assert_eq!(
        fs::read(codex_home.join("config.toml"))?,
        config_before_remove
    );
    assert_eq!(
        list_host_installations_for_integration(
            runtime_home.path(),
            "agent_project_uninstall_dry"
        )?
        .len(),
        1
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_claude_project_install_reports_action_required(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-claude-project")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    fs::create_dir_all(&bin_dir)?;
    let mcp = write_agent_mcp(&bin_dir, AgentMcpFixture::Complete)?;
    fs::rename(&mcp, bin_dir.join("harness-mcp"))?;
    write_fake_claude_mcp_get(&bin_dir, "⏸ Pending approval")?;

    let install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "claude-code",
            "--scope",
            "project",
            "--integration-id",
            "agent_claude_project",
            "--server-name",
            "harness-claude",
            "--project-id",
            "project_claude",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--allow-repository-write",
        ],
        &[("PATH", path_text(&bin_dir))],
    )?;
    assert_success(&install);
    assert!(stdout(&install).contains("status: action_required"));
    let project_config: Value =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".mcp.json"))?)?;
    assert_eq!(
        project_config["mcpServers"]["harness-claude"]["command"],
        "harness-mcp"
    );
    let installations =
        list_host_installations_for_integration(runtime_home.path(), "agent_claude_project")?;
    assert_eq!(
        installations[0].last_verified_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn harness_binary_agent_mcp_tool_discovery_failures_are_partial_failure(
) -> Result<(), Box<dyn Error>> {
    for (fixture, expected) in [
        (
            AgentMcpFixture::MissingInstructions,
            "missing server instructions",
        ),
        (AgentMcpFixture::MissingUtilityTool, "harness.list_projects"),
    ] {
        let runtime_home = TempRuntimeHome::new("cli-bin-agent-mcp-fail")?;
        let repo_root = runtime_home.create_product_repo("product-repo")?;
        let codex_home = runtime_home.path().join("codex-home");
        let mcp_command = write_agent_mcp(runtime_home.path(), fixture)?;
        let output = run_with_home_and_env(
            runtime_home.path(),
            [
                "agent",
                "install",
                "--host",
                "codex",
                "--scope",
                "user",
                "--integration-id",
                "agent_mcp_failure",
                "--server-name",
                "harness-failure",
                "--project-id",
                "project_failure",
                "--repo-root",
                path_text(&repo_root).as_str(),
                "--mcp-command",
                path_text(&mcp_command).as_str(),
            ],
            &[("CODEX_HOME", path_text(&codex_home))],
        )?;
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("status: partial_failure"));
        assert!(stdout(&output).contains(expected));
        let installations =
            list_host_installations_for_integration(runtime_home.path(), "agent_mcp_failure")?;
        assert_eq!(
            installations[0].last_verified_status,
            VERIFIED_STATUS_PARTIAL_FAILURE
        );
    }
    Ok(())
}

fn initialize_historical_setup(
    runtime_home: &Path,
    repo_root: &Path,
    project_id: &str,
) -> Result<(), Box<dyn Error>> {
    initialize_runtime_home(runtime_home, "runtime_home_binary_historical", "{}")?;
    register_project(
        runtime_home,
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root: fs::canonicalize(repo_root)?,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let state_path = project_state_db_path(runtime_home, project_id);
    fs::remove_file(&state_path)?;
    let mut conn = Connection::open(&state_path)?;
    create_project_state_fixture_version(&mut conn, project_id, PROJECT_STATE_SCHEMA_VERSION - 1)?;
    conn.execute(
        "INSERT INTO surfaces (
            project_id,
            surface_id,
            surface_instance_id,
            surface_kind,
            display_name,
            capability_profile_json,
            local_access_json,
            registered_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, 'mcp', 'Agent MCP', ?4, ?5, 't0', '{}')",
        params![
            project_id,
            SETUP_AGENT_SURFACE_ID,
            SETUP_AGENT_SURFACE_INSTANCE_ID,
            capability_profile_json(&baseline_workflow_access_classes(), None)?,
            local_access_json(&baseline_workflow_access_classes())?
        ],
    )?;
    Ok(())
}

fn create_registry_fixture_with_project(
    runtime_home: &Path,
    repo_root: &Path,
    project_id: &str,
    registry_version: i64,
) -> Result<ProjectRecord, Box<dyn Error>> {
    let repo_root = fs::canonicalize(repo_root)?;
    let project_home = runtime_home.join("projects").join(project_id);
    fs::create_dir_all(&project_home)?;
    let state_db_path = project_home.join("state.sqlite");
    let mut state = Connection::open(&state_db_path)?;
    create_project_state_fixture_version(&mut state, project_id, PROJECT_STATE_SCHEMA_VERSION)?;
    drop(state);

    let repo_root_text = path_text(&repo_root);
    let project_home_text = path_text(&project_home);
    let state_db_path_text = path_text(&state_db_path);
    let mut registry = Connection::open(registry_db_path(runtime_home))?;
    create_registry_fixture_version(
        &mut registry,
        "runtime_home_binary_agent_fixture",
        registry_version,
        &[RegistryFixtureProject {
            project_id,
            repo_root: &repo_root_text,
            project_home: &project_home_text,
            state_db_path: &state_db_path_text,
            status: ACTIVE_PROJECT_STATUS,
            metadata_json: "{}",
        }],
    )?;
    drop(registry);

    Ok(ProjectRecord {
        project_id: project_id.to_owned(),
        runtime_home_id: "runtime_home_binary_agent_fixture".to_owned(),
        repo_root,
        project_home,
        state_db_path,
        status: ACTIVE_PROJECT_STATUS.to_owned(),
        metadata_json: "{}".to_owned(),
    })
}

fn insert_future_registry_migration(runtime_home: &Path) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open(registry_db_path(runtime_home))?;
    conn.execute(
        "INSERT INTO schema_migrations (
            database_kind,
            version,
            name,
            storage_profile,
            applied_at
        )
        VALUES (?1, ?2, 'registry_future_v999', ?3, 't_future')",
        params![
            REGISTRY_DATABASE_KIND,
            REGISTRY_SCHEMA_VERSION + 997,
            STORAGE_PROFILE
        ],
    )?;
    Ok(())
}

fn migration_count(path: &Path) -> Result<i64, Box<dyn Error>> {
    migration_count_for(path, PROJECT_STATE_DATABASE_KIND)
}

fn migration_count_for(path: &Path, database_kind: &str) -> Result<i64, Box<dyn Error>> {
    let conn = open_read_only_database(path)?;
    Ok(conn.query_row(
        "SELECT COUNT(*)
           FROM schema_migrations
          WHERE database_kind = ?1",
        [database_kind],
        |row| row.get(0),
    )?)
}

fn registry_schema_version(path: &Path) -> Result<i64, Box<dyn Error>> {
    let conn = open_read_only_database(path)?;
    Ok(conn.query_row(
        "SELECT schema_version FROM runtime_home WHERE singleton_id = 1",
        [],
        |row| row.get(0),
    )?)
}

fn table_exists(path: &Path, table: &str) -> Result<bool, Box<dyn Error>> {
    let conn = open_read_only_database(path)?;
    Ok(conn.query_row(
        "SELECT EXISTS (
            SELECT 1
              FROM sqlite_master
             WHERE type = 'table'
               AND name = ?1
        )",
        [table],
        |row| row.get::<_, i64>(0),
    )? == 1)
}

fn file_hash(path: &Path) -> Result<u64, Box<dyn Error>> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    fs::read(path)?.hash(&mut hasher);
    Ok(hasher.finish())
}

fn existing_sidecars(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut sidecars = Vec::new();
    for path in paths {
        for sidecar in sqlite_sidecar_paths(path) {
            if sidecar.exists() {
                sidecars.push(sidecar);
            }
        }
    }
    sidecars.sort();
    sidecars
}

fn sqlite_sidecar_paths(path: &Path) -> Vec<PathBuf> {
    ["-wal", "-shm", "-journal"]
        .iter()
        .map(|suffix| {
            let mut raw = OsString::from(path.as_os_str());
            raw.push(suffix);
            PathBuf::from(raw)
        })
        .collect()
}

fn run_without_home<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    command.args(args);
    Ok(command.output()?)
}

fn run_without_home_and_env<const N: usize>(
    args: [&str; N],
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    for (name, value) in envs {
        command.env(name, value);
    }
    command.args(args);
    Ok(command.output()?)
}

fn run_with_home<const N: usize>(
    runtime_home: &Path,
    args: [&str; N],
) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    command.env("HARNESS_HOME", runtime_home);
    command.args(args);
    Ok(command.output()?)
}

fn run_with_home_and_env<const N: usize>(
    runtime_home: &Path,
    args: [&str; N],
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
    command.env("HARNESS_HOME", runtime_home);
    for (name, value) in envs {
        command.env(name, value);
    }
    command.args(args);
    Ok(command.output()?)
}

fn assert_agent_help<const N: usize>(args: [&str; N]) -> Result<(), Box<dyn Error>> {
    let output = run_without_home(args)?;
    assert_success(&output);
    assert!(stdout(&output).contains("harness agent install"));
    assert!(stdout(&output).contains("harness agent guidance apply"));
    assert!(stdout(&output).contains("--guidance none|codex"));
    assert!(stdout(&output).contains("--default-project-id ID"));
    assert!(stdout(&output).contains("--surface-id ID"));
    assert!(stdout(&output).contains("--export-path PATH"));
    assert!(stdout(&output).contains("--remove-managed"));
    Ok(())
}

fn assert_guidance_item_state(value: &Value, target: &str, state: &str) {
    let items = value["guidance"]["items"]
        .as_array()
        .expect("guidance items array");
    assert!(
        items
            .iter()
            .any(|item| item["target"] == target && item["state"] == state),
        "expected guidance item {target}={state}, got {items:?}"
    );
}

fn base_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_harness"));
    command.env_clear();
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
}

fn access_classes(local_access_json: &str) -> Result<Value, Box<dyn Error>> {
    let value: Value = serde_json::from_str(local_access_json)?;
    Ok(value["authorized_access_classes"].clone())
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

fn state_version(runtime_home: &Path, project_id: &str) -> Result<i64, Box<dyn Error>> {
    let conn = rusqlite::Connection::open(project_state_db_path(runtime_home, project_id))?;
    Ok(conn.query_row(
        "SELECT state_version FROM project_state WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?)
}

fn initialized_project(
    prefix: &str,
    project_id: &str,
    runtime_home_id: &str,
) -> Result<TempRuntimeHome, Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new(prefix)?;
    let repo_root = runtime_home.create_product_repo(format!("repo-{project_id}"))?;
    initialize_runtime_home(runtime_home.path(), runtime_home_id, "{}")?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(runtime_home)
}

fn single_project_record(
    runtime_home: &Path,
    project_id: &str,
) -> Result<ProjectRecord, Box<dyn Error>> {
    let project = list_projects(runtime_home)?
        .into_iter()
        .find(|project| project.project_id == project_id)
        .expect("project should remain registry-visible");
    Ok(project)
}

fn assert_registry_unchanged_and_cli_visible(
    runtime_home: &Path,
    project_id: &str,
    expected: &ProjectRecord,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(&single_project_record(runtime_home, project_id)?, expected);

    let projects = run_with_home(runtime_home, ["project", "list"])?;
    assert_success(&projects);
    assert!(stdout(&projects).contains(project_id));
    assert!(stdout(&projects).contains(&path_text(&expected.repo_root)));
    Ok(())
}

fn assert_only_state_db_path_changed(
    original: &ProjectRecord,
    damaged: &ProjectRecord,
    alternate_state_path: &Path,
) {
    assert_eq!(damaged.project_id, original.project_id);
    assert_eq!(damaged.runtime_home_id, original.runtime_home_id);
    assert_eq!(damaged.repo_root, original.repo_root);
    assert_eq!(damaged.project_home, original.project_home);
    assert_eq!(damaged.status, original.status);
    assert_eq!(damaged.metadata_json, original.metadata_json);
    assert_eq!(damaged.state_db_path, alternate_state_path);
    assert_ne!(damaged.state_db_path, original.state_db_path);
}

fn assert_invalid_project_path_error(output: &Output, relationship: &str) {
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(output);
    assert!(stdout(output).is_empty());
    assert!(stderr.contains("registered Product Repository conflicts with Runtime Home"));
    assert!(stderr.contains(&format!("relationship {relationship}")));
    assert!(stderr.contains("Harness Runtime Home"));
    assert!(stderr.contains("Product Repository"));
}

fn assert_state_db_path_mismatch_stderr(stderr: &str) {
    assert!(stderr.contains("registered project state database path conflicts with project_home"));
    assert!(stderr.contains("field state_db_path"));
    assert!(stderr.contains("relationship state_db_path_mismatch"));
}

fn surface_rows(state_path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let conn = open_read_only_database(state_path)?;
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            surface_id,
            surface_instance_id,
            surface_kind,
            interaction_role,
            COALESCE(display_name, ''),
            capability_profile_json,
            local_access_json,
            metadata_json
         FROM surfaces
         ORDER BY surface_id, surface_instance_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(vec![
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
        ])
    })?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

fn surface_count(state_path: &Path) -> Result<i64, Box<dyn Error>> {
    let conn = open_read_only_database(state_path)?;
    Ok(conn.query_row("SELECT COUNT(*) FROM surfaces", [], |row| row.get(0))?)
}

fn column_exists(state_path: &Path, table: &str, column: &str) -> Result<bool, Box<dyn Error>> {
    let conn = open_read_only_database(state_path)?;
    let escaped_table = table.replace('"', "\"\"");
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{escaped_table}\")"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn assert_historical_project_state_unchanged(
    state_path: &Path,
    expected_migration_count: i64,
    expected_surface_count: i64,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(migration_count(state_path)?, expected_migration_count);
    assert_eq!(surface_count(state_path)?, expected_surface_count);
    assert!(!column_exists(
        state_path,
        "project_state",
        "enforcement_profile_json"
    )?);
    assert!(!column_exists(state_path, "surfaces", "interaction_role")?);
    Ok(())
}

fn replace_project_repo_root(
    runtime_home: &Path,
    project_id: &str,
    repo_root: &Path,
) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open(registry_db_path(runtime_home))?;
    conn.execute(
        "UPDATE projects SET repo_root = ?2 WHERE project_id = ?1",
        params![project_id, repo_root.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

fn replace_project_home(
    runtime_home: &Path,
    project_id: &str,
    project_home: &Path,
) -> Result<(), Box<dyn Error>> {
    let state_db_path = project_home.join("state.sqlite");
    let conn = Connection::open(registry_db_path(runtime_home))?;
    conn.execute(
        "UPDATE projects
            SET project_home = ?2,
                state_db_path = ?3
          WHERE project_id = ?1",
        params![
            project_id,
            project_home.to_string_lossy().as_ref(),
            state_db_path.to_string_lossy().as_ref()
        ],
    )?;
    Ok(())
}

fn replace_project_state_db_path(
    runtime_home: &Path,
    project_id: &str,
    state_db_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open(registry_db_path(runtime_home))?;
    conn.execute(
        "UPDATE projects SET state_db_path = ?2 WHERE project_id = ?1",
        params![project_id, state_db_path.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
enum InvalidProjectRelationship {
    SamePath,
    RepositoryInsideRuntimeHome,
    RuntimeHomeInsideRepository,
}

impl InvalidProjectRelationship {
    const ALL: [Self; 3] = [
        Self::SamePath,
        Self::RepositoryInsideRuntimeHome,
        Self::RuntimeHomeInsideRepository,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::SamePath => "same",
            Self::RepositoryInsideRuntimeHome => "repo_inside_runtime",
            Self::RuntimeHomeInsideRepository => "runtime_inside_repo",
        }
    }

    fn expected_error(self) -> &'static str {
        match self {
            Self::SamePath => "same_path",
            Self::RepositoryInsideRuntimeHome => "runtime_home_contains_product_repository",
            Self::RuntimeHomeInsideRepository => "product_repository_contains_runtime_home",
        }
    }

    fn prefix(self, suffix: &str) -> String {
        format!("cli-bin-surface-{suffix}-{}", self.name())
    }

    fn project_id(self, suffix: &str) -> String {
        format!("project_cli_surface_{suffix}_{}", self.name())
    }

    fn replace_repo_root(
        self,
        runtime_home: &TempRuntimeHome,
        project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let repo_root = match self {
            Self::SamePath => runtime_home.path().to_path_buf(),
            Self::RepositoryInsideRuntimeHome => {
                let repo_root = runtime_home.path().join("legacy-product-repo");
                fs::create_dir_all(&repo_root)?;
                repo_root
            }
            Self::RuntimeHomeInsideRepository => runtime_home
                .path()
                .parent()
                .expect("runtime home should have a parent")
                .to_path_buf(),
        };
        replace_project_repo_root(runtime_home.path(), project_id, &repo_root)
    }
}

fn temporary_files(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            files.push(name);
        }
    }
    Ok(files)
}

#[cfg(unix)]
fn write_test_mcp(dir: &Path, marker: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let path = dir.join("test-harness-mcp");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" != \"--check\" ]; then\n\
             printf 'unexpected argument\\n' >&2\n\
             exit 2\n\
             fi\n\
             printf '%s\\n' \"$HARNESS_SURFACE_ID\" >> {}\n\
             if [ \"$HARNESS_SURFACE_ID\" = \"{}\" ]; then\n\
             role='user_interaction'\n\
             baseline='not_applicable'\n\
             access='read_status,core_mutation'\n\
             else\n\
             role='agent'\n\
             baseline='full'\n\
             access='read_status,core_mutation,write_authorization,artifact_registration,run_recording'\n\
             fi\n\
             printf 'configuration: valid\\n'\n\
             printf 'transport: stdio\\n'\n\
             printf 'runtime_home: %s\\n' \"$HARNESS_HOME\"\n\
             printf 'project_id: %s\\n' \"$HARNESS_PROJECT_ID\"\n\
             printf 'surface_id: %s\\n' \"$HARNESS_SURFACE_ID\"\n\
             printf 'surface_instance_id: %s\\n' \"$HARNESS_SURFACE_INSTANCE_ID\"\n\
             printf 'interaction_role: %s\\n' \"$role\"\n\
             printf 'access_classes: %s\\n' \"$access\"\n\
             printf 'baseline_workflow_access: %s\\n' \"$baseline\"\n\
             printf 'missing_access_classes: \\n'\n",
            shell_quote(marker),
            SETUP_USER_SURFACE_ID
        ),
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn write_failing_mcp(dir: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let path = dir.join("failing-harness-mcp");
    fs::write(
        &path,
        "#!/bin/sh\nprintf 'forced preflight failure\\n' >&2\nexit 1\n",
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum AgentMcpFixture {
    Complete,
    MissingInstructions,
    MissingUtilityTool,
}

#[cfg(unix)]
fn write_agent_mcp(
    dir: &Path,
    fixture: AgentMcpFixture,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let name = match fixture {
        AgentMcpFixture::Complete => "agent-mcp-complete",
        AgentMcpFixture::MissingInstructions => "agent-mcp-missing-instructions",
        AgentMcpFixture::MissingUtilityTool => "agent-mcp-missing-utility",
    };
    let path = dir.join(name);
    let initialize = match fixture {
        AgentMcpFixture::MissingInstructions => {
            r#"printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"harness-mcp","version":"test"}}}'"#
        }
        _ => {
            r#"printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"harness-mcp","version":"test"},"instructions":"Use Harness."}}'"#
        }
    };
    let tools = match fixture {
        AgentMcpFixture::MissingUtilityTool => public_tool_json(false),
        _ => public_tool_json(true),
    };
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--check\" ]; then\n\
             shift\n\
             if [ \"$1\" != \"--integration\" ]; then printf 'missing integration\\n' >&2; exit 2; fi\n\
             integration=\"$2\"\n\
             printf 'configuration: valid\\n'\n\
             printf 'transport: stdio\\n'\n\
             printf 'runtime_home: %s\\n' \"$HARNESS_HOME\"\n\
             printf 'integration_id: %s\\n' \"$integration\"\n\
             printf 'interaction_role: agent\\n'\n\
             printf 'surface_id: surface_test\\n'\n\
             printf 'surface_instance_id: surface_instance_test\\n'\n\
             printf 'enabled: true\\n'\n\
             printf 'allowed_projects: 1\\n'\n\
             printf 'available_projects: 1\\n'\n\
             printf 'default_project_id: project_test\\n'\n\
             printf 'verification_scope: startup_check_only\\n'\n\
             exit 0\n\
             fi\n\
             if [ \"$1\" = \"--integration\" ]; then\n\
             while IFS= read -r line; do\n\
             case \"$line\" in\n\
             *'\"method\":\"notifications/initialized\"'*) ;;\n\
             *'\"method\":\"initialize\"'*) {initialize} ;;\n\
             *'\"method\":\"tools/list\"'*) printf '%s\\n' '{tools}'; exit 0 ;;\n\
             esac\n\
             done\n\
             exit 0\n\
             fi\n\
             printf 'unexpected invocation\\n' >&2\n\
             exit 2\n"
        ),
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn write_fake_claude_mcp_get(dir: &Path, status: &str) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("claude");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"get\" ]; then\n\
             printf '%s\\n' '{}'\n\
             exit 0\n\
             fi\n\
             printf 'unexpected claude invocation\\n' >&2\n\
             exit 2\n",
            status
        ),
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn public_tool_json(include_utility: bool) -> String {
    let mut names = vec![
        "harness.intake",
        "harness.update_scope",
        "harness.status",
        "harness.prepare_write",
        "harness.stage_artifact",
        "harness.record_run",
        "harness.request_user_judgment",
        "harness.record_user_judgment",
        "harness.close_task",
    ];
    if include_utility {
        names.push("harness.list_projects");
    }
    let tools = names
        .into_iter()
        .map(|name| json!({ "name": name }))
        .collect::<Vec<_>>();
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": {
            "tools": tools
        }
    })
    .to_string()
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}
