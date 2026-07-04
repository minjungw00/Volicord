#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    list_connection_projects, AgentConnectionRegistration, ConnectionProjectRegistration,
    CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
    VERIFIED_STATUS_ACTION_REQUIRED, VERIFIED_STATUS_COMPLETE,
};
use volicord_store::guards::{
    guard_event, insert_unrecorded_change, list_guard_installations, UnrecordedChangeInsert,
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
    UserJudgmentOptionId, UserJudgmentOptionInput, ADAPTER_UTILITY_TOOL_NAMES,
    READ_ONLY_METHOD_TOOL_NAMES, RECONCILE_CHANGES_TOOL_NAME,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING, WORKFLOW_METHOD_TOOL_NAMES,
};

#[test]
fn binary_help_uses_agent_connection_model() -> Result<(), Box<dyn Error>> {
    let help = run_without_home(["--help"])?;
    assert_success(&help);
    let text = stdout(&help);

    assert!(text.contains("volicord init --host"));
    assert!(text.contains("volicord status"));
    assert!(text.contains("volicord doctor"));
    assert!(text.contains("volicord project use"));
    assert!(text.contains("volicord connection add [HOST]"));
    assert!(text.contains("volicord export authority-bundle"));
    assert!(text.contains("volicord connection list [--repo PATH]"));
    assert!(text.contains("volicord connection status [HOST]"));
    assert!(text.contains("volicord changes reconcile"));
    assert!(text.contains("volicord serve --transport local-http"));
    assert!(text.contains("volicord mcp --stdio"));
    assert!(text.contains("volicord inbox answer <judgment-id> --choice <choice>"));
    assert!(text.contains("User Channel"));

    let init_help = run_without_home(["init", "--help"])?;
    assert_success(&init_help);
    let init_text = stdout(&init_help);
    assert!(init_text.contains("volicord init --host codex|claude-code --repo PATH"));
    assert!(init_text.contains("--profile record|detective"));
    assert!(init_text.contains("--home PATH"));
    assert!(init_text.contains("--mcp-command PATH"));
    assert!(init_text.contains("--dry-run"));
    assert!(init_text.contains("--json"));

    let status_help = run_without_home(["status", "--help"])?;
    assert_success(&status_help);
    assert!(stdout(&status_help).contains("volicord status [--repo PATH]"));

    let connection_help = run_without_home(["connection", "add", "--help"])?;
    assert_success(&connection_help);
    let connection_text = stdout(&connection_help);
    assert!(connection_text.contains("volicord connection add [HOST]"));
    assert!(connection_text.contains("--repo PATH"));
    assert!(connection_text.contains("--shared|--global"));
    assert!(connection_text.contains("--read-only"));
    Ok(())
}

#[test]
fn binary_help_options_match_supported_contracts() -> Result<(), Box<dyn Error>> {
    assert_help_options(
        ["--help"],
        &[
            "--version",
            "--home",
            "--mcp-command",
            "--json",
            "--repo",
            "--shared",
            "--global",
            "--read-only",
            "--dry-run",
            "--task",
            "--choice",
            "--note",
            "--stdio",
            "--check",
            "--connection",
            "--project",
            "--output",
            "--transport",
            "--listen",
            "--container-listen",
            "--token",
            "--token-file",
            "--generate-token",
            "--allow-origin",
            "--host",
            "--profile",
            "--privacy-footprint",
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
            "--container-listen",
            "--home",
            "--connection",
            "--project",
            "--token",
            "--token-file",
            "--generate-token",
            "--allow-origin",
        ],
    )?;
    assert_help_options(["status", "--help"], &["--repo", "--task", "--json"])?;
    assert_help_options(["doctor", "--help"], &["--json", "--privacy-footprint"])?;
    assert_help_options(
        ["connection", "add", "--help"],
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
            "--profile",
            "--home",
            "--mcp-command",
            "--dry-run",
            "--json",
        ],
    )?;
    assert_help_options(["connection", "list", "--help"], &["--repo", "--json"])?;
    assert_help_options(
        ["connection", "--help"],
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
        ["changes", "--help"],
        &["--repo", "--task", "--dry-run", "--json"],
    )?;
    assert_help_options(["export", "--help"], &["--repo", "--output", "--json"])?;
    assert_help_options(
        ["export", "authority-bundle", "--help"],
        &["--repo", "--output", "--json"],
    )?;
    assert_help_options(["project", "--help"], &["--repo", "--json"])?;
    assert_help_options(
        ["inbox", "--help"],
        &["--repo", "--task", "--choice", "--note", "--json"],
    )?;
    Ok(())
}

#[test]
fn export_help_lists_authority_bundle() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(["export", "--help"])?;
    assert_success(&output);
    let text = stdout(&output);

    assert!(text.contains("volicord export authority-bundle --output PATH"));
    assert!(!text.contains("mcp-config [--output"));
    assert!(!text.contains("--read-only"));
    Ok(())
}

#[test]
fn export_authority_bundle_help_shows_authority_bundle_usage() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(["export", "authority-bundle", "--help"])?;
    assert_success(&output);
    let text = stdout(&output);

    assert!(text.contains("volicord export authority-bundle --output PATH"));
    assert!(!text.contains("mcp-config [--output"));
    assert!(!text.contains("--read-only"));
    Ok(())
}

#[test]
fn export_mcp_config_is_not_public_command() -> Result<(), Box<dyn Error>> {
    assert_mcp_config_export_rejected(run_without_home(["export", "mcp-config"])?);
    assert_mcp_config_export_rejected(run_without_home(["export", "mcp-config", "--help"])?);
    Ok(())
}

#[test]
fn generic_host_guidance_does_not_suggest_export_command() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-generic-host-guidance")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_generic_guidance", "{}")?;
    write_test_installation_profile(runtime_home.path())?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
            "generic",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &[],
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());

    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("generic MCP host configuration is user-managed"));
    assert!(diagnostic.contains("supported managed connection hosts are `codex` and `claude-code`"));
    assert!(diagnostic.contains("after a supported Agent Connection exists"));
    assert!(!diagnostic.contains(&["volicord", "export", "mcp-config"].join(" ")));
    assert!(!diagnostic.contains(&["use", "the", "export", "command"].join(" ")));
    assert!(!diagnostic.contains(&["generic", "export"].join(" ")));
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
    assert_eq!(value["summary_card"]["recording"], "diagnostic_observation");
    assert_eq!(
        value["summary_card"]["next"],
        "Run volicord init --host <host> --repo <path> from the Product Repository to initialize the primary host connection."
    );
    assert_eq!(value["states"]["prompt_capture_status"], "not_checked");
    let doctor_text = run_with_home_env(runtime_home.path(), ["doctor"], &[])?;
    assert_success(&doctor_text);
    let text = stdout(&doctor_text);
    assert!(text.contains("Result: action_required (not a fatal CLI error)"));
    assert!(text.contains(
        "Why: local init or profile repair is required before Volicord workflows are usable"
    ));
    assert!(text.contains("runtime_home_state: ready"));
    assert!(text.contains("installation_profile_state: missing_or_invalid"));
    assert!(text.contains("mcp_config_state: unknown"));
    assert!(text.contains("prompt_capture_state: not_checked"));
    assert!(text.contains(
        "Next: Run volicord init --host <host> --repo <path> from the Product Repository to initialize the primary host connection."
    ));
    assert_eq!(fs::read_dir(runtime_home.path())?.count(), 0);
    Ok(())
}

#[test]
fn doctor_privacy_footprint_reports_runtime_home_scope() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-doctor-privacy")?;

    let json_output = run_with_home_env(
        runtime_home.path(),
        ["doctor", "--privacy-footprint", "--json"],
        &[],
    )?;
    assert_success(&json_output);
    let value = json_stdout(&json_output)?;
    assert_eq!(value["status"], "complete");
    assert_eq!(value["privacy_footprint"]["registry_state"], "missing");
    assert!(value["privacy_footprint"]["stores"]
        .as_array()
        .expect("stores should be an array")
        .iter()
        .any(|entry| entry
            .as_str()
            .unwrap_or_default()
            .contains("session-watch baselines")));
    assert!(value["privacy_footprint"]["does_not_prove"]
        .as_array()
        .expect("does_not_prove should be an array")
        .iter()
        .any(|entry| entry.as_str().unwrap_or_default() == "actor attribution"));
    assert!(value["privacy_footprint"]["does_not_store"]
        .as_array()
        .expect("does_not_store should be an array")
        .iter()
        .any(|entry| entry
            .as_str()
            .unwrap_or_default()
            .contains("Product Repository file contents")));

    let text_output =
        run_with_home_env(runtime_home.path(), ["doctor", "--privacy-footprint"], &[])?;
    assert_success(&text_output);
    let text = stdout(&text_output);
    assert!(text.contains("Volicord Runtime Home privacy footprint"));
    assert!(text.contains("does_not_prove: actor attribution"));
    assert!(text.contains("full filesystem monitoring"));
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
            "--profile",
            "detective",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["host"], "codex");
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["required_hook_phases"], "configured");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["hook_root_resolution"]["basis"], "git_work_tree");
    assert_eq!(value["hook_root_resolution"]["all_cwd_independent"], true);
    assert_eq!(value["states"]["hook_path_safety"], "ok");
    assert_eq!(value["states"]["hook_commands_cwd_independent"], true);
    assert_eq!(value["states"]["hook_commands_subdirectory_safe"], true);
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let hooks = fs::read_to_string(repo_root.join(".codex/hooks.json"))?;
    assert!(!hooks.contains("\"command\": \".codex/hooks/"));
    assert!(hooks.contains("git rev-parse --show-toplevel"));
    assert!(hooks.contains(".codex/hooks/volicord-dispatch.sh"));
    assert!(hooks.contains("session-start"));
    assert!(hooks.contains("pre-tool"));
    assert!(hooks.contains("post-tool"));
    assert!(hooks.contains("prompt-capture"));
    assert!(hooks.contains("stop"));
    assert!(!hooks.contains("volicord _hook "));
    assert!(hooks.contains(
        "Bash|apply_patch|Edit|Write|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*"
    ));
    let dispatch = repo_root.join(".codex/hooks/volicord-dispatch.sh");
    assert!(fs::read_to_string(&dispatch)?.contains("phase=dispatch"));
    assert!(is_executable(&dispatch)?);
    let wrapper = repo_root.join(".codex/hooks/volicord-pre-tool.sh");
    let wrapper_text = fs::read_to_string(&wrapper)?;
    assert!(wrapper_text.contains("exec volicord _hook pre-tool"));
    assert!(wrapper_text.contains(&format!("--connection {connection_id}")));
    assert!(wrapper_text.contains("--guard-installation"));
    assert!(wrapper_text.contains("--host codex"));
    assert!(wrapper_text.contains("--policy-hash"));
    assert!(wrapper_text.contains("--host-output codex"));
    assert!(is_executable(&wrapper)?);
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
            "--profile",
            "detective",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["host"], "claude-code");
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["hook_root_resolution"]["basis"], "claude_project_dir");
    assert_eq!(value["hook_root_resolution"]["all_cwd_independent"], true);
    assert_eq!(value["states"]["hook_path_safety"], "ok");
    assert_eq!(value["states"]["hook_commands_cwd_independent"], true);
    assert_eq!(value["states"]["hook_commands_subdirectory_safe"], true);
    assert!(repo_root.join(".mcp.json").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    let settings = fs::read_to_string(repo_root.join(".claude/settings.json"))?;
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-session-start.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-post-tool.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-prompt-capture.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-stop.sh"));
    assert!(!settings.contains("\"command\": \".claude/hooks/"));
    assert!(settings.contains("\"args\": []"));
    assert!(!settings.contains("volicord _hook "));
    assert!(settings.contains(
        "Bash|Edit|Write|MultiEdit|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*"
    ));
    let wrapper = repo_root.join(".claude/hooks/volicord-pre-tool.sh");
    let wrapper_text = fs::read_to_string(&wrapper)?;
    assert!(wrapper_text.contains("exec volicord _hook pre-tool"));
    assert!(wrapper_text.contains("--host claude-code"));
    assert!(wrapper_text.contains("--policy-hash"));
    assert!(wrapper_text.contains("--host-output claude-code"));
    assert!(is_executable(&wrapper)?);
    assert!(repo_root.join(".claude/rules/volicord.md").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_guarded_hook_command_runs_from_subdirectory_with_spaces() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-codex-hook-subdir")?;
    let repo_root = create_real_git_repo(&runtime_home, "product repo with spaces")?;
    let src_dir = repo_root.join("src");
    fs::create_dir_all(&src_dir)?;
    let bin_dir = runtime_home.path().join("bin with spaces");
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
            "--profile",
            "detective",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&output);
    let init_json = json_stdout(&output)?;
    assert_eq!(init_json["hook_root_resolution"]["basis"], "git_work_tree");
    assert_eq!(init_json["states"]["hook_path_safety"], "ok");
    let connection_id = init_json["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let project_id = init_json["connection"]["project_id"]
        .as_str()
        .expect("project_id should be present");

    let hooks: Value =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".codex/hooks.json"))?)?;
    assert_no_bare_hook_commands(&hooks, ".codex/hooks/");
    let command = codex_pre_tool_command(&hooks);
    assert!(command.contains("git rev-parse --show-toplevel"));
    assert!(command.contains(".codex/hooks/volicord-dispatch.sh"));
    assert!(command.contains("pre-tool"));
    assert!(!command.contains("volicord _hook "));

    let event_id = "generated_codex_pre_tool_from_src";
    let event = pre_tool_write_event(event_id);
    let hook_output = run_shell_hook_command(
        command,
        runtime_home.path(),
        &src_dir,
        &event,
        &[("PATH", hook_execution_path_env(&bin_dir)?)],
    )?;
    let value = assert_host_native_pre_tool_deny_output(&hook_output)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PreToolUse");

    let stored = guard_event(runtime_home.path(), project_id, event_id)?
        .expect("generated Codex hook command should invoke volicord _hook");
    assert_eq!(stored.connection_internal_id, connection_id);
    assert_eq!(stored.decision, "deny");
    let installations =
        list_guard_installations(runtime_home.path(), connection_id, Some(project_id))?;
    assert!(installations.iter().any(|installation| {
        installation.installation_status == "active"
            && installation.last_seen_phase.as_deref() == Some("pre_tool")
            && installation.observed_host_kind.as_deref() == Some("codex")
    }));
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_claude_code_guarded_hook_command_runs_from_subdirectory_with_spaces(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-claude-hook-subdir")?;
    let repo_root = create_real_git_repo(&runtime_home, "product repo with spaces")?;
    let src_dir = repo_root.join("src");
    fs::create_dir_all(&src_dir)?;
    let bin_dir = runtime_home.path().join("bin with spaces");
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
            "--profile",
            "detective",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&output);
    let init_json = json_stdout(&output)?;
    assert_eq!(
        init_json["hook_root_resolution"]["basis"],
        "claude_project_dir"
    );
    assert_eq!(init_json["states"]["hook_path_safety"], "ok");
    let connection_id = init_json["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let project_id = init_json["connection"]["project_id"]
        .as_str()
        .expect("project_id should be present");

    let settings: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".claude/settings.json"),
    )?)?;
    assert_no_bare_hook_commands(&settings, ".claude/hooks/");
    let (command, args) = claude_pre_tool_command(&settings);
    assert_eq!(
        command,
        "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh"
    );
    assert!(args.is_empty());
    let expanded_command = expand_claude_project_command(command, &repo_root)?;
    assert_eq!(
        expanded_command,
        repo_root.join(".claude/hooks/volicord-pre-tool.sh")
    );

    let event_id = "generated_claude_pre_tool_from_src";
    let event = pre_tool_write_event(event_id);
    let hook_output = run_executable_hook_command(
        &expanded_command,
        args,
        runtime_home.path(),
        &src_dir,
        &event,
        &[
            ("PATH", hook_execution_path_env(&bin_dir)?),
            ("CLAUDE_PROJECT_DIR", path_text(&repo_root)),
        ],
    )?;
    let value = assert_host_native_pre_tool_deny_output(&hook_output)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PreToolUse");

    let stored = guard_event(runtime_home.path(), project_id, event_id)?
        .expect("generated Claude Code hook command should invoke volicord _hook");
    assert_eq!(stored.connection_internal_id, connection_id);
    assert_eq!(stored.decision, "deny");
    let installations =
        list_guard_installations(runtime_home.path(), connection_id, Some(project_id))?;
    assert!(installations.iter().any(|installation| {
        installation.installation_status == "active"
            && installation.last_seen_phase.as_deref() == Some("pre_tool")
            && installation.observed_host_kind.as_deref() == Some("claude_code")
    }));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_downgrades_relative_codex_hook_command() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-relative-hook-downgrade")?;
    let repo_root = create_real_git_repo(&runtime_home, "product repo with spaces")?;
    let src_dir = repo_root.join("src");
    fs::create_dir_all(&src_dir)?;
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
            "--profile",
            "detective",
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
        .expect("connection_id should be present");

    let hooks_path = repo_root.join(".codex/hooks.json");
    let hooks: Value = serde_json::from_str(&fs::read_to_string(&hooks_path)?)?;
    let command = codex_pre_tool_command(&hooks);
    let active_event = pre_tool_write_event("relative_hook_before_downgrade");
    let hook_output = run_shell_hook_command(
        command,
        runtime_home.path(),
        &src_dir,
        &active_event,
        &[("PATH", hook_execution_path_env(&bin_dir)?)],
    )?;
    assert_host_native_pre_tool_deny_output(&hook_output)?;

    let active_status = run_with_home_env(
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
    assert_success(&active_status);
    let active_status_json = json_stdout(&active_status)?;
    assert_eq!(
        active_status_json["states"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        active_status_json["states"]["control_surface"]["host_hooks_active"],
        true
    );
    assert_eq!(
        active_status_json["states"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(active_status_json["states"]["hook_path_safety"], "ok");

    let mut hooks_json: Value = serde_json::from_str(&fs::read_to_string(&hooks_path)?)?;
    hooks_json["hooks"]["PreToolUse"][0]["hooks"][0]["command"] =
        serde_json::json!(".codex/hooks/volicord-pre-tool.sh");
    fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_json)?)?;

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
    assert_eq!(value["connection"]["connection_id"], connection_id);
    assert_eq!(value["states"]["hook_path_safety"], "relative_path_unsafe");
    assert_eq!(value["states"]["generated_config_verified"], false);
    assert_eq!(
        value["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(value["states"]["control_surface"]["os_enforced"], false);
    assert_eq!(value["primary_next_action"]["id"], "guard_hook_path_safety");
    assert_eq!(value["summary_card"]["transport"], "Agent Connection");
    assert!(value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord init --host codex --repo"));
    assert!(value["host_hook"]["stale_files"]
        .as_array()
        .expect("stale_files should be an array")
        .iter()
        .any(|path| path == &path_text(&hooks_path)));
    assert!(value["host_hook"]["hook_path_safety_details"]
        .as_array()
        .expect("hook path details should be an array")
        .iter()
        .any(|detail| detail["wrapper_resolution_status"] == "relative_path_unsafe"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_record_profile_skips_host_hooks() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-record")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    fs::write(repo_root.join("AGENTS.md"), "Existing project guidance\n")?;

    let text_output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&text_output);
    let init_text = stdout(&text_output);
    assert!(init_text.contains("Volicord initialized for Codex"));
    assert!(init_text.contains("Profile:\n  record"));
    assert!(init_text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(init_text.contains("Repo file changes:"));
    assert!(init_text.contains("created .codex/config.toml"));
    assert!(init_text.contains("created .volicord/policy.json"));
    assert!(init_text.contains("updated AGENTS.md"));
    assert!(init_text.contains(&format!(
        "Stored local Volicord state:\n  {}",
        runtime_home.path().display()
    )));
    assert!(init_text.contains("Next:"));
    assert!(init_text.contains("Open, restart, or reload Codex in this repository."));
    assert!(init_text.contains("Trust or approve the project configuration if Codex asks."));
    assert!(init_text.contains(&format!(
        "Run volicord connection verify codex --shared --repo {}.",
        repo_root.display()
    )));
    assert!(init_text.contains("Limits:"));
    assert!(init_text.contains("The record profile records Volicord setup"));
    assert!(init_text.contains("OS sandboxing, network isolation, malware defense"));
    assert!(init_text.contains("actor identity proof, correctness proof"));
    assert!(init_text.contains("Diagnostics:"));
    assert!(init_text.contains(&format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    )));
    assert_init_text_omits_internal_diagnostics(&init_text);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["selected_profile"], "record");
    assert_eq!(value["states"]["selected_profile"], "record");
    assert_eq!(
        value["changed_repo_files"]
            .as_array()
            .expect("changed_repo_files should be an array")
            .len(),
        0
    );
    assert_eq!(
        value["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        value["states"]["control_surface"]["cooperative_pre_tool_warning_available"],
        false
    );
    assert_eq!(
        value["states"]["control_surface"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(value["states"]["control_surface"]["os_enforced"], false);
    assert_eq!(value["states"]["hook_config"], "disabled");
    assert_eq!(value["states"]["rule_instruction_config"], "not_applicable");
    assert_eq!(value["states"]["required_hook_phases"], "disabled");
    assert_eq!(value["states"]["prompt_capture"], "not_configured");
    assert_eq!(value["states"]["guard_effective"], "inactive");
    assert_eq!(
        value["states"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(value["states"]["post_tool_correlation_available"], false);
    assert_eq!(value["states"]["bypass_detection_active"], false);
    assert!(repo_root.join(".codex/config.toml").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    assert!(!repo_root.join(".codex/hooks.json").exists());
    assert!(!repo_root.join(".codex/rules/volicord.rules").exists());

    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    let guard_installations = list_guard_installations(
        runtime_home.path(),
        connection_id,
        Some(&projects[0].project_id),
    )?;
    assert_eq!(guard_installations.len(), 1);
    assert_eq!(guard_installations[0].guard_mode, "record");
    let capability: Value = serde_json::from_str(&guard_installations[0].host_capability_json)?;
    assert_eq!(capability["selected_profile"], "record");
    assert_eq!(capability["prompt_capture"], true);
    assert!(capability["missing_required_hooks"]
        .as_array()
        .expect("missing hooks should be an array")
        .is_empty());

    let rerun_text_output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&rerun_text_output);
    let rerun_text = stdout(&rerun_text_output);
    assert!(rerun_text.contains("Volicord initialized for Codex"));
    assert!(rerun_text.contains("Repo file changes:\n  none"));
    assert!(!rerun_text.contains("created .codex/config.toml"));
    assert!(!rerun_text.contains("updated AGENTS.md"));
    assert_init_text_omits_internal_diagnostics(&rerun_text);
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_record_profile_succeeds_without_host_hooks_or_watcher() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-init-record-without-detective-prereqs")?;
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
            "--profile",
            "record",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["selected_profile"], "record");
    assert_eq!(value["states"]["hook_config"], "disabled");
    assert_eq!(value["states"]["guard_effective"], "inactive");
    assert!(repo_root.join(".codex/config.toml").exists());
    assert!(!repo_root.join(".codex/hooks.json").exists());
    assert!(!repo_root.join(".codex/rules/volicord.rules").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_detective_profile_fails_without_observe_prerequisites() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-init-detective-missing-prereqs")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let unsupported_runtime_home = repo_root.join(".volicord-runtime");
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
            "--profile",
            "detective",
            "--home",
            path_text(&unsupported_runtime_home).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;

    assert!(!output.status.success());
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("DETECTIVE_WATCHER_UNSUPPORTED"));
    assert!(diagnostic.contains("--profile record"));
    assert!(diagnostic.contains("supported host"));
    assert!(diagnostic.contains("repository configuration"));
    assert!(!repo_root.join(".codex/hooks.json").exists());
    assert!(!repo_root.join(".volicord/policy.json").exists());
    assert!(!repo_root.join("AGENTS.md").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_rejects_invalid_profile_without_artifacts() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-invalid-profile")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "managed",
            "--json",
        ],
        &[],
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown integration profile"));
    assert!(stderr(&output).contains("record or detective"));
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
            "--profile",
            "detective",
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
    assert_eq!(value["selected_profile"], "detective");
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
    assert_eq!(
        value["generated_files"]
            .as_array()
            .expect("generated files should be an array")
            .iter()
            .filter(|file| file["kind"] == "host_hook_dispatch")
            .count(),
        1
    );
    assert_eq!(
        value["generated_files"]
            .as_array()
            .expect("generated files should be an array")
            .iter()
            .filter(|file| file["kind"] == "host_hook_wrapper")
            .count(),
        5
    );
    assert!(value["generated_files"]
        .as_array()
        .expect("generated files should be an array")
        .iter()
        .any(|file| file["kind"] == "host_rule_instruction"));
    assert!(!runtime_home.registry_db_path().exists());
    assert!(!repo_root.join(".codex/config.toml").exists());
    assert!(!repo_root.join(".codex/hooks.json").exists());
    assert!(!repo_root.join(".codex/hooks/volicord-dispatch.sh").exists());
    assert!(!repo_root.join(".codex/hooks/volicord-pre-tool.sh").exists());
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
            "--profile",
            "detective",
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
            "--profile",
            "detective",
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
    assert_eq!(value["selected_profile"], "detective");
    assert_eq!(value["states"]["runtime_home"], "ready");
    assert_eq!(value["states"]["project_registration"], "registered");
    assert_eq!(value["states"]["mcp_config"], "match");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["selected_profile"], "detective");
    assert_eq!(
        value["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        value["states"]["control_surface"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(value["states"]["control_surface"]["os_enforced"], false);
    assert_eq!(
        value["states"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(value["states"]["post_tool_correlation_available"], false);
    assert_eq!(value["states"]["bypass_detection_active"], false);
    assert_eq!(value["states"]["local_web_consent_available"], false);
    assert_eq!(value["states"]["agents_managed_block"], "updated");
    assert_eq!(value["states"]["volicord_policy_file"], "created");
    assert_eq!(value["states"]["rule_instruction_config"], "created");
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["required_hook_phases"], "configured");
    assert_eq!(value["states"]["guard_observed"], false);
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["states"]["host_reload_required"], true);
    assert_eq!(value["primary_next_action"]["id"], "reload_required");
    assert!(value["primary_next_action"]["instruction"]
        .as_str()
        .expect("primary action instruction should be text")
        .contains("then run volicord connection verify codex --shared --repo"));
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
    let project_id = value["connection"]["project_id"]
        .as_str()
        .expect("project_id should be present");
    assert_eq!(
        value["mcp"]["args"],
        serde_json::json!([
            "mcp",
            "--stdio",
            "--connection",
            connection_id,
            "--project",
            project_id
        ])
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
            "--profile",
            "detective",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&text_output);
    let init_text = stdout(&text_output);
    assert!(init_text.contains("Volicord initialized for Codex"));
    assert!(init_text.contains("Profile:\n  detective"));
    assert!(init_text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(init_text.contains("Repo file changes:\n  none"));
    assert!(init_text.contains("Stored local Volicord state:"));
    assert!(init_text.contains("Next:"));
    assert!(init_text.contains("Open, restart, or reload Codex in this repository."));
    assert!(init_text.contains("Trust or approve the project configuration if Codex asks."));
    assert!(init_text.contains(&format!(
        "Run volicord connection verify codex --shared --repo {}.",
        repo_root.display()
    )));
    assert!(init_text.contains("Limits:"));
    assert!(init_text.contains("The detective profile adds cooperative host observation"));
    assert!(init_text.contains("OS sandboxing, network isolation, malware defense"));
    assert!(init_text.contains("Diagnostics:"));
    assert!(init_text.contains(&format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    )));
    assert_init_text_omits_internal_diagnostics(&init_text);

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
    assert!(status_without_intent_json["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("Restart or reload codex"));
    assert!(status_without_intent_json["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("then run volicord connection verify codex --shared --repo"));
    assert_eq!(
        status_without_intent_json["states"]["hook_config"],
        "installed"
    );
    assert_eq!(
        status_without_intent_json["states"]["guard_observed"],
        false
    );
    assert_eq!(
        status_without_intent_json["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        status_without_intent_json["states"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(
        status_without_intent_json["states"]["local_web_consent_available"],
        false
    );
    let status_without_intent_text = run_with_home_env(
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
    assert_success(&status_without_intent_text);
    let status_text = stdout(&status_without_intent_text);
    assert!(status_text.contains("Result: action_required (not a fatal CLI error)"));
    assert!(status_text
        .contains("Why: Host configuration is present, but the host has not reloaded it yet."));
    assert!(status_text.contains("Next: Restart or reload codex"));
    assert!(status_text.contains("then run volicord connection verify codex --shared --repo"));
    assert!(status_text.contains("Does not prove: OS sandboxing"));

    let config = fs::read_to_string(repo_root.join(".codex/config.toml"))?;
    assert!(config.contains(&format!(
        "args = [\"mcp\", \"--stdio\", \"--connection\", \"{connection_id}\", \"--project\", \"{project_id}\"]"
    )));
    let hooks = fs::read_to_string(repo_root.join(".codex/hooks.json"))?;
    assert!(hooks.contains("SessionStart"));
    assert!(hooks.contains("PreToolUse"));
    assert!(hooks.contains("PostToolUse"));
    assert!(hooks.contains("UserPromptSubmit"));
    assert!(hooks.contains("Stop"));
    assert!(!hooks.contains("\"command\": \".codex/hooks/"));
    assert!(hooks.contains("git rev-parse --show-toplevel"));
    assert!(hooks.contains(".codex/hooks/volicord-dispatch.sh"));
    assert!(hooks.contains("session-start"));
    assert!(hooks.contains("pre-tool"));
    assert!(hooks.contains("post-tool"));
    assert!(hooks.contains("prompt-capture"));
    assert!(hooks.contains("stop"));
    assert!(!hooks.contains("volicord _hook "));
    assert!(hooks.contains(
        "Bash|apply_patch|Edit|Write|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*"
    ));
    let rules = fs::read_to_string(repo_root.join(".codex/rules/volicord.rules"))?;
    assert!(rules.contains("# BEGIN VOLICORD MANAGED CODEX RULES v1"));
    assert!(rules.contains("prefix_rule("));
    assert!(rules.contains("git rev-parse --show-toplevel"));
    assert!(rules.contains(".codex/hooks/volicord-dispatch.sh"));
    assert!(rules.contains("session-start"));
    assert!(rules.contains("stop"));

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
    assert_eq!(policy["selected_profile"], "detective");
    assert_eq!(policy["mcp"]["command"], "volicord");
    assert_eq!(
        policy["mcp"]["args"],
        serde_json::json!([
            "mcp",
            "--stdio",
            "--connection",
            connection_id,
            "--project",
            project_id
        ])
    );
    assert_eq!(policy["host_hook"]["enabled"], true);
    assert_guard_policy_invokes_required_phases(&policy, &connection_id);
    assert_eq!(
        policy["host_hook"]["commands"]["pre_tool"]["command"],
        "volicord"
    );
    assert_eq!(
        policy["host_hook"]["commands"]["pre_tool"]["args"][0],
        "_hook"
    );
    assert_eq!(
        policy["host_hook"]["commands"]["pre_tool"]["args"][1],
        "pre-tool"
    );
    assert!(policy["host_hook"]["commands"]["pre_tool"]["args"]
        .as_array()
        .expect("host-hook args should be an array")
        .windows(2)
        .any(|pair| pair[0] == "--connection" && pair[1] == connection_id));

    let guard_installations = list_guard_installations(
        runtime_home.path(),
        &connection_id,
        Some(&projects[0].project_id),
    )?;
    assert_eq!(guard_installations.len(), 1);
    assert_eq!(guard_installations[0].host_kind, "codex");
    assert_eq!(guard_installations[0].guard_mode, "detective");
    assert_eq!(guard_installations[0].installation_status, "configured");
    let capability: Value = serde_json::from_str(&guard_installations[0].host_capability_json)?;
    assert_eq!(capability["schema"], "volicord-host-hook-capability-v1");
    assert_eq!(
        capability["policy_hash"],
        value["guard_installation"]["policy_hash"]
    );
    assert_eq!(capability["prompt_capture"], true);
    assert_eq!(capability["selected_profile"], "detective");
    assert_eq!(capability["native_host_output_adapter"], "codex");
    assert_eq!(capability["native_host_output_adapter_verified"], true);
    assert_eq!(capability["bash_shell_mutation_coverage"], true);
    assert_eq!(capability["direct_file_write_matcher_coverage"], true);
    assert_eq!(capability["missing_required_hooks"], serde_json::json!([]));
    assert_eq!(capability["host_capabilities"]["pre_tool_hook"], true);
    assert_eq!(
        capability["host_capabilities"]["user_prompt_submit_hook"],
        true
    );
    assert!(capability["commands"]["pre_tool"]["args"]
        .as_array()
        .expect("capability host-hook args should be an array")
        .windows(2)
        .any(|pair| pair[0] == "--host-output" && pair[1] == "codex"));
    let dispatch_path = repo_root.join(".codex/hooks/volicord-dispatch.sh");
    let dispatch = fs::read_to_string(&dispatch_path)?;
    assert!(dispatch.contains("phase=dispatch"));
    assert!(dispatch.contains("git rev-parse --show-toplevel"));
    assert!(dispatch.contains(".codex/hooks/volicord-$phase.sh"));
    assert!(dispatch.contains("exec \"$wrapper\""));
    assert!(is_executable(&dispatch_path)?);
    let wrapper_path = repo_root.join(".codex/hooks/volicord-pre-tool.sh");
    let wrapper = fs::read_to_string(&wrapper_path)?;
    assert!(wrapper.contains("exec volicord _hook pre-tool"));
    assert!(wrapper.contains(&format!("--connection {connection_id}")));
    assert!(wrapper.contains("--guard-installation"));
    assert!(wrapper.contains("--host codex"));
    assert!(wrapper.contains("--integration-profile detective"));
    assert!(wrapper.contains("--policy-hash"));
    assert!(wrapper.contains(
        capability["policy_hash"]
            .as_str()
            .expect("capability should include policy hash")
    ));
    assert!(wrapper.contains("--host-output codex"));
    assert!(is_executable(&wrapper_path)?);
    assert!(capability["files"]
        .as_array()
        .expect("capability files should be an array")
        .iter()
        .any(|file| file["kind"] == "host_hook_dispatch"
            && file["path"] == path_text(&dispatch_path)
            && file["executable_required"] == true));
    assert!(capability["files"]
        .as_array()
        .expect("capability files should be an array")
        .iter()
        .any(|file| file["kind"] == "host_hook_wrapper"
            && file["path"] == path_text(&wrapper_path)
            && file["executable_required"] == true));

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
    assert_eq!(doctor_json["states"]["selected_profile"], "detective");
    assert_eq!(
        doctor_json["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        doctor_json["states"]["control_surface"]["os_enforced"],
        false
    );
    assert_eq!(
        doctor_json["states"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(
        doctor_json["states"]["post_tool_correlation_available"],
        false
    );
    assert_eq!(doctor_json["states"]["bash_shell_mutation_coverage"], true);
    assert_eq!(doctor_json["states"]["bypass_detection_active"], false);
    assert_eq!(doctor_json["states"]["agents_managed_block"], "installed");
    assert_eq!(doctor_json["states"]["volicord_policy_file"], "installed");
    assert_eq!(
        doctor_json["states"]["rule_instruction_config"],
        "installed"
    );
    assert_eq!(doctor_json["states"]["hook_config"], "installed");
    assert_eq!(doctor_json["states"]["required_hook_phases"], "configured");
    assert_eq!(
        doctor_json["states"]["prompt_capture"],
        "action_recommended"
    );
    assert_eq!(
        doctor_json["states"]["prompt_capture_status"],
        "configured_unobserved"
    );
    assert_eq!(doctor_json["states"]["watcher_status"], "not_started");
    assert_eq!(
        doctor_json["states"]["watcher_scan_summary"]["files_scanned"],
        0
    );
    assert_eq!(
        doctor_json["states"]["watcher_scan_summary"]["not_full_filesystem_monitoring"],
        true
    );

    let second = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
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
    assert_eq!(second_json["states"]["hook_config"], "unchanged");
    assert_eq!(second_json["states"]["prompt_capture"], "configured");
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
            "--profile",
            "detective",
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
    assert_eq!(value["selected_profile"], "detective");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["mcp"]["command"], "volicord");
    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let project_id = value["connection"]["project_id"]
        .as_str()
        .expect("project_id should be present");

    let mcp_config: Value =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".mcp.json"))?)?;
    let server = &mcp_config["mcpServers"]["volicord"];
    assert_eq!(server["command"], "volicord");
    assert_eq!(
        server["args"],
        serde_json::json!([
            "mcp",
            "--stdio",
            "--connection",
            connection_id,
            "--project",
            project_id
        ])
    );

    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["host"], "claude-code");
    assert_eq!(policy["host_hook"]["enabled"], true);
    assert_guard_policy_invokes_required_phases(&policy, connection_id);
    assert_eq!(
        policy["host_hook"]["commands"]["session_start"]["command"],
        "volicord"
    );
    let settings = fs::read_to_string(repo_root.join(".claude/settings.json"))?;
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-session-start.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-post-tool.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-prompt-capture.sh"));
    assert!(settings.contains("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-stop.sh"));
    assert!(!settings.contains("\"command\": \".claude/hooks/"));
    assert!(settings.contains("\"args\": []"));
    assert!(!settings.contains("volicord _hook "));
    assert!(settings.contains(
        "\"matcher\": \"Bash|Edit|Write|MultiEdit|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*\""
    ));
    assert!(repo_root.join(".claude/rules/volicord.md").exists());
    let rule = fs::read_to_string(repo_root.join(".claude/rules/volicord.md"))?;
    assert!(rule.contains(".volicord/policy.json"));
    assert!(rule.contains("Configured local detective host-hook commands"));
    assert!(rule.contains(".claude/hooks/volicord-session-start.sh"));
    assert!(rule.contains(".claude/hooks/volicord-pre-tool.sh"));
    assert!(rule.contains(".claude/hooks/volicord-prompt-capture.sh"));

    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    let guard_installations = list_guard_installations(
        runtime_home.path(),
        connection_id,
        Some(&projects[0].project_id),
    )?;
    assert_eq!(guard_installations.len(), 1);
    assert_eq!(guard_installations[0].host_kind, "claude_code");
    assert_eq!(guard_installations[0].guard_mode, "detective");
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
    let wrapper_path = repo_root.join(".claude/hooks/volicord-pre-tool.sh");
    let wrapper = fs::read_to_string(&wrapper_path)?;
    assert!(wrapper.contains("exec volicord _hook pre-tool"));
    assert!(wrapper.contains(&format!("--connection {connection_id}")));
    assert!(wrapper.contains("--guard-installation"));
    assert!(wrapper.contains("--host claude-code"));
    assert!(wrapper.contains("--integration-profile detective"));
    assert!(wrapper.contains("--policy-hash"));
    assert!(wrapper.contains(
        capability["policy_hash"]
            .as_str()
            .expect("capability should include policy hash")
    ));
    assert!(wrapper.contains("--host-output claude-code"));
    assert!(is_executable(&wrapper_path)?);
    Ok(())
}

#[test]
fn ordinary_command_before_profile_instructs_init() -> Result<(), Box<dyn Error>> {
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

#[test]
fn export_authority_bundle_writes_integrity_metadata_without_mutating_project_state(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-authority-bundle")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_authority_bundle", "{}")?;
    write_test_installation_profile(runtime_home.path())?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    register_project(
        runtime_home.path(),
        ProjectRegistration {
            project_id: "project_authority_bundle".to_owned(),
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let state_db_path = runtime_home.project_state_db_path("project_authority_bundle");
    let registry_before = file_sha256_hex(&runtime_home.registry_db_path())?;
    let state_before = file_sha256_hex(&state_db_path)?;
    let output_dir = runtime_home.path().join("authority-bundle-output");
    let repo_arg = path_text(&repo_root);
    let output_arg = path_text(&output_dir);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "export",
            "authority-bundle",
            "--repo",
            repo_arg.as_str(),
            "--output",
            output_arg.as_str(),
            "--json",
        ],
        &[],
    )?;
    assert_success(&output);
    let command_json = json_stdout(&output)?;
    assert_eq!(command_json["bundle_kind"], "authority_bundle");
    assert_eq!(command_json["record_count"], 1);
    assert_eq!(command_json["artifact_count"], 0);

    let manifest_path = output_dir.join("manifest.json");
    let records_path = output_dir.join("records.jsonl");
    let checksums_path = output_dir.join("checksums.sha256");
    let readme_path = output_dir.join("README.txt");
    assert!(manifest_path.exists());
    assert!(records_path.exists());
    assert!(checksums_path.exists());
    assert!(readme_path.exists());
    assert!(output_dir.join("artifacts").is_dir());

    let manifest: Value = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    let files = manifest["files"]
        .as_array()
        .expect("manifest files should be an array")
        .iter()
        .map(|entry| {
            entry["path"]
                .as_str()
                .expect("manifest file paths should be strings")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    for expected in [
        "manifest.json",
        "records.jsonl",
        "checksums.sha256",
        "README.txt",
        "artifacts/",
    ] {
        assert!(
            files.contains(expected),
            "manifest should include {expected}: {manifest}"
        );
    }
    assert_eq!(manifest["hash_algorithm"], "sha256");
    assert_eq!(manifest["records"]["record_count"], 1);
    assert_eq!(manifest["records"]["path"], "records.jsonl");
    assert_eq!(manifest["project"]["repo_root"], path_text(&repo_root));

    let records = fs::read_to_string(&records_path)?;
    let exported_record: Value = serde_json::from_str(
        records
            .lines()
            .next()
            .expect("records.jsonl should contain the project_state row"),
    )?;
    assert_eq!(exported_record["database"], "project_state");
    assert_eq!(exported_record["table"], "project_state");
    assert_eq!(
        exported_record["row"]["project_id"],
        "project_authority_bundle"
    );

    let checksum_text = fs::read_to_string(&checksums_path)?;
    let checksum_paths = checksum_text
        .lines()
        .map(|line| verify_checksum_line(&output_dir, line))
        .collect::<Result<BTreeSet<_>, _>>()?;
    assert!(checksum_paths.contains("manifest.json"));
    assert!(checksum_paths.contains("records.jsonl"));
    assert!(checksum_paths.contains("README.txt"));
    assert!(!checksum_paths.contains("checksums.sha256"));

    let readme = fs::read_to_string(&readme_path)?;
    assert!(readme.contains("integrity-labeled copy of local Volicord records"));
    assert!(readme.contains("not proof that the Runtime Home was never modified before export"));
    assert!(readme
        .contains("not a correctness, test sufficiency, review completion, or deployment proof"));

    assert_eq!(
        registry_before,
        file_sha256_hex(&runtime_home.registry_db_path())?
    );
    assert_eq!(state_before, file_sha256_hex(&state_db_path)?);
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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let dry_run = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
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
            "connection",
            "add",
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
        "args = [\"mcp\", \"--stdio\", \"--connection\", \"{connection_id}\", \"--project\", \"{}\"]",
        projects[0].project_id
    )));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_verification_handshake_does_not_create_session_watch_records(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-verification-watch-skip")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    prepare_runtime_home(runtime_home.path(), Path::new(volicord_bin()))?;
    let volicord_dir = Path::new(volicord_bin())
        .parent()
        .expect("volicord test binary path should have a parent");

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path(), volicord_dir]))],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(
        value["verification"]["preflight"]["status"], "passed",
        "{value}"
    );
    assert_eq!(
        value["verification"]["mcp_handshake"]["status"], "passed",
        "{value}"
    );

    let connection_id = value["connection"]["connection_id"]
        .as_str()
        .expect("connection_id should be present");
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(
        session_watch_record_counts(runtime_home.path(), &projects[0].project_id)?,
        (0, 0)
    );
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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["connection", "add", "codex", "--json"],
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
    assert!(status_text.contains("Next: none"));
    Ok(())
}

#[test]
fn connect_codex_global_reports_supported_intents() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-codex-global")?;
    initialize_runtime_home(runtime_home.path(), "runtime_home_codex_global", "{}")?;
    write_test_installation_profile(runtime_home.path())?;

    let output = run_with_home_env(
        runtime_home.path(),
        ["connection", "add", "codex", "--global"],
        &[],
    )?;

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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
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
            "--profile",
            "detective",
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
    assert_eq!(value["summary_card"]["recording"], "diagnostic_observation");
    assert!(value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord init --host codex --repo"));
    assert_eq!(
        value["primary_next_action"]["command"],
        format!(
            "volicord init --host codex --repo {}",
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
fn connection_verify_fails_when_workflow_reconcile_changes_tool_is_missing(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-missing-reconcile")?;
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
            "--profile",
            "detective",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&init);

    write_fake_mcp_missing_workflow_reconcile(&bin_dir)?;
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
    assert_eq!(value["status"], "failed");
    assert_eq!(value["connection"]["verification_status"], "failed");
    assert_eq!(value["verification"]["preflight"]["status"], "passed");
    assert_eq!(value["verification"]["mcp_handshake"]["status"], "failed");
    assert!(value["verification"]["mcp_handshake"]["details"]
        .as_str()
        .expect("handshake details should be text")
        .contains(RECONCILE_CHANGES_TOOL_NAME));
    assert!(!value["verification"]["tools"]
        .as_array()
        .expect("failed handshake should still report a tools array")
        .iter()
        .any(|tool| tool == RECONCILE_CHANGES_TOOL_NAME));
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
            "--profile",
            "detective",
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
    assert!(value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("reinstall missing detective host-hook files"));
    assert!(value["host_hook"]["missing_files"]
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
            "--profile",
            "detective",
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
    assert!(value["host_hook"]["stale_files"]
        .as_array()
        .expect("stale_files should be an array")
        .iter()
        .any(|path| path == &path_text(&policy_path)));

    let status_text = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &[],
    )?;
    assert_success(&status_text);
    let text = stdout(&status_text);
    assert!(
        text.contains("Result: action_required (not a fatal CLI error)")
            || text.contains("Result: complete (detective diagnostics degraded)")
    );
    assert!(text.contains(
        "Why: Detective host-hook files no longer match the recorded managed configuration."
    ));
    assert!(text.contains("Next: Run volicord init --host codex --repo"));
    assert!(text.contains("Does not prove: OS sandboxing"));

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let doctor_json = json_stdout(&doctor)?;
    assert_eq!(doctor_json["states"]["guard_files"], "action_recommended");
    assert_eq!(doctor_json["states"]["volicord_policy_file"], "stale");
    assert_eq!(
        doctor_json["primary_next_action"]["id"],
        "repair_guard_files"
    );
    assert!(doctor_json["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .starts_with("recommended: Run volicord init"));
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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

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
            "connection",
            "add",
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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let connect = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
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
    assert_diagnostic_disclosure(&connect_json);

    let connect_second = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
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
        ["connection", "list", "--repo", path_text(&repo_a).as_str()],
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
    let status_json = json_stdout(&status)?;
    assert_eq!(status_json["connection"]["connection_id"], connection_id);
    assert_diagnostic_disclosure(&status_json);

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
    assert_diagnostic_disclosure(&verify_json);
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
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    for codex_home in [&codex_home_a, &codex_home_b] {
        let connect = run_with_home_env(
            runtime_home.path(),
            [
                "connection",
                "add",
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

    let status = run_with_home_env_in_dir(runtime_home.path(), ["status"], &[], &repo_root)?;
    assert_success(&status);
    let status_text = stdout(&status);
    assert!(status_text.contains("User Channel status"));
    assert!(status_text.contains("Close Status: blocked"));
    assert!(status_text.contains("User Judgment: pending (1)"));
    assert!(status_text.contains(
        "Available answer paths: host prompt unavailable; chat capture unavailable; local consent unavailable; CLI inbox available"
    ));
    assert!(status_text.contains("Next:"));
    assert!(status_text.contains("Does not prove:"));
    assert!(status_text.contains("risk-free outcome"));

    let status_json =
        run_with_home_env_in_dir(runtime_home.path(), ["status", "--json"], &[], &repo_root)?;
    assert_success(&status_json);
    let status_value = json_stdout(&status_json)?;
    assert_eq!(status_value["summary_card"]["close_status"], "blocked");
    assert_eq!(status_value["summary_card"]["user_judgment"], "pending (1)");
    assert_eq!(
        channel_path(&status_value["user_channel_availability"], "cli")["available"],
        true
    );
    assert_eq!(
        channel_path(
            &status_value["user_channel_availability"],
            "local_web_consent"
        )["available"],
        false
    );

    let list = run_with_home_env_in_dir(runtime_home.path(), ["inbox"], &[], &repo_root)?;
    assert_success(&list);
    let list_text = stdout(&list);
    assert!(list_text.contains("Judgment Inbox"));
    assert!(list_text.contains("1. Should the focused CLI user-channel choice be accepted?"));
    assert!(list_text.contains("id: "));
    assert!(list_text.contains("accept: Accept focused choice"));
    assert!(list_text.contains(
        "Available answer paths: host prompt unavailable; chat capture unavailable; local consent unavailable; CLI inbox available"
    ));
    assert!(list_text.contains("volicord inbox answer"));
    assert!(list_text.contains("Does not prove: approval"));
    assert!(!list_text.contains("project_user_channel"));

    let list_json =
        run_with_home_env_in_dir(runtime_home.path(), ["inbox", "--json"], &[], &repo_root)?;
    assert_success(&list_json);
    let list_value = json_stdout(&list_json)?;
    assert_eq!(list_value["summary_card"]["user_judgment"], "pending (1)");
    assert_eq!(
        channel_path(&list_value["user_channel_availability"], "cli")["available"],
        true
    );
    assert_eq!(
        channel_path(&list_value["user_channel_availability"], "mcp_elicitation")["available"],
        false
    );
    assert!(list_value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord inbox answer"));
    let first = &list_value["pending_judgment_inbox_items"][0];
    assert_eq!(first["judgment_id"], judgment_id.as_str());
    assert_eq!(first["requirement_status"], "required");
    assert_eq!(first["choices"][0]["choice_id"], "accept");
    assert_eq!(
        channel_path(&first["answer_path_availability"], "cli")["available"],
        true
    );
    assert_eq!(first["preferred_capture_path"]["kind"], "cli");
    assert!(first["preferred_capture_path"]["command"]
        .as_str()
        .expect("CLI command should be present")
        .contains("volicord inbox answer"));
    assert!(first["choices"][0].get("machine_action").is_none());
    assert!(first["choices"][0].get("resolution_outcome").is_none());

    let open = run_with_home_env_in_dir(
        runtime_home.path(),
        ["inbox", "open", judgment_id.as_str()],
        &[],
        &repo_root,
    )?;
    assert_success(&open);
    let open_text = stdout(&open);
    assert!(open_text.contains("Judgment Inbox open action_required"));
    assert!(open_text.contains("Result: action_required (not a fatal CLI error)"));
    assert!(open_text.contains("Why: No local consent URL is available"));
    assert!(open_text.contains("Next: Use the URL shown in the MCP Judgment Inbox item"));
    assert!(open_text.contains("volicord inbox answer"));
    assert!(open_text.contains("Does not prove: approval"));

    let record_note = "Recorded from inbox CLI";
    let record = run_with_home_env_in_dir(
        runtime_home.path(),
        [
            "inbox",
            "answer",
            judgment_id.as_str(),
            "--choice",
            "accept",
            "--note",
            record_note,
        ],
        &[],
        &repo_root,
    )?;
    assert_success(&record);
    let text = stdout(&record);
    assert!(text.contains("Judgment Inbox answer recorded"));
    assert!(text.contains("selected: Accept focused choice"));
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

    let dry_json_output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["changes", "reconcile", "--dry-run", "--json"],
        &[],
        &repo_root,
    )?;
    assert_success(&dry_json_output);
    let dry_json = json_stdout(&dry_json_output)?;
    assert_eq!(dry_json["base"]["response_kind"], "dry_run");
    assert_eq!(dry_json["base"]["effect_kind"], "no_effect");
    assert_eq!(
        dry_json["dry_run_summary"]["planned_effects"][0]["action"],
        "classify"
    );
    assert!(dry_json["dry_run_summary"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .iter()
        .any(|diagnostic| diagnostic == "automatically_reconcilable_changes=1"));
    assert_non_guarantees(
        &dry_json["base"]["disclosure"],
        &[
            "NotActorAttributionProof",
            "NotIntentProof",
            "NotCorrectnessProof",
        ],
    );

    let dry_text_output = run_with_home_env_in_dir(
        runtime_home.path(),
        ["changes", "reconcile", "--dry-run"],
        &[],
        &repo_root,
    )?;
    assert_success(&dry_text_output);
    let dry_text = stdout(&dry_text_output);
    assert!(dry_text.contains("Changes reconciliation (dry run)"));
    assert!(dry_text.contains("automatically_reconcilable_changes=1"));
    assert!(dry_text.contains("Does not prove:"));
    assert!(dry_text.contains("intent proof"));

    let conn =
        rusqlite::Connection::open(runtime_home.project_state_db_path("project_user_channel"))?;
    let unresolved_after_dry_run: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM unrecorded_changes
          WHERE unrecorded_change_id = 'unrecorded_cli_changes_reconcile'
            AND status = 'unresolved'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(unresolved_after_dry_run, 1);
    let reconcile_invocations_after_dry_run: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM tool_invocations
          WHERE tool_name = 'volicord.reconcile_changes'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(reconcile_invocations_after_dry_run, 0);

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
    assert_eq!(value["summary_card"]["recording"], "core_committed");
    assert_eq!(value["summary_card"]["changes"], "none");

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
    assert!(text.contains("Changes reconciliation"));
    assert!(text.contains("Changes: none"));
    assert!(text.contains("Next:"));
    assert!(text.contains("Does not prove:"));
    assert!(text.contains("product-file write occurred"));
    assert!(!text.contains("reconciled changes:"));

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

fn assert_init_text_omits_internal_diagnostics(text: &str) {
    for forbidden in [
        "Volicord init action_required",
        "Result: action_required",
        "Why:",
        "connection_id:",
        "observation_summary:",
        "observation_capabilities:",
        "detective_installation_state:",
        "detective_effective_state:",
        "generated_file_count:",
    ] {
        assert!(
            !text.contains(forbidden),
            "init text should not expose `{forbidden}`:\n{text}"
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

fn prepare_runtime_home(runtime_home: &Path, mcp_command: &Path) -> Result<(), Box<dyn Error>> {
    initialize_runtime_home(runtime_home, "runtime_home_binary_admin_fixture", "{}")?;
    write_installation_profile(
        runtime_home,
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: path_text(mcp_command),
            volicord_mcp_command: path_text(mcp_command),
            bin_dir: mcp_command
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| runtime_home.join("bin")),
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
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

fn assert_mcp_config_export_rejected(output: Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());

    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("unknown export command: mcp-config"));
    assert!(diagnostic.contains("volicord export authority-bundle --output PATH"));
    assert!(!diagnostic.contains("mcp-config [--output"));
    assert!(!diagnostic.contains("--read-only"));
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

fn channel_path<'a>(availability: &'a Value, kind: &str) -> &'a Value {
    let paths = availability["paths"]
        .as_array()
        .expect("user_channel_availability.paths should be an array");
    paths
        .iter()
        .find(|path| path["kind"] == kind)
        .unwrap_or_else(|| panic!("expected user channel path {kind}, got {paths:?}"))
}

fn assert_diagnostic_disclosure(value: &Value) {
    let disclosure = value
        .get("disclosure")
        .expect("diagnostic output should include disclosure");
    assert_eq!(disclosure["guarantee_class"], "detective_observation");
    assert_non_guarantees(
        disclosure,
        &[
            "NotOsSandbox",
            "NotActorAttributionProof",
            "NotCorrectnessProof",
        ],
    );
}

fn assert_non_guarantees(disclosure: &Value, expected: &[&str]) {
    let values = disclosure["non_guarantees"]
        .as_array()
        .expect("disclosure should include non_guarantees");
    for expected_value in expected {
        assert!(
            values
                .iter()
                .any(|value| value.as_str() == Some(expected_value)),
            "missing non-guarantee {expected_value}: {disclosure}"
        );
    }
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

fn session_watch_record_counts(
    runtime_home: &Path,
    project_id: &str,
) -> Result<(i64, i64), Box<dyn Error>> {
    let conn = rusqlite::Connection::open(
        runtime_home
            .join("projects")
            .join(project_id)
            .join("state.sqlite"),
    )?;
    let agent_sessions = conn.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    let watch_baselines = conn.query_row(
        "SELECT COUNT(*) FROM session_watch_baselines WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    Ok((agent_sessions, watch_baselines))
}

fn verify_checksum_line(root: &Path, line: &str) -> Result<String, Box<dyn Error>> {
    let (expected_hash, relative_path) = line.split_once("  ").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("checksum line should use sha256sum format: {line}"),
        )
    })?;
    assert_eq!(expected_hash, file_sha256_hex(&root.join(relative_path))?);
    Ok(relative_path.to_owned())
}

fn file_sha256_hex(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(lowercase_hex_bytes(&hasher.finalize()))
}

fn lowercase_hex_bytes(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn count_occurrences(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[cfg(unix)]
fn codex_pre_tool_command(hooks: &Value) -> &str {
    hooks["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .expect("Codex PreToolUse command should be present")
}

#[cfg(unix)]
fn claude_pre_tool_command(settings: &Value) -> (&str, Vec<String>) {
    let command = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .expect("Claude Code PreToolUse command should be present");
    let args = settings["hooks"]["PreToolUse"][0]["hooks"][0]["args"]
        .as_array()
        .expect("Claude Code PreToolUse args should be present")
        .iter()
        .map(|arg| {
            arg.as_str()
                .expect("Claude Code hook args should be strings")
                .to_owned()
        })
        .collect();
    (command, args)
}

#[cfg(unix)]
fn assert_no_bare_hook_commands(value: &Value, prefix: &str) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "command" {
                    let command = value
                        .as_str()
                        .expect("hook command values should be strings");
                    assert!(
                        !contains_bare_hook_path(command, prefix),
                        "hook command must not use a cwd-relative wrapper path: {command}"
                    );
                }
                assert_no_bare_hook_commands(value, prefix);
            }
        }
        Value::Array(values) => {
            for value in values {
                assert_no_bare_hook_commands(value, prefix);
            }
        }
        _ => {}
    }
}

#[cfg(unix)]
fn contains_bare_hook_path(command: &str, prefix: &str) -> bool {
    let trimmed = command.trim_start_matches([' ', '\'', '"']);
    trimmed.starts_with(prefix)
        || trimmed.starts_with(&format!("./{prefix}"))
        || command.contains(&format!(" {prefix}"))
        || command.contains(&format!(" './{prefix}"))
        || command.contains(&format!(" \"./{prefix}"))
        || command.contains(&format!(" '{prefix}"))
        || command.contains(&format!(" \"{prefix}"))
}

#[cfg(unix)]
fn pre_tool_write_event(event_id: &str) -> Value {
    serde_json::json!({
        "event_id": event_id,
        "session_id": "generated_hook_session",
        "tool_name": "Bash",
        "tool_call_id": format!("{event_id}_tool"),
        "command": "touch src/lib.rs",
        "paths": ["src/lib.rs"],
        "timestamp": "2026-07-01T00:00:00Z"
    })
}

#[cfg(unix)]
fn run_shell_hook_command(
    command_text: &str,
    runtime_home: &Path,
    current_dir: &Path,
    event: &Value,
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(command_text)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in envs {
        command.env(name, value);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().expect("hook stdin should be piped");
    stdin.write_all(event.to_string().as_bytes())?;
    drop(stdin);
    Ok(child.wait_with_output()?)
}

#[cfg(unix)]
fn run_executable_hook_command(
    executable: &Path,
    args: Vec<String>,
    runtime_home: &Path,
    current_dir: &Path,
    event: &Value,
    envs: &[(&str, String)],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in envs {
        command.env(name, value);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().expect("hook stdin should be piped");
    stdin.write_all(event.to_string().as_bytes())?;
    drop(stdin);
    Ok(child.wait_with_output()?)
}

#[cfg(unix)]
fn expand_claude_project_command(
    command: &str,
    repo_root: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let relative = command
        .strip_prefix("${CLAUDE_PROJECT_DIR}/")
        .ok_or("Claude Code hook command must start with ${CLAUDE_PROJECT_DIR}/")?;
    Ok(repo_root.join(relative))
}

#[cfg(unix)]
fn assert_host_native_pre_tool_deny_output(output: &Output) -> Result<Value, Box<dyn Error>> {
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stderr(output).is_empty(),
        "host-native hook output should keep stderr empty: {}",
        stderr(output)
    );
    let text = stdout(output);
    assert!(
        text.trim_start().starts_with('{'),
        "host-native hook stdout should be JSON, got {text:?}"
    );
    assert!(
        !text.contains("schema_version") && !text.contains("\"result\""),
        "host-native hook stdout must not contain Volicord wrapper JSON: {text}"
    );
    let value: Value = serde_json::from_str(&text)?;
    let object = value
        .as_object()
        .expect("host-native hook stdout should be a JSON object");
    for key in [
        "schema_version",
        "phase",
        "allowed",
        "guard_event_id",
        "session_id",
        "result",
    ] {
        assert!(
            !object.contains_key(key),
            "host-native hook stdout must not expose Volicord wrapper field `{key}`: {value}"
        );
    }
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .expect("deny reason should be a string")
        .contains("no_active_task"));
    Ok(value)
}

#[cfg(unix)]
fn assert_guard_policy_invokes_required_phases(policy: &Value, connection_id: &str) {
    let commands = policy["host_hook"]["commands"]
        .as_object()
        .expect("host-hook commands should be an object");
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
        "policy should define exactly the required host-hook phase commands"
    );

    for (policy_key, command_name) in phases {
        let command = commands
            .get(policy_key)
            .unwrap_or_else(|| panic!("missing host-hook command for {policy_key}"));
        assert_eq!(command["command"], "volicord");
        let args = command["args"]
            .as_array()
            .expect("host-hook command args should be an array");
        assert_eq!(args.first().and_then(Value::as_str), Some("_hook"));
        assert_eq!(args.get(1).and_then(Value::as_str), Some(command_name));
        assert!(arg_pair(args, "--connection", connection_id));
        let host_output = match policy["host"].as_str() {
            Some("codex") => "codex",
            Some("claude-code") => "claude-code",
            other => panic!("unexpected host-hook policy host: {other:?}"),
        };
        assert!(arg_pair(args, "--host-output", host_output));
    }
}

#[cfg(unix)]
fn arg_pair(args: &[Value], key: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0] == key && pair[1] == value)
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
fn create_real_git_repo(
    runtime_home: &TempRuntimeHome,
    name: impl AsRef<Path>,
) -> Result<PathBuf, Box<dyn Error>> {
    let repo_root = runtime_home.create_product_repo(name)?;
    init_real_git_repo(&repo_root)?;
    Ok(repo_root)
}

#[cfg(unix)]
fn init_real_git_repo(repo_root: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git init failed\nstdout:\n{}\nstderr:\n{}",
            stdout(&output),
            stderr(&output)
        )
        .into());
    }
    Ok(())
}

#[cfg(unix)]
fn path_env(path_dirs: &[&Path]) -> String {
    std::env::join_paths(path_dirs)
        .expect("test PATH should be valid")
        .to_string_lossy()
        .into_owned()
}

#[cfg(unix)]
fn hook_execution_path_env(fake_bin_dir: &Path) -> Result<String, Box<dyn Error>> {
    let volicord_dir = Path::new(volicord_bin())
        .parent()
        .ok_or("volicord test binary path should have a parent")?;
    path_env_with_existing(&[volicord_dir, fake_bin_dir])
}

#[cfg(unix)]
fn path_env_with_existing(path_dirs: &[&Path]) -> Result<String, Box<dyn Error>> {
    let mut paths = path_dirs
        .iter()
        .map(|path| (*path).to_path_buf())
        .collect::<Vec<_>>();
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    Ok(std::env::join_paths(paths)?.to_string_lossy().into_owned())
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
    let workflow_tools = workflow_mcp_tool_names().collect::<Vec<_>>();
    write_fake_mcp_with_workflow_tools(dir, &workflow_tools)
}

#[cfg(unix)]
fn write_fake_mcp_missing_workflow_reconcile(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let workflow_tools = workflow_mcp_tool_names()
        .filter(|tool| *tool != RECONCILE_CHANGES_TOOL_NAME)
        .collect::<Vec<_>>();
    write_fake_mcp_with_workflow_tools(dir, &workflow_tools)
}

#[cfg(unix)]
fn write_fake_mcp_with_workflow_tools(
    dir: &Path,
    workflow_tools: &[&str],
) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("volicord");
    let read_only_tools = read_only_mcp_tool_names().collect::<Vec<_>>();
    let workflow_response = shell_single_quoted(&fake_tools_list_response(workflow_tools));
    let read_only_response = shell_single_quoted(&fake_tools_list_response(&read_only_tools));
    let mut script = "#!/bin/sh\n\
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
         if [ \"$mode\" = \"workflow\" ]; then\n"
        .to_owned();
    script.push_str("         printf '%s\\n' ");
    script.push_str(&workflow_response);
    script.push_str(
        "\n\
         else\n",
    );
    script.push_str("         printf '%s\\n' ");
    script.push_str(&read_only_response);
    script.push_str(
        "\n\
         fi\n\
         exit 0 ;;\n\
         esac\n\
         done\n\
         exit 0\n\
         fi\n\
         printf 'unexpected invocation\\n' >&2\n\
         exit 2\n",
    );
    fs::write(&path, script)?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn workflow_mcp_tool_names() -> impl Iterator<Item = &'static str> {
    WORKFLOW_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

#[cfg(unix)]
fn read_only_mcp_tool_names() -> impl Iterator<Item = &'static str> {
    READ_ONLY_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

#[cfg(unix)]
fn fake_tools_list_response(tool_names: &[&str]) -> String {
    let tools = tool_names
        .iter()
        .map(|name| json!({ "name": name }))
        .collect::<Vec<_>>();
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": { "tools": tools },
    })
    .to_string()
}

#[cfg(unix)]
fn shell_single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
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
fn is_executable(path: &Path) -> Result<bool, Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0)
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
