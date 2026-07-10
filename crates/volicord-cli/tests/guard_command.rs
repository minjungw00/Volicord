#![forbid(unsafe_code)]

mod support;

use std::{error::Error, fs};

use serde_json::{json, Value};
use support::{
    assertions::{assert_success, json_stdout, stderr, stdout},
    guard_fixture::*,
};
use volicord_store::guards::{
    expected_write, guard_event, guard_health_record, guard_installation,
    list_pending_expected_writes, list_unresolved_unrecorded_changes, prompt_capture,
    prompt_capture_availability, unrecorded_change,
};

#[cfg(unix)]
use volicord_types::JudgmentKind;

#[cfg(unix)]
use support::assertions::{assert_close_blocker, assert_no_close_blocker, close_blocker_codes};

#[test]
fn guard_session_start_injects_context_and_records_event() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-session-start")?;
    let event = json!({
        "event_id": "guard_session_start_event",
        "session_id": "guard_session_a",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "inject_context");
    assert_eq!(value["allowed"], true);
    assert_eq!(value["session_id"], "guard_session_a");
    assert_eq!(
        value["result"]["context"]["project_id"],
        fixture.project_id()
    );
    let scan_summary = &value["result"]["context"]["session_watch_scan_summary"];
    assert_eq!(scan_summary["not_full_filesystem_monitoring"], true);
    assert_eq!(scan_summary["follows_symlinks"], false);
    assert!(scan_summary["default_excluded_paths"]
        .as_array()
        .expect("default exclusions should be listed")
        .iter()
        .any(|path| path == ".git"));
    assert!(scan_summary["degraded_reasons"]
        .as_array()
        .expect("degraded reasons should be listed")
        .iter()
        .any(|reason| reason == "skipped_by_policy"));

    let stored = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_session_start_event",
    )?
    .expect("host-hook event should be stored");
    assert_eq!(stored.decision, "inject_context");
    assert_eq!(stored.event_kind, "session_start");
    Ok(())
}

#[test]
fn guard_accepts_only_supported_integration_profiles() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-supported-profiles")?;

    for profile in ["record", "detective"] {
        let event_id = format!("guard_supported_profile_{profile}");
        let event = json!({
            "event_id": event_id,
            "session_id": format!("guard_session_{profile}"),
            "connection_id": fixture.connection_id(),
            "host_kind": "codex"
        });
        let output = run_guard(
            fixture.runtime_home(),
            fixture.repo_root(),
            [
                "_hook",
                "session-start",
                "--repo",
                fixture.repo_arg(),
                "--integration-profile",
                profile,
            ],
            &event,
        )?;
        assert_success(&output);
        let stored = guard_event(fixture.runtime_home(), fixture.project_id(), &event_id)?
            .expect("supported profile should allow host-hook event persistence");
        assert_eq!(stored.event_kind, "session_start");
    }

    let event = json!({
        "event_id": "guard_unsupported_profile",
        "session_id": "guard_session_unsupported_profile",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex"
    });
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "session-start",
            "--repo",
            fixture.repo_arg(),
            "--integration-profile",
            "unsupported",
        ],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("integration profile must be record or detective"));
    assert!(guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_unsupported_profile"
    )?
    .is_none());
    Ok(())
}

#[test]
fn guard_session_start_promotes_matching_installation_active() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-session-activates")?;
    let (guard_installation_id, policy_hash) = fixture.install_guard_policy()?;
    let event = json!({
        "event_id": "guard_session_activate_event",
        "session_id": "guard_session_activate",
        "connection_id": fixture.connection_id(),
        "guard_installation_id": guard_installation_id,
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
        "timestamp": "2026-06-30T04:00:00Z"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);

    let stored = guard_installation(fixture.runtime_home(), &guard_installation_id)?
        .expect("guard installation should be stored");
    assert_eq!(stored.installation_status, "active");
    assert_eq!(
        stored.first_seen_at.as_deref(),
        Some("2026-06-30T04:00:00Z")
    );
    assert_eq!(stored.last_seen_at.as_deref(), Some("2026-06-30T04:00:00Z"));
    assert_eq!(stored.last_seen_phase.as_deref(), Some("session_start"));
    assert_eq!(
        stored.observed_host_kind.as_deref(),
        Some(PROMPT_CAPTURE_TEST_HOST_KIND)
    );
    assert_eq!(
        stored.observed_policy_hash.as_deref(),
        Some(policy_hash.as_str())
    );
    assert_eq!(
        stored.observed_binary_version.as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn guard_session_start_with_stale_policy_hash_does_not_activate_installation(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-session-stale-policy-hash")?;
    let (guard_installation_id, _) = fixture.install_guard_policy()?;
    let event = json!({
        "event_id": "guard_session_stale_policy_hash_event",
        "session_id": "guard_session_stale_policy_hash",
        "connection_id": fixture.connection_id(),
        "guard_installation_id": guard_installation_id,
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
        "timestamp": "2026-06-30T04:00:00Z"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "session-start",
            "--repo",
            fixture.repo_arg(),
            "--policy-hash",
            "sha256:stale",
        ],
        &event,
    )?;
    assert_success(&output);

    let stored = guard_installation(fixture.runtime_home(), &guard_installation_id)?
        .expect("guard installation should be stored");
    assert_eq!(stored.installation_status, "configured");
    assert_eq!(stored.first_seen_at, None);
    assert_eq!(stored.last_seen_at, None);
    assert_eq!(stored.last_seen_phase, None);
    assert_eq!(stored.observed_policy_hash, None);
    Ok(())
}

#[test]
fn guard_pre_tool_denies_product_write_without_active_task() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-no-task")?;
    let event = json!({
        "event_id": "guard_pre_no_task",
        "session_id": "guard_session_pre_no_task",
        "connection_id": fixture.connection_id(),
        "host": {"kind": "claude_code"},
        "tool_name": "Bash",
        "command": "touch src/lib.rs",
        "paths": ["src/lib.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "deny");
    assert_cooperative_disclosure(&value);
    assert_cooperative_disclosure(&value["result"]);
    assert_reason(&value, "no_active_task");

    let stored = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_pre_no_task",
    )?
    .expect("deny event should be stored");
    assert_eq!(stored.decision, "deny");
    Ok(())
}

#[test]
fn guard_pre_tool_allows_read_status_without_active_task() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-read")?;
    let event = json!({
        "event_id": "guard_pre_read",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "git status --short"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(value["allowed"], true);
    assert_eq!(value["result"]["tool"]["classification"], "read_only");
    assert!(value["result"]["reasons"]
        .as_array()
        .expect("reasons should be an array")
        .is_empty());
    assert!(value["result"]["expected_write"].is_null());
    assert!(list_pending_expected_writes(
        fixture.runtime_home(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .is_empty());
    Ok(())
}

#[test]
fn guard_pre_tool_codex_host_output_deny_is_native_json() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-host-codex-deny")?;
    let event = json!({
        "event_id": "guard_host_codex_pre_deny",
        "session_id": "guard_host_codex_pre_deny_session",
        "connection_id": fixture.connection_id(),
        "tool_name": "Bash",
        "command": "touch src/lib.rs",
        "paths": ["src/lib.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "pre-tool",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "codex",
        ],
        &event,
    )?;
    assert_success(&output);
    assert!(stderr(&output).is_empty());
    assert!(!stdout(&output).contains("schema_version"));
    assert!(!stdout(&output).contains("\"result\""));
    let value = json_stdout(&output)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .expect("deny reason should be a string")
        .contains("no_active_task"));

    let stored = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_host_codex_pre_deny",
    )?
    .expect("host-native deny event should be stored");
    assert_eq!(stored.decision, "deny");
    Ok(())
}

#[test]
fn guard_pre_tool_claude_host_output_deny_never_uses_exit_one() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-host-claude-deny")?;
    let event = json!({
        "event_id": "guard_host_claude_pre_deny",
        "session_id": "guard_host_claude_pre_deny_session",
        "connection_id": fixture.connection_id(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "src/lib.rs",
            "content": "changed"
        }
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "pre-tool",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "claude-code",
        ],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(0));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    Ok(())
}

#[test]
fn guard_pre_tool_host_output_allow_has_empty_streams() -> Result<(), Box<dyn Error>> {
    for host_output in ["codex", "claude-code"] {
        let fixture = GuardCliFixture::new(&format!("guard-host-allow-{host_output}"))?;
        let event = json!({
            "event_id": format!("guard_host_pre_allow_{host_output}"),
            "connection_id": fixture.connection_id(),
            "tool_name": "Bash",
            "command": "git status --short"
        });

        let output = run_guard(
            fixture.runtime_home(),
            fixture.repo_root(),
            [
                "_hook",
                "pre-tool",
                "--repo",
                fixture.repo_arg(),
                "--host-output",
                host_output,
            ],
            &event,
        )?;
        assert_success(&output);
        assert!(stdout(&output).is_empty());
        assert!(stderr(&output).is_empty());
    }
    Ok(())
}

#[test]
fn guard_session_start_host_output_injects_context() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-host-session-context")?;
    let event = json!({
        "event_id": "guard_host_session_context",
        "session_id": "guard_host_session_context_session",
        "connection_id": fixture.connection_id()
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "session-start",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "codex",
        ],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "SessionStart");
    assert!(value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("context should be a string")
        .contains("Volicord context"));
    assert!(!stdout(&output).contains("schema_version"));
    Ok(())
}

#[test]
fn guard_codex_native_output_contract_uses_checked_in_hook_events() -> Result<(), Box<dyn Error>> {
    let session = GuardCliFixture::new("guard-codex-native-session")?;
    let event = host_fixture_event(
        &session,
        CODEX_SESSION_START_EVENT,
        "guard_codex_native_session",
        "codex",
    )?;
    let output = run_host_guard(&session, "session-start", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(&value, "SessionStart", "Volicord context");

    let pre_apply = GuardCliFixture::new("guard-codex-native-pre-apply")?;
    let event = host_fixture_event(
        &pre_apply,
        CODEX_PRE_TOOL_WRITE_EVENT,
        "guard_codex_native_pre_apply",
        "codex",
    )?;
    let output = run_host_guard(&pre_apply, "pre-tool", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_pre_tool_deny_output(&value, "no_active_task");

    let pre_bash = GuardCliFixture::new("guard-codex-native-pre-bash")?;
    let event = host_fixture_event(
        &pre_bash,
        CODEX_PRE_TOOL_BASH_WRITE_EVENT,
        "guard_codex_native_pre_bash",
        "codex",
    )?;
    let output = run_host_guard(&pre_bash, "pre-tool", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_pre_tool_deny_output(&value, "no_active_task");

    let post = GuardCliFixture::new("guard-codex-native-post-bash")?;
    post.create_active_task()?;
    let event = host_fixture_event(
        &post,
        CODEX_POST_TOOL_BASH_WRITE_EVENT,
        "guard_codex_native_post_bash",
        "codex",
    )?;
    let output = run_host_guard(&post, "post-tool", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(
        &value,
        "PostToolUse",
        "unresolved Product Repository change",
    );

    let prompt = GuardCliFixture::new("guard-codex-native-prompt")?;
    prompt.install_guard_policy_for_host("codex")?;
    let judgment_id = prompt.create_pending_authority_judgment("codex_native")?;
    let verification_code = prompt.prompt_verification_code(&judgment_id)?;
    let mut event = host_fixture_event(
        &prompt,
        CODEX_USER_PROMPT_JUDGMENT_EVENT,
        "guard_codex_native_prompt",
        "codex",
    )?;
    replace_prompt_verification_code(&mut event, &verification_code);
    let output = run_host_guard(&prompt, "prompt-capture", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(&value, "UserPromptSubmit", "Volicord recorded");
    prompt.assert_recorded_prompt_judgment(&judgment_id, "accepted", "accept")?;

    let stop = GuardCliFixture::new("guard-codex-native-stop")?;
    stop.create_active_task()?;
    let event = host_fixture_event(&stop, CODEX_STOP_EVENT, "guard_codex_native_stop", "codex")?;
    let output = run_host_guard(&stop, "stop", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_block_output(&value, "close_readiness_blocked");
    Ok(())
}

#[test]
fn guard_claude_native_output_contract_uses_checked_in_hook_events() -> Result<(), Box<dyn Error>> {
    let session = GuardCliFixture::new("guard-claude-native-session")?;
    let event = host_fixture_event(
        &session,
        CLAUDE_SESSION_START_EVENT,
        "guard_claude_native_session",
        "claude_code",
    )?;
    let output = run_host_guard(&session, "session-start", "claude-code", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(&value, "SessionStart", "Volicord context");

    let pre_write = GuardCliFixture::new("guard-claude-native-pre-write")?;
    let event = host_fixture_event(
        &pre_write,
        CLAUDE_PRE_TOOL_WRITE_EVENT,
        "guard_claude_native_pre_write",
        "claude_code",
    )?;
    let output = run_host_guard(&pre_write, "pre-tool", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_pre_tool_deny_output(&value, "no_active_task");

    let pre_bash = GuardCliFixture::new("guard-claude-native-pre-bash")?;
    let event = host_fixture_event(
        &pre_bash,
        CLAUDE_PRE_TOOL_BASH_WRITE_EVENT,
        "guard_claude_native_pre_bash",
        "claude_code",
    )?;
    let output = run_host_guard(&pre_bash, "pre-tool", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_pre_tool_deny_output(&value, "no_active_task");

    let post = GuardCliFixture::new("guard-claude-native-post-bash")?;
    post.create_active_task()?;
    let event = host_fixture_event(
        &post,
        CLAUDE_POST_TOOL_BASH_WRITE_EVENT,
        "guard_claude_native_post_bash",
        "claude_code",
    )?;
    let output = run_host_guard(&post, "post-tool", "claude-code", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(
        &value,
        "PostToolUse",
        "unresolved Product Repository change",
    );

    let prompt = GuardCliFixture::new("guard-claude-native-prompt")?;
    prompt.install_guard_policy_for_host("claude_code")?;
    let judgment_id = prompt.create_pending_authority_judgment("claude_native")?;
    let verification_code = prompt.prompt_verification_code(&judgment_id)?;
    let mut event = host_fixture_event(
        &prompt,
        CLAUDE_USER_PROMPT_JUDGMENT_EVENT,
        "guard_claude_native_prompt",
        "claude_code",
    )?;
    replace_prompt_verification_code(&mut event, &verification_code);
    let output = run_host_guard(&prompt, "prompt-capture", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(&value, "UserPromptSubmit", "Volicord recorded");
    prompt.assert_recorded_prompt_judgment(&judgment_id, "accepted", "accept")?;

    let prompt_block = GuardCliFixture::new("guard-claude-native-prompt-block")?;
    prompt_block.install_guard_policy_for_host("claude_code")?;
    prompt_block.create_pending_authority_judgment("claude_native_block")?;
    let event = host_fixture_event(
        &prompt_block,
        CLAUDE_USER_PROMPT_JUDGMENT_EVENT,
        "guard_claude_native_prompt_block",
        "claude_code",
    )?;
    let output = run_host_guard(&prompt_block, "prompt-capture", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_block_output(&value, "malformed_judgment_command");

    let stop = GuardCliFixture::new("guard-claude-native-stop")?;
    stop.create_active_task()?;
    let event = host_fixture_event(
        &stop,
        CLAUDE_STOP_EVENT,
        "guard_claude_native_stop",
        "claude_code",
    )?;
    let output = run_host_guard(&stop, "stop", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_block_output(&value, "close_readiness_blocked");
    Ok(())
}

#[test]
fn guard_volicord_json_deny_is_not_host_native_output() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-volicord-json-not-host-native")?;
    let event = host_fixture_event(
        &fixture,
        CODEX_PRE_TOOL_WRITE_EVENT,
        "guard_volicord_json_not_host_native",
        "codex",
    )?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "deny");
    assert!(value.get("result").is_some());
    assert!(!is_host_native_pre_tool_deny(&value));
    Ok(())
}

#[test]
fn guard_host_native_debug_logging_does_not_mix_text_into_stdout() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-host-native-debug-stdout")?;
    let event = host_fixture_event(
        &fixture,
        CLAUDE_PRE_TOOL_WRITE_EVENT,
        "guard_host_native_debug_stdout",
        "claude_code",
    )?;

    let output = run_host_guard(
        &fixture,
        "pre-tool",
        "claude-code",
        &event,
        &[("RUST_LOG", "debug"), ("VOLICORD_LOG", "debug")],
    )?;
    let text = stdout(&output);
    assert!(text.trim_start().starts_with('{'));
    assert!(!text.contains("DEBUG"));
    assert!(!text.contains("debug:"));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_pre_tool_deny_output(&value, "no_active_task");
    Ok(())
}

#[test]
fn guard_unsupported_host_output_mode_fails_clearly() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-unsupported-host-output")?;

    let output = run_guard_file(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "pre-tool",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "unsupported",
        ],
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("--host-output must be codex or claude-code"));
    Ok(())
}

#[test]
fn guard_host_output_non_policy_error_uses_stderr_not_stdout() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-host-non-policy-error")?;
    let event = json!({
        "event_id": "guard_host_non_policy_error",
        "tool_name": "Bash",
        "command": "git status --short"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "pre-tool",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "claude-code",
        ],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("requires --connection"));
    Ok(())
}

#[test]
fn guard_pre_tool_rejects_paths_outside_project_allowlist() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-outside-project")?;
    let event = json!({
        "event_id": "guard_pre_outside_project",
        "session_id": "guard_session_pre_outside_project",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "read",
        "paths": ["../outside-product-repo.txt"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "deny");
    assert_reason(&value, "target_outside_project_allowlist");

    let stored = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_pre_outside_project",
    )?
    .expect("outside-project host-hook event should be stored");
    assert_eq!(stored.decision, "deny");
    assert_eq!(stored.event_kind, "pre_tool");
    Ok(())
}

#[test]
fn guard_pre_tool_requires_current_write_ticket() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-write-ready")?;
    let task_id = fixture.create_active_task()?;
    let denied_event = json!({
        "event_id": "guard_pre_missing_write_ticket",
        "session_id": "guard_session_write_ready",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"]
    });

    let denied = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &denied_event,
    )?;
    assert_eq!(denied.status.code(), Some(1));
    let denied_value = json_stdout(&denied)?;
    assert_reason(&denied_value, "write_ticket_missing");
    assert_eq!(
        denied_value["result"]["write_ticket_backing"]["status"],
        "missing_ticket"
    );
    assert!(denied_value["result"]["write_ticket_backing"]["disclosure"]
        .as_str()
        .expect("write-ticket disclosure should be present")
        .contains("not OS-level enforcement"));

    fixture.prepare_write(&task_id)?;
    let allowed_event = json!({
        "event_id": "guard_pre_with_write_ticket",
        "session_id": "guard_session_write_ready",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"]
    });
    let allowed = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &allowed_event,
    )?;
    assert_success(&allowed);
    let value = json_stdout(&allowed)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(value["allowed"], true);
    assert_eq!(value["result"]["tool"]["classification"], "mutating");
    assert_eq!(
        value["result"]["write_ticket_backing"]["status"],
        "ticket_backed"
    );
    assert_eq!(
        value["result"]["write_ticket_backing"]["ticket_backed"],
        true
    );
    assert_eq!(value["result"]["expected_write"]["ticket_backed"], true);
    assert!(value["result"]["expected_write"]["expected_write_id"].is_string());

    let out_of_scope_event = json!({
        "event_id": "guard_pre_write_ticket_out_of_scope",
        "session_id": "guard_session_write_ready",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "touch src/other.rs",
        "paths": ["src/other.rs"]
    });
    let out_of_scope = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &out_of_scope_event,
    )?;
    assert_eq!(out_of_scope.status.code(), Some(1));
    let out_of_scope_value = json_stdout(&out_of_scope)?;
    assert_reason(&out_of_scope_value, "write_ticket_path_scope_violation");
    assert_eq!(
        out_of_scope_value["result"]["write_ticket_backing"]["status"],
        "out_of_scope"
    );
    Ok(())
}

#[test]
fn guard_post_tool_records_unrecorded_product_file_changes() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-post-unrecorded")?;
    let task_id = fixture.create_active_task()?;
    let event = json!({
        "event_id": "guard_post_changed",
        "session_id": "guard_session_post",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "warn");
    assert_eq!(
        value["result"]["unrecorded_changes"][0]["observed_paths"][0],
        "src/export.rs"
    );
    let unresolved = list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?;
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].task_id.as_deref(), Some(task_id.as_str()));
    let stored = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_post_changed",
    )?
    .expect("post-tool host-hook event should be stored");
    assert_eq!(stored.decision, "warn");
    assert_eq!(stored.event_kind, "post_tool");
    Ok(())
}

#[test]
fn guard_post_tool_links_active_write_ticket_without_expected_write() -> Result<(), Box<dyn Error>>
{
    let fixture = GuardCliFixture::new("guard-post-ticket-backed")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let event = json!({
        "event_id": "guard_post_ticket_backed",
        "session_id": "guard_session_ticket_backed",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(
        value["result"]["ticket_backed_observations"][0]["status"],
        "ticket_backed"
    );
    assert_eq!(
        value["result"]["ticket_backed_observations"][0]["observed_paths"],
        json!(["src/export.rs"])
    );
    assert!(value["result"]["matched_expected_writes"]
        .as_array()
        .expect("matched expected writes should be an array")
        .is_empty());
    assert!(value["result"]["unrecorded_changes"]
        .as_array()
        .expect("unrecorded changes should be an array")
        .is_empty());
    assert!(list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?
    .is_empty());
    Ok(())
}

#[test]
fn guard_post_tool_host_output_warns_with_context() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-host-post-context")?;
    fixture.create_active_task()?;
    let event = json!({
        "event_id": "guard_host_post_context",
        "session_id": "guard_host_post_context_session",
        "connection_id": fixture.connection_id(),
        "host_kind": "claude_code",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "src/export.rs",
            "content": "changed"
        },
        "tool_response": {
            "filePath": "src/export.rs",
            "success": true
        },
        "changed_paths": ["src/export.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "post-tool",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "claude-code",
        ],
        &event,
    )?;
    assert_success(&output);
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    assert!(value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("context should be a string")
        .contains("unresolved Product Repository change"));
    assert!(!stdout(&output).contains("\"result\""));
    Ok(())
}

#[test]
fn guard_post_tool_matches_expected_allowed_write() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-post-expected")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let pre = json!({
        "event_id": "guard_pre_expected",
        "session_id": "guard_session_expected",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_call_expected",
        "tool_input": {
            "command": "touch src/export.rs"
        },
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:00:00Z"
    });

    let pre_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &pre,
    )?;
    assert_success(&pre_output);
    let pre_value = json_stdout(&pre_output)?;
    assert_eq!(pre_value["decision"], "allow");
    let expected_id = pre_value["result"]["expected_write"]["expected_write_id"]
        .as_str()
        .expect("expected write id should be present")
        .to_owned();
    assert_eq!(
        list_pending_expected_writes(
            fixture.runtime_home(),
            fixture.project_id(),
            fixture.connection_id(),
        )?
        .len(),
        1
    );

    let post = json!({
        "event_id": "guard_post_expected",
        "session_id": "guard_session_expected",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_call_expected",
        "tool_input": {
            "command": "touch src/export.rs"
        },
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:01:00Z"
    });
    let post_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &post,
    )?;
    assert_success(&post_output);
    let post_value = json_stdout(&post_output)?;
    assert_eq!(post_value["decision"], "allow");
    assert_eq!(
        post_value["result"]["matched_expected_writes"][0]["expected_write_id"],
        expected_id
    );
    assert_eq!(
        post_value["result"]["matched_expected_writes"][0]["ticket_backed"],
        true
    );
    assert!(post_value["result"]["unrecorded_changes"]
        .as_array()
        .expect("unrecorded changes should be an array")
        .is_empty());
    assert!(list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?
    .is_empty());
    let stored_expected =
        expected_write(fixture.runtime_home(), fixture.project_id(), &expected_id)?
            .expect("expected write should be stored");
    assert_eq!(stored_expected.status, "matched");
    assert_eq!(stored_expected.tool_name.as_deref(), Some("Bash"));
    assert_eq!(
        stored_expected.matched_post_tool_guard_event_id.as_deref(),
        Some("guard_post_expected")
    );
    Ok(())
}

#[test]
fn guard_post_tool_records_out_of_scope_expected_write() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-post-out-of-scope")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let pre = json!({
        "event_id": "guard_pre_scope_expected",
        "session_id": "guard_session_scope_expected",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_call_scope_expected",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:10:00Z"
    });
    assert_success(&run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &pre,
    )?);

    let post = json!({
        "event_id": "guard_post_scope_changed",
        "session_id": "guard_session_scope_expected",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_call_scope_expected",
        "command": "touch src/other.rs",
        "success": true,
        "changed_paths": ["src/other.rs"],
        "timestamp": "2026-06-30T05:11:00Z"
    });
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &post,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "warn");
    assert_eq!(
        value["result"]["unrecorded_changes"][0]["observed_paths"][0],
        "src/other.rs"
    );
    let change_id = value["result"]["unrecorded_changes"][0]["unrecorded_change_id"]
        .as_str()
        .expect("unrecorded change id should be present");
    let change = unrecorded_change(fixture.runtime_home(), fixture.project_id(), change_id)?
        .expect("unrecorded change should be stored");
    let detection: Value = serde_json::from_str(&change.detection_json)?;
    assert_eq!(
        detection["correlation_status"],
        "out_of_scope_expected_write"
    );
    assert_eq!(detection["ticket_scope_violation"], true);
    assert_eq!(detection["does_not_prevent_writes"], true);
    assert_eq!(detection["does_not_identify_actor"], true);
    assert_eq!(
        list_pending_expected_writes(
            fixture.runtime_home(),
            fixture.project_id(),
            fixture.connection_id(),
        )?
        .len(),
        1
    );
    Ok(())
}

#[test]
fn guard_pre_tool_ambiguous_shell_does_not_create_expected_write() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-ambiguous-shell")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let pre = json!({
        "event_id": "guard_pre_ambiguous_shell",
        "session_id": "guard_session_ambiguous_shell",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "python scripts/rewrite.py src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:20:00Z"
    });
    let pre_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &pre,
    )?;
    assert_success(&pre_output);
    let pre_value = json_stdout(&pre_output)?;
    assert_eq!(pre_value["decision"], "warn");
    assert_reason(&pre_value, "unknown_mutation_risk");
    assert!(pre_value["result"]["expected_write"].is_null());
    assert!(list_pending_expected_writes(
        fixture.runtime_home(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .is_empty());

    let denied_pre = json!({
        "event_id": "guard_pre_ambiguous_shell_policy_deny",
        "session_id": "guard_session_ambiguous_shell",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "python scripts/rewrite.py src/export.rs",
        "paths": ["src/export.rs"],
        "guard_policy": {
            "unknown_mutation_decision": "deny"
        },
        "timestamp": "2026-06-30T05:20:30Z"
    });
    let denied_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &denied_pre,
    )?;
    assert_eq!(denied_output.status.code(), Some(1));
    let denied_value = json_stdout(&denied_output)?;
    assert_eq!(denied_value["decision"], "deny");
    assert_reason(&denied_value, "unknown_mutation_risk");
    assert!(denied_value["result"]["expected_write"].is_null());
    assert!(list_pending_expected_writes(
        fixture.runtime_home(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .is_empty());

    let post = json!({
        "event_id": "guard_post_ambiguous_shell",
        "session_id": "guard_session_ambiguous_shell",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "python scripts/rewrite.py src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:21:00Z"
    });
    let post_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &post,
    )?;
    assert_success(&post_output);
    let post_value = json_stdout(&post_output)?;
    assert_eq!(post_value["decision"], "allow");
    assert_eq!(
        post_value["result"]["ticket_backed_observations"][0]["status"],
        "ticket_backed"
    );
    assert!(list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?
    .is_empty());
    Ok(())
}

#[test]
fn guard_expected_write_does_not_leak_between_sessions() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-session-isolation")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let pre = json!({
        "event_id": "guard_pre_session_a",
        "session_id": "guard_session_a",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "shared_tool_call",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:30:00Z"
    });
    assert_success(&run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &pre,
    )?);

    let post_other_session = json!({
        "event_id": "guard_post_session_b",
        "session_id": "guard_session_b",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "shared_tool_call",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:31:00Z"
    });
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &post_other_session,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(
        value["result"]["ticket_backed_observations"][0]["status"],
        "ticket_backed"
    );
    assert!(value["result"]["matched_expected_writes"]
        .as_array()
        .expect("matched expected writes should be an array")
        .is_empty());
    assert_eq!(
        list_pending_expected_writes(
            fixture.runtime_home(),
            fixture.project_id(),
            fixture.connection_id(),
        )?
        .len(),
        1
    );
    assert!(list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?
    .is_empty());
    Ok(())
}

#[test]
fn guard_expected_write_does_not_leak_between_projects() -> Result<(), Box<dyn Error>> {
    let first = GuardCliFixture::new("guard-project-isolation-a")?;
    let task_id = first.create_active_task()?;
    first.prepare_write(&task_id)?;
    let pre = json!({
        "event_id": "guard_pre_project_a",
        "session_id": "guard_session_project",
        "connection_id": first.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_call_project",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:40:00Z"
    });
    assert_success(&run_guard(
        first.runtime_home(),
        first.repo_root(),
        ["_hook", "pre-tool", "--repo", first.repo_arg()],
        &pre,
    )?);

    let second = GuardCliFixture::new("guard-project-isolation-b")?;
    let task_id = second.create_active_task()?;
    let post = json!({
        "event_id": "guard_post_project_b",
        "session_id": "guard_session_project",
        "connection_id": second.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_call_project",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:41:00Z"
    });
    let output = run_guard(
        second.runtime_home(),
        second.repo_root(),
        ["_hook", "post-tool", "--repo", second.repo_arg()],
        &post,
    )?;
    assert_success(&output);
    assert_eq!(json_stdout(&output)?["decision"], "warn");
    let unresolved = list_unresolved_unrecorded_changes(
        second.runtime_home(),
        second.project_id(),
        Some(second.connection_id()),
    )?;
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].task_id.as_deref(), Some(task_id.as_str()));
    Ok(())
}

#[test]
fn guard_prompt_capture_hashes_prompt_and_omits_text() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-prompt-capture")?;
    let event_file = fixture.repo_root().join("prompt-event.json");
    fs::write(
        &event_file,
        json!({
            "event_id": "guard_prompt_event",
            "prompt_capture_id": "guard_prompt_capture_a",
            "session_id": "guard_session_prompt",
            "connection_id": fixture.connection_id(),
            "host": {"kind": PROMPT_CAPTURE_TEST_HOST_KIND},
            "message": "Please prepare the write carefully."
        })
        .to_string(),
    )?;

    let output = run_guard_file(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "prompt-capture",
            "--repo",
            fixture.repo_arg(),
            "--file",
            event_file.to_str().expect("test path should be UTF-8"),
        ],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(
        value["result"]["prompt_capture"]["prompt_capture_id"],
        "guard_prompt_capture_a"
    );
    assert_eq!(
        value["result"]["prompt_capture"]["prompt_text_omitted"],
        true
    );

    let stored = prompt_capture(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_prompt_capture_a",
    )?
    .expect("prompt capture should be stored");
    assert!(stored.prompt_text.is_none());
    assert!(stored.prompt_sha256.starts_with("sha256:"));
    Ok(())
}

#[test]
fn guard_session_start_shows_chat_judgment_instructions() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-instructions")?;
    fixture.create_pending_authority_judgment("instructions")?;
    let event = json!({
        "event_id": "guard_session_chat_instructions",
        "session_id": "guard_session_chat_instructions",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(
        value["result"]["context"]["prompt_capture_status"],
        "configured"
    );
    assert_eq!(value["result"]["context"]["prompt_capture_enabled"], true);
    let pending = &value["result"]["context"]["pending_user_judgments"][0];
    assert_eq!(pending["chat_id"], "J-1");
    let verification_code = pending["verification_code"]
        .as_str()
        .expect("verification code should be present");
    assert!(verification_code.starts_with('#'));
    assert_eq!(
        pending["answer_instruction"],
        format!("Volicord: answer J-1 1 {verification_code}")
    );
    assert_eq!(
        pending["note_instruction"],
        format!("Volicord: note J-1 \"text\" {verification_code}")
    );
    assert_eq!(
        pending["options"][1]["instruction"],
        format!("Volicord: answer J-1 reject {verification_code}")
    );
    assert_eq!(
        pending["options"][2]["instruction"],
        format!("Volicord: answer J-1 defer {verification_code}")
    );
    Ok(())
}

#[test]
fn guard_session_start_hides_chat_judgment_instructions_without_prompt_capture(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-chat-instructions-not-configured")?;
    fixture.install_guard_policy_with(true, false, "configured")?;
    fixture.create_pending_authority_judgment("instructions_not_configured")?;
    let event = json!({
        "event_id": "guard_session_chat_instructions_not_configured",
        "session_id": "guard_session_chat_instructions_not_configured",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(
        value["result"]["context"]["prompt_capture_status"],
        "not_configured"
    );
    assert_eq!(value["result"]["context"]["prompt_capture_enabled"], false);
    assert_eq!(value["result"]["context"]["pending_user_judgment_count"], 1);
    assert_eq!(
        value["result"]["context"]["pending_user_judgments"]
            .as_array()
            .expect("pending judgments should be an array")
            .len(),
        0
    );
    Ok(())
}

#[test]
fn guard_session_start_omits_stale_chat_judgment_instructions() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-instructions-stale")?;
    let judgment_id = fixture.create_pending_authority_judgment("instructions_stale")?;
    fixture.set_judgment_basis_status(&judgment_id, "stale")?;
    let event = json!({
        "event_id": "guard_session_chat_instructions_stale",
        "session_id": "guard_session_chat_instructions_stale",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["result"]["context"]["pending_user_judgment_count"], 1);
    assert_eq!(
        value["result"]["context"]["pending_user_judgments"]
            .as_array()
            .expect("pending judgments should be an array")
            .len(),
        0
    );
    Ok(())
}

#[test]
fn guard_session_start_omits_expired_chat_judgment_instructions() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-instructions-expired")?;
    let judgment_id = fixture.create_pending_authority_judgment("instructions_expired")?;
    fixture.set_judgment_expires_at(&judgment_id, "2000-01-01T00:00:00Z")?;
    let event = json!({
        "event_id": "guard_session_chat_instructions_expired",
        "session_id": "guard_session_chat_instructions_expired",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["result"]["context"]["pending_user_judgment_count"], 1);
    assert_eq!(
        value["result"]["context"]["pending_user_judgments"]
            .as_array()
            .expect("pending judgments should be an array")
            .len(),
        0
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_records_answer_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-answer")?;
    let judgment_id = fixture.create_pending_authority_judgment("answer")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let mut event = prompt_event(
        &fixture,
        "guard_prompt_answer",
        "guard_prompt_capture_answer",
        &message,
    );
    event["guard_installation_id"] = json!(fixture.guard_installation_id());

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "inject_context");
    assert_eq!(
        value["result"]["recognized_judgment_command"]["selected_option_id"],
        "accept"
    );
    assert_eq!(
        value["result"]["recognized_judgment_command"]["verification_code"],
        verification_code
    );
    assert_eq!(
        value["result"]["recognized_judgment_command"]["resolution_outcome"],
        "accepted"
    );
    assert!(value["result"]["model_context"]
        .as_str()
        .expect("model context should be present")
        .contains("Volicord recorded the user-owned judgment"));
    let health = guard_health_record(
        fixture.runtime_home(),
        fixture.project_id(),
        fixture.connection_id(),
    )?;
    assert_eq!(
        prompt_capture_availability(&health)?.status.as_str(),
        "active"
    );
    fixture.assert_recorded_prompt_judgment(&judgment_id, "accepted", "accept")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_host_output_injects_recorded_context() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-host-prompt-context")?;
    let judgment_id = fixture.create_pending_authority_judgment("host_context")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_host_prompt_context",
        "guard_host_prompt_capture_context",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "prompt-capture",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "codex",
        ],
        &event,
    )?;
    assert_success(&output);
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output)?;
    assert_eq!(
        value["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    assert!(value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("context should be a string")
        .contains("Volicord recorded"));
    assert!(!stdout(&output).contains("schema_version"));
    fixture.assert_recorded_prompt_judgment(&judgment_id, "accepted", "accept")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_records_reject_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-reject")?;
    let judgment_id = fixture.create_pending_authority_judgment("reject")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 reject {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_reject",
        "guard_prompt_capture_reject",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    fixture.assert_recorded_prompt_judgment(&judgment_id, "rejected", "reject")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_records_defer_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-defer")?;
    let judgment_id = fixture.create_pending_authority_judgment("defer")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 defer {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_defer",
        "guard_prompt_capture_defer",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    fixture.assert_recorded_prompt_judgment(&judgment_id, "deferred", "defer")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_records_note_as_deferred_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-note")?;
    let judgment_id = fixture.create_pending_authority_judgment("note")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: note J-1 \"Need to review this later\" {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_note",
        "guard_prompt_capture_note",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    fixture.assert_recorded_prompt_judgment(&judgment_id, "deferred", "defer")?;
    let resolution = fixture.judgment_resolution(&judgment_id)?;
    assert_eq!(resolution["note"], "Need to review this later");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_unsupported_host_without_recording() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-chat-unsupported-host")?;
    fixture.install_guard_policy_with(false, true, "configured")?;
    let judgment_id = fixture.create_pending_authority_judgment("unsupported_host")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let capture_id = "guard_prompt_capture_unsupported";
    let event = prompt_event(&fixture, "guard_prompt_unsupported", capture_id, &message);

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "prompt_capture_unsupported");
    assert_eq!(value["result"]["prompt_capture"]["captured"], false);
    assert_eq!(
        value["result"]["prompt_capture"]["prompt_capture_status"],
        "unsupported_by_host"
    );
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    assert!(prompt_capture(fixture.runtime_home(), fixture.project_id(), capture_id)?.is_none());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_not_configured_without_recording() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-chat-not-configured")?;
    fixture.install_guard_policy_with(true, false, "configured")?;
    let judgment_id = fixture.create_pending_authority_judgment("not_configured")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let capture_id = "guard_prompt_capture_not_configured";
    let event = prompt_event(
        &fixture,
        "guard_prompt_not_configured",
        capture_id,
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "prompt_capture_not_configured");
    assert_eq!(value["result"]["prompt_capture"]["captured"], false);
    assert_eq!(
        value["result"]["prompt_capture"]["prompt_capture_status"],
        "not_configured"
    );
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    assert!(prompt_capture(fixture.runtime_home(), fixture.project_id(), capture_id)?.is_none());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_policy_mismatch_without_recording() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-policy-mismatch")?;
    let judgment_id = fixture.create_pending_authority_judgment("policy_mismatch")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    fs::write(
        fixture.repo_root().join(".volicord").join("policy.json"),
        json!({
            "schema": "volicord-policy-v1",
            "managed_by": "volicord",
            "host": PROMPT_CAPTURE_TEST_HOST_KIND,
            "selected_profile": "detective",
            "connection_id": fixture.connection_id(),
            "guard_installation_id": "guard_installation_cli_activation",
            "changed": true
        })
        .to_string(),
    )?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let capture_id = "guard_prompt_capture_policy_mismatch";
    let event = prompt_event(
        &fixture,
        "guard_prompt_policy_mismatch",
        capture_id,
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "prompt_capture_reload_required");
    assert_eq!(value["result"]["prompt_capture"]["captured"], false);
    assert_eq!(
        value["result"]["prompt_capture"]["prompt_capture_status"],
        "reload_required"
    );
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    assert!(prompt_capture(fixture.runtime_home(), fixture.project_id(), capture_id)?.is_none());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_malformed_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-malformed")?;
    let judgment_id = fixture.create_pending_authority_judgment("malformed")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: note J-1 not-quoted {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_malformed",
        "guard_prompt_capture_malformed",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "malformed_judgment_command");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_host_output_blocks_malformed_prompt() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-host-prompt-block")?;
    let judgment_id = fixture.create_pending_authority_judgment("host_block")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: note J-1 not-quoted {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_host_prompt_block",
        "guard_host_prompt_capture_block",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "prompt-capture",
            "--repo",
            fixture.repo_arg(),
            "--host-output",
            "claude-code",
        ],
        &event,
    )?;
    assert_success(&output);
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "block");
    assert!(value["reason"]
        .as_str()
        .expect("block reason should be a string")
        .contains("malformed_judgment_command"));
    assert!(!stdout(&output).contains("schema_version"));
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_missing_verification_code() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-missing-code")?;
    let judgment_id = fixture.create_pending_authority_judgment("missing_code")?;
    let event = prompt_event(
        &fixture,
        "guard_prompt_missing_code",
        "guard_prompt_capture_missing_code",
        "Volicord: answer J-1 1",
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "malformed_judgment_command");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_wrong_verification_code() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-wrong-code")?;
    let judgment_id = fixture.create_pending_authority_judgment("wrong_code")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let wrong_code = if verification_code == "#AAAAAA" {
        "#BBBBBB"
    } else {
        "#AAAAAA"
    };
    let message = format!("Volicord: answer J-1 1 {wrong_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_wrong_code",
        "guard_prompt_capture_wrong_code",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "wrong_verification_code");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_ignores_non_command_prompt() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-non-command")?;
    let event = prompt_event(
        &fixture,
        "guard_prompt_non_command",
        "guard_prompt_capture_non_command",
        "Please explain what Volicord should do next.",
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "allow");
    assert!(value["result"]["recognized_judgment_command"].is_null());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_invalid_chat_id() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-invalid-id")?;
    let judgment_id = fixture.create_pending_authority_judgment("invalid_id")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-99 1 {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_invalid_id",
        "guard_prompt_capture_invalid_id",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "unknown_judgment_id");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_mismatched_project() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-project-mismatch")?;
    let judgment_id = fixture.create_pending_authority_judgment("project_mismatch")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let mut event = prompt_event(
        &fixture,
        "guard_prompt_project_mismatch",
        "guard_prompt_capture_project_mismatch",
        &message,
    );
    event["project_id"] = json!("other_project");

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "project_mismatch");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_mismatched_connection() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-connection-mismatch")?;
    let judgment_id = fixture.create_pending_authority_judgment("connection_mismatch")?;
    fixture.register_extra_connection("other_connection")?;
    fixture.install_guard_policy_for_connection("other_connection", true, true, "configured")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let mut event = prompt_event(
        &fixture,
        "guard_prompt_connection_mismatch",
        "guard_prompt_capture_connection_mismatch",
        &message,
    );
    event["connection_id"] = json!("other_connection");

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "connection_mismatch");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_stale_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-stale")?;
    let judgment_id = fixture.create_pending_authority_judgment("stale")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    fixture.set_judgment_basis_status(&judgment_id, "stale")?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_stale",
        "guard_prompt_capture_stale",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "stale_judgment");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_replays_duplicate_same_answer() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-duplicate")?;
    let judgment_id = fixture.create_pending_authority_judgment("duplicate")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let first = prompt_event(
        &fixture,
        "guard_prompt_duplicate_first",
        "guard_prompt_capture_duplicate_first",
        &message,
    );
    let second = prompt_event(
        &fixture,
        "guard_prompt_duplicate_second",
        "guard_prompt_capture_duplicate_second",
        &message,
    );

    assert_success(&run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &first,
    )?);
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &second,
    )?;
    assert_success(&output);
    assert_eq!(json_stdout(&output)?["decision"], "inject_context");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "resolved");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_conflicting_duplicate_answer() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-conflicting-duplicate")?;
    let judgment_id = fixture.create_pending_authority_judgment("conflicting_duplicate")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let first_message = format!("Volicord: answer J-1 1 {verification_code}");
    let second_message = format!("Volicord: answer J-1 reject {verification_code}");
    let first = prompt_event(
        &fixture,
        "guard_prompt_conflicting_duplicate_first",
        "guard_prompt_capture_conflicting_duplicate_first",
        &first_message,
    );
    let second = prompt_event(
        &fixture,
        "guard_prompt_conflicting_duplicate_second",
        "guard_prompt_capture_conflicting_duplicate_second",
        &second_message,
    );

    assert_success(&run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &first,
    )?);
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &second,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "conflicting_judgment_command");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "resolved");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_expired_verification_code() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-expired-code")?;
    let judgment_id = fixture.create_pending_authority_judgment("expired_code")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    fixture.set_judgment_expires_at(&judgment_id, "2000-01-01T00:00:00Z")?;
    let message = format!("Volicord: answer J-1 1 {verification_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_expired_code",
        "guard_prompt_capture_expired_code",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "expired_verification_code");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_multiple_commands() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-ambiguous")?;
    let judgment_id = fixture.create_pending_authority_judgment("ambiguous")?;
    let verification_code = fixture.prompt_verification_code(&judgment_id)?;
    let message = format!(
        "Volicord: answer J-1 1 {verification_code}\nVolicord: answer J-1 reject {verification_code}"
    );
    let event = prompt_event(
        &fixture,
        "guard_prompt_ambiguous",
        "guard_prompt_capture_ambiguous",
        &message,
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "ambiguous_judgment_command");
    assert_eq!(fixture.judgment_status(&judgment_id)?, "pending");
    Ok(())
}

#[test]
fn guard_stop_denies_false_completion_when_close_readiness_blocks() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-stop-blocked")?;
    fixture.create_active_task()?;
    let event = json!({
        "event_id": "guard_stop_blocked",
        "session_id": "guard_session_stop",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "message": "All done."
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "stop", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "deny");
    assert_reason(&value, "close_readiness_blocked");
    assert!(value["result"]["close_status"]["close_blockers"]
        .as_array()
        .expect("close blockers should be an array")
        .iter()
        .any(|blocker| blocker["code"] == "missing_current_close_basis"));
    Ok(())
}

#[test]
fn guard_stop_host_output_blocks_and_allows_continue() -> Result<(), Box<dyn Error>> {
    let blocked = GuardCliFixture::new("guard-host-stop-block")?;
    blocked.create_active_task()?;
    let blocked_event = json!({
        "event_id": "guard_host_stop_block",
        "session_id": "guard_host_stop_block_session",
        "connection_id": blocked.connection_id(),
        "host_kind": "codex",
        "message": "All done."
    });
    let blocked_output = run_guard(
        blocked.runtime_home(),
        blocked.repo_root(),
        [
            "_hook",
            "stop",
            "--repo",
            blocked.repo_arg(),
            "--host-output",
            "codex",
        ],
        &blocked_event,
    )?;
    assert_success(&blocked_output);
    let blocked_value = json_stdout(&blocked_output)?;
    assert_eq!(blocked_value["decision"], "block");
    assert!(blocked_value["reason"]
        .as_str()
        .expect("stop block reason should be a string")
        .contains("close_readiness_blocked"));

    let allowed = GuardCliFixture::new("guard-host-stop-allow")?;
    let allowed_event = json!({
        "event_id": "guard_host_stop_allow",
        "session_id": "guard_host_stop_allow_session",
        "connection_id": allowed.connection_id(),
        "host_kind": "claude_code",
        "message": "Nothing active."
    });
    let allowed_output = run_guard(
        allowed.runtime_home(),
        allowed.repo_root(),
        [
            "_hook",
            "stop",
            "--repo",
            allowed.repo_arg(),
            "--host-output",
            "claude-code",
        ],
        &allowed_event,
    )?;
    assert_success(&allowed_output);
    assert_eq!(json_stdout(&allowed_output)?["continue"], true);
    assert!(stderr(&allowed_output).is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn guarded_init_hook_write_prompt_lifecycle_closes() -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-close", "detective")?;
    assert_guard_init_state_is_installed_or_degraded(&fixture.init_output);
    fixture.mark_required_hooks_supported()?;
    fixture.activate_guard("guard_lifecycle_session_start")?;

    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("happy")?;
    let write_ticket_id = fixture.prepare_write(&task_id, &change_unit_id, "happy")?;

    let pre = json!({
        "event_id": "guard_lifecycle_pre",
        "session_id": fixture.session_id(),
        "connection_id": fixture.connection_id(),
        "guard_installation_id": fixture.guard_installation_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_lifecycle_write",
        "command": "touch src/export.rs",
        "paths": [DEFAULT_PRODUCT_PATH],
        "timestamp": "2026-06-30T06:01:00Z"
    });
    let pre_output = fixture.run_guard_event("pre-tool", &pre)?;
    assert_success(&pre_output);
    let pre_value = json_stdout(&pre_output)?;
    assert_eq!(pre_value["decision"], "allow");
    assert!(pre_value["result"]["expected_write"]["expected_write_id"].is_string());

    fixture.apply_product_change("happy path guarded write")?;
    let post = json!({
        "event_id": "guard_lifecycle_post",
        "session_id": fixture.session_id(),
        "connection_id": fixture.connection_id(),
        "guard_installation_id": fixture.guard_installation_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_lifecycle_write",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": [DEFAULT_PRODUCT_PATH],
        "timestamp": "2026-06-30T06:02:00Z"
    });
    let post_output = fixture.run_guard_event("post-tool", &post)?;
    assert_success(&post_output);
    let post_value = json_stdout(&post_output)?;
    assert_eq!(post_value["decision"], "allow");
    assert!(post_value["result"]["unrecorded_changes"]
        .as_array()
        .expect("unrecorded changes should be an array")
        .is_empty());
    assert!(list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?
    .is_empty());

    fixture.record_product_write_close_basis(
        &task_id,
        &change_unit_id,
        &write_ticket_id,
        "happy",
    )?;
    let final_judgment_id = fixture.request_final_acceptance(&task_id, &change_unit_id, "happy")?;
    fixture.answer_pending_judgment_through_prompt(
        &task_id,
        &final_judgment_id,
        "guard_lifecycle_final_prompt",
        "guard_lifecycle_final_capture",
    )?;

    let check = fixture.check_close(&task_id)?;
    assert_eq!(
        check.response_value["close_state"], "ready",
        "{}",
        check.response_value
    );
    assert!(
        close_blocker_codes(&check.response_value).is_empty(),
        "expected ready close, got blockers {:?}",
        check.response_value["blockers"]
    );

    let close = fixture.close_task(&task_id, "happy")?;
    assert_eq!(close.response_value["close_state"], "closed");
    assert_eq!(
        close.response_value["guard_health"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        close.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_eq!(
        close.response_value["guard_health"]["session_watch_coverage_basis"],
        "mcp_start"
    );
    assert_eq!(
        close.response_value["guard_health"]["session_watch_partial_coverage_warning"],
        Value::Null
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn guarded_bypass_reconcile_prompt_acceptance_unblocks_close() -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-bypass", "detective")?;
    fixture.mark_required_hooks_supported()?;
    fixture.activate_guard("guard_bypass_session_start")?;
    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("bypass")?;
    fixture.record_non_write_close_basis(&task_id, &change_unit_id, "bypass")?;
    let final_judgment_id =
        fixture.request_final_acceptance(&task_id, &change_unit_id, "bypass")?;
    fixture.answer_pending_judgment_through_prompt(
        &task_id,
        &final_judgment_id,
        "guard_bypass_final_prompt",
        "guard_bypass_final_capture",
    )?;

    fixture.apply_product_change("bypass write without pre-tool readiness")?;
    let post = json!({
        "event_id": "guard_bypass_post",
        "session_id": fixture.session_id(),
        "connection_id": fixture.connection_id(),
        "guard_installation_id": fixture.guard_installation_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": "tool_bypass_write",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": [DEFAULT_PRODUCT_PATH],
        "timestamp": "2026-06-30T06:22:00Z"
    });
    let post_output = fixture.run_guard_event("post-tool", &post)?;
    assert_success(&post_output);
    assert_eq!(json_stdout(&post_output)?["decision"], "warn");

    let unresolved = list_unresolved_unrecorded_changes(
        fixture.runtime_home(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?;
    assert_eq!(unresolved.len(), 1);
    let unrecorded_change_id = unresolved[0].unrecorded_change_id.clone();

    let blocked = fixture.check_close(&task_id)?;
    assert_eq!(blocked.response_value["close_state"], "blocked");
    assert_close_blocker(&blocked.response_value, "unresolved_unrecorded_changes");

    let first_reconcile = fixture.reconcile_changes(&task_id, "bypass_first")?;
    assert_eq!(
        first_reconcile.response_value["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        1
    );
    let reconciliation_judgment_id = first_reconcile.response_value["pending_user_judgment_refs"]
        [0]["record_id"]
        .as_str()
        .expect("reconciliation judgment id should be present")
        .to_owned();
    fixture.answer_pending_judgment_through_prompt(
        &task_id,
        &reconciliation_judgment_id,
        "guard_bypass_accept_prompt",
        "guard_bypass_accept_capture",
    )?;

    let second_reconcile = fixture.reconcile_changes(&task_id, "bypass_second")?;
    assert_eq!(
        second_reconcile.response_value["resolved_changes"][0]["resolution_basis"],
        "accepted_by_user"
    );
    let row = unrecorded_change(
        fixture.runtime_home(),
        fixture.project_id(),
        &unrecorded_change_id,
    )?
    .expect("unrecorded change should remain inspectable");
    assert_eq!(row.status, "resolved");

    let after = fixture.check_close(&task_id)?;
    assert_no_close_blocker(&after.response_value, "unresolved_unrecorded_changes");
    assert_eq!(
        after.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn guarded_close_missing_required_hooks_remain_after_session_start() -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-health", "detective")?;
    fixture.mark_required_hooks_missing()?;
    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("health")?;
    fixture.record_non_write_close_basis(&task_id, &change_unit_id, "health")?;
    let final_judgment_id =
        fixture.request_final_acceptance(&task_id, &change_unit_id, "health")?;
    fixture.record_judgment_direct(&task_id, &final_judgment_id, JudgmentKind::FinalAcceptance)?;

    let before = fixture.check_close(&task_id)?;
    assert_eq!(before.response_value["close_state"], "blocked");
    assert!(
        close_blocker_codes(&before.response_value)
            .iter()
            .any(|code| matches!(
                code.as_str(),
                "guard_degraded"
                    | "guard_required_hooks_missing"
                    | "guard_reload_required"
                    | "guard_not_observed"
            )),
        "expected a guard health blocker before session-start, got {:?}",
        close_blocker_codes(&before.response_value)
    );

    fixture.activate_guard("guard_health_session_start")?;
    let after = fixture.check_close(&task_id)?;
    assert_close_blocker(&after.response_value, "guard_required_hooks_missing");
    assert_no_close_blocker(&after.response_value, "guard_reload_required");
    assert_no_close_blocker(&after.response_value, "guard_not_observed");
    assert_eq!(
        after.response_value["close_state"], "blocked",
        "{}",
        after.response_value
    );
    assert_eq!(
        after.response_value["guard_health"]["guard_hook_observed"],
        true
    );
    assert_eq!(
        after.response_value["guard_health"]["effective_guard_status"],
        "degraded"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_only_init_skips_guard_observation_but_keeps_user_judgment_blocker(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-mcp-only", "record")?;
    assert_eq!(
        fixture.init_output["states"]["guard_installation"],
        "configured"
    );
    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("record")?;
    fixture.record_non_write_close_basis(&task_id, &change_unit_id, "record")?;
    fixture.request_final_acceptance(&task_id, &change_unit_id, "record")?;

    let check = fixture.check_close(&task_id)?;
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_close_blocker(&check.response_value, "pending_user_judgment");
    assert_no_close_blocker(&check.response_value, "guard_not_observed");
    assert_eq!(
        check.response_value["guard_health"]["selected_profile"],
        "record"
    );
    assert_eq!(
        check.response_value["guard_health"]["guard_hook_observed"],
        false
    );
    Ok(())
}
