#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use volicord_core::{CoreService, InvocationContext};
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    list_connection_projects, AgentConnectionRegistration, ConnectionProjectRegistration,
    CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_KIND_GENERIC,
    HOST_SCOPE_EXPORT, HOST_SCOPE_PROJECT, VERIFIED_STATUS_ACTION_REQUIRED,
    VERIFIED_STATUS_COMPLETE,
};
use volicord_store::guards::{
    insert_unrecorded_change, list_guard_installations, UnrecordedChangeInsert,
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

const SETUP_HELP_OPTIONS: &[&str] = &["--home", "--link-bin", "--mcp-command", "--json"];

#[test]
fn binary_help_uses_agent_connection_model() -> Result<(), Box<dyn Error>> {
    let help = run_without_home(["--help"])?;
    assert_success(&help);
    let text = stdout(&help);

    assert!(text.contains("volicord setup"));
    assert!(text.contains("volicord init --host"));
    assert!(text.contains("volicord doctor"));
    assert!(text.contains("volicord export mcp-config"));
    assert!(text.contains("volicord guard session-start"));
    assert!(text.contains("volicord connect [HOST]"));
    assert!(text.contains("volicord connections [--repo PATH]"));
    assert!(text.contains("volicord connection status [HOST]"));
    assert!(!text.contains("volicord agent connect"));
    assert!(text.contains("volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID"));
    assert!(text.contains("User Channel"));

    let setup_help = run_without_home(["setup", "--help"])?;
    assert_success(&setup_help);
    let setup_text = stdout(&setup_help);
    assert!(setup_text.contains("volicord setup"));
    assert!(setup_text.contains("--home PATH"));
    assert!(setup_text.contains("--link-bin PATH"));
    assert!(setup_text.contains("--mcp-command PATH"));
    assert!(setup_text.contains("--json"));

    let init_help = run_without_home(["init", "--help"])?;
    assert_success(&init_help);
    let init_text = stdout(&init_help);
    assert!(init_text.contains("volicord init --host codex|claude-code --repo PATH"));
    assert!(init_text.contains("--mode mcp-only|guarded|managed"));
    assert!(init_text.contains("--allow-degraded"));
    assert!(init_text.contains("--home PATH"));
    assert!(init_text.contains("--mcp-command PATH"));
    assert!(init_text.contains("--dry-run"));
    assert!(init_text.contains("--json"));

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

    let connection_help = run_without_home(["connection", "status", "--help"])?;
    assert_success(&connection_help);
    let connection_text = stdout(&connection_help);
    assert!(connection_text.contains("volicord connection status [HOST]"));

    let export_help = run_without_home(["export", "mcp-config", "--help"])?;
    assert_success(&export_help);
    let export_text = stdout(&export_help);
    assert!(export_text.contains("volicord export mcp-config"));
    assert!(export_text.contains("--output PATH"));
    Ok(())
}

#[test]
fn binary_help_options_match_supported_contracts() -> Result<(), Box<dyn Error>> {
    assert_help_options(
        ["--help"],
        &[
            "--version",
            "--home",
            "--link-bin",
            "--mcp-command",
            "--json",
            "--output",
            "--repo",
            "--shared",
            "--global",
            "--allow-degraded",
            "--read-only",
            "--dry-run",
            "--task",
            "--note",
            "--stdio",
            "--check",
            "--connection",
            "--project",
            "--transport",
            "--listen",
            "--token",
            "--generate-token",
            "--allow-origin",
            "--allow-nonlocal-listen",
            "--file",
            "--connection",
            "--session",
            "--guard-installation",
            "--host",
            "--guard-mode",
            "--mode",
            "--text",
        ],
    )?;
    assert_help_options(
        ["mcp", "--help"],
        &["--stdio", "--check", "--connection", "--project"],
    )?;
    assert_help_options(
        ["serve", "--help"],
        &[
            "--transport",
            "--listen",
            "--home",
            "--connection",
            "--project",
            "--token",
            "--generate-token",
            "--allow-origin",
            "--allow-nonlocal-listen",
        ],
    )?;
    assert_help_options(
        ["guard", "--help"],
        &[
            "--file",
            "--repo",
            "--connection",
            "--session",
            "--guard-installation",
            "--host",
            "--guard-mode",
            "--text",
        ],
    )?;
    assert_help_options(["setup", "--help"], SETUP_HELP_OPTIONS)?;
    assert_help_options(["doctor", "--help"], &["--json"])?;
    assert_help_options(
        ["connect", "--help"],
        &[
            "--repo",
            "--shared",
            "--global",
            "--read-only",
            "--dry-run",
            "--json",
        ],
    )?;
    assert_help_options(
        ["init", "--help"],
        &[
            "--host",
            "--repo",
            "--mode",
            "--allow-degraded",
            "--home",
            "--mcp-command",
            "--dry-run",
            "--json",
        ],
    )?;
    assert_help_options(["connections", "--help"], &["--repo", "--json"])?;
    assert_help_options(
        ["connection", "--help"],
        &["--repo", "--shared", "--global", "--dry-run", "--json"],
    )?;
    assert_help_options(
        ["connection", "status", "--help"],
        &["--repo", "--shared", "--global", "--json"],
    )?;
    assert_help_options(
        ["connection", "verify", "--help"],
        &["--repo", "--shared", "--global", "--json"],
    )?;
    assert_help_options(
        ["connection", "mode", "--help"],
        &["--repo", "--shared", "--global", "--json"],
    )?;
    assert_help_options(
        ["connection", "remove", "--help"],
        &["--repo", "--shared", "--global", "--dry-run", "--json"],
    )?;
    assert_help_options(
        ["export", "mcp-config", "--help"],
        &["--output", "--repo", "--read-only", "--json"],
    )?;
    assert_help_options(["project", "--help"], &["--repo", "--json"])?;
    assert_help_options(
        ["user", "--help"],
        &["--repo", "--task", "--note", "--json"],
    )?;
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
    assert_eq!(setup_json["status"], "action_required");
    assert_eq!(
        setup_json["status_meaning"],
        "installation profile setup needs a named user action"
    );
    assert_eq!(setup_json["setup_report"]["status"], "action_required");
    assert_eq!(
        setup_json["setup_report"]["installation_profile"]["status"],
        "complete"
    );
    assert_eq!(
        setup_json["installation_profile"]["volicord_mcp_command"],
        path_text(&mcp)
    );
    assert_eq!(
        setup_json["installation_profile"]["default_connection_mode"],
        "workflow"
    );
    assert!(setup_json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| {
            check["id"] == "volicord_mcp_command"
                && check["status"] == "passed"
                && check["details"]["path"] == path_text(&mcp)
                && check["details"]["source"] == "explicit"
        }));
    assert!(setup_json["actions_required"]
        .as_array()
        .expect("actions_required should be an array")
        .iter()
        .any(|action| action["id"] == "make_volicord_command_available"));
    assert_eq!(
        setup_json["primary_next_action"]["id"],
        "make_volicord_command_available"
    );
    assert_eq!(
        setup_json["states"]["command_availability"],
        "action_required"
    );

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let doctor_json = json_stdout(&doctor)?;
    assert_eq!(doctor_json["status"], "complete");
    assert_eq!(
        doctor_json["status_meaning"],
        "installation profile is usable; warnings name recommended follow-up actions"
    );
    assert_eq!(
        doctor_json["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .len(),
        0
    );
    assert!(
        doctor_json["warning_count"]
            .as_u64()
            .expect("warning_count should be numeric")
            >= 1
    );
    assert!(doctor_json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "installation_profile" && check["status"] == "passed"));
    let mcp_availability = doctor_json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "volicord_mcp_command_availability")
        .expect("doctor should report MCP launch command availability");
    assert_eq!(mcp_availability["status"], "warning");
    assert_eq!(
        mcp_availability["details"]["profile_command"],
        path_text(&mcp)
    );
    assert_eq!(mcp_availability["details"]["path_matches_profile"], false);
    assert_eq!(
        mcp_availability["details"]["agent_host_restart_or_reload_may_be_needed"],
        true
    );
    assert!(doctor_json["actions_recommended"]
        .as_array()
        .expect("actions_recommended should be an array")
        .iter()
        .any(|action| action["id"] == "make_profile_commands_available"));
    assert_eq!(
        doctor_json["primary_next_action"]["id"],
        "make_profile_commands_available"
    );
    assert_eq!(
        doctor_json["primary_next_action"]["requirement"],
        "recommended"
    );
    assert_eq!(doctor_json["states"]["host_reload_required"], true);

    let doctor_text = run_with_home_env(runtime_home.path(), ["doctor"], &[])?;
    assert_success(&doctor_text);
    let text = stdout(&doctor_text);
    assert!(text.contains("Volicord doctor complete"));
    assert!(text.contains(
        "status_meaning: installation profile is usable; warnings name recommended follow-up actions"
    ));
    assert!(text.contains("command_state: action_recommended"));
    assert!(text.contains("host_reload_required: yes"));
    assert!(text.contains("next_action: recommended:"));
    assert!(text.contains("restart or reload existing agent hosts"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn setup_plain_non_tty_reports_actions_without_prompting_or_shell_edits(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-non-tty")?;
    let bin_dir = runtime_home.path().join("bin");
    let path_dir = runtime_home.path().join("path-bin");
    let home = runtime_home.path().join("home");
    fs::create_dir_all(&path_dir)?;
    fs::create_dir_all(&home)?;
    let mcp = write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        ["setup", "--mcp-command", path_text(&mcp).as_str()],
        &[
            ("PATH", path_env(&[path_dir.as_path()])),
            ("HOME", path_text(&home)),
            ("SHELL", "/bin/zsh".to_owned()),
        ],
    )?;

    assert_success(&output);
    let text = stdout(&output);
    assert!(text.contains("Volicord setup action_required"));
    assert!(text.contains("status_meaning: installation profile setup needs a named user action"));
    assert!(text.contains("command_state: action_required"));
    assert!(text.contains("next_action:"));
    assert!(text.contains("optional_action_count:"));
    assert!(!text.contains("Choices:"));
    assert!(!text.contains("Choice ["));
    assert!(!text.contains("Managed block to write"));
    assert!(!home.join(".zshrc").exists());
    assert!(!home.join(".local/bin/volicord").exists());
    assert!(!home.join(".local/bin/volicord-mcp").exists());
    Ok(())
}

#[test]
fn doctor_without_setup_reports_action_required() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-doctor-missing")?;
    assert_eq!(fs::read_dir(runtime_home.path())?.count(), 0);

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let value = json_stdout(&doctor)?;
    assert_eq!(value["status"], "action_required");
    assert_eq!(
        value["status_meaning"],
        "local init or profile repair is required before Volicord workflows are usable"
    );
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "run_init"));
    assert_eq!(value["primary_next_action"]["id"], "run_init");
    assert_eq!(value["primary_next_action"]["requirement"], "required");
    assert_eq!(value["states"]["prompt_capture_status"], "not_checked");
    let doctor_text = run_with_home_env(runtime_home.path(), ["doctor"], &[])?;
    assert_success(&doctor_text);
    let text = stdout(&doctor_text);
    assert!(text.contains("runtime_home_state: ready"));
    assert!(text.contains("installation_profile_state: missing_or_invalid"));
    assert!(text.contains("mcp_config_state: unknown"));
    assert!(text.contains("prompt_capture_state: not_checked"));
    assert!(text.contains("next_action: Run volicord init --host <host> --repo <path>"));
    assert_eq!(fs::read_dir(runtime_home.path())?.count(), 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_guarded_without_degraded_opt_in_generates_hooks() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-guarded-hooks")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["host"], "codex");
    assert_eq!(value["degraded"]["allowed"], false);
    assert_eq!(
        value["degraded"]["missing_required_hooks"],
        serde_json::json!([])
    );
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["required_guard_phases"], "configured");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let hooks = fs::read_to_string(repo_root.join(".codex/hooks.json"))?;
    assert!(hooks.contains("volicord guard session-start"));
    assert!(hooks.contains("volicord guard pre-tool"));
    assert!(hooks.contains("volicord guard post-tool"));
    assert!(hooks.contains("volicord guard prompt-capture"));
    assert!(hooks.contains("volicord guard stop"));
    assert!(hooks.contains(&format!("--connection {connection_id}")));
    assert!(hooks.contains("--guard-installation"));
    assert!(hooks.contains("--host codex"));
    assert!(repo_root.join(".codex/rules/volicord.rules").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_claude_code_guarded_without_degraded_opt_in_generates_hooks() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-init-claude-guarded-hooks")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["host"], "claude-code");
    assert_eq!(value["degraded"]["allowed"], false);
    assert_eq!(
        value["degraded"]["missing_required_hooks"],
        serde_json::json!([])
    );
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert!(repo_root.join(".mcp.json").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    let settings = fs::read_to_string(repo_root.join(".claude/settings.json"))?;
    assert!(settings.contains("volicord guard session-start"));
    assert!(settings.contains("volicord guard pre-tool"));
    assert!(settings.contains("volicord guard post-tool"));
    assert!(settings.contains("volicord guard prompt-capture"));
    assert!(settings.contains("volicord guard stop"));
    assert!(settings.contains("--host claude-code"));
    assert!(settings.contains("Edit|Write|MultiEdit"));
    assert!(repo_root.join(".claude/rules/volicord.md").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_managed_unsupported_fails_without_guarded_artifacts() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-managed-unsupported")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--mode",
            "managed",
            "--allow-degraded",
            "--json",
        ],
        &[],
    )?;

    assert!(!output.status.success());
    let value = json_stdout(&output)?;
    assert_eq!(value["status"], "failed");
    assert_eq!(value["error_code"], "MANAGED_MODE_UNSUPPORTED");
    assert_eq!(value["mode"], "managed");
    assert_eq!(value["managed_mode"]["supported"], false);
    assert_eq!(
        value["managed_mode"]["allow_degraded_effect"],
        "not_applied"
    );
    assert_eq!(value["primary_next_action"]["id"], "choose_supported_mode");
    assert!(!repo_root.join(".codex/hooks.json").exists());
    assert!(!repo_root.join(".volicord/policy.json").exists());
    assert!(!repo_root.join("AGENTS.md").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_dry_run_does_not_write_runtime_or_repo_files() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-dry-run")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--dry-run",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["action"], "init");
    assert_eq!(value["status"], "dry_run");
    assert_eq!(value["host"], "codex");
    assert_eq!(value["mode"], "guarded");
    assert_eq!(value["degraded"]["allowed"], false);
    assert_eq!(
        value["degraded"]["missing_required_hooks"],
        serde_json::json!([])
    );
    assert_eq!(value["profile"]["status"], "planned");
    assert_eq!(value["mcp"]["command"], "volicord");
    assert_eq!(value["mcp"]["args"][0], "mcp");
    assert_eq!(value["mcp"]["args"][1], "--stdio");
    assert_eq!(value["generated_files"][0]["kind"], "agents_managed_block");
    assert_eq!(value["generated_files"][0]["status"], "planned_create");
    assert_eq!(value["generated_files"][1]["kind"], "volicord_policy");
    assert_eq!(value["generated_files"][1]["status"], "planned_create");
    assert!(value["generated_files"]
        .as_array()
        .expect("generated files should be an array")
        .iter()
        .any(|file| file["kind"] == "host_hook_config"));
    assert!(value["generated_files"]
        .as_array()
        .expect("generated files should be an array")
        .iter()
        .any(|file| file["kind"] == "host_rule_instruction"));
    assert!(!runtime_home.registry_db_path().exists());
    assert!(!repo_root.join(".codex/config.toml").exists());
    assert!(!repo_root.join(".codex/hooks.json").exists());
    assert!(!repo_root.join(".codex/rules/volicord.rules").exists());
    assert!(!repo_root.join("AGENTS.md").exists());
    assert!(!repo_root.join(".volicord/policy.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_guarded_rejects_unmanaged_hook_config() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-codex-hook-conflict")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let hooks_path = repo_root.join(".codex/hooks.json");
    fs::create_dir_all(hooks_path.parent().expect("hook path should have parent"))?;
    fs::write(
        &hooks_path,
        r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo user"}]}]}}"#,
    )?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_eq!(output.status.code(), Some(1));
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("host_hook_config already exists with unmanaged content"));
    assert!(diagnostic.contains(&path_text(&hooks_path)));
    assert!(!runtime_home.registry_db_path().exists());
    assert!(!repo_root.join(".codex/config.toml").exists());
    assert!(!repo_root.join("AGENTS.md").exists());
    assert!(!repo_root.join(".volicord/policy.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_guarded_writes_policy_mcp_and_guard_status_idempotently() -> Result<(), Box<dyn Error>>
{
    const START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE v1 -->";
    const END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE v1 -->";

    let runtime_home = TempRuntimeHome::new("cli-bin-init-codex")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    fs::write(
        repo_root.join("AGENTS.md"),
        format!("Existing top\n{START_MARKER}\nold managed text\n{END_MARKER}\nExisting bottom\n"),
    )?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["action"], "init");
    assert_eq!(value["host"], "codex");
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["mode"], "guarded");
    assert_eq!(value["guard_mode"], "guarded");
    assert_eq!(value["states"]["runtime_home"], "ready");
    assert_eq!(value["states"]["project_registration"], "registered");
    assert_eq!(value["states"]["mcp_config"], "match");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["guard_degraded_allowed"], false);
    assert_eq!(value["degraded"]["allowed"], false);
    assert_eq!(value["states"]["agents_managed_block"], "updated");
    assert_eq!(value["states"]["volicord_policy_file"], "created");
    assert_eq!(value["states"]["rule_instruction_config"], "created");
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["required_guard_phases"], "configured");
    assert_eq!(value["states"]["guard_observed"], false);
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["states"]["host_reload_required"], true);
    assert_eq!(value["primary_next_action"]["id"], "reload_required");
    assert_eq!(value["profile"]["status"], "created");
    assert_eq!(value["connection"]["host_kind"], "codex");
    assert_eq!(value["connection"]["connection_intent"], "shared");
    assert_eq!(value["connection"]["host_scope"], "project");
    assert_eq!(value["connection"]["mode"], CONNECTION_MODE_WORKFLOW);
    assert_eq!(value["mcp"]["command"], "volicord");
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present")
        .to_owned();
    assert_eq!(
        value["mcp"]["args"],
        serde_json::json!(["mcp", "--stdio", "--connection", connection_id])
    );
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "reload_required"));

    let text_output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&text_output);
    let init_text = stdout(&text_output);
    assert!(init_text.contains("Volicord init action_required"));
    assert!(init_text.contains("connection_state: action_required"));
    assert!(init_text.contains("mcp_config_state: match"));
    assert!(init_text.contains("guard_installation_state: configured"));
    assert!(init_text.contains("guard_degraded_allowed: no"));
    assert!(init_text.contains("agents_block_state: unchanged"));
    assert!(init_text.contains("volicord_policy_file_state: unchanged"));
    assert!(init_text.contains("rule_instruction_config_state: unchanged"));
    assert!(init_text.contains("hook_config_state: unchanged"));
    assert!(init_text.contains("required_guard_phases_state: configured"));
    assert!(init_text.contains("guard_observed: no"));
    assert!(init_text.contains("prompt_capture_state: configured"));
    assert!(init_text.contains("host_reload_required: yes"));
    assert!(init_text.contains("next_action: Restart or reload codex"));

    let record = agent_connection_record(runtime_home.path(), &connection_id)?
        .expect("connection should be stored");
    assert_eq!(record.mode, CONNECTION_MODE_WORKFLOW);
    assert_eq!(record.host_kind, "codex");
    let projects = list_connection_projects(runtime_home.path(), &connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project.repo_root, repo_root);

    let status_without_intent = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[],
    )?;
    assert_success(&status_without_intent);
    let status_without_intent_json = json_stdout(&status_without_intent)?;
    assert_eq!(
        status_without_intent_json["connection"]["connection_id"],
        connection_id
    );
    assert_eq!(
        status_without_intent_json["connection"]["connection_intent"],
        "shared"
    );
    assert_eq!(
        status_without_intent_json["primary_next_action"]["id"],
        "reload_required"
    );
    assert_eq!(
        status_without_intent_json["states"]["hook_config"],
        "installed"
    );
    assert_eq!(
        status_without_intent_json["states"]["guard_observed"],
        false
    );

    let config = fs::read_to_string(repo_root.join(".codex/config.toml"))?;
    assert!(config.contains(&format!(
        "args = [\"mcp\", \"--stdio\", \"--connection\", \"{connection_id}\"]"
    )));
    let hooks = fs::read_to_string(repo_root.join(".codex/hooks.json"))?;
    assert!(hooks.contains("SessionStart"));
    assert!(hooks.contains("PreToolUse"));
    assert!(hooks.contains("PostToolUse"));
    assert!(hooks.contains("UserPromptSubmit"));
    assert!(hooks.contains("Stop"));
    assert!(hooks.contains(&format!("--connection {connection_id}")));
    assert!(hooks.contains("--guard-installation"));
    assert!(hooks.contains("--host codex"));
    assert!(hooks.contains("--guard-mode guarded"));
    assert!(hooks.contains("volicord guard prompt-capture"));
    let rules = fs::read_to_string(repo_root.join(".codex/rules/volicord.rules"))?;
    assert!(rules.contains("# BEGIN VOLICORD MANAGED CODEX RULES v1"));
    assert!(rules.contains("prefix_rule("));
    assert!(rules.contains("volicord guard session-start"));
    assert!(rules.contains("volicord guard stop"));

    let agents = fs::read_to_string(repo_root.join("AGENTS.md"))?;
    assert_eq!(count_occurrences(&agents, START_MARKER), 1);
    assert!(agents.contains("Existing top"));
    assert!(agents.contains("Existing bottom"));
    assert!(agents.contains("Check Volicord status before planning"));
    assert!(agents.contains("Start a task before planning implementation"));
    assert!(agents.contains("Prepare write before product-file changes"));
    assert!(agents.contains("Request user judgment through Volicord"));
    assert!(agents.contains("Check close before claiming completion"));
    assert!(agents.contains("If Volicord tools are unavailable"));
    assert!(!agents.contains("old managed text"));

    let policy_path = repo_root.join(".volicord/policy.json");
    let policy: Value = serde_json::from_str(&fs::read_to_string(&policy_path)?)?;
    assert_eq!(policy["schema"], "volicord-policy-v1");
    assert_eq!(policy["managed_by"], "volicord");
    assert_eq!(policy["host"], "codex");
    assert_eq!(policy["mode"], "guarded");
    assert_eq!(policy["guard_mode"], "guarded");
    assert_eq!(policy["mcp"]["command"], "volicord");
    assert_eq!(
        policy["mcp"]["args"],
        serde_json::json!(["mcp", "--stdio", "--connection", connection_id])
    );
    assert_eq!(policy["guard"]["enabled"], true);
    assert_guard_policy_invokes_required_phases(&policy, &connection_id);
    assert_eq!(
        policy["guard"]["commands"]["pre_tool"]["command"],
        "volicord"
    );
    assert_eq!(policy["guard"]["commands"]["pre_tool"]["args"][0], "guard");
    assert_eq!(
        policy["guard"]["commands"]["pre_tool"]["args"][1],
        "pre-tool"
    );
    assert!(policy["guard"]["commands"]["pre_tool"]["args"]
        .as_array()
        .expect("guard args should be an array")
        .windows(2)
        .any(|pair| pair[0] == "--connection" && pair[1] == connection_id));

    let guard_installations = list_guard_installations(
        runtime_home.path(),
        &connection_id,
        Some(&projects[0].project_id),
    )?;
    assert_eq!(guard_installations.len(), 1);
    assert_eq!(guard_installations[0].host_kind, "codex");
    assert_eq!(guard_installations[0].guard_mode, "guarded");
    assert_eq!(guard_installations[0].installation_status, "configured");
    let capability: Value = serde_json::from_str(&guard_installations[0].host_capability_json)?;
    assert_eq!(capability["schema"], "volicord-guard-capability-v1");
    assert_eq!(
        capability["policy_hash"],
        value["guard_installation"]["policy_hash"]
    );
    assert_eq!(capability["allow_degraded"], false);
    assert_eq!(capability["prompt_capture"], true);
    assert_eq!(capability["guard_profile"], "host_hook_guarded");
    assert_eq!(capability["managed_source"], "project_local_host_hooks");
    assert_eq!(capability["managed_bundle_hash"], Value::Null);
    assert_eq!(capability["managed_verification_status"], "not_applicable");
    assert_eq!(capability["missing_required_hooks"], serde_json::json!([]));
    assert_eq!(capability["host_capabilities"]["pre_tool_hook"], true);
    assert_eq!(
        capability["host_capabilities"]["user_prompt_submit_hook"],
        true
    );
    assert!(capability["commands"]["pre_tool"]["args"]
        .as_array()
        .expect("capability guard args should be an array")
        .iter()
        .any(|arg| arg == "--json"));

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let doctor_json = json_stdout(&doctor)?;
    let registry_counts = doctor_json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "registry_counts")
        .expect("doctor should report registry counts");
    assert_eq!(registry_counts["details"]["guard_installations"], 1);
    assert_eq!(doctor_json["states"]["guard_profile"], "host_hook_guarded");
    assert_eq!(
        doctor_json["states"]["managed_source"],
        "project_local_host_hooks"
    );
    assert_eq!(doctor_json["states"]["managed_bundle_hash"], Value::Null);
    assert_eq!(
        doctor_json["states"]["managed_verification_status"],
        "not_applicable"
    );
    assert_eq!(doctor_json["states"]["agents_managed_block"], "installed");
    assert_eq!(doctor_json["states"]["volicord_policy_file"], "installed");
    assert_eq!(
        doctor_json["states"]["rule_instruction_config"],
        "installed"
    );
    assert_eq!(doctor_json["states"]["hook_config"], "installed");
    assert_eq!(doctor_json["states"]["required_guard_phases"], "configured");
    assert_eq!(
        doctor_json["states"]["prompt_capture"],
        "action_recommended"
    );
    assert_eq!(
        doctor_json["states"]["prompt_capture_status"],
        "configured_unobserved"
    );

    let second = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&second);
    let second_json = json_stdout(&second)?;
    assert_eq!(second_json["connection"]["connection_id"], connection_id);
    assert_eq!(second_json["profile"]["status"], "reused");
    assert_eq!(second_json["states"]["guard_installation"], "configured");
    assert_eq!(second_json["states"]["guard_degraded_allowed"], false);
    assert_eq!(second_json["states"]["hook_config"], "unchanged");
    assert_eq!(second_json["states"]["prompt_capture"], "configured");
    assert_eq!(second_json["degraded"]["allowed"], false);
    assert_eq!(
        count_occurrences(
            &fs::read_to_string(repo_root.join("AGENTS.md"))?,
            START_MARKER
        ),
        1
    );
    assert_eq!(
        list_guard_installations(
            runtime_home.path(),
            &connection_id,
            Some(&projects[0].project_id)
        )?
        .len(),
        1
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_claude_code_guarded_writes_project_mcp_policy_and_rule() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-claude")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["action"], "init");
    assert_eq!(value["host"], "claude-code");
    assert_eq!(value["mode"], "guarded");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["guard_degraded_allowed"], false);
    assert_eq!(value["degraded"]["allowed"], false);
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["mcp"]["command"], "volicord");
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");

    let mcp_config: Value =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".mcp.json"))?)?;
    let server = &mcp_config["mcpServers"]["volicord"];
    assert_eq!(server["command"], "volicord");
    assert_eq!(
        server["args"],
        serde_json::json!(["mcp", "--stdio", "--connection", connection_id])
    );

    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["host"], "claude-code");
    assert_eq!(policy["guard"]["enabled"], true);
    assert_guard_policy_invokes_required_phases(&policy, connection_id);
    assert_eq!(
        policy["guard"]["commands"]["session_start"]["command"],
        "volicord"
    );
    let settings = fs::read_to_string(repo_root.join(".claude/settings.json"))?;
    assert!(settings.contains("volicord guard session-start"));
    assert!(settings.contains("volicord guard pre-tool"));
    assert!(settings.contains("volicord guard post-tool"));
    assert!(settings.contains("volicord guard prompt-capture"));
    assert!(settings.contains("volicord guard stop"));
    assert!(settings.contains(&format!("--connection {connection_id}")));
    assert!(settings.contains("--guard-installation"));
    assert!(settings.contains("--host claude-code"));
    assert!(settings.contains("\"matcher\": \"Edit|Write|MultiEdit\""));
    assert!(repo_root.join(".claude/rules/volicord.md").exists());
    let rule = fs::read_to_string(repo_root.join(".claude/rules/volicord.md"))?;
    assert!(rule.contains(".volicord/policy.json"));
    assert!(rule.contains("Configured local guard commands"));
    assert!(rule.contains("volicord guard session-start"));
    assert!(rule.contains("volicord guard pre-tool"));
    assert!(rule.contains("volicord guard prompt-capture"));

    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    let guard_installations = list_guard_installations(
        runtime_home.path(),
        connection_id,
        Some(&projects[0].project_id),
    )?;
    assert_eq!(guard_installations.len(), 1);
    assert_eq!(guard_installations[0].host_kind, "claude_code");
    assert_eq!(guard_installations[0].guard_mode, "guarded");
    assert_eq!(
        guard_installations[0].installation_status,
        "reload_required"
    );
    let capability: Value = serde_json::from_str(&guard_installations[0].host_capability_json)?;
    assert_eq!(capability["host_capabilities"]["rule_file_support"], true);
    assert_eq!(
        capability["host_capabilities"]["user_prompt_submit_hook"],
        true
    );
    assert_eq!(capability["allow_degraded"], false);
    assert!(capability["missing_required_hooks"]
        .as_array()
        .expect("missing hooks should be an array")
        .is_empty());
    assert!(capability["files"]
        .as_array()
        .expect("files should be an array")
        .iter()
        .any(|file| file["kind"] == "host_hook_config"
            && file["managed_projection"] == "claude_code_settings_hooks"));
    Ok(())
}

#[test]
fn ordinary_command_before_setup_instructs_setup() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-setup-required")?;

    let output = run_with_home_env(runtime_home.path(), ["project", "list"], &[])?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("volicord init --host <host> --repo <path>"));
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
    assert!(config.contains(&format!(
        "args = [\"mcp\", \"--stdio\", \"--connection\", \"{connection_id}\"]"
    )));
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

    let status_text = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&projects[0].project.repo_root).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status_text);
    let status_text = stdout(&status_text);
    assert!(status_text.contains("connection_state: complete"));
    assert!(status_text.contains("runtime_home_state: ready"));
    assert!(status_text.contains("project_registration_state: registered"));
    assert!(status_text.contains("mcp_config_state: match"));
    assert!(status_text.contains("host_reload_required: no"));
    assert!(status_text.contains("next_action: none"));
    Ok(())
}

#[test]
fn connect_codex_global_reports_supported_intents() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-codex-global")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_codex_global", "{}")?;
    write_test_installation_profile(runtime_home.path())?;

    let output = run_with_home_env(runtime_home.path(), ["connect", "codex", "--global"], &[])?;

    assert_eq!(output.status.code(), Some(2));
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("codex does not support --global"));
    assert!(diagnostic.contains("supported connection intents: personal, shared"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_output_prioritizes_missing_host_binary_action() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-missing-host-binary")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connect",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["primary_next_action"]["id"], "path_binary_not_found");
    assert_eq!(value["states"]["mcp_config"], "match");
    assert_eq!(value["states"]["host_reload_required"], false);
    assert!(value["primary_next_action"]["instruction"]
        .as_str()
        .expect("instruction should be text")
        .contains("Codex executable `codex` was not found on PATH"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_verify_reports_missing_mcp_config_as_primary_action() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-missing-mcp")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--allow-degraded",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&init);
    fs::remove_file(repo_root.join(".codex/config.toml"))?;

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;

    assert_success(&verify);
    let value = json_stdout(&verify)?;
    assert_eq!(value["states"]["mcp_config"], "missing");
    assert_eq!(value["primary_next_action"]["id"], "mcp_config_missing");
    assert_eq!(
        value["primary_next_action"]["command"],
        format!(
            "volicord init --host codex --repo {} --allow-degraded",
            path_text(&repo_root)
        )
    );
    assert!(value["primary_next_action"]["instruction"]
        .as_str()
        .expect("instruction should be text")
        .contains("volicord init --host codex --repo"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_reports_missing_guard_files_as_primary_action() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-missing-guard")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--allow-degraded",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&init);
    fs::remove_file(repo_root.join(".volicord/policy.json"))?;

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[],
    )?;

    assert_success(&status);
    let value = json_stdout(&status)?;
    assert_eq!(value["states"]["guard_installation"], "files_missing");
    assert_eq!(value["states"]["prompt_capture"], "not_configured");
    assert_eq!(value["primary_next_action"]["id"], "guard_files_missing");
    assert!(value["guard"]["missing_files"]
        .as_array()
        .expect("missing_files should be an array")
        .iter()
        .any(|path| path == &path_text(&repo_root.join(".volicord/policy.json"))));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_reports_stale_guard_files_as_primary_action() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-stale-guard")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--allow-degraded",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&init);
    let init_json = json_stdout(&init)?;
    let connection_id = init_json["connection"]["connection_id"]
        .as_str()
        .expect("connection id should be present");
    let policy_path = repo_root.join(".volicord/policy.json");
    fs::write(
        &policy_path,
        fs::read_to_string(&policy_path)?.replace(connection_id, "conn_changed"),
    )?;

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[],
    )?;

    assert_success(&status);
    let value = json_stdout(&status)?;
    assert_eq!(value["states"]["guard_installation"], "stale");
    assert_eq!(value["primary_next_action"]["id"], "guard_files_stale");
    assert!(value["guard"]["stale_files"]
        .as_array()
        .expect("stale_files should be an array")
        .iter()
        .any(|path| path == &path_text(&policy_path)));

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let doctor_json = json_stdout(&doctor)?;
    assert_eq!(doctor_json["states"]["guard_files"], "action_recommended");
    assert_eq!(doctor_json["states"]["volicord_policy_file"], "stale");
    assert_eq!(
        doctor_json["primary_next_action"]["id"],
        "repair_guard_files"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_selector_distinguishes_project_registration_and_allowlist(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-selector-actions")?;
    let repo_a = create_git_repo(&runtime_home, "product-a")?;
    let repo_b = create_git_repo(&runtime_home, "product-b")?;
    let repo_c = create_git_repo(&runtime_home, "product-c")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let unregistered = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_c).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_eq!(unregistered.status.code(), Some(1));
    assert!(stderr(&unregistered).contains("PROJECT_NOT_REGISTERED"));

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
    assert_success(&run_with_home_env(
        runtime_home.path(),
        ["project", "use", path_text(&repo_b).as_str(), "--json"],
        &[],
    )?);

    let mismatch = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_eq!(mismatch.status.code(), Some(1));
    assert!(stderr(&mismatch).contains("CONNECTION_ALLOWLIST_MISMATCH"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connect_claude_code_global_is_accepted() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-claude-global")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connect",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--global",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["action"], "connected");
    assert_eq!(value["connection"]["host_kind"], "claude_code");
    assert_eq!(value["connection"]["connection_intent"], "global");
    assert_eq!(value["connection"]["host_scope"], "user");
    Ok(())
}

#[cfg(unix)]
#[test]
fn export_mcp_config_uses_default_file_when_output_is_omitted() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-export-default")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let invocation_dir = runtime_home.path().join("invocation-dir");
    fs::create_dir_all(&invocation_dir)?;
    let bin_dir = runtime_home.path().join("bin");
    let mcp = write_fake_mcp(&bin_dir)?;
    assert_success(&run_setup(runtime_home.path(), &mcp)?);

    let output = run_with_home_env_in_dir(
        runtime_home.path(),
        [
            "export",
            "mcp-config",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &[],
        &invocation_dir,
    )?;
    assert_success(&output);
    let text = stdout(&output);
    let output_path = repo_root.join("volicord.mcp.json");
    assert!(text.contains("MCP configuration exported"));
    assert!(text.contains(&format!("output: {}", path_text(&output_path))));
    assert!(!text.contains("mcpServers"));
    assert!(!text.contains("--connection"));
    assert!(!invocation_dir.join("volicord.mcp.json").exists());

    let connection_id = assert_exported_mcp_config(&output_path, &mcp, runtime_home.path())?;

    let record = agent_connection_record(runtime_home.path(), &connection_id)?
        .expect("generic export connection should be stored");
    assert_eq!(record.host_kind, HOST_KIND_GENERIC);
    assert_eq!(record.host_scope, HOST_SCOPE_EXPORT);
    assert_eq!(record.mode, CONNECTION_MODE_WORKFLOW);
    assert_eq!(record.config_target, path_text(&output_path));
    assert_eq!(
        record.last_verification_status,
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    let projects = list_connection_projects(runtime_home.path(), &connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project.repo_root, repo_root);
    Ok(())
}

#[cfg(unix)]
#[test]
fn export_mcp_config_writes_explicit_output_path() -> Result<(), Box<dyn Error>> {
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
    let default_output_path = repo_root.join("volicord.mcp.json");
    assert_eq!(first_json["output_path"], path_text(&output_path));
    assert_eq!(first_json["mode"], CONNECTION_MODE_READ_ONLY);
    assert_eq!(first_json["connection"]["status"], "created");
    assert_eq!(
        assert_exported_mcp_config(&output_path, &mcp, runtime_home.path())?,
        connection_id
    );
    assert!(!default_output_path.exists());

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
    let status_text = stdout(&status);
    assert!(status_text.contains("User Channel status"));
    assert!(status_text.contains("close_readiness: blocked"));
    assert!(status_text.contains("close_blockers:"));
    assert!(status_text.contains("next_action:"));
    assert!(status_text.contains("pending judgments: 1"));
    assert!(status_text.contains("judgment_path:"));

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

#[test]
fn changes_reconcile_runs_as_local_recovery() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-changes-reconcile")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_changes_reconcile", "{}")?;
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
    ensure_agent_connection(
        runtime_home.path(),
        AgentConnectionRegistration {
            connection_internal_id: "connection_cli_user_channel".to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            intent: volicord_store::agent_connections::CONNECTION_INTENT_SHARED.to_owned(),
            host_scope: HOST_SCOPE_PROJECT.to_owned(),
            server_name: "volicord-cli-changes-test".to_owned(),
            config_target: runtime_home
                .path()
                .join("agent-connections")
                .join("connection_cli_user_channel")
                .to_string_lossy()
                .into_owned(),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: true,
            managed_fingerprint: "fixture:cli-changes".to_owned(),
            last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
            last_verification_report_json: "{}".to_owned(),
            last_user_actions_json: "[]".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        runtime_home.path(),
        ConnectionProjectRegistration {
            connection_internal_id: "connection_cli_user_channel".to_owned(),
            project_id: "project_user_channel".to_owned(),
        },
    )?;
    let service = CoreService::new(runtime_home.path());
    let intake = service.intake(
        intake_request(
            "req_cli_changes_reconcile_intake",
            "idem_cli_changes_reconcile_intake",
            Some(0),
        ),
        core_invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = record_id(&intake.response_value["task_ref"])?;
    insert_unrecorded_change(
        runtime_home.path(),
        "project_user_channel",
        UnrecordedChangeInsert {
            unrecorded_change_id: "unrecorded_cli_changes_reconcile".to_owned(),
            session_id: None,
            connection_internal_id: "connection_cli_user_channel".to_owned(),
            task_id: Some(task_id.clone()),
            summary: "Product Repository change observed outside a recorded run.".to_owned(),
            observed_paths_json: "[]".to_owned(),
            detection_json: "{}".to_owned(),
            detected_at: "2026-06-30T00:05:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;

    let output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["changes", "reconcile", "--json"],
        &[],
        &repo_root,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["base"]["response_kind"], "result");
    assert_eq!(
        value["resolved_changes"][0]["resolution_basis"],
        "not_product_change"
    );
    assert_eq!(
        value["resolved_changes"][0]["resolved_by_actor_source"],
        "system"
    );

    insert_unrecorded_change(
        runtime_home.path(),
        "project_user_channel",
        UnrecordedChangeInsert {
            unrecorded_change_id: "unrecorded_cli_changes_reconcile_text".to_owned(),
            session_id: None,
            connection_internal_id: "connection_cli_user_channel".to_owned(),
            task_id: Some(task_id.clone()),
            summary: "Second Product Repository change observed outside a recorded run.".to_owned(),
            observed_paths_json: "[]".to_owned(),
            detection_json: "{}".to_owned(),
            detected_at: "2026-06-30T00:06:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let text_output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["changes", "reconcile"],
        &[],
        &repo_root,
    )?;
    assert_success(&text_output);
    let text = stdout(&text_output);
    assert!(text.contains("changes recovery:"));
    assert!(!text.contains("reconciled changes:"));

    let conn =
        rusqlite::Connection::open(runtime_home.project_state_db_path("project_user_channel"))?;
    let (actor_source, operation_category): (String, String) = conn.query_row(
        "SELECT actor_source, operation_category
           FROM tool_invocations
          WHERE tool_name = 'volicord.reconcile_changes'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(actor_source, "local_user");
    assert_eq!(operation_category, "local_recovery");
    Ok(())
}

fn run_without_home<const N: usize>(args: [&str; N]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin()).args(args).output()?)
}

fn assert_help_options<const N: usize>(
    args: [&str; N],
    expected: &[&str],
) -> Result<(), Box<dyn Error>> {
    let command = format!("volicord {}", args.join(" "));
    let output = run_without_home(args)?;
    assert_success(&output);
    let text = stdout(&output);
    let actual = help_option_tokens(&text);
    let expected = expected_options(expected);
    assert_eq!(
        actual, expected,
        "help options for `{command}` should match the supported option allowlist:\n{text}"
    );
    Ok(())
}

fn help_option_tokens(text: &str) -> BTreeSet<String> {
    text.split_whitespace()
        .flat_map(|token| token.split('|'))
        .filter_map(normalize_help_option_token)
        .filter(|token| token != "-h" && token != "--help")
        .collect()
}

fn normalize_help_option_token(token: &str) -> Option<String> {
    let token = token.trim_matches(|character: char| {
        matches!(
            character,
            '[' | ']' | '(' | ')' | '{' | '}' | ',' | ':' | ';' | '.'
        )
    });
    if !token.starts_with('-') {
        return None;
    }

    let option_len = token
        .char_indices()
        .find_map(|(index, character)| {
            if character == '-' || character.is_ascii_alphanumeric() {
                None
            } else {
                Some(index)
            }
        })
        .unwrap_or(token.len());
    let option = &token[..option_len];
    if option == "-" || option == "--" {
        None
    } else {
        Some(option.to_owned())
    }
}

fn expected_options(options: &[&str]) -> BTreeSet<String> {
    options.iter().map(|option| (*option).to_owned()).collect()
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

fn count_occurrences(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[cfg(unix)]
fn assert_guard_policy_invokes_required_phases(policy: &Value, connection_id: &str) {
    let commands = policy["guard"]["commands"]
        .as_object()
        .expect("guard commands should be an object");
    let phases = [
        ("session_start", "session-start"),
        ("pre_tool", "pre-tool"),
        ("post_tool", "post-tool"),
        ("prompt_capture", "prompt-capture"),
        ("stop", "stop"),
    ];
    assert_eq!(
        commands.len(),
        phases.len(),
        "policy should define exactly the required guard phase commands"
    );

    for (policy_key, command_name) in phases {
        let command = commands
            .get(policy_key)
            .unwrap_or_else(|| panic!("missing guard command for {policy_key}"));
        assert_eq!(command["command"], "volicord");
        let args = command["args"]
            .as_array()
            .expect("guard command args should be an array");
        assert_eq!(args.first().and_then(Value::as_str), Some("guard"));
        assert_eq!(args.get(1).and_then(Value::as_str), Some(command_name));
        assert!(arg_pair(args, "--connection", connection_id));
        assert!(args.iter().any(|arg| arg == "--json"));
    }
}

#[cfg(unix)]
fn arg_pair(args: &[Value], key: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0] == key && pair[1] == value)
}

#[cfg(unix)]
fn assert_exported_mcp_config(
    output_path: &Path,
    mcp_command: &Path,
    runtime_home: &Path,
) -> Result<String, Box<dyn Error>> {
    let config: Value = serde_json::from_str(&fs::read_to_string(output_path)?)?;
    let server = &config["mcpServers"]["volicord"];
    let connection_id = server["args"]
        .as_array()
        .and_then(|args| match args.as_slice() {
            [mcp, stdio, flag, id]
                if mcp.as_str() == Some("mcp")
                    && stdio.as_str() == Some("--stdio")
                    && flag.as_str() == Some("--connection") =>
            {
                id.as_str()
            }
            _ => None,
        })
        .expect("exported MCP config should bind a connection id");

    assert_eq!(server["command"], path_text(mcp_command));
    assert_eq!(
        server["args"],
        serde_json::json!(["mcp", "--stdio", "--connection", connection_id])
    );
    assert_eq!(server["env"]["VOLICORD_HOME"], path_text(runtime_home));
    Ok(connection_id.to_owned())
}

fn write_test_installation_profile(runtime_home: &Path) -> Result<(), Box<dyn Error>> {
    write_installation_profile(
        runtime_home,
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: "volicord".to_owned(),
            volicord_mcp_command: "volicord".to_owned(),
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
fn write_fake_claude_code(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("claude");
    let state_path = path.with_extension("state");
    let state_text = state_path.display().to_string().replace('\'', "'\\''");
    let mut script = format!("#!/bin/sh\nstate='{state_text}'\n");
    script.push_str(
        "if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"get\" ]; then\n\
         if [ -f \"$state\" ]; then cat \"$state\"; exit 0; fi\n\
         printf 'Server not found\\n' >&2\n\
         exit 1\n\
         fi\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"add\" ]; then\n\
         shift 2\n\
         scope=\"\"\n\
         env_line=\"\"\n\
         command=\"\"\n\
         args=\"\"\n\
         while [ \"$#\" -gt 0 ]; do\n\
         case \"$1\" in\n\
         --env) env_line=\"$2\"; shift 2 ;;\n\
         --transport) shift 2 ;;\n\
         --scope) scope=\"$2\"; shift 2 ;;\n\
         --) shift; command=\"$1\"; shift; args=\"$*\"; break ;;\n\
         *) shift ;;\n\
         esac\n\
         done\n\
         {\n\
         printf 'Status: Connected\\n'\n\
         printf 'Scope: %s\\n' \"$scope\"\n\
         printf 'Command: %s\\n' \"$command\"\n\
         printf 'Args: %s\\n' \"$args\"\n\
         if [ -n \"$env_line\" ]; then printf 'Environment:\\n  %s\\n' \"$env_line\"; fi\n\
         } > \"$state\"\n\
         exit 0\n\
         fi\n\
         printf 'unexpected claude invocation\\n' >&2\n\
         exit 2\n",
    );
    fs::write(&path, script)?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn write_fake_mcp(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("volicord");
    fs::write(
        &path,
        "#!/bin/sh\n\
         mode=\"${VOLICORD_TEST_CONNECTION_MODE:-read_only}\"\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"--check\" ]; then\n\
         shift 2\n\
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
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"--stdio\" ] && [ \"$3\" = \"--connection\" ]; then\n\
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
        OperationCategory::UserOnly
        | OperationCategory::AdminLocal
        | OperationCategory::LocalRecovery => ActorSource::LocalUser,
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
