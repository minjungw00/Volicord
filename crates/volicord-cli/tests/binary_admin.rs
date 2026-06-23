#![forbid(unsafe_code)]

use std::{
    error::Error,
    ffi::OsString,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use volicord_cli::registration::{
    baseline_workflow_access_classes, capability_profile_json, local_access_json,
};
use volicord_store::{
    agent_integrations::{
        agent_integration_record, list_host_installations_for_integration,
        list_integration_projects, update_host_installation_verification,
        VERIFIED_STATUS_ACTION_REQUIRED, VERIFIED_STATUS_COMPLETE, VERIFIED_STATUS_FAILED,
    },
    bootstrap::{
        initialize_runtime_home, list_projects, list_surfaces, register_project, register_surface,
        ProjectRecord, ProjectRegistration, SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    migrations::{
        PROJECT_STATE_DATABASE_KIND, REGISTRY_DATABASE_KIND, REGISTRY_SCHEMA_VERSION,
        STORAGE_PROFILE,
    },
    sqlite::{
        open_project_state_database, open_read_only_database, project_state_db_path,
        registry_db_path,
    },
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::SurfaceInteractionRole;

const PROJECT_ID: &str = "project_binary_admin";
const AGENT_SURFACE_ID: &str = "surface_binary_agent";
const AGENT_INSTANCE_ID: &str = "surface_instance_binary_agent";
const USER_SURFACE_ID: &str = "surface_binary_user";
const USER_INSTANCE_ID: &str = "surface_instance_binary_user";
const GUIDANCE_BEGIN_MARKER: &str = "<!-- BEGIN HARNESS MANAGED GUIDANCE v1 -->";

#[test]
fn volicord_binary_runs_administrative_initialization_and_registration(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-admin")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let repo_root_text = path_text(&repo_root);

    let help = run_without_home(["--help"])?;
    assert_success(&help);
    assert!(stdout(&help).contains("harness init"));
    assert!(!stdout(&help).contains("harness setup"));

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
fn volicord_binary_setup_command_family_is_unknown() -> Result<(), Box<dyn Error>> {
    for output in [
        run_without_home(["setup"])?,
        run_without_home(["setup", "local-mcp"])?,
    ] {
        assert_eq!(output.status.code(), Some(2));
        assert!(stdout(&output).is_empty());
        assert!(stderr(&output).contains("unknown command: setup"));
        assert!(!stderr(&output).contains("harness setup local-mcp"));
    }
    Ok(())
}

#[test]
fn volicord_binary_project_register_rejects_invalid_project_id() -> Result<(), Box<dyn Error>> {
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
fn volicord_binary_surface_commands_reject_invalid_legacy_project_paths(
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
        assert_registry_unchanged_and_project_list_rejects(
            runtime_home.path(),
            &project_id,
            &registry_before,
            relationship.expected_error(),
        )?;

        let list = run_with_home(
            runtime_home.path(),
            ["surface", "list", "--project-id", project_id.as_str()],
        )?;
        assert_invalid_project_path_error(&list, relationship.expected_error());
        assert_eq!(migration_count(&state_path)?, migrations_before);
        assert_eq!(surface_rows(&state_path)?, surfaces_before);
        assert_registry_unchanged_and_project_list_rejects(
            runtime_home.path(),
            &project_id,
            &registry_before,
            relationship.expected_error(),
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
        assert_registry_unchanged_and_project_list_rejects(
            missing_runtime_home.path(),
            &missing_project_id,
            &missing_registry_before,
            relationship.expected_error(),
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
        assert_registry_unchanged_and_project_list_rejects(
            missing_runtime_home.path(),
            &missing_project_id,
            &missing_registry_before,
            relationship.expected_error(),
        )?;

        let existing_project_id = relationship.project_id("existing");
        let existing_runtime_home = initialized_project(
            &relationship.prefix("existing"),
            &existing_project_id,
            "runtime_home_cli_surface_existing",
        )?;
        relationship.replace_repo_root(&existing_runtime_home, &existing_project_id)?;
        let existing_registry_before =
            single_project_record(existing_runtime_home.path(), &existing_project_id)?;
        let existing_state_path =
            project_state_db_path(existing_runtime_home.path(), &existing_project_id);
        fs::remove_file(&existing_state_path)?;
        let existing = open_project_state_database(&existing_state_path)?;
        drop(existing);
        let existing_migrations_before = migration_count(&existing_state_path)?;
        let existing_surface_count_before = surface_count(&existing_state_path)?;

        let existing_list = run_with_home(
            existing_runtime_home.path(),
            [
                "surface",
                "list",
                "--project-id",
                existing_project_id.as_str(),
            ],
        )?;
        assert_invalid_project_path_error(&existing_list, relationship.expected_error());
        assert_project_state_unchanged(
            &existing_state_path,
            existing_migrations_before,
            existing_surface_count_before,
        )?;

        let existing_register = run_with_home(
            existing_runtime_home.path(),
            [
                "surface",
                "register",
                "--project-id",
                existing_project_id.as_str(),
                "--surface-id",
                "surface_existing",
                "--surface-instance-id",
                "surface_instance_existing",
            ],
        )?;
        assert_invalid_project_path_error(&existing_register, relationship.expected_error());
        assert_project_state_unchanged(
            &existing_state_path,
            existing_migrations_before,
            existing_surface_count_before,
        )?;
        assert_registry_unchanged_and_project_list_rejects(
            existing_runtime_home.path(),
            &existing_project_id,
            &existing_registry_before,
            relationship.expected_error(),
        )?;
    }

    Ok(())
}

#[test]
fn volicord_binary_project_list_rejects_state_db_path_mismatch_without_creating_alternate(
) -> Result<(), Box<dyn Error>> {
    let project_id = "project_cli_state_db_mismatch";
    let runtime_home = initialized_project(
        "cli-project-list-state-db-mismatch",
        project_id,
        "runtime_home_cli_project_list_state_db",
    )?;
    let registry_before = single_project_record(runtime_home.path(), project_id)?;
    let alternate_state_path = runtime_home
        .path()
        .join("alternate")
        .join("mismatched-state.sqlite");
    replace_project_state_db_path(runtime_home.path(), project_id, &alternate_state_path)?;
    assert!(!alternate_state_path.exists());

    let output = run_with_home(runtime_home.path(), ["project", "list"])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("state_db_path_mismatch"));
    assert!(!alternate_state_path.exists());
    let registry_after = single_project_record(runtime_home.path(), project_id)?;
    assert_eq!(registry_after.state_db_path, alternate_state_path);
    assert_eq!(registry_after.repo_root, registry_before.repo_root);
    assert_eq!(registry_after.project_home, registry_before.project_home);
    Ok(())
}

#[test]
fn volicord_binary_agent_help_covers_nested_commands() -> Result<(), Box<dyn Error>> {
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
fn volicord_binary_agent_option_alignment_for_server_name_and_mcp_command(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-options")?;
    let repo_user = runtime_home.create_product_repo("repo-user")?;
    let repo_project = runtime_home.create_product_repo("repo-project")?;
    let repo_local = runtime_home.create_product_repo("repo-local")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;

    let omitted_server = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_server_name_default",
            "--project-id",
            "project_server_name",
            "--repo-root",
            path_text(&repo_user).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&omitted_server);
    let omitted_server_json: Value = serde_json::from_str(&stdout(&omitted_server))?;
    assert_eq!(
        omitted_server_json["host"]["server_name"],
        "harness-agent_server_name_default"
    );

    let project_absolute = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "project",
            "--integration-id",
            "agent_project_absolute",
            "--project-id",
            "project_absolute",
            "--repo-root",
            path_text(&repo_project).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
        ],
    )?;
    assert_eq!(project_absolute.status.code(), Some(2));
    assert!(stderr(&project_absolute).contains("project-scoped host configuration"));
    assert!(stderr(&project_absolute).contains("harness-mcp"));

    let project_portable = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "project",
            "--integration-id",
            "agent_project_portable",
            "--project-id",
            "project_portable",
            "--repo-root",
            path_text(&repo_project).as_str(),
            "--mcp-command",
            "harness-mcp",
            "--dry-run",
            "--output",
            "json",
        ],
    )?;
    assert_success(&project_portable);
    let project_portable_json: Value = serde_json::from_str(&stdout(&project_portable))?;
    assert_eq!(project_portable_json["status"], "dry_run");
    assert_eq!(project_portable_json["host"]["host_scope"], "project");

    let local_absolute = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "claude_code",
            "--scope",
            "local",
            "--integration-id",
            "agent_local_absolute",
            "--project-id",
            "project_local_absolute",
            "--repo-root",
            path_text(&repo_local).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--dry-run",
            "--output",
            "json",
        ],
    )?;
    assert_success(&local_absolute);
    let local_absolute_json: Value = serde_json::from_str(&stdout(&local_absolute))?;
    assert_eq!(local_absolute_json["status"], "dry_run");
    assert_eq!(local_absolute_json["host"]["host_scope"], "local");
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_codex_user_install_verify_and_uninstall() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
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
        &codex_env,
    )?;
    assert_success(&status);
    assert!(stdout(&status).contains("host_state"));

    let verify = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "verify", "--integration-id", "agent_codex_user"],
        &codex_env,
    )?;
    assert_success(&verify);
    assert!(stdout(&verify).contains("status: complete"));

    let uninstall = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "uninstall", "--integration-id", "agent_codex_user"],
        &codex_env,
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
fn volicord_binary_agent_codex_existing_user_target_survives_environment_changes_and_uninstall(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex-stored-target")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let stored_codex_home = runtime_home.path().join("codex-home-a");
    let ambient_codex_home = runtime_home.path().join("codex-home-b");
    let later_home = runtime_home.path().join("later-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let install_env = codex_env(&stored_codex_home, &[runtime_home.path()]);

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
            "agent_codex_stored_target",
            "--server-name",
            "harness-stored-target",
            "--project-id",
            "project_codex_stored_target",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &install_env,
    )?;
    assert_success(&install);
    let stored_config = stored_codex_home.join("config.toml");
    let stored_config_text = path_text(&stored_config);
    let installations =
        list_host_installations_for_integration(runtime_home.path(), "agent_codex_stored_target")?;
    assert_eq!(installations[0].config_target, stored_config_text);

    fs::create_dir_all(&ambient_codex_home)?;
    let ambient_config = ambient_codex_home.join("config.toml");
    let ambient_text =
        "[mcp_servers.harness-stored-target]\ncommand = \"ambient-codex-home\"\n".to_owned();
    fs::write(&ambient_config, &ambient_text)?;
    let ambient_env = codex_env(&ambient_codex_home, &[runtime_home.path()]);
    let status = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "status",
            "--integration-id",
            "agent_codex_stored_target",
        ],
        &ambient_env,
    )?;
    assert_success(&status);
    assert!(stdout(&status).contains("configured_ready"));

    fs::create_dir_all(later_home.join(".codex"))?;
    fs::write(
        later_home.join(".codex").join("config.toml"),
        "[mcp_servers.harness-stored-target]\ncommand = \"ambient-home\"\n",
    )?;
    let no_codex_home_env = vec![
        ("HOME", path_text(&later_home)),
        ("PATH", path_env(&[runtime_home.path()])),
    ];
    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_codex_stored_target",
            "--output",
            "json",
        ],
        &no_codex_home_env,
    )?;
    assert_success(&verify);
    let value: Value = serde_json::from_str(&stdout(&verify))?;
    assert_eq!(value["status"], "complete");
    assert_eq!(
        value["installation_verifications"][0]["config_target"],
        stored_config_text
    );
    assert_eq!(fs::read_to_string(&ambient_config)?, ambient_text);

    let uninstall = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "uninstall",
            "--integration-id",
            "agent_codex_stored_target",
        ],
        &ambient_env,
    )?;
    assert_success(&uninstall);
    assert_no_codex_server(&stored_config, "harness-stored-target")?;
    assert_eq!(fs::read_to_string(&ambient_config)?, ambient_text);
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_codex_verify_reports_stored_missing_without_ambient_fallback(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex-stored-missing")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let stored_codex_home = runtime_home.path().join("codex-home-a");
    let ambient_codex_home = runtime_home.path().join("codex-home-b");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let install_env = codex_env(&stored_codex_home, &[runtime_home.path()]);

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
            "agent_codex_stored_missing",
            "--server-name",
            "harness-stored-missing",
            "--project-id",
            "project_codex_stored_missing",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &install_env,
    )?;
    assert_success(&install);

    let stored_config = stored_codex_home.join("config.toml");
    let valid_config = fs::read_to_string(&stored_config)?;
    fs::create_dir_all(&ambient_codex_home)?;
    fs::write(ambient_codex_home.join("config.toml"), &valid_config)?;
    fs::remove_file(&stored_config)?;
    let ambient_env = codex_env(&ambient_codex_home, &[runtime_home.path()]);

    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_codex_stored_missing",
            "--output",
            "json",
        ],
        &ambient_env,
    )?;
    assert_eq!(verify.status.code(), Some(1));
    let value: Value = serde_json::from_str(&stdout(&verify))?;
    assert_eq!(value["status"], "failed");
    assert_eq!(value["verification"]["managed_config"], "missing");
    assert_eq!(
        value["installation_verifications"][0]["host_state"],
        "missing"
    );
    assert_eq!(
        value["installation_verifications"][0]["config_target"],
        path_text(&stored_config)
    );
    assert_eq!(
        fs::read_to_string(ambient_codex_home.join("config.toml"))?,
        valid_config
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_codex_user_install_requires_codex_executable() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex-missing")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_log = runtime_home.path().join("mcp.log");
    let mcp_command = write_recording_agent_mcp(
        runtime_home.path(),
        "agent-mcp-codex-missing",
        AgentMcpFixture::Complete,
        &mcp_log,
    )?;
    fs::write(&mcp_log, "")?;

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
            "agent_codex_missing",
            "--server-name",
            "harness-codex-missing",
            "--project-id",
            "project_codex_missing",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;

    assert_success(&install);
    let value: Value = serde_json::from_str(&stdout(&install))?;
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["verification"]["status"], "action_required");
    assert_eq!(value["verification"]["host_executable"], "unavailable");
    assert_eq!(value["verification"]["mcp_handshake_diagnostic"], false);
    assert!(value["verification"]["details"]
        .as_str()
        .unwrap_or_default()
        .contains("not found on PATH"));
    let mcp_log_text = fs::read_to_string(&mcp_log)?;
    assert!(mcp_log_text.contains("--check --integration agent_codex_missing"));
    assert!(!mcp_log_text
        .lines()
        .any(|line| line.contains("--integration agent_codex_missing")
            && !line.contains("--check --integration")));
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_codex_missing")?[0]
            .last_verified_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_codex_user_verify_requires_codex_executable() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex-verify-missing")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_log = runtime_home.path().join("mcp.log");
    let mcp_command = write_recording_agent_mcp(
        runtime_home.path(),
        "agent-mcp-codex-verify-missing",
        AgentMcpFixture::Complete,
        &mcp_log,
    )?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let ready_env = codex_env(&codex_home, &[runtime_home.path()]);

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
            "agent_codex_verify_missing",
            "--server-name",
            "harness-codex-verify-missing",
            "--project-id",
            "project_codex_verify_missing",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &ready_env,
    )?;
    assert_success(&install);
    fs::write(&mcp_log, "")?;

    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_codex_verify_missing",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;

    assert_success(&verify);
    let value: Value = serde_json::from_str(&stdout(&verify))?;
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["verification"]["host_executable"], "unavailable");
    let results = value["installation_verifications"]
        .as_array()
        .expect("installation verifications");
    assert_eq!(results[0]["mcp_handshake_result"]["status"], "skipped");
    assert_eq!(results[0]["final_status"], "action_required");
    assert_eq!(fs::read_to_string(&mcp_log)?, "");
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_codex_verify_missing")?
            [0]
        .last_verified_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_codex_user_failing_executable_is_action_required(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-codex-version-fails")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_log = runtime_home.path().join("mcp.log");
    let mcp_command = write_recording_agent_mcp(
        runtime_home.path(),
        "agent-mcp-codex-version-fails",
        AgentMcpFixture::Complete,
        &mcp_log,
    )?;
    write_fake_codex(runtime_home.path(), CodexFixture::VersionFails)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);
    fs::write(&mcp_log, "")?;

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
            "agent_codex_version_fails",
            "--server-name",
            "harness-codex-version-fails",
            "--project-id",
            "project_codex_version_fails",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--output",
            "json",
        ],
        &codex_env,
    )?;

    assert_success(&install);
    let value: Value = serde_json::from_str(&stdout(&install))?;
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["verification"]["host_executable"], "unavailable");
    assert_eq!(value["verification"]["mcp_handshake_diagnostic"], false);
    assert!(value["verification"]["host_diagnostic"]
        .as_str()
        .unwrap_or_default()
        .contains("status 17"));
    let mcp_log_text = fs::read_to_string(&mcp_log)?;
    assert!(mcp_log_text.contains("--check --integration agent_codex_version_fails"));
    assert!(!mcp_log_text.lines().any(|line| line
        .contains("--integration agent_codex_version_fails")
        && !line.contains("--check --integration")));
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_dry_run_writes_nothing_and_rejects_invalid_scope(
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
fn volicord_binary_agent_dry_run_missing_runtime_home_creates_nothing() -> Result<(), Box<dyn Error>>
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
fn volicord_binary_agent_generic_export_dry_run_creates_no_export_files(
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
fn volicord_binary_agent_dry_run_old_profile_registry_is_read_only_and_rejected(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-registry-old-profile-dry")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    create_registry_fixture_with_project(
        runtime_home.path(),
        &repo_root,
        "project_registry_old_profile_dry",
    )?;
    mark_registry_old_profile(runtime_home.path())?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = runtime_home.path().join("harness-mcp-old-profile-dry");
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
            "agent_registry_old_profile_dry",
            "--project-id",
            "project_registry_old_profile_dry",
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

    assert_eq!(dry_run.status.code(), Some(1));
    assert!(stdout(&dry_run).is_empty());
    let error = stderr(&dry_run);
    assert!(error.contains("baseline_sqlite"));
    assert!(error.contains("baseline_sqlite_v2"));
    assert!(error.contains("explicitly reinitialize the Runtime Home"));
    assert_eq!(file_hash(&registry_path)?, hash_before);
    assert_eq!(
        migration_count_for(&registry_path, REGISTRY_DATABASE_KIND)?,
        migrations_before
    );
    assert_eq!(
        registry_storage_profile(&registry_path, "runtime_home")?,
        "baseline_sqlite"
    );
    assert!(table_exists(&registry_path, "agent_integrations")?);
    assert_eq!(existing_sidecars(&[registry_path]), sidecars_before);
    assert!(!codex_home.join("config.toml").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_dry_run_current_registry_is_byte_identical() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-registry-v2-dry")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
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
        &codex_env,
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
fn volicord_binary_agent_dry_run_unsupported_future_registry_is_read_only(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-registry-future-dry")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    create_registry_fixture_with_project(
        runtime_home.path(),
        &repo_root,
        "project_registry_future_dry",
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
fn volicord_binary_agent_guidance_apply_status_and_remove_flow() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-flow")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
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
fn volicord_binary_agent_install_guidance_both_and_uninstall_managed() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-install")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
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
        &codex_env,
    )?;
    assert_success(&uninstall);
    assert!(stdout(&uninstall).contains("status: complete"));
    assert!(!agents_path.exists());
    assert!(!repo_root.join(".claude").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_uninstall_preserves_changed_guidance() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-conflict")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
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
        &codex_env,
    )?;
    assert_eq!(uninstall.status.code(), Some(1));
    assert!(stdout(&uninstall).contains("status: partial_failure"));
    assert!(stdout(&uninstall).contains("residual guidance preserved"));
    assert!(fs::read_to_string(&agents_path)?.contains("guess project_id"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_install_compensates_new_guidance_after_verification_failure(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-compensate")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::MissingUtilityTool)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);
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
        &codex_env,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("status: partial_failure"));
    assert!(stdout(&output).contains("harness.list_projects"));
    assert!(stdout(&output).contains("effects:"));
    assert!(stdout(&output).contains("residual_effects:"));
    assert!(stdout(&output).contains("rollback_state: rolled_back"));
    let agents = fs::read_to_string(&agents_path)?;
    assert!(agents.contains("# Existing instructions\nKeep this.\n"));
    assert!(!agents.contains(GUIDANCE_BEGIN_MARKER));
    assert_no_codex_server(
        &codex_home.join("config.toml"),
        "harness-guidance-compensate",
    )?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_install_faults_roll_back_reversible_effects() -> Result<(), Box<dyn Error>>
{
    for step in [
        "preflight",
        "host_apply",
        "inventory_record",
        "final_verification_status_update",
    ] {
        let runtime_home = TempRuntimeHome::new(&format!("cli-bin-agent-fault-{step}"))?;
        let repo_root = runtime_home.create_product_repo("product-repo")?;
        let project_id = format!("project_fault_{step}");
        let integration_id = format!("agent_fault_{step}");
        let surface_id = format!("surface_fault_{step}");
        let surface_instance_id = format!("surface_instance_fault_{step}");
        let server_name = format!("harness-fault-{}", step.replace('_', "-"));
        initialize_agent_install_fixture(
            runtime_home.path(),
            &repo_root,
            &project_id,
            &surface_id,
            &surface_instance_id,
        )?;
        let codex_home = runtime_home.path().join("codex-home");
        let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
        write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
        let mut env = codex_env(&codex_home, &[runtime_home.path()]);
        env.push(("HARNESS_TEST_AGENT_INSTALL_FAIL_STEP", step.to_owned()));

        let output = run_with_home_and_env_slice(
            runtime_home.path(),
            &[
                "agent",
                "install",
                "--host",
                "codex",
                "--scope",
                "user",
                "--integration-id",
                &integration_id,
                "--server-name",
                &server_name,
                "--project-id",
                &project_id,
                "--repo-root",
                path_text(&repo_root).as_str(),
                "--surface-id",
                &surface_id,
                "--surface-instance-id",
                &surface_instance_id,
                "--mcp-command",
                path_text(&mcp_command).as_str(),
                "--output",
                "json",
            ],
            &env,
        )?;

        assert_eq!(output.status.code(), Some(1), "step {step}");
        let value: Value = serde_json::from_str(&stdout(&output))?;
        assert_eq!(value["status"], "failed", "step {step}");
        assert!(value["residual_effects"]
            .as_array()
            .expect("residual effects array")
            .is_empty());
        assert_effect_rollback(&value, "integration", "rolled_back");
        assert_effect_rollback(&value, "project_allowlist", "rolled_back");
        assert_effect_rollback(&value, "default_project", "rolled_back");
        if matches!(
            step,
            "inventory_record" | "final_verification_status_update"
        ) {
            assert_effect_rollback(&value, "host_config", "rolled_back");
        }
        if step == "final_verification_status_update" {
            assert_effect_rollback(&value, "host_inventory", "rolled_back");
        }
        assert!(agent_integration_record(runtime_home.path(), &integration_id)?.is_none());
        assert_no_codex_server(&codex_home.join("config.toml"), &server_name)?;
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_install_guidance_apply_faults_are_compensated(
) -> Result<(), Box<dyn Error>> {
    for step in ["guidance_apply_1", "guidance_apply_2"] {
        let runtime_home = TempRuntimeHome::new(&format!("cli-bin-agent-guidance-{step}"))?;
        let repo_root = runtime_home.create_product_repo("product-repo")?;
        let project_id = format!("project_{step}");
        let integration_id = format!("agent_{step}");
        let surface_id = format!("surface_{step}");
        let surface_instance_id = format!("surface_instance_{step}");
        let server_name = format!("harness-{}", step.replace('_', "-"));
        initialize_agent_install_fixture(
            runtime_home.path(),
            &repo_root,
            &project_id,
            &surface_id,
            &surface_instance_id,
        )?;
        let codex_home = runtime_home.path().join("codex-home");
        let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
        write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
        let mut env = codex_env(&codex_home, &[runtime_home.path()]);
        env.push(("HARNESS_TEST_AGENT_INSTALL_FAIL_STEP", step.to_owned()));

        let output = run_with_home_and_env_slice(
            runtime_home.path(),
            &[
                "agent",
                "install",
                "--host",
                "codex",
                "--scope",
                "user",
                "--integration-id",
                &integration_id,
                "--server-name",
                &server_name,
                "--project-id",
                &project_id,
                "--repo-root",
                path_text(&repo_root).as_str(),
                "--surface-id",
                &surface_id,
                "--surface-instance-id",
                &surface_instance_id,
                "--mcp-command",
                path_text(&mcp_command).as_str(),
                "--guidance",
                "both",
                "--allow-repository-write",
                "--output",
                "json",
            ],
            &env,
        )?;

        assert_eq!(output.status.code(), Some(1), "step {step}");
        let value: Value = serde_json::from_str(&stdout(&output))?;
        assert_eq!(value["status"], "failed", "step {step}");
        assert!(value["residual_effects"]
            .as_array()
            .expect("residual effects array")
            .is_empty());
        if step == "guidance_apply_2" {
            assert_effect_rollback(&value, "guidance", "rolled_back");
        }
        assert!(agent_integration_record(runtime_home.path(), &integration_id)?.is_none());
        assert_no_codex_server(&codex_home.join("config.toml"), &server_name)?;
        if repo_root.join("AGENTS.md").exists() {
            assert!(
                !fs::read_to_string(repo_root.join("AGENTS.md"))?.contains(GUIDANCE_BEGIN_MARKER)
            );
        }
        assert!(!repo_root
            .join(".claude")
            .join("rules")
            .join("harness.md")
            .exists());
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_install_reports_rollback_residuals() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-host-rollback-residual")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    initialize_agent_install_fixture(
        runtime_home.path(),
        &repo_root,
        "project_host_rollback",
        "surface_host_rollback",
        "surface_instance_host_rollback",
    )?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let mut host_env = codex_env(&codex_home, &[runtime_home.path()]);
    host_env.push((
        "HARNESS_TEST_AGENT_INSTALL_FAIL_STEP",
        "inventory_record".to_owned(),
    ));
    host_env.push((
        "HARNESS_TEST_AGENT_INSTALL_ROLLBACK_FAIL",
        "host".to_owned(),
    ));
    let host_residual = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_host_rollback",
            "--server-name",
            "harness-host-rollback",
            "--project-id",
            "project_host_rollback",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--surface-id",
            "surface_host_rollback",
            "--surface-instance-id",
            "surface_instance_host_rollback",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--output",
            "json",
        ],
        &host_env,
    )?;
    assert_eq!(host_residual.status.code(), Some(1));
    let value: Value = serde_json::from_str(&stdout(&host_residual))?;
    assert_eq!(value["status"], "partial_failure");
    assert_effect_rollback(&value, "host_config", "failed");
    assert!(value["residual_effects"]
        .as_array()
        .expect("residual effects array")
        .iter()
        .any(|residual| residual["component"] == "host_config"));
    assert!(fs::read_to_string(codex_home.join("config.toml"))?
        .contains("[mcp_servers.harness-host-rollback]"));

    let runtime_home = TempRuntimeHome::new("cli-bin-agent-guidance-rollback-residual")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    initialize_agent_install_fixture(
        runtime_home.path(),
        &repo_root,
        "project_guidance_rollback",
        "surface_guidance_rollback",
        "surface_instance_guidance_rollback",
    )?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let mut guidance_env = codex_env(&codex_home, &[runtime_home.path()]);
    guidance_env.push((
        "HARNESS_TEST_AGENT_INSTALL_FAIL_STEP",
        "final_verification_status_update".to_owned(),
    ));
    guidance_env.push((
        "HARNESS_TEST_AGENT_INSTALL_ROLLBACK_FAIL",
        "guidance".to_owned(),
    ));
    let guidance_residual = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_guidance_rollback",
            "--server-name",
            "harness-guidance-rollback",
            "--project-id",
            "project_guidance_rollback",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--surface-id",
            "surface_guidance_rollback",
            "--surface-instance-id",
            "surface_instance_guidance_rollback",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--guidance",
            "codex",
            "--allow-repository-write",
            "--output",
            "json",
        ],
        &guidance_env,
    )?;
    assert_eq!(guidance_residual.status.code(), Some(1));
    let value: Value = serde_json::from_str(&stdout(&guidance_residual))?;
    assert_eq!(value["status"], "partial_failure");
    assert_effect_rollback(&value, "guidance", "failed");
    assert!(value["residual_effects"]
        .as_array()
        .expect("residual effects array")
        .iter()
        .any(|residual| residual["component"] == "guidance"));
    assert!(fs::read_to_string(repo_root.join("AGENTS.md"))?.contains(GUIDANCE_BEGIN_MARKER));
    assert_no_codex_server(&codex_home.join("config.toml"), "harness-guidance-rollback")?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_user_scope_project_membership_is_single_host_entry(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-project-add")?;
    let repo_a = runtime_home.create_product_repo("repo-a")?;
    let repo_b = runtime_home.create_product_repo("repo-b")?;
    let repo_c = runtime_home.create_product_repo("repo-c")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
    )?;
    assert_success(&install);
    assert_eq!(
        agent_integration_record(runtime_home.path(), "agent_multi_project")?
            .expect("integration should exist")
            .default_project_id
            .as_deref(),
        Some("project_a")
    );

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
        &codex_env,
    )?;
    assert_success(&add);
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_multi_project")?.len(),
        1
    );

    let register_c = run_with_home(
        runtime_home.path(),
        [
            "project",
            "register",
            "--project-id",
            "project_c",
            "--repo-root",
            path_text(&repo_c).as_str(),
        ],
    )?;
    assert_success(&register_c);

    let non_member_default = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "set",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_c",
        ],
    )?;
    assert_eq!(non_member_default.status.code(), Some(1));
    assert!(stderr(&non_member_default).contains("not allowed"));

    let unregistered_default = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "set",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_missing",
        ],
    )?;
    assert_eq!(unregistered_default.status.code(), Some(1));
    assert!(stderr(&unregistered_default).contains("not registered"));

    let same_default = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "set",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_a",
            "--output",
            "json",
        ],
    )?;
    assert_success(&same_default);
    let same_default_json: Value = serde_json::from_str(&stdout(&same_default))?;
    assert_eq!(same_default_json["default_project"]["result"], "reused");
    assert_eq!(
        same_default_json["default_project"]["prior_default_project_id"],
        "project_a"
    );
    assert_eq!(
        same_default_json["default_project"]["resulting_default_project_id"],
        "project_a"
    );

    let registry_path = registry_db_path(runtime_home.path());
    let hash_before_dry_set = file_hash(&registry_path)?;
    let dry_set = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "set",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_b",
            "--dry-run",
            "--output",
            "json",
        ],
    )?;
    assert_success(&dry_set);
    let dry_set_json: Value = serde_json::from_str(&stdout(&dry_set))?;
    assert_eq!(dry_set_json["status"], "dry_run");
    assert_eq!(dry_set_json["default_project"]["result"], "changed");
    assert_eq!(file_hash(&registry_path)?, hash_before_dry_set);
    assert_eq!(
        agent_integration_record(runtime_home.path(), "agent_multi_project")?
            .expect("integration should exist")
            .default_project_id
            .as_deref(),
        Some("project_a")
    );

    let set_b = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "set",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_b",
            "--output",
            "json",
        ],
    )?;
    assert_success(&set_b);
    let set_b_json: Value = serde_json::from_str(&stdout(&set_b))?;
    assert_eq!(set_b_json["default_project"]["result"], "changed");
    assert_eq!(
        set_b_json["default_project"]["prior_default_project_id"],
        "project_a"
    );
    assert_eq!(
        set_b_json["default_project"]["resulting_default_project_id"],
        "project_b"
    );
    assert_eq!(
        agent_integration_record(runtime_home.path(), "agent_multi_project")?
            .expect("integration should exist")
            .default_project_id
            .as_deref(),
        Some("project_b")
    );

    let remove_current_default = run_with_home_and_env(
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
        &codex_env,
    )?;
    assert_eq!(remove_current_default.status.code(), Some(1));
    assert!(stderr(&remove_current_default).contains("default project"));
    assert!(stderr(&remove_current_default).contains("agent project default set"));
    assert!(stderr(&remove_current_default).contains("agent project default clear"));

    let remove_a_after_change = run_with_home_and_env(
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
        &codex_env,
    )?;
    assert_success(&remove_a_after_change);

    let add_a_again = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "add",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_a",
        ],
        &codex_env,
    )?;
    assert_success(&add_a_again);

    let hash_before_dry_clear = file_hash(&registry_path)?;
    let dry_clear = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "clear",
            "--integration-id",
            "agent_multi_project",
            "--dry-run",
            "--output",
            "json",
        ],
    )?;
    assert_success(&dry_clear);
    let dry_clear_json: Value = serde_json::from_str(&stdout(&dry_clear))?;
    assert_eq!(dry_clear_json["status"], "dry_run");
    assert_eq!(dry_clear_json["default_project"]["result"], "cleared");
    assert_eq!(file_hash(&registry_path)?, hash_before_dry_clear);
    assert_eq!(
        agent_integration_record(runtime_home.path(), "agent_multi_project")?
            .expect("integration should exist")
            .default_project_id
            .as_deref(),
        Some("project_b")
    );

    let clear = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "clear",
            "--integration-id",
            "agent_multi_project",
            "--output",
            "json",
        ],
    )?;
    assert_success(&clear);
    let clear_json: Value = serde_json::from_str(&stdout(&clear))?;
    assert_eq!(clear_json["default_project"]["result"], "cleared");
    assert_eq!(
        clear_json["default_project"]["prior_default_project_id"],
        "project_b"
    );
    assert!(clear_json["default_project"]["resulting_default_project_id"].is_null());
    assert!(clear_json["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|warning| warning
            .as_str()
            .unwrap_or_default()
            .contains("explicit project_id")));

    let repeated_clear = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "project",
            "default",
            "clear",
            "--integration-id",
            "agent_multi_project",
        ],
    )?;
    assert_success(&repeated_clear);
    assert!(stdout(&repeated_clear).contains("result: reused"));

    let remove_a_after_clear = run_with_home_and_env(
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
        &codex_env,
    )?;
    assert_success(&remove_a_after_clear);

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
        &codex_env,
    )?;
    assert_success(&remove_b);
    assert!(stdout(&remove_b).contains("allowed_project_count: 0"));
    assert!(stdout(&remove_b).contains("not executable until one is added"));
    assert!(list_integration_projects(runtime_home.path(), "agent_multi_project")?.is_empty());
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_multi_project")?.len(),
        1
    );

    let status = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "status",
            "--integration-id",
            "agent_multi_project",
            "--output",
            "json",
        ],
    )?;
    assert_success(&status);
    let status_json: Value = serde_json::from_str(&stdout(&status))?;
    assert_eq!(status_json["project"]["allowed_project_count"], 0);
    assert_eq!(
        status_json["allowed_projects"]
            .as_array()
            .expect("allowed projects array")
            .len(),
        0
    );
    assert!(status_json["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|warning| warning
            .as_str()
            .unwrap_or_default()
            .contains("not executable")));

    let verify_empty = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_multi_project",
            "--output",
            "json",
        ],
        &codex_env,
    )?;
    assert_eq!(verify_empty.status.code(), Some(1));
    let verify_empty_json: Value = serde_json::from_str(&stdout(&verify_empty))?;
    assert_eq!(verify_empty_json["status"], "failed");
    assert_eq!(verify_empty_json["verification"]["status"], "failed");
    assert!(verify_empty_json["verification"]["details"]
        .as_str()
        .unwrap_or_default()
        .contains("no allowed projects"));
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_multi_project")?[0]
            .last_verified_status,
        VERIFIED_STATUS_FAILED
    );

    let add_b_again = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "add",
            "--integration-id",
            "agent_multi_project",
            "--project-id",
            "project_b",
        ],
        &codex_env,
    )?;
    assert_success(&add_b_again);
    assert_eq!(
        list_host_installations_for_integration(runtime_home.path(), "agent_multi_project")?.len(),
        1
    );

    let verify_restored = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "verify", "--integration-id", "agent_multi_project"],
        &codex_env,
    )?;
    assert_success(&verify_restored);
    assert!(stdout(&verify_restored).contains("status: complete"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_project_and_uninstall_dry_runs_are_read_only() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-project-uninstall-dry")?;
    let repo_a = runtime_home.create_product_repo("repo-a")?;
    let repo_b = runtime_home.create_product_repo("repo-b")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

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
        &codex_env,
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
        &codex_env,
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
        &codex_env,
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
        &codex_env,
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
fn volicord_binary_agent_claude_project_install_reports_action_required(
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
    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_claude_project",
        ],
        &[("PATH", path_text(&bin_dir))],
    )?;
    assert_success(&verify);
    assert!(stdout(&verify).contains("status: action_required"));
    assert!(stdout(&verify).contains("mcp_handshake_result: complete"));
    assert!(stdout(&verify).contains("final_status: action_required"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_claude_project_connected_verify_completes() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-claude-connected")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    fs::create_dir_all(&bin_dir)?;
    let mcp = write_agent_mcp(&bin_dir, AgentMcpFixture::Complete)?;
    fs::rename(&mcp, bin_dir.join("harness-mcp"))?;
    write_fake_claude_mcp_get(
        &bin_dir,
        "Status: ✓ Connected\nScope: project\nCommand: harness-mcp\nArgs: [\"--integration\",\"agent_claude_connected\"]",
    )?;

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
            "agent_claude_connected",
            "--server-name",
            "harness-claude-connected",
            "--project-id",
            "project_claude_connected",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--allow-repository-write",
        ],
        &[("PATH", path_text(&bin_dir))],
    )?;
    assert_success(&install);
    assert!(stdout(&install).contains("status: complete"));

    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_claude_connected",
        ],
        &[("PATH", path_text(&bin_dir))],
    )?;
    assert_success(&verify);
    assert!(stdout(&verify).contains("status: complete"));
    assert!(stdout(&verify).contains("host_state: configured_ready"));
    assert!(stdout(&verify).contains("tool_discovery_result: complete"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_generic_export_verify_remains_action_required(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-generic-verify")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    let export_path = runtime_home.path().join("harness-generic.mcp.json");
    let install = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "generic",
            "--scope",
            "export",
            "--integration-id",
            "agent_generic_verify",
            "--server-name",
            "harness-generic",
            "--project-id",
            "project_generic_verify",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--export-path",
            path_text(&export_path).as_str(),
        ],
    )?;
    assert_success(&install);
    assert!(stdout(&install).contains("status: action_required"));

    let verify = run_with_home(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_generic_verify",
        ],
    )?;
    assert_success(&verify);
    assert!(stdout(&verify).contains("status: action_required"));
    assert!(stdout(&verify).contains("host_kind: generic"));
    assert!(stdout(&verify).contains("mcp_handshake_result: complete"));
    assert!(stdout(&verify).contains("final_status: action_required"));
    let installations =
        list_host_installations_for_integration(runtime_home.path(), "agent_generic_verify")?;
    assert_eq!(
        installations[0].last_verified_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_verify_reports_persistence_update_failure() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let runtime_home = TempRuntimeHome::new("cli-bin-agent-verify-persist")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let mcp_command = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);
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
            "agent_verify_persist",
            "--server-name",
            "harness-persist",
            "--project-id",
            "project_verify_persist",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &codex_env,
    )?;
    assert_success(&install);

    let registry = registry_db_path(runtime_home.path());
    let original_permissions = fs::metadata(&registry)?.permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_mode(0o444);
    fs::set_permissions(&registry, readonly)?;
    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_verify_persist",
        ],
        &codex_env,
    )?;
    fs::set_permissions(&registry, original_permissions)?;

    assert_eq!(verify.status.code(), Some(1));
    assert!(stdout(&verify).contains("status: partial_failure"));
    assert!(stdout(&verify).contains("persistence_result: failed"));
    assert!(stdout(&verify).contains("failed to update Host Installation"));
    assert!(stdout(&verify).contains("final_status: partial_failure"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_mcp_tool_discovery_failures_are_partial_failure(
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
        write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
        let codex_env = codex_env(&codex_home, &[runtime_home.path()]);
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
            &codex_env,
        )?;
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("status: partial_failure"));
        assert!(stdout(&output).contains(expected));
        assert!(stdout(&output).contains("residual_effects:"));
        assert!(agent_integration_record(runtime_home.path(), "agent_mcp_failure")?.is_none());
        assert_no_codex_server(&codex_home.join("config.toml"), "harness-failure")?;
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_verify_selected_installation_uses_only_selected_command(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-verify-selected")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let first_codex_home = runtime_home.path().join("codex-home-first");
    let second_codex_home = runtime_home.path().join("codex-home-second");
    let ambient_codex_home = runtime_home.path().join("codex-home-ambient");
    let bin_dir = runtime_home.path().join("bin");
    let first_log = runtime_home.path().join("first.log");
    let second_log = runtime_home.path().join("second.log");
    let first_mcp = write_recording_agent_mcp(
        &bin_dir,
        "agent-mcp-first",
        AgentMcpFixture::Complete,
        &first_log,
    )?;
    let second_mcp = write_recording_agent_mcp(
        &bin_dir,
        "agent-mcp-second",
        AgentMcpFixture::Complete,
        &second_log,
    )?;
    write_fake_codex(&bin_dir, CodexFixture::Ready)?;

    for (server_name, command, codex_home) in [
        ("harness-first", &first_mcp, &first_codex_home),
        ("harness-second", &second_mcp, &second_codex_home),
    ] {
        let codex_env = codex_env(codex_home, &[&bin_dir]);
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
                "agent_verify_selected",
                "--server-name",
                server_name,
                "--project-id",
                "project_verify_selected",
                "--repo-root",
                path_text(&repo_root).as_str(),
                "--mcp-command",
                path_text(command).as_str(),
            ],
            &codex_env,
        )?;
        assert_success(&install);
    }

    let installations =
        list_host_installations_for_integration(runtime_home.path(), "agent_verify_selected")?;
    assert_eq!(installations.len(), 2);
    let selected = installations
        .iter()
        .find(|installation| installation.server_name == "harness-first")
        .expect("first installation")
        .clone();
    let unselected = installations
        .iter()
        .find(|installation| installation.server_name == "harness-second")
        .expect("second installation")
        .clone();
    update_host_installation_verification(
        runtime_home.path(),
        &unselected.installation_id,
        VERIFIED_STATUS_ACTION_REQUIRED,
        &unselected.managed_fingerprint,
    )?;
    fs::write(&first_log, "")?;
    fs::write(&second_log, "")?;
    fs::create_dir_all(&ambient_codex_home)?;
    fs::write(
        ambient_codex_home.join("config.toml"),
        "[mcp_servers.harness-first]\ncommand = \"ambient\"\n",
    )?;
    let ambient_env = codex_env(&ambient_codex_home, &[&bin_dir]);

    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_verify_selected",
            "--installation-id",
            selected.installation_id.as_str(),
        ],
        &ambient_env,
    )?;
    assert_success(&verify);
    assert!(stdout(&verify).contains("installation_verifications:"));
    assert!(stdout(&verify).contains(&selected.installation_id));
    assert!(!stdout(&verify).contains(&unselected.installation_id));
    let first_log_text = fs::read_to_string(&first_log)?;
    assert!(first_log_text.contains("--check --integration"));
    assert!(first_log_text.contains("--integration agent_verify_selected"));
    assert!(first_log_text.contains("exit"));
    assert_eq!(fs::read_to_string(&second_log)?, "");

    let after =
        list_host_installations_for_integration(runtime_home.path(), "agent_verify_selected")?;
    assert_eq!(
        after
            .iter()
            .find(|installation| installation.installation_id == selected.installation_id)
            .expect("selected after")
            .last_verified_status,
        VERIFIED_STATUS_COMPLETE
    );
    assert_eq!(
        after
            .iter()
            .find(|installation| installation.installation_id == unselected.installation_id)
            .expect("unselected after")
            .last_verified_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_verify_all_outputs_json_and_aggregates_action_required(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-verify-all")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let bin_dir = runtime_home.path().join("bin");
    let user_log = runtime_home.path().join("user.log");
    let project_log = runtime_home.path().join("project.log");
    let user_mcp = write_recording_agent_mcp(
        &bin_dir,
        "agent-mcp-user",
        AgentMcpFixture::Complete,
        &user_log,
    )?;
    let _project_mcp = write_recording_agent_mcp(
        &bin_dir,
        "harness-mcp",
        AgentMcpFixture::Complete,
        &project_log,
    )?;
    write_fake_codex(&bin_dir, CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[&bin_dir]);

    let user_install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "user",
            "--integration-id",
            "agent_verify_all",
            "--server-name",
            "harness-user",
            "--project-id",
            "project_verify_all",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mcp-command",
            path_text(&user_mcp).as_str(),
        ],
        &codex_env,
    )?;
    assert_success(&user_install);
    let project_install = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "install",
            "--host",
            "codex",
            "--scope",
            "project",
            "--integration-id",
            "agent_verify_all",
            "--server-name",
            "harness-project",
            "--project-id",
            "project_verify_all",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--allow-repository-write",
        ],
        &codex_env,
    )?;
    assert_success(&project_install);
    assert!(stdout(&project_install).contains("status: action_required"));
    fs::write(&user_log, "")?;
    fs::write(&project_log, "")?;

    let verify = run_with_home_and_env(
        runtime_home.path(),
        [
            "agent",
            "verify",
            "--integration-id",
            "agent_verify_all",
            "--output",
            "json",
        ],
        &codex_env,
    )?;
    assert_success(&verify);
    let value: Value = serde_json::from_str(&stdout(&verify))?;
    assert_eq!(value["status"], "action_required");
    let results = value["installation_verifications"]
        .as_array()
        .expect("installation verification results");
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|result| {
        result["server_name"] == "harness-user" && result["final_status"] == "complete"
    }));
    assert!(results.iter().any(|result| {
        result["server_name"] == "harness-project"
            && result["final_status"] == "action_required"
            && result["mcp_handshake_result"]["status"] == "complete"
            && result["required_user_action"][0]
                .as_str()
                .unwrap_or_default()
                .contains("Codex project trust")
    }));
    let user_log_text = fs::read_to_string(&user_log)?;
    let project_log_text = fs::read_to_string(&project_log)?;
    assert!(user_log_text.contains("--integration agent_verify_all"));
    assert!(user_log_text.contains("exit"));
    assert!(project_log_text.contains("--integration agent_verify_all"));
    assert!(project_log_text.contains("exit"));

    let after = list_host_installations_for_integration(runtime_home.path(), "agent_verify_all")?;
    assert_eq!(
        after
            .iter()
            .find(|installation| installation.server_name == "harness-user")
            .expect("user installation")
            .last_verified_status,
        VERIFIED_STATUS_COMPLETE
    );
    assert_eq!(
        after
            .iter()
            .find(|installation| installation.server_name == "harness-project")
            .expect("project installation")
            .last_verified_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn volicord_binary_agent_verify_failed_installation_is_not_hidden() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-agent-verify-failed")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let codex_home = runtime_home.path().join("codex-home");
    let first_mcp = write_agent_mcp(runtime_home.path(), AgentMcpFixture::Complete)?;
    let second_mcp = write_recording_agent_mcp(
        runtime_home.path(),
        "agent-mcp-changed",
        AgentMcpFixture::Complete,
        &runtime_home.path().join("changed.log"),
    )?;
    write_fake_codex(runtime_home.path(), CodexFixture::Ready)?;
    let codex_env = codex_env(&codex_home, &[runtime_home.path()]);

    for (server_name, command) in [("harness-ok", &first_mcp), ("harness-changed", &second_mcp)] {
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
                "agent_verify_failed",
                "--server-name",
                server_name,
                "--project-id",
                "project_verify_failed",
                "--repo-root",
                path_text(&repo_root).as_str(),
                "--mcp-command",
                path_text(command).as_str(),
            ],
            &codex_env,
        )?;
        assert_success(&install);
    }
    let config_path = codex_home.join("config.toml");
    let changed_config = fs::read_to_string(&config_path)?
        .replace(&path_text(&second_mcp), "/bin/not-the-installed-command");
    fs::write(&config_path, changed_config)?;

    let verify = run_with_home_and_env(
        runtime_home.path(),
        ["agent", "verify", "--integration-id", "agent_verify_failed"],
        &codex_env,
    )?;
    assert_eq!(verify.status.code(), Some(1));
    assert!(stdout(&verify).contains("status: failed"));
    assert!(stdout(&verify).contains("harness-ok"));
    assert!(stdout(&verify).contains("final_status: complete"));
    assert!(stdout(&verify).contains("harness-changed"));
    assert!(stdout(&verify).contains("fingerprint_state: changed"));
    assert!(stdout(&verify).contains("final_status: failed"));

    let after =
        list_host_installations_for_integration(runtime_home.path(), "agent_verify_failed")?;
    assert_eq!(
        after
            .iter()
            .find(|installation| installation.server_name == "harness-changed")
            .expect("changed installation")
            .last_verified_status,
        VERIFIED_STATUS_FAILED
    );
    Ok(())
}

fn create_registry_fixture_with_project(
    runtime_home: &Path,
    repo_root: &Path,
    project_id: &str,
) -> Result<ProjectRecord, Box<dyn Error>> {
    initialize_runtime_home(runtime_home, "runtime_home_binary_agent_fixture", "{}")?;
    Ok(register_project(
        runtime_home,
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root: repo_root.to_path_buf(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?)
}

fn mark_registry_old_profile(runtime_home: &Path) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open(registry_db_path(runtime_home))?;
    conn.execute(
        "UPDATE schema_migrations
            SET storage_profile = 'baseline_sqlite'
          WHERE database_kind = ?1",
        [REGISTRY_DATABASE_KIND],
    )?;
    conn.execute(
        "UPDATE runtime_home
            SET storage_profile = 'baseline_sqlite'",
        [],
    )?;
    Ok(())
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

fn registry_storage_profile(path: &Path, table: &str) -> Result<String, Box<dyn Error>> {
    let conn = open_read_only_database(path)?;
    let escaped_table = table.replace('"', "\"\"");
    Ok(conn.query_row(
        &format!("SELECT storage_profile FROM \"{escaped_table}\" WHERE singleton_id = 1"),
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

fn run_with_home_and_env_slice(
    runtime_home: &Path,
    args: &[&str],
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
    assert!(stdout(&output).contains("harness agent project default set"));
    assert!(stdout(&output).contains("harness agent project default clear"));
    assert!(stdout(&output).contains("--guidance none|codex"));
    assert!(stdout(&output).contains("--default-project-id ID"));
    assert!(stdout(&output).contains("--surface-id ID"));
    assert!(stdout(&output).contains("--export-path PATH"));
    assert!(stdout(&output).contains("--remove-managed"));
    assert!(!stdout(&output).contains("--yes"));
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
    let mut command = Command::new(env!("CARGO_BIN_EXE_volicord"));
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

fn initialize_agent_install_fixture(
    runtime_home: &Path,
    repo_root: &Path,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> Result<(), Box<dyn Error>> {
    initialize_runtime_home(runtime_home, "runtime_home_agent_install_fixture", "{}")?;
    register_project(
        runtime_home,
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root: repo_root.to_path_buf(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let access = baseline_workflow_access_classes();
    register_surface(
        runtime_home,
        SurfaceRegistration {
            project_id: project_id.to_owned(),
            surface_id: surface_id.to_owned(),
            surface_instance_id: surface_instance_id.to_owned(),
            surface_kind: "mcp".to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Harness Agent MCP".to_owned()),
            capability_profile_json: capability_profile_json(&access, None)?,
            local_access_json: local_access_json(&access)?,
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn single_project_record(
    runtime_home: &Path,
    project_id: &str,
) -> Result<ProjectRecord, Box<dyn Error>> {
    let conn = open_read_only_database(registry_db_path(runtime_home))?;
    Ok(conn.query_row(
        "SELECT
            project_id,
            runtime_home_id,
            repo_root,
            project_home,
            state_db_path,
            status,
            metadata_json
         FROM projects
         WHERE project_id = ?1",
        [project_id],
        |row| {
            Ok(ProjectRecord {
                project_id: row.get(0)?,
                runtime_home_id: row.get(1)?,
                repo_root: PathBuf::from(row.get::<_, String>(2)?),
                project_home: PathBuf::from(row.get::<_, String>(3)?),
                state_db_path: PathBuf::from(row.get::<_, String>(4)?),
                status: row.get(5)?,
                metadata_json: row.get(6)?,
            })
        },
    )?)
}

fn assert_registry_unchanged_and_project_list_rejects(
    runtime_home: &Path,
    project_id: &str,
    expected: &ProjectRecord,
    relationship: &str,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(&single_project_record(runtime_home, project_id)?, expected);

    let projects = run_with_home(runtime_home, ["project", "list"])?;
    assert_invalid_project_path_error(&projects, relationship);
    Ok(())
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

fn assert_effect_rollback(value: &Value, component: &str, rollback_state: &str) {
    let effects = value["effects"].as_array().expect("effects array");
    assert!(
        effects.iter().any(|effect| {
            effect["component"] == component && effect["rollback_state"] == rollback_state
        }),
        "expected {component} rollback_state={rollback_state}, got {effects:?}"
    );
}

fn assert_no_codex_server(config_path: &Path, server_name: &str) -> Result<(), Box<dyn Error>> {
    if config_path.exists() {
        assert!(!fs::read_to_string(config_path)?.contains(&format!("[mcp_servers.{server_name}]")));
    }
    Ok(())
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

fn assert_project_state_unchanged(
    state_path: &Path,
    expected_migration_count: i64,
    expected_surface_count: i64,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(migration_count(state_path)?, expected_migration_count);
    assert_eq!(surface_count(state_path)?, expected_surface_count);
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

#[cfg(unix)]
#[derive(Clone, Copy)]
enum AgentMcpFixture {
    Complete,
    MissingInstructions,
    MissingUtilityTool,
}

#[derive(Clone, Copy)]
enum CodexFixture {
    Ready,
    VersionFails,
}

#[cfg(unix)]
fn codex_env(codex_home: &Path, path_dirs: &[&Path]) -> Vec<(&'static str, String)> {
    vec![
        ("CODEX_HOME", path_text(codex_home)),
        ("PATH", path_env(path_dirs)),
    ]
}

#[cfg(unix)]
fn path_env(path_dirs: &[&Path]) -> String {
    std::env::join_paths(path_dirs)
        .expect("test PATH should be valid")
        .to_string_lossy()
        .into_owned()
}

#[cfg(unix)]
fn write_fake_codex(dir: &Path, fixture: CodexFixture) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("codex");
    let version_exit = match fixture {
        CodexFixture::Ready => "printf 'codex 1.2.3-test\\n'\nexit 0",
        CodexFixture::VersionFails => "printf 'codex unavailable\\n' >&2\nexit 17",
    };
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             {version_exit}\n\
             fi\n\
             printf 'unexpected codex invocation\\n' >&2\n\
             exit 2\n"
        ),
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn write_agent_mcp(
    dir: &Path,
    fixture: AgentMcpFixture,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    write_agent_mcp_script(dir, None, fixture, None)
}

#[cfg(unix)]
fn write_recording_agent_mcp(
    dir: &Path,
    name: &str,
    fixture: AgentMcpFixture,
    log_path: &Path,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    write_agent_mcp_script(dir, Some(name), fixture, Some(log_path))
}

#[cfg(unix)]
fn write_agent_mcp_script(
    dir: &Path,
    explicit_name: Option<&str>,
    fixture: AgentMcpFixture,
    log_path: Option<&Path>,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let name = explicit_name.unwrap_or(match fixture {
        AgentMcpFixture::Complete => "agent-mcp-complete",
        AgentMcpFixture::MissingInstructions => "agent-mcp-missing-instructions",
        AgentMcpFixture::MissingUtilityTool => "agent-mcp-missing-utility",
    });
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
    let record_start = log_path
        .map(|path| format!("printf '%s\\n' \"$0 $*\" >> '{}'\n", path_text(path)))
        .unwrap_or_default();
    let record_exit = log_path
        .map(|path| format!("printf '%s\\n' 'exit' >> '{}'\n", path_text(path)))
        .unwrap_or_default();
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             {record_start}\
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
             {record_exit}\
             exit 0\n\
             fi\n\
             if [ \"$1\" = \"--integration\" ]; then\n\
             while IFS= read -r line; do\n\
             case \"$line\" in\n\
             *'\"method\":\"notifications/initialized\"'*) ;;\n\
             *'\"method\":\"initialize\"'*) {initialize} ;;\n\
             *'\"method\":\"tools/list\"'*) printf '%s\\n' '{tools}'; {record_exit}exit 0 ;;\n\
             esac\n\
             done\n\
             {record_exit}\
             exit 0\n\
             fi\n\
             printf 'unexpected invocation\\n' >&2\n\
             {record_exit}\
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
