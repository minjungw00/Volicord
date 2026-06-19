#![forbid(unsafe_code)]

use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Output},
};

use harness_cli::setup::{
    AGENT_SURFACE_ID as SETUP_AGENT_SURFACE_ID,
    AGENT_SURFACE_INSTANCE_ID as SETUP_AGENT_SURFACE_INSTANCE_ID,
    USER_INTERACTION_SURFACE_ID as SETUP_USER_SURFACE_ID,
    USER_INTERACTION_SURFACE_INSTANCE_ID as SETUP_USER_INSTANCE_ID,
};
use harness_store::{
    bootstrap::{list_projects, list_surfaces},
    sqlite::{project_state_db_path, registry_db_path},
};
use harness_test_support::TempRuntimeHome;
use serde_json::{json, Value};

const PROJECT_ID: &str = "project_binary_admin";
const AGENT_SURFACE_ID: &str = "surface_binary_agent";
const AGENT_INSTANCE_ID: &str = "surface_instance_binary_agent";
const USER_SURFACE_ID: &str = "surface_binary_user";
const USER_INSTANCE_ID: &str = "surface_instance_binary_user";

#[test]
fn harness_binary_runs_administrative_initialization_and_registration() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-admin")?;
    let repo_root = runtime_home.path().join("product-repo");
    fs::create_dir_all(&repo_root)?;
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
    assert!(stdout(&local_help).contains("--runtime-home PATH"));
    assert!(stdout(&local_help).contains("--dry-run"));

    let invalid = run_without_home(["setup", "local-mcp", "--not-real"])?;
    assert_eq!(invalid.status.code(), Some(2));
    assert!(stderr(&invalid).contains("unknown option"));

    Ok(())
}

#[test]
fn harness_binary_json_dry_run_is_parseable_and_does_not_register() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-dry-run")?;
    let repo_root = runtime_home.path().join("product-repo");
    let mcp_command = runtime_home.path().join("harness-mcp");
    fs::create_dir_all(&repo_root)?;
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

#[cfg(unix)]
#[test]
fn harness_binary_local_mcp_setup_flow() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-real")?;
    let repo_root = runtime_home.path().join("product-repo");
    let config_dir = runtime_home.path().join("configs");
    let marker = runtime_home.path().join("preflight-marker.txt");
    fs::create_dir_all(&repo_root)?;
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
    let repo_root = runtime_home.path().join("product-repo");
    let config_dir = runtime_home.path().join("configs");
    fs::create_dir_all(&repo_root)?;
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

fn run_without_home<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    let mut command = base_command();
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
