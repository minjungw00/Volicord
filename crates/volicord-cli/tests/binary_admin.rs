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
    CONNECTION_MODE_WORKFLOW,
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
    assert!(text.contains("volicord agent connect"));
    assert!(text.contains("--connection-id ID"));
    assert!(text.contains("--mode read_only|workflow"));
    assert!(text.contains("volicord user judgment record --project-id ID"));
    assert!(text.contains("User Channel"));

    let unknown_user =
        run_without_home(["user", "not-a-real-command", "--project-id", "project_a"])?;
    assert_eq!(unknown_user.status.code(), Some(2));
    assert!(stderr(&unknown_user).contains("unknown user command: not-a-real-command"));
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
fn agent_connect_respects_explicit_read_only_and_writes_connection_config(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-read-only")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "connect",
            "--host",
            "codex",
            "--scope",
            "project",
            "--project-id",
            "project_read_only",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mode",
            "read_only",
            "--allow-repository-write",
            "--output",
            "json",
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
    assert_eq!(value["status"], "action_required");

    let record = agent_connection_record(runtime_home.path(), connection_id)?
        .expect("connection should be stored");
    assert_eq!(record.mode, CONNECTION_MODE_READ_ONLY);
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project_id, "project_read_only");

    let config = fs::read_to_string(repo_root.join(".codex").join("config.toml"))?;
    assert!(config.contains(&format!("args = [\"--connection\", \"{connection_id}\"]")));
    Ok(())
}

#[cfg(unix)]
#[test]
fn agent_connect_uses_explicit_workflow_mode() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-workflow")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "connect",
            "--host",
            "codex",
            "--scope",
            "project",
            "--project-id",
            "project_workflow",
            "--repo-root",
            path_text(&repo_root).as_str(),
            "--mode",
            "workflow",
            "--server-name",
            "volicord-workflow",
            "--allow-repository-write",
            "--output",
            "json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");

    assert_eq!(value["connection"]["mode"], CONNECTION_MODE_WORKFLOW);
    let record = agent_connection_record(runtime_home.path(), connection_id)?
        .expect("connection should be stored");
    assert_eq!(record.mode, CONNECTION_MODE_WORKFLOW);
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_project_enable_disable_and_uninstall_flow() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-lifecycle")?;
    let repo_a = runtime_home.create_product_repo("product-a")?;
    let repo_b = runtime_home.create_product_repo("product-b")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    let mcp = write_fake_mcp(&bin_dir)?;
    write_fake_codex(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let connect = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "connect",
            "--host",
            "codex",
            "--scope",
            "user",
            "--project-id",
            "project_a",
            "--repo-root",
            path_text(&repo_a).as_str(),
            "--output",
            "json",
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

    let add = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "add",
            "--connection-id",
            connection_id.as_str(),
            "--project-id",
            "project_b",
            "--repo-root",
            path_text(&repo_b).as_str(),
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&add);
    assert_eq!(
        list_connection_projects(runtime_home.path(), &connection_id)?.len(),
        2
    );

    let disable = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "disable",
            "--connection-id",
            connection_id.as_str(),
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&disable);
    assert_eq!(json_stdout(&disable)?["connection"]["enabled"], false);

    let enable = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "enable",
            "--connection-id",
            connection_id.as_str(),
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&enable);
    assert_eq!(json_stdout(&enable)?["connection"]["enabled"], true);

    let remove = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "project",
            "remove",
            "--connection-id",
            connection_id.as_str(),
            "--project-id",
            "project_b",
            "--output",
            "json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&remove);
    assert_eq!(
        list_connection_projects(runtime_home.path(), &connection_id)?.len(),
        1
    );

    let uninstall = run_with_home_env(
        runtime_home.path(),
        [
            "agent",
            "uninstall",
            "--connection-id",
            connection_id.as_str(),
            "--output",
            "json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&uninstall);
    assert!(agent_connection_record(runtime_home.path(), &connection_id)?.is_none());
    let config = fs::read_to_string(codex_home.join("config.toml"))?;
    assert!(!config.contains(&connection_id));
    Ok(())
}

#[test]
fn user_channel_records_pending_judgment_with_local_user_provenance() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-user-channel")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_user_channel", "{}")?;
    write_test_installation_profile(runtime_home.path())?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "project_user_channel".to_owned(),
            repo_root,
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

    let show = run_with_home_env(
        runtime_home.path(),
        [
            "user",
            "judgment",
            "show",
            "--project-id",
            "project_user_channel",
            "--judgment-id",
            judgment_id.as_str(),
        ],
        &[],
    )?;
    assert_success(&show);
    assert!(stdout(&show).contains("UserJudgment"));
    assert!(stdout(&show).contains("- accept: Accept focused choice"));

    let record = run_with_home_env(
        runtime_home.path(),
        [
            "user",
            "judgment",
            "record",
            "--project-id",
            "project_user_channel",
            "--judgment-id",
            judgment_id.as_str(),
            "--option-id",
            "accept",
            "--expected-state-version",
            "2",
            "--request-id",
            "req_cli_user_record",
            "--idempotency-key",
            "idem_cli_user_record",
        ],
        &[],
    )?;
    assert_success(&record);
    let text = stdout(&record);
    assert!(text.contains("resolved_by_actor_source: local_user"));
    assert!(text.contains("operation_category: user_only"));

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
    Ok(())
}

fn run_without_home<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin()).args(args).output()?)
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
         printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"volicord.intake\"},{\"name\":\"volicord.update_scope\"},{\"name\":\"volicord.status\"},{\"name\":\"volicord.prepare_write\"},{\"name\":\"volicord.stage_artifact\"},{\"name\":\"volicord.record_run\"},{\"name\":\"volicord.request_user_judgment\"},{\"name\":\"volicord.close_task\"},{\"name\":\"volicord.list_projects\"}]}}'\n\
         else\n\
         printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"volicord.status\"},{\"name\":\"volicord.close_task\"},{\"name\":\"volicord.list_projects\"}]}}'\n\
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
