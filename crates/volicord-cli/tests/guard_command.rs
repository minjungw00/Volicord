#![forbid(unsafe_code)]

mod support;

use std::{error::Error, fs};

use serde_json::{json, Value};
use support::{
    assertions::{assert_success, json_stdout, stderr, stdout},
    guard_fixture::*,
};
use volicord_store::diagnostics::{
    diagnostics_db_path, read_diagnostic_session, start_diagnostic_session, DiagnosticHostKind,
    DiagnosticSessionStart, DiagnosticTransport,
};
use volicord_store::guards::{
    agent_session, expected_write, guard_event, guard_health_record, guard_installation,
    insert_agent_session, list_pending_expected_writes, list_unresolved_unrecorded_changes,
    prompt_capture, prompt_capture_availability, unrecorded_change, AgentSessionInsert,
};
use volicord_store::session_watch::latest_watch_baseline_for_session;

#[cfg(unix)]
#[cfg(unix)]
use support::assertions::{assert_close_blocker, assert_no_close_blocker, close_blocker_codes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuardDurableCounts {
    agent_sessions: i64,
    guard_events: i64,
    watch_baselines: i64,
    expected_writes: i64,
    prompt_captures: i64,
}

fn guard_durable_counts(fixture: &GuardCliFixture) -> Result<GuardDurableCounts, Box<dyn Error>> {
    let connection = rusqlite::Connection::open_with_flags(
        fixture
            .runtime_home()
            .join("projects")
            .join(fixture.project_id())
            .join("state.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let count = |table: &str| -> Result<i64, rusqlite::Error> {
        let sql = match table {
            "agent_sessions" => "SELECT COUNT(*) FROM agent_sessions",
            "guard_events" => "SELECT COUNT(*) FROM guard_events",
            "session_watch_baselines" => "SELECT COUNT(*) FROM session_watch_baselines",
            "expected_writes" => "SELECT COUNT(*) FROM expected_writes",
            "prompt_captures" => "SELECT COUNT(*) FROM prompt_captures",
            _ => unreachable!("test helper uses a fixed table allowlist"),
        };
        connection.query_row(sql, [], |row| row.get(0))
    };
    Ok(GuardDurableCounts {
        agent_sessions: count("agent_sessions")?,
        guard_events: count("guard_events")?,
        watch_baselines: count("session_watch_baselines")?,
        expected_writes: count("expected_writes")?,
        prompt_captures: count("prompt_captures")?,
    })
}

fn assert_output_excludes(output: &std::process::Output, markers: &[&str]) {
    let stdout = stdout(output);
    let stderr = stderr(output);
    for marker in markers {
        assert!(!stdout.contains(marker), "stdout leaked {marker:?}");
        assert!(!stderr.contains(marker), "stderr leaked {marker:?}");
    }
}

fn assert_runtime_home_excludes(
    path: &std::path::Path,
    markers: &[&str],
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        if file_type.is_dir() {
            assert_runtime_home_excludes(&entry_path, markers)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&entry_path)?;
            for marker in markers {
                assert!(
                    !bytes
                        .windows(marker.len())
                        .any(|window| window == marker.as_bytes()),
                    "durable file {} leaked {marker:?}",
                    entry_path.display()
                );
            }
        }
    }
    Ok(())
}

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
    assert_eq!(
        value["session_id"],
        volicord_types::managed_host_session_id(
            "codex",
            fixture.connection_id(),
            "guard_session_a",
        )?
    );
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

    let guard_event_id = value["guard_event_id"]
        .as_str()
        .expect("managed guard event id should be returned");
    assert_ne!(guard_event_id, "guard_session_start_event");
    let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
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
        let value = json_stdout(&output)?;
        let guard_event_id = value["guard_event_id"]
            .as_str()
            .expect("managed guard event id should be returned");
        assert_ne!(guard_event_id, event_id);
        let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
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

    let guard_event_id = value["guard_event_id"]
        .as_str()
        .expect("managed guard event id should be returned");
    let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
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
        &fixture.only_guard_event_id("pre_tool")?,
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
    let user_action_request_id = prompt.create_pending_user_action("codex_native")?;
    let verification_code = prompt.prompt_verification_code(&user_action_request_id)?;
    let mut event = host_fixture_event(
        &prompt,
        CODEX_USER_PROMPT_ACTION_EVENT,
        "guard_codex_native_prompt",
        "codex",
    )?;
    replace_prompt_user_action_binding(&mut event, &user_action_request_id, &verification_code);
    let output = run_host_guard(&prompt, "prompt-capture", "codex", &event, &[])?;
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(&value, "UserPromptSubmit", "Volicord resolved");
    prompt.assert_resolved_prompt_user_action(&user_action_request_id, "accepted", "accept")?;

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
    let user_action_request_id = prompt.create_pending_user_action("claude_native")?;
    let verification_code = prompt.prompt_verification_code(&user_action_request_id)?;
    let mut event = host_fixture_event(
        &prompt,
        CLAUDE_USER_PROMPT_ACTION_EVENT,
        "guard_claude_native_prompt",
        "claude_code",
    )?;
    replace_prompt_user_action_binding(&mut event, &user_action_request_id, &verification_code);
    let output = run_host_guard(&prompt, "prompt-capture", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_context_output(&value, "UserPromptSubmit", "Volicord resolved");
    prompt.assert_resolved_prompt_user_action(&user_action_request_id, "accepted", "accept")?;

    let prompt_block = GuardCliFixture::new("guard-claude-native-prompt-block")?;
    prompt_block.install_guard_policy_for_host("claude_code")?;
    prompt_block.create_pending_user_action("claude_native_block")?;
    let event = host_fixture_event(
        &prompt_block,
        CLAUDE_USER_PROMPT_ACTION_EVENT,
        "guard_claude_native_prompt_block",
        "claude_code",
    )?;
    let output = run_host_guard(&prompt_block, "prompt-capture", "claude-code", &event, &[])?;
    assert_ne!(output.status.code(), Some(1));
    let value = assert_host_native_json_stdout(&output, 0)?;
    assert_block_output(&value, "malformed_user_action_command");

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
fn guard_builtin_sessions_are_canonical_and_raw_native_ids_are_not_persisted(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-managed-session-binding")?;
    let native_session_id = "codex.native:secret-1";
    let event = json!({
        "event_id": "guard_managed_session_binding",
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "transcript_path": format!("/tmp/{native_session_id}.jsonl")
    });
    let expected = expected_managed_session_id(&fixture, &event)?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "session-start",
            "--repo",
            fixture.repo_arg(),
            "--session",
            expected.as_str(),
        ],
        &event,
    )?;
    assert_success(&output);
    let output_value = json_stdout(&output)?;
    assert_eq!(output_value["session_id"], expected);
    let guard_event_id = output_value["guard_event_id"]
        .as_str()
        .expect("managed guard event id should be returned");
    assert_ne!(guard_event_id, "guard_managed_session_binding");

    let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
        .expect("managed guard event should be stored");
    assert_eq!(stored.session_id.as_deref(), Some(expected.as_str()));
    assert!(!stored.subject_json.contains(native_session_id));
    let subject: Value = serde_json::from_str(&stored.subject_json)?;
    assert_eq!(subject["raw_event"]["session_id"], expected);
    assert_eq!(subject["raw_event"]["thread_id"], expected);
    assert!(subject["raw_event"]["transcript_path"]
        .as_str()
        .is_some_and(|path| !path.contains(native_session_id)));
    Ok(())
}

#[test]
fn guard_managed_session_exact_preseed_and_replay_are_idempotent() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-managed-preseed-exact")?;
    let native_session_id = "native.session:preseed-exact";
    let session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        native_session_id,
    )?;
    let seeded = insert_agent_session(
        fixture.runtime_home(),
        fixture.project_id(),
        AgentSessionInsert {
            session_id: session_id.clone(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_kind: "codex".to_owned(),
            guard_mode: "record".to_owned(),
            started_at: "2026-07-14T00:00:00Z".to_owned(),
            metadata_json: json!({"source": "managed_preseed_exact"}).to_string(),
        },
    )?;
    let event = json!({
        "event_id": "native-event-preseed-exact",
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex"
    });

    let first = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&first);
    let first_value = json_stdout(&first)?;
    assert_eq!(first_value["session_id"], session_id);
    let after_first_core = fixture.core_effect_counts()?;
    let after_first_durable = guard_durable_counts(&fixture)?;
    let after_first_diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(session_id.as_str()))?
            .expect("managed guard diagnostics should exist after the first event");
    assert_eq!(
        after_first_diagnostics.connection_id.as_deref(),
        Some(fixture.connection_id())
    );
    assert_eq!(after_first_diagnostics.host_kind.as_deref(), Some("codex"));

    let replay = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&replay);
    assert_eq!(json_stdout(&replay)?, first_value);
    assert_eq!(fixture.core_effect_counts()?, after_first_core);
    assert_eq!(guard_durable_counts(&fixture)?, after_first_durable);
    let after_replay_diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(session_id.as_str()))?
            .expect("managed guard diagnostics should remain available");
    assert_eq!(
        after_replay_diagnostics.connection_id,
        after_first_diagnostics.connection_id
    );
    assert_eq!(
        after_replay_diagnostics.host_kind,
        after_first_diagnostics.host_kind
    );
    assert_eq!(
        agent_session(
            fixture.runtime_home(),
            fixture.project_id(),
            session_id.as_str(),
        )?,
        Some(seeded)
    );
    Ok(())
}

#[test]
fn guard_managed_session_preseed_conflict_has_zero_effects() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-managed-preseed-conflict")?;
    let native_session_id = "native.session:preseed-conflict";
    let session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        native_session_id,
    )?;
    let seeded = insert_agent_session(
        fixture.runtime_home(),
        fixture.project_id(),
        AgentSessionInsert {
            session_id: session_id.clone(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_kind: "generic".to_owned(),
            guard_mode: "record".to_owned(),
            started_at: "2026-07-14T00:00:00Z".to_owned(),
            metadata_json: json!({"source": "managed_preseed_conflict"}).to_string(),
        },
    )?;
    let before_core = fixture.core_effect_counts()?;
    let before_durable = guard_durable_counts(&fixture)?;
    let event = json!({
        "event_id": "native-event-preseed-conflict",
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("MANAGED_HOST_SESSION_BINDING_CONFLICT"));
    assert_output_excludes(
        &output,
        &[native_session_id, "native-event-preseed-conflict"],
    );
    assert_eq!(fixture.core_effect_counts()?, before_core);
    assert_eq!(guard_durable_counts(&fixture)?, before_durable);
    assert_eq!(
        agent_session(
            fixture.runtime_home(),
            fixture.project_id(),
            session_id.as_str(),
        )?,
        Some(seeded)
    );
    assert!(read_diagnostic_session(fixture.runtime_home(), Some(session_id.as_str()))?.is_none());
    Ok(())
}

#[test]
fn guard_managed_diagnostic_preseed_conflict_has_zero_project_effects() -> Result<(), Box<dyn Error>>
{
    let fixture = GuardCliFixture::new("guard-managed-diagnostic-preseed-conflict")?;
    let native_session_id = "native.session:diagnostic-preseed-conflict";
    let session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        native_session_id,
    )?;
    start_diagnostic_session(
        fixture.runtime_home(),
        DiagnosticSessionStart {
            session_id: session_id.as_str(),
            connection_id: Some("connection_diagnostic_other"),
            project_id: Some(fixture.project_id()),
            transport: DiagnosticTransport::GuardHook,
            host_kind: Some(DiagnosticHostKind::Codex),
            package_version: "test",
            build_id: "test",
        },
    )?;
    let before_core = fixture.core_effect_counts()?;
    let before_durable = guard_durable_counts(&fixture)?;
    let event = json!({
        "event_id": "native-event-diagnostic-preseed-conflict",
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;

    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("already bound to a different connection or host"));
    assert_output_excludes(
        &output,
        &[
            native_session_id,
            "native-event-diagnostic-preseed-conflict",
        ],
    );
    assert_eq!(fixture.core_effect_counts()?, before_core);
    assert_eq!(guard_durable_counts(&fixture)?, before_durable);
    let unchanged = read_diagnostic_session(fixture.runtime_home(), Some(session_id.as_str()))?
        .expect("conflicting diagnostic binding should remain unchanged");
    assert_eq!(
        unchanged.connection_id.as_deref(),
        Some("connection_diagnostic_other")
    );
    assert_eq!(unchanged.host_kind.as_deref(), Some("codex"));
    assert_eq!(unchanged.totals.event_count, 0);
    Ok(())
}

#[test]
fn guard_generic_sessions_cannot_claim_the_managed_prefix() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-generic-reserved-managed-prefix")?;
    let reserved_session_id = format!("mhs_{}", "a".repeat(64));
    let before_core = fixture.core_effect_counts()?;
    let before_durable = guard_durable_counts(&fixture)?;
    let event = json!({
        "event_id": "generic-event-reserved-prefix",
        "session_id": reserved_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "generic"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("reserved for managed Codex and Claude Code"));
    assert_eq!(fixture.core_effect_counts()?, before_core);
    assert_eq!(guard_durable_counts(&fixture)?, before_durable);
    assert!(read_diagnostic_session(fixture.runtime_home(), None)?.is_none());
    Ok(())
}

#[test]
fn guard_managed_native_ids_never_reach_output_or_durable_storage_and_correlation_survives(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-managed-native-id-privacy")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let native_session_id = "native.session:privacy-sentinel";
    let native_start_event_id = "native-event-start-privacy-sentinel";
    let native_pre_event_id = "native-event-pre-privacy-sentinel";
    let native_post_event_id = "native-event-post-privacy-sentinel";
    let native_prompt_event_id = "native-event-prompt-privacy-sentinel";
    let native_hook_event_id = "native-hook-event-privacy-sentinel";
    let native_tool_id = "native-tool-call-privacy-sentinel";
    let native_capture_id = "native-prompt-capture-privacy-sentinel";
    let native_turn_id = "native-turn-privacy-sentinel";
    let native_transcript_id = "native-transcript-privacy-sentinel";
    let native_bare_id = "native-bare-id-privacy-sentinel";
    let native_ids = [
        native_session_id,
        native_start_event_id,
        native_pre_event_id,
        native_post_event_id,
        native_prompt_event_id,
        native_hook_event_id,
        native_tool_id,
        native_capture_id,
        native_turn_id,
        native_transcript_id,
        native_bare_id,
    ];
    let expected_session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        native_session_id,
    )?;

    let mut start_event = json!({
        "event_id": native_start_event_id,
        "hook_event_id": native_hook_event_id,
        "id": native_bare_id,
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "transcript_id": native_transcript_id,
        "turn_id": native_turn_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "metadata": {
            "event_echo": format!("event:{native_start_event_id}"),
            "hook_echo": format!("hook:{native_hook_event_id}"),
            "turn_echo": format!("turn:{native_turn_id}"),
            "transcript_echo": format!("transcript:{native_transcript_id}"),
            "bare_echo": format!("bare:{native_bare_id}")
        }
    });
    start_event["metadata"]
        .as_object_mut()
        .expect("start metadata object")
        .insert(
            format!("dynamic-{native_start_event_id}-key"),
            json!("native event identifiers in keys must be omitted"),
        );
    let start_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &start_event,
    )?;
    assert_success(&start_output);
    assert_output_excludes(&start_output, &native_ids);
    let start_value = json_stdout(&start_output)?;
    assert_eq!(start_value["session_id"], expected_session_id);

    let mut pre_event = json!({
        "event_id": native_pre_event_id,
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": native_tool_id,
        "turn_id": native_turn_id,
        "tool_input": {"command": "touch src/export.rs"},
        "paths": ["src/export.rs"],
        "metadata": {
            "event_echo": format!("event:{native_pre_event_id}"),
            "tool_echo": format!("tool:{native_tool_id}"),
            "turn_echo": format!("turn:{native_turn_id}")
        }
    });
    pre_event["metadata"]
        .as_object_mut()
        .expect("pre-tool metadata object")
        .insert(
            format!("dynamic-{native_tool_id}-key"),
            json!("native tool identifiers in keys must be omitted"),
        );
    let pre_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "pre-tool", "--repo", fixture.repo_arg()],
        &pre_event,
    )?;
    assert_success(&pre_output);
    assert_output_excludes(&pre_output, &native_ids);
    let pre_value = json_stdout(&pre_output)?;
    let expected_write_id = pre_value["result"]["expected_write"]["expected_write_id"]
        .as_str()
        .expect("managed pre-tool event should create an ExpectedWrite");
    let opaque_invocation_id = pre_value["result"]["expected_write"]["host_invocation_id"]
        .as_str()
        .expect("managed ExpectedWrite should preserve an opaque invocation coordinate");
    assert_ne!(opaque_invocation_id, native_tool_id);

    let post_event = json!({
        "event_id": native_post_event_id,
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "tool_call_id": native_tool_id,
        "turn_id": native_turn_id,
        "tool_input": {"command": "touch src/export.rs"},
        "tool_result": {"tool_call_id": native_tool_id, "success": true},
        "success": true,
        "changed_paths": ["src/export.rs"],
        "metadata": {
            "event_echo": format!("event:{native_post_event_id}"),
            "tool_echo": format!("tool:{native_tool_id}"),
            "turn_echo": format!("turn:{native_turn_id}")
        }
    });
    let post_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "post-tool", "--repo", fixture.repo_arg()],
        &post_event,
    )?;
    assert_success(&post_output);
    assert_output_excludes(&post_output, &native_ids);
    let post_value = json_stdout(&post_output)?;
    assert_eq!(
        post_value["result"]["matched_expected_writes"][0]["expected_write_id"],
        expected_write_id
    );
    assert_eq!(
        post_value["result"]["matched_expected_writes"][0]["host_invocation_id"],
        opaque_invocation_id
    );

    let prompt_event = json!({
        "event_id": native_prompt_event_id,
        "prompt_capture_id": native_capture_id,
        "session_id": native_session_id,
        "thread_id": native_session_id,
        "turn_id": native_turn_id,
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "message": "Please keep working on the active task.",
        "metadata": {
            "event_echo": format!("event:{native_prompt_event_id}"),
            "capture_echo": format!("capture:{native_capture_id}"),
            "turn_echo": format!("turn:{native_turn_id}")
        }
    });
    let prompt_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &prompt_event,
    )?;
    assert_success(&prompt_output);
    assert_output_excludes(&prompt_output, &native_ids);
    let prompt_value = json_stdout(&prompt_output)?;
    let opaque_capture_id = prompt_value["result"]["prompt_capture"]["prompt_capture_id"]
        .as_str()
        .expect("managed prompt capture should return an opaque coordinate");
    assert_ne!(opaque_capture_id, native_capture_id);

    let stored_expected = expected_write(
        fixture.runtime_home(),
        fixture.project_id(),
        expected_write_id,
    )?
    .expect("managed ExpectedWrite should remain queryable");
    assert_eq!(
        stored_expected.host_invocation_id.as_deref(),
        Some(opaque_invocation_id)
    );
    assert_eq!(
        stored_expected.matched_post_tool_guard_event_id.as_deref(),
        post_value["guard_event_id"].as_str()
    );
    let stored_capture = prompt_capture(
        fixture.runtime_home(),
        fixture.project_id(),
        opaque_capture_id,
    )?
    .expect("managed PromptCapture should remain queryable");
    assert_eq!(stored_capture.session_id, expected_session_id);
    let baseline = latest_watch_baseline_for_session(
        fixture.runtime_home(),
        fixture.project_id(),
        expected_session_id.as_str(),
    )?
    .expect("managed session should retain a canonical watch baseline");
    assert_eq!(baseline.session_id, expected_session_id);
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(expected_session_id.as_str()))?
            .expect("managed session should retain canonical diagnostics");
    assert_eq!(diagnostics.session_id, expected_session_id);

    for value in [&start_value, &pre_value, &post_value, &prompt_value] {
        let guard_event_id = value["guard_event_id"]
            .as_str()
            .expect("managed output should expose an opaque GuardEvent coordinate");
        assert!(guard_event_id.starts_with("guard_event_"));
        let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
            .expect("managed GuardEvent should remain queryable by its opaque coordinate");
        let stored_text = format!("{stored:?}");
        for native_id in native_ids {
            assert!(!stored_text.contains(native_id));
        }
    }
    for durable_text in [
        format!("{stored_expected:?}"),
        format!("{stored_capture:?}"),
        format!("{baseline:?}"),
        serde_json::to_string(&diagnostics)?,
    ] {
        for native_id in native_ids {
            assert!(!durable_text.contains(native_id));
        }
    }
    assert_runtime_home_excludes(fixture.runtime_home(), &native_ids)?;
    Ok(())
}

#[test]
fn guard_builtin_sessions_reject_missing_inconsistent_and_noncanonical_bindings(
) -> Result<(), Box<dyn Error>> {
    for (label, event) in [
        (
            "missing",
            json!({
                "event_id": "guard_managed_missing_session",
                "session_id": null,
                "host_kind": "codex"
            }),
        ),
        (
            "invalid",
            json!({
                "event_id": "guard_managed_invalid_session",
                "session_id": "native session with space",
                "host_kind": "codex"
            }),
        ),
        (
            "inconsistent",
            json!({
                "event_id": "guard_managed_inconsistent_session",
                "session_id": "native-a",
                "thread_id": "native-b",
                "host_kind": "codex"
            }),
        ),
    ] {
        let fixture = GuardCliFixture::new(&format!("guard-managed-{label}"))?;
        let mut event = event;
        event["connection_id"] = json!(fixture.connection_id());
        let output = run_guard(
            fixture.runtime_home(),
            fixture.repo_root(),
            ["_hook", "session-start", "--repo", fixture.repo_arg()],
            &event,
        )?;
        assert_eq!(output.status.code(), Some(2), "{label} should fail closed");
        assert!(guard_event(
            fixture.runtime_home(),
            fixture.project_id(),
            event["event_id"].as_str().expect("fixture event id"),
        )?
        .is_none());
    }

    let fixture = GuardCliFixture::new("guard-managed-raw-override")?;
    let event = json!({
        "event_id": "guard_managed_raw_override",
        "session_id": "native-a",
        "host_kind": "codex",
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
            "--session",
            "native-a",
        ],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("mhs_"));
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

    let guard_event_id = value["guard_event_id"]
        .as_str()
        .expect("managed guard event id should be returned");
    let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
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
fn guard_pre_tool_uses_core_clock_despite_future_host_timestamp() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-future-host-clock")?;
    let task_id = fixture.create_active_task()?;
    fixture.prepare_write(&task_id)?;
    let event = json!({
        "event_id": "guard_pre_future_host_clock",
        "session_id": "guard_session_future_host_clock",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "Bash",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2999-01-01T00:00:00Z"
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
    assert_eq!(
        value["result"]["write_ticket_backing"]["status"],
        "ticket_backed"
    );
    let guard_event_id = value["guard_event_id"]
        .as_str()
        .expect("managed guard event id should be returned");
    let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
        .expect("future-dated host event should remain stored as an observation");
    assert_eq!(stored.occurred_at, "2999-01-01T00:00:00Z");
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
    let guard_event_id = value["guard_event_id"]
        .as_str()
        .expect("managed guard event id should be returned");
    let stored = guard_event(fixture.runtime_home(), fixture.project_id(), guard_event_id)?
        .expect("post-tool host-hook event should be stored");
    assert_eq!(stored.decision, "warn");
    assert_eq!(stored.event_kind, "post_tool");
    let diagnostic_session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        "guard_session_post",
    )?;
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(&diagnostic_session_id))?
            .expect("post-tool diagnostics");
    assert_eq!(diagnostics.totals.product_file_write_count, 1);
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
        post_value["guard_event_id"].as_str()
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
    let prompt_capture_id = value["result"]["prompt_capture"]["prompt_capture_id"]
        .as_str()
        .expect("managed prompt capture id should be returned");
    assert_ne!(prompt_capture_id, "guard_prompt_capture_a");
    assert_eq!(
        value["result"]["prompt_capture"]["prompt_text_omitted"],
        true
    );

    let stored = prompt_capture(
        fixture.runtime_home(),
        fixture.project_id(),
        prompt_capture_id,
    )?
    .expect("prompt capture should be stored");
    assert!(stored.prompt_text.is_none());
    assert!(stored.prompt_sha256.starts_with("sha256:"));
    Ok(())
}

#[test]
fn guard_session_start_shows_only_the_exact_safe_user_action_summary() -> Result<(), Box<dyn Error>>
{
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-instructions")?;
    fixture.create_pending_user_action("instructions")?;
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
    let pending = &value["result"]["context"]["pending_user_actions"][0];
    assert_eq!(
        pending
            .as_object()
            .expect("pending summary should be an object")
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        ["next_actor", "status", "user_action_request_id"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    assert_eq!(pending["status"], "pending");
    assert_eq!(pending["next_actor"], "user");
    let rendered = stdout(&output);
    for forbidden in [
        "verification_code",
        "resolve_instruction",
        "form_type",
        "choice_id",
        "volicord inbox resolve",
    ] {
        assert!(!rendered.contains(forbidden), "leaked {forbidden}");
    }
    Ok(())
}

#[test]
fn guard_session_start_requires_user_only_channel_for_sensitive_complete_presentation(
) -> Result<(), Box<dyn Error>> {
    const PRESENTATION_MARKER: &str = "GUARD_SENSITIVE_PRESENTATION_MARKER";

    for host_output in ["codex", "claude-code"] {
        let fixture = GuardCliFixture::with_prompt_capture(&format!(
            "guard-sensitive-session-presentation-{host_output}"
        ))?;
        let action = fixture.create_pending_sensitive_evidence_observation(
            &format!("session_presentation_{host_output}"),
            PRESENTATION_MARKER,
        )?;
        let verification_code = fixture.prompt_verification_code(&action.user_action_request_id)?;
        let before = fixture.core_effect_counts()?;
        let inbox = support::binary_fixture::run_with_home_env_in_dir(
            fixture.runtime_home(),
            ["inbox"],
            &[],
            fixture.repo_root(),
        )?;
        assert_success(&inbox);
        let inbox_text = stdout(&inbox);
        assert!(inbox_text.contains(PRESENTATION_MARKER));
        assert!(inbox_text.contains("An API key must be handled only in a user-only channel"));
        assert!(inbox_text.contains("credential-material"));
        assert_eq!(fixture.core_effect_counts()?, before);
        let event = json!({
            "event_id": format!("guard_sensitive_session_{host_output}"),
            "session_id": format!("guard_sensitive_session_{host_output}"),
            "connection_id": fixture.connection_id(),
            "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND
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
                host_output,
            ],
            &event,
        )?;
        assert_success(&output);
        let value = json_stdout(&output)?;
        let context = value["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .expect("SessionStart should return bounded generic context");
        assert!(context.contains(action.user_action_request_id.as_str()));
        assert!(context.contains("status pending"));
        assert!(context.contains("next_actor user"));
        assert!(context.contains("volicord inbox"));
        for hidden in [
            PRESENTATION_MARKER,
            verification_code.as_str(),
            "Volicord: resolve",
            "--request",
            "Canonical closed form",
        ] {
            assert!(!context.contains(hidden), "host context exposed {hidden}");
        }
        assert!(!stdout(&output).contains(PRESENTATION_MARKER));
        assert!(stderr(&output).is_empty());
        assert_eq!(fixture.core_effect_counts()?, before);
        assert_eq!(
            fixture.user_action_status(&action.user_action_request_id)?,
            "pending"
        );
    }
    Ok(())
}

#[test]
fn guard_session_start_keeps_safe_summary_without_prompt_capture() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-chat-instructions-not-configured")?;
    fixture.install_guard_policy_with(true, false, "configured")?;
    fixture.create_pending_user_action("instructions_not_configured")?;
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
    assert_eq!(value["result"]["context"]["pending_user_action_count"], 1);
    let pending = value["result"]["context"]["pending_user_actions"]
        .as_array()
        .expect("pending user actions should be an array");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["status"], "pending");
    assert_eq!(pending[0]["next_actor"], "user");
    Ok(())
}

#[test]
fn guard_session_start_omits_stale_chat_user_action_instructions() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-instructions-stale")?;
    let user_action_request_id = fixture.create_pending_user_action("instructions_stale")?;
    fixture.set_user_action_basis_status(&user_action_request_id, "stale")?;
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
    assert_eq!(value["result"]["context"]["pending_user_action_count"], 0);
    assert_eq!(
        value["result"]["context"]["pending_user_actions"]
            .as_array()
            .expect("pending user actions should be an array")
            .len(),
        0
    );
    Ok(())
}

#[test]
fn guard_session_start_omits_expired_chat_user_action_instructions() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-instructions-expired")?;
    let user_action_request_id = fixture.create_pending_user_action("instructions_expired")?;
    fixture.expire_user_action_at_core_clock(&user_action_request_id)?;
    let event = json!({
        "event_id": "guard_session_chat_instructions_expired",
        "session_id": "guard_session_chat_instructions_expired",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
        "timestamp": "2000-01-01T00:00:00Z"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "session-start", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["result"]["context"]["pending_user_action_count"], 0);
    assert_eq!(
        value["result"]["context"]["pending_user_actions"]
            .as_array()
            .expect("pending user actions should be an array")
            .len(),
        0
    );
    Ok(())
}

#[test]
fn guard_user_actions_use_core_clock_despite_skewed_host_timestamps() -> Result<(), Box<dyn Error>>
{
    for (label, host_timestamp) in [
        ("past", "2000-01-01T00:00:00Z"),
        ("future", "2999-01-01T00:00:00Z"),
    ] {
        let fixture =
            GuardCliFixture::with_prompt_capture(&format!("guard-user-action-clock-{label}"))?;
        let request_id = fixture.create_pending_user_action(&format!("clock_{label}"))?;
        let session_event_id = format!("guard_session_user_action_clock_{label}");
        let session_event = json!({
            "event_id": session_event_id,
            "session_id": format!("guard_session_user_action_clock_{label}"),
            "connection_id": fixture.connection_id(),
            "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
            "timestamp": host_timestamp
        });

        let session_output = run_guard(
            fixture.runtime_home(),
            fixture.repo_root(),
            ["_hook", "session-start", "--repo", fixture.repo_arg()],
            &session_event,
        )?;
        assert_success(&session_output);
        let session_value = json_stdout(&session_output)?;
        assert_eq!(
            session_value["result"]["context"]["pending_user_action_count"],
            1
        );
        assert_eq!(
            session_value["result"]["context"]["pending_user_actions"]
                .as_array()
                .expect("pending user actions should be an array")
                .len(),
            1
        );

        let verification_code = fixture.prompt_verification_code(&request_id)?;
        let message =
            format!("Volicord: resolve A-1 --request {request_id} --choice 1 {verification_code}");
        let prompt_event_id = format!("guard_prompt_user_action_clock_{label}");
        let mut event = prompt_event(
            &fixture,
            &prompt_event_id,
            &format!("guard_prompt_capture_user_action_clock_{label}"),
            &message,
        );
        event["session_id"] = session_event["session_id"].clone();
        event["timestamp"] = json!(host_timestamp);
        event["guard_installation_id"] = json!(fixture.guard_installation_id());

        let prompt_output = run_guard(
            fixture.runtime_home(),
            fixture.repo_root(),
            ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
            &event,
        )?;
        assert_success(&prompt_output);
        let prompt_value = json_stdout(&prompt_output)?;
        assert_eq!(
            prompt_value["result"]["recognized_user_action_command"]["replayed"],
            false
        );
        fixture.assert_resolved_prompt_user_action(&request_id, "accepted", "accept")?;

        let stored = guard_event(
            fixture.runtime_home(),
            fixture.project_id(),
            prompt_value["guard_event_id"]
                .as_str()
                .expect("managed guard event id should be returned"),
        )?
        .expect("skewed host event should remain stored as an observation");
        assert_eq!(stored.occurred_at, host_timestamp);
    }
    Ok(())
}

#[test]
fn guard_prompt_capture_resolves_choice_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-answer")?;
    let user_action_request_id = fixture.create_pending_user_action("resolve")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
        value["result"]["recognized_user_action_command"]["selected_option_id"],
        "accept"
    );
    assert_eq!(
        value["result"]["recognized_user_action_command"]["verification_code"],
        verification_code
    );
    assert_eq!(
        value["result"]["recognized_user_action_command"]["action_type"],
        "choice"
    );
    assert!(value["result"]["model_context"]
        .as_str()
        .expect("model context should be present")
        .contains("Volicord resolved user action"));
    let health = guard_health_record(
        fixture.runtime_home(),
        fixture.project_id(),
        fixture.connection_id(),
    )?;
    assert_eq!(
        prompt_capture_availability(&health)?.status.as_str(),
        "active"
    );
    fixture.assert_resolved_prompt_user_action(&user_action_request_id, "accepted", "accept")?;
    let diagnostic_session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        "guard_session_chat",
    )?;
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(&diagnostic_session_id))?
            .expect("prompt hook diagnostics");
    assert_eq!(diagnostics.user_channel_counts["prompt_capture"], 1);
    assert_eq!(diagnostics.totals.core_reached_count, 1);
    assert_eq!(diagnostics.totals.core_committed_count, 1);
    assert_eq!(diagnostics.totals.replayed_count, 0);

    let replay_event = prompt_event(
        &fixture,
        "guard_prompt_answer_replay",
        "guard_prompt_capture_answer_replay",
        &message,
    );
    let replay_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &replay_event,
    )?;
    assert_success(&replay_output);
    let replay_value = json_stdout(&replay_output)?;
    assert_eq!(
        replay_value["result"]["recognized_user_action_command"]["replayed"],
        true
    );
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(&diagnostic_session_id))?
            .expect("prompt hook replay diagnostics");
    assert_eq!(diagnostics.user_channel_counts["prompt_capture"], 2);
    assert_eq!(diagnostics.totals.core_reached_count, 2);
    assert_eq!(diagnostics.totals.core_committed_count, 1);
    assert_eq!(diagnostics.totals.replayed_count, 1);
    Ok(())
}

#[test]
fn guard_prompt_capture_exact_replay_survives_active_task_switch_without_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-active-task-replay")?;
    let user_action_request_id = fixture.create_pending_user_action("active_task_replay")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
    let first_event = prompt_event(
        &fixture,
        "guard_prompt_active_task_replay_first",
        "guard_prompt_capture_active_task_replay_first",
        &message,
    );

    let first_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &first_event,
    )?;
    assert_success(&first_output);
    assert_eq!(
        json_stdout(&first_output)?["result"]["recognized_user_action_command"]["replayed"],
        false
    );
    let resolution_before =
        serde_json::to_vec(&fixture.user_action_resolution(&user_action_request_id)?)?;

    let switched_task_id = fixture.create_additional_active_task("active_task_replay")?;
    let before_replay = fixture.replay_effect_snapshot()?;
    assert_eq!(before_replay.2.as_deref(), Some(switched_task_id.as_str()));
    let replay_message = message.clone();
    assert_eq!(replay_message.as_bytes(), message.as_bytes());
    let replay_event = prompt_event(
        &fixture,
        "guard_prompt_active_task_replay_second",
        "guard_prompt_capture_active_task_replay_second",
        &replay_message,
    );

    let replay_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &replay_event,
    )?;
    assert_success(&replay_output);
    let replay = json_stdout(&replay_output)?;
    assert_eq!(replay["decision"], "inject_context");
    assert_eq!(
        replay["result"]["recognized_user_action_command"]["replayed"],
        true
    );
    assert_eq!(fixture.replay_effect_snapshot()?, before_replay);
    assert_eq!(
        serde_json::to_vec(&fixture.user_action_resolution(&user_action_request_id)?)?,
        resolution_before,
        "exact replay must preserve the immutable stored resolution bytes"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_resolves_canonical_evidence_observation_without_disclosing_summary(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-evidence-observation")?;
    let action = fixture.create_pending_evidence_observation("resolve")?;
    let verification_code = fixture.prompt_verification_code(&action.user_action_request_id)?;
    let criterion_id = match &action.target {
        volicord_types::EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => acceptance_criterion_id.as_str(),
        _ => return Err("prompt fixture should expose an acceptance-criterion target".into()),
    };
    let selected_artifact = action.artifact_candidates[1].clone();
    let private_summary =
        "private-prompt-evidence-summary-must-not-enter-host-output-or-diagnostics";
    let message = format!(
        "Volicord: resolve A-1 --request {} --criterion {criterion_id} --artifact {} --summary \"{private_summary}\" --contradicted {verification_code}",
        action.user_action_request_id,
        selected_artifact.artifact_id
    );
    let mut event = prompt_event(
        &fixture,
        "guard_prompt_evidence_observation",
        "guard_prompt_capture_evidence_observation",
        &message,
    );
    event["guard_installation_id"] = json!(fixture.guard_installation_id());
    let before = fixture.core_effect_counts()?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;

    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "inject_context");
    let recognized = &value["result"]["recognized_user_action_command"];
    assert_eq!(recognized["action_type"], "evidence_observation");
    assert_eq!(
        recognized["selected_target"],
        format!("--criterion {criterion_id}")
    );
    assert_eq!(
        recognized["artifact_ids"],
        json!([selected_artifact.artifact_id.as_str()])
    );
    assert_eq!(recognized["relevance_status"], "contradicted");
    assert_eq!(recognized["summary_text_omitted"], true);
    assert_eq!(recognized["replayed"], false);
    assert!(value["result"]["model_context"]
        .as_str()
        .expect("resolved prompt should inject bounded model context")
        .contains("Volicord resolved user action"));
    assert!(!stdout(&output).contains(private_summary));
    assert!(!stderr(&output).contains(private_summary));

    fixture.assert_resolved_prompt_evidence_action(
        &action,
        &selected_artifact,
        volicord_types::EvidenceRelevanceStatus::Contradicted,
        private_summary,
    )?;
    let after = fixture.core_effect_counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_action_requests, before.user_action_requests);
    assert_eq!(
        after.user_action_resolutions,
        before.user_action_resolutions + 1
    );
    let diagnostic_session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        "guard_session_chat",
    )?;
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(&diagnostic_session_id))?
            .expect("prompt hook should create bounded diagnostics");
    assert_eq!(diagnostics.user_channel_counts["prompt_capture"], 1);
    assert_eq!(diagnostics.totals.core_reached_count, 1);
    assert_eq!(diagnostics.totals.core_committed_count, 1);
    assert_eq!(diagnostics.totals.replayed_count, 0);
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(private_summary));
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_sensitive_presentation_without_core_effect(
) -> Result<(), Box<dyn Error>> {
    const PRESENTATION_MARKER: &str = "GUARD_SENSITIVE_COMMAND_PRESENTATION_MARKER";
    const SUMMARY_MARKER: &str = "GUARD_PRIVATE_SENSITIVE_SUMMARY_MARKER";

    let fixture = GuardCliFixture::with_prompt_capture("guard-sensitive-prompt-command")?;
    let action = fixture
        .create_pending_sensitive_evidence_observation("prompt_command", PRESENTATION_MARKER)?;
    let verification_code = fixture.prompt_verification_code(&action.user_action_request_id)?;
    let claim_id = match &action.target {
        volicord_types::EvidenceTarget::SupplementalClaim {
            evidence_claim_id, ..
        } => evidence_claim_id.as_str(),
        _ => return Err("sensitive fixture should expose a supplemental claim".into()),
    };
    let artifact_id = action.artifact_candidates[0].artifact_id.as_str();
    let message = format!(
        "Volicord: resolve A-1 --request {} --claim {claim_id} --artifact {artifact_id} --summary \"{SUMMARY_MARKER}\" {verification_code}",
        action.user_action_request_id
    );
    let mut event = prompt_event(
        &fixture,
        "guard_sensitive_prompt_command",
        "guard_sensitive_prompt_capture",
        &message,
    );
    event["guard_installation_id"] = json!(fixture.guard_installation_id());
    let before = fixture.core_effect_counts()?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;

    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "prompt_capture_presentation_user_only");
    assert_eq!(
        value["result"]["recognized_user_action_command"],
        Value::Null
    );
    assert!(!stdout(&output).contains(PRESENTATION_MARKER));
    assert!(!stdout(&output).contains(SUMMARY_MARKER));
    assert!(!stderr(&output).contains(PRESENTATION_MARKER));
    assert!(!stderr(&output).contains(SUMMARY_MARKER));
    assert_eq!(fixture.core_effect_counts()?, before);
    assert_eq!(
        fixture.user_action_status(&action.user_action_request_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_host_output_injects_recorded_context() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-host-prompt-context")?;
    let user_action_request_id = fixture.create_pending_user_action("host_context")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
        .contains("Volicord resolved"));
    assert!(!stdout(&output).contains("schema_version"));
    fixture.assert_resolved_prompt_user_action(&user_action_request_id, "accepted", "accept")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_records_reject_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-reject")?;
    let user_action_request_id = fixture.create_pending_user_action("reject")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice reject {verification_code}"
    );
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
    fixture.assert_resolved_prompt_user_action(&user_action_request_id, "rejected", "reject")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_records_defer_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-defer")?;
    let user_action_request_id = fixture.create_pending_user_action("defer")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice defer {verification_code}"
    );
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
    fixture.assert_resolved_prompt_user_action(&user_action_request_id, "deferred", "defer")?;
    Ok(())
}

#[test]
fn guard_prompt_capture_records_choice_note() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-note")?;
    let user_action_request_id = fixture.create_pending_user_action("note")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice defer --note \"Need to review this later\" {verification_code}"
    );
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
    fixture.assert_resolved_prompt_user_action(&user_action_request_id, "deferred", "defer")?;
    let resolution = fixture.user_action_resolution(&user_action_request_id)?;
    assert_eq!(resolution["note"], "Need to review this later");
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_unsupported_host_without_recording() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-chat-unsupported-host")?;
    fixture.install_guard_policy_with(false, true, "configured")?;
    let user_action_request_id = fixture.create_pending_user_action("unsupported_host")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert!(prompt_capture(fixture.runtime_home(), fixture.project_id(), capture_id)?.is_none());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_not_configured_without_recording() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-chat-not-configured")?;
    fixture.install_guard_policy_with(true, false, "configured")?;
    let user_action_request_id = fixture.create_pending_user_action("not_configured")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert!(prompt_capture(fixture.runtime_home(), fixture.project_id(), capture_id)?.is_none());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_policy_mismatch_without_recording() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-policy-mismatch")?;
    let user_action_request_id = fixture.create_pending_user_action("policy_mismatch")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
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
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert!(prompt_capture(fixture.runtime_home(), fixture.project_id(), capture_id)?.is_none());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_malformed_command() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-malformed")?;
    let user_action_request_id = fixture.create_pending_user_action("malformed")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice defer --note {verification_code}"
    );
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
    assert_reason(&value, "malformed_user_action_command");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_host_output_blocks_malformed_prompt() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-host-prompt-block")?;
    let user_action_request_id = fixture.create_pending_user_action("host_block")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice defer --note {verification_code}"
    );
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
        .contains("malformed_user_action_command"));
    assert!(!stdout(&output).contains("schema_version"));
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_missing_verification_code() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-missing-code")?;
    let user_action_request_id = fixture.create_pending_user_action("missing_code")?;
    let event = prompt_event(
        &fixture,
        "guard_prompt_missing_code",
        "guard_prompt_capture_missing_code",
        &format!("Volicord: resolve A-1 --request {user_action_request_id} --choice 1"),
    );

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "malformed_user_action_command");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_wrong_verification_code() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-wrong-code")?;
    let user_action_request_id = fixture.create_pending_user_action("wrong_code")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let wrong_code = if verification_code == "#AAAAAA" {
        "#BBBBBB"
    } else {
        "#AAAAAA"
    };
    let message =
        format!("Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {wrong_code}");
    let event = prompt_event(
        &fixture,
        "guard_prompt_wrong_code",
        "guard_prompt_capture_wrong_code",
        &message,
    );
    let before = fixture.replay_effect_snapshot()?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "wrong_verification_code");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert_eq!(fixture.replay_effect_snapshot()?, before);
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
    assert!(value["result"]["recognized_user_action_command"].is_null());
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_invalid_chat_id() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-invalid-id")?;
    let user_action_request_id = fixture.create_pending_user_action("invalid_id")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-99 --request {user_action_request_id} --choice 1 {verification_code}"
    );
    let event = prompt_event(
        &fixture,
        "guard_prompt_invalid_id",
        "guard_prompt_capture_invalid_id",
        &message,
    );
    let before = fixture.replay_effect_snapshot()?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "unknown_user_action_id");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert_eq!(fixture.replay_effect_snapshot()?, before);
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_unknown_request_binding_without_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-request-binding")?;
    let user_action_request_id = fixture.create_pending_user_action("request_binding")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request unknown_request_binding --choice 1 {verification_code}"
    );
    let event = prompt_event(
        &fixture,
        "guard_prompt_request_binding",
        "guard_prompt_capture_request_binding",
        &message,
    );
    let before = fixture.replay_effect_snapshot()?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "unknown_user_action_request");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert_eq!(fixture.replay_effect_snapshot()?, before);
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_mismatched_project() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-project-mismatch")?;
    let user_action_request_id = fixture.create_pending_user_action("project_mismatch")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_mismatched_connection() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-connection-mismatch")?;
    let user_action_request_id = fixture.create_pending_user_action("connection_mismatch")?;
    fixture.register_extra_connection("other_connection")?;
    fixture.install_guard_policy_for_connection("other_connection", true, true, "configured")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
    let mut event = prompt_event(
        &fixture,
        "guard_prompt_connection_mismatch",
        "guard_prompt_capture_connection_mismatch",
        &message,
    );
    event["connection_id"] = json!("other_connection");
    let before = fixture.replay_effect_snapshot()?;

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "connection_mismatch");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    assert_eq!(fixture.replay_effect_snapshot()?, before);
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_stale_user_action() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-stale")?;
    let user_action_request_id = fixture.create_pending_user_action("stale")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    fixture.set_user_action_basis_status(&user_action_request_id, "stale")?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
    assert_reason(&json_stdout(&output)?, "user_action_not_pending");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "stale"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_replays_duplicate_same_answer_after_basis_stales(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-duplicate")?;
    let user_action_request_id = fixture.create_pending_user_action("duplicate")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
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
    fixture.set_user_action_basis_status(&user_action_request_id, "stale")?;
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &second,
    )?;
    assert_success(&output);
    let replay = json_stdout(&output)?;
    assert_eq!(replay["decision"], "inject_context");
    assert_eq!(
        replay["result"]["recognized_user_action_command"]["replayed"],
        true
    );
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "stale"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_conflicting_duplicate_answer() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-conflicting-duplicate")?;
    let user_action_request_id = fixture.create_pending_user_action("conflicting_duplicate")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let first_message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
    let second_message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice reject {verification_code}"
    );
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
    fixture.set_user_action_basis_status(&user_action_request_id, "stale")?;
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &second,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "conflicting_user_action_command");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "stale"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_expired_verification_code() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-expired-code")?;
    let user_action_request_id = fixture.create_pending_user_action("expired_code")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    fixture.expire_user_action_at_core_clock(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}"
    );
    let mut event = prompt_event(
        &fixture,
        "guard_prompt_expired_code",
        "guard_prompt_capture_expired_code",
        &message,
    );
    event["timestamp"] = json!("2999-01-01T00:00:00Z");

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["_hook", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_reason(&json_stdout(&output)?, "user_action_not_pending");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn guard_prompt_capture_rejects_multiple_commands() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::with_prompt_capture("guard-chat-ambiguous")?;
    let user_action_request_id = fixture.create_pending_user_action("ambiguous")?;
    let verification_code = fixture.prompt_verification_code(&user_action_request_id)?;
    let message = format!(
        "Volicord: resolve A-1 --request {user_action_request_id} --choice 1 {verification_code}\nVolicord: resolve A-1 --request {user_action_request_id} --choice reject {verification_code}"
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
    assert_reason(&json_stdout(&output)?, "ambiguous_user_action_command");
    assert_eq!(
        fixture.user_action_status(&user_action_request_id)?,
        "pending"
    );
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
    assert_eq!(
        value["result"]["close_status"]["authority_receipt"]["state_version"],
        value["result"]["context"]["state_version"]
    );
    assert_eq!(
        value["result"]["close_status"]["authority_receipt"]["task_ref"]["record_id"],
        value["result"]["close_status"]["active_task"]
    );
    Ok(())
}

#[test]
fn guard_stop_denies_when_authoritative_status_refresh_is_rejected() -> Result<(), Box<dyn Error>> {
    const CORRUPT_OWNER_VALUE: &str =
        "{\"private_refresh_body\":\"must-not-appear-in-stop-output\"";
    let fixture = GuardCliFixture::new("guard-stop-rejected-refresh")?;
    let task_id = fixture.create_active_task()?;
    fixture.corrupt_current_close_basis(&task_id, CORRUPT_OWNER_VALUE)?;
    let before = fixture.core_effect_counts()?;
    let event = json!({
        "event_id": "guard_stop_rejected_refresh",
        "session_id": "guard_session_stop_rejected_refresh",
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
    assert_eq!(value["allowed"], false);
    assert_reason(&value, "authoritative_refresh_failed");
    assert_eq!(
        value["result"]["close_status"]["authoritative_refresh"],
        json!({
            "response_kind": "rejected",
            "error_codes": ["MCP_UNAVAILABLE"]
        })
    );
    assert!(value["result"]["close_status"]
        .get("status_summary")
        .is_none());
    assert!(value["result"]["close_status"].get("close_state").is_none());
    assert!(value["result"]["close_status"]
        .get("close_blockers")
        .is_none());
    let rendered = serde_json::to_string(&value)?;
    assert!(!rendered.contains(CORRUPT_OWNER_VALUE));
    assert!(!rendered.contains("private_refresh_body"));
    assert!(!rendered.contains("Core storage is unavailable"));
    assert!(!rendered.contains("owner_state_error"));
    assert_eq!(fixture.core_effect_counts()?, before);
    let diagnostic_session_id = volicord_types::managed_host_session_id(
        "codex",
        fixture.connection_id(),
        "guard_session_stop_rejected_refresh",
    )?;
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home(), Some(&diagnostic_session_id))?
            .expect("stop-hook diagnostics");
    assert_eq!(diagnostics.totals.authoritative_refresh_failures, 1);
    assert_eq!(diagnostics.totals.core_reached_count, 1);
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home()))?;
    let diagnostics_text = String::from_utf8_lossy(&diagnostics_bytes);
    assert!(!diagnostics_text.contains(CORRUPT_OWNER_VALUE));
    assert!(!diagnostics_text.contains("private_refresh_body"));
    Ok(())
}

#[test]
fn guard_stop_host_output_blocks_and_allows_continue() -> Result<(), Box<dyn Error>> {
    let blocked = GuardCliFixture::new("guard-host-stop-block")?;
    let (blocked_installation_id, blocked_policy_hash) =
        blocked.install_guard_policy_for_host("codex")?;
    let blocked_task_id = blocked.create_active_task()?;
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
            "--guard-installation",
            &blocked_installation_id,
            "--host",
            "codex",
            "--integration-profile",
            "detective",
            "--policy-hash",
            &blocked_policy_hash,
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
    let receipt_message = blocked_value["systemMessage"]
        .as_str()
        .expect("active Stop output should display the fresh authority receipt");
    let receipt_json = receipt_message
        .strip_prefix("Volicord authority receipt: ")
        .expect("fresh authority receipt should use the dedicated UI prefix");
    let receipt: Value = serde_json::from_str(receipt_json)?;
    assert_eq!(receipt["project_id"], blocked.project_id());
    assert_eq!(receipt["task_ref"]["record_id"], blocked_task_id);
    assert_eq!(
        receipt["task_ref"]["produced_at_state_version"],
        receipt["state_version"]
    );
    assert!(blocked_output.stdout.len() <= 8 * 1024);

    let allowed = GuardCliFixture::new("guard-host-stop-allow")?;
    let allowed_connection_id = "connection_guard_host_stop_allow_claude";
    allowed.register_extra_connection_for_host(allowed_connection_id, "claude_code")?;
    let (allowed_installation_id, allowed_policy_hash) = allowed
        .install_guard_policy_for_connection_and_host(allowed_connection_id, "claude_code")?;
    let allowed_event = json!({
        "event_id": "guard_host_stop_allow",
        "session_id": "guard_host_stop_allow_session",
        "connection_id": allowed_connection_id,
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
            "--guard-installation",
            &allowed_installation_id,
            "--host",
            "claude-code",
            "--integration-profile",
            "detective",
            "--policy-hash",
            &allowed_policy_hash,
            "--host-output",
            "claude-code",
        ],
        &allowed_event,
    )?;
    assert_success(&allowed_output);
    let allowed_value = json_stdout(&allowed_output)?;
    assert_eq!(allowed_value["continue"], true);
    let allowed_message = allowed_value["systemMessage"]
        .as_str()
        .expect("no-active-Task output should use the fixed UI fallback");
    assert!(allowed_message.contains("no active Task is available"));
    assert!(allowed_message.contains("volicord status --json"));
    assert!(!allowed_message.contains("status --task"));
    assert!(allowed_output.stdout.len() <= 8 * 1024);
    assert!(stderr(&allowed_output).is_empty());
    Ok(())
}

#[test]
fn guard_stop_host_output_reports_status_fallback_when_refresh_fails() -> Result<(), Box<dyn Error>>
{
    const CORRUPT_OWNER_VALUE: &str =
        "{\"private_refresh_body\":\"must-not-appear-in-host-stop-output\"";
    let fixture = GuardCliFixture::new("guard-host-stop-refresh-fallback")?;
    let connection_id = "connection_guard_host_stop_refresh_claude";
    fixture.register_extra_connection_for_host(connection_id, "claude_code")?;
    let (guard_installation_id, policy_hash) =
        fixture.install_guard_policy_for_connection_and_host(connection_id, "claude_code")?;
    let task_id = fixture.create_active_task()?;
    fixture.corrupt_current_close_basis(&task_id, CORRUPT_OWNER_VALUE)?;
    let event = json!({
        "event_id": "guard_host_stop_refresh_fallback",
        "session_id": "guard_host_stop_refresh_fallback_session",
        "connection_id": connection_id,
        "host_kind": "claude_code",
        "message": "All done."
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        [
            "_hook",
            "stop",
            "--repo",
            fixture.repo_arg(),
            "--guard-installation",
            &guard_installation_id,
            "--host",
            "claude-code",
            "--integration-profile",
            "detective",
            "--policy-hash",
            &policy_hash,
            "--host-output",
            "claude-code",
        ],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "block");
    assert!(value["reason"]
        .as_str()
        .expect("refresh failure should preserve the Stop block reason")
        .contains("authoritative_refresh_failed"));
    let message = value["systemMessage"]
        .as_str()
        .expect("refresh failure should display the status fallback");
    assert!(message.contains(&format!("project_id={}", fixture.project_id())));
    assert!(message.contains(&format!("task_id={task_id}")));
    assert!(message.contains("state_version="));
    assert!(message.contains(&format!("volicord status --task {task_id} --json")));
    assert!(!message.contains("authority receipt: {"));
    assert!(!message.contains(CORRUPT_OWNER_VALUE));
    assert!(!message.contains("private_refresh_body"));
    assert!(output.stdout.len() <= 8 * 1024);
    Ok(())
}

#[test]
fn guard_stop_exact_replay_preserves_history_but_refreshes_current_authority(
) -> Result<(), Box<dyn Error>> {
    const PRIVATE_FINAL_PROSE: &str =
        "private-final-model-prose-must-not-become-authority-or-durable-guard-data";
    let fixture = GuardCliFixture::new("guard-host-stop-fresh-replay")?;
    let (guard_installation_id, policy_hash) = fixture.install_guard_policy_for_host("codex")?;
    let first_task_id = fixture.create_active_task()?;
    let event = json!({
        "session_id": "guard_host_stop_fresh_replay_session",
        "connection_id": fixture.connection_id(),
        "guard_installation_id": guard_installation_id,
        "host_kind": "codex",
        "last_assistant_message": PRIVATE_FINAL_PROSE
    });
    let args = [
        "_hook",
        "stop",
        "--repo",
        fixture.repo_arg(),
        "--guard-installation",
        guard_installation_id.as_str(),
        "--host",
        "codex",
        "--integration-profile",
        "detective",
        "--policy-hash",
        policy_hash.as_str(),
        "--host-output",
        "codex",
    ];

    let first_output = run_guard(fixture.runtime_home(), fixture.repo_root(), args, &event)?;
    assert_success(&first_output);
    let first_value = json_stdout(&first_output)?;
    let first_message = first_value["systemMessage"]
        .as_str()
        .expect("first Stop should display a fresh authority receipt");
    let first_receipt: Value = serde_json::from_str(
        first_message
            .strip_prefix("Volicord authority receipt: ")
            .expect("first Stop should use the complete receipt prefix"),
    )?;
    assert_eq!(first_receipt["task_ref"]["record_id"], first_task_id);
    assert!(!stdout(&first_output).contains(PRIVATE_FINAL_PROSE));

    let guard_event_id = fixture.only_guard_event_id("stop")?;
    let stored_before = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        &guard_event_id,
    )?
    .expect("first Stop should persist one historical GuardEvent");
    assert!(!stored_before.subject_json.contains(PRIVATE_FINAL_PROSE));
    assert!(!stored_before.result_json.contains(PRIVATE_FINAL_PROSE));

    let current_task_id = fixture.create_additional_active_task("stop_fresh_replay")?;
    let before_replay = fixture.replay_effect_snapshot()?;
    assert_eq!(before_replay.2.as_deref(), Some(current_task_id.as_str()));

    let replay_output = run_guard(fixture.runtime_home(), fixture.repo_root(), args, &event)?;
    assert_success(&replay_output);
    let replay_value = json_stdout(&replay_output)?;
    assert_eq!(replay_value["decision"], first_value["decision"]);
    let replay_message = replay_value["systemMessage"]
        .as_str()
        .expect("exact replay should display a freshly read authority receipt");
    let replay_receipt: Value = serde_json::from_str(
        replay_message
            .strip_prefix("Volicord authority receipt: ")
            .expect("replay should use the complete receipt prefix"),
    )?;
    assert_eq!(replay_receipt["task_ref"]["record_id"], current_task_id);
    assert_ne!(
        replay_receipt["task_ref"]["record_id"],
        first_receipt["task_ref"]["record_id"]
    );
    assert!(replay_receipt["state_version"].as_u64() > first_receipt["state_version"].as_u64());
    assert!(replay_output.stdout.len() <= 8 * 1024);
    assert!(!stdout(&replay_output).contains(PRIVATE_FINAL_PROSE));
    assert_eq!(fixture.replay_effect_snapshot()?, before_replay);
    assert_eq!(
        guard_event(
            fixture.runtime_home(),
            fixture.project_id(),
            &guard_event_id,
        )?
        .expect("exact replay should retain the historical GuardEvent"),
        stored_before
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn guarded_init_hook_write_prompt_lifecycle_fails_closed_without_producer_evidence(
) -> Result<(), Box<dyn Error>> {
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
    let final_user_action_request_id =
        fixture.request_final_acceptance_action(&task_id, &change_unit_id, "happy")?;
    fixture.resolve_pending_user_action_through_prompt(
        &task_id,
        &final_user_action_request_id,
        "guard_lifecycle_final_prompt",
        "guard_lifecycle_final_capture",
    )?;

    let check = fixture.check_close(&task_id)?;
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_close_blocker(&check.response_value, "evidence_agent_report_only");
    assert_eq!(
        check.response_value["guard_health"]["selected_profile"],
        "detective"
    );
    assert_eq!(
        check.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    assert_eq!(
        check.response_value["guard_health"]["session_watch_coverage_basis"],
        "mcp_start"
    );
    assert_eq!(
        check.response_value["guard_health"]["session_watch_partial_coverage_warning"],
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
    let final_user_action_request_id =
        fixture.request_final_acceptance_action(&task_id, &change_unit_id, "bypass")?;
    fixture.resolve_pending_user_action_through_prompt(
        &task_id,
        &final_user_action_request_id,
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
        first_reconcile.response_value["pending_user_action_summaries"]
            .as_array()
            .expect("pending safe summaries should be an array")
            .len(),
        1
    );
    let reconciliation_user_action_request_id = first_reconcile.response_value
        ["pending_user_action_summaries"][0]["user_action_request_id"]
        .as_str()
        .expect("reconciliation user-action request id should be present")
        .to_owned();
    fixture.resolve_pending_user_action_through_prompt(
        &task_id,
        &reconciliation_user_action_request_id,
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
    let final_user_action_request_id =
        fixture.request_final_acceptance_action(&task_id, &change_unit_id, "health")?;
    fixture.resolve_user_action_direct(&task_id, &final_user_action_request_id)?;

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
fn mcp_only_init_skips_guard_observation_but_keeps_user_action_blocker(
) -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-mcp-only", "record")?;
    assert_eq!(
        fixture.init_output["states"]["guard_installation"],
        "configured"
    );
    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("record")?;
    fixture.record_non_write_close_basis(&task_id, &change_unit_id, "record")?;
    fixture.request_final_acceptance_action(&task_id, &change_unit_id, "record")?;

    let check = fixture.check_close(&task_id)?;
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_close_blocker(&check.response_value, "pending_user_action");
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
