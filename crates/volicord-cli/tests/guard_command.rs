#![forbid(unsafe_code)]

use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_INTENT_SHARED,
    CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT, VERIFIED_STATUS_COMPLETE,
};
use volicord_store::guards::{
    expected_write, guard_event, guard_health_record, guard_installation, list_guard_installations,
    list_pending_expected_writes, list_unresolved_unrecorded_changes, prompt_capture,
    prompt_capture_availability, unrecorded_change, upsert_guard_installation,
    GuardInstallationUpsert,
};
use volicord_store::{bootstrap::list_projects, core_pipeline::CoreProjectStore};
use volicord_test_support::{
    core_fixtures::{
        answer_payload, supported_evidence_update, CoreFixture, UpdateScopeFixture,
        UserJudgmentFixture, DEFAULT_BASELINE_REF, DEFAULT_PRODUCT_PATH,
    },
    TempRuntimeHome,
};
use volicord_types::{
    chat_judgment_verification_code, ActorSource, BaselineRef, ChangeUnitId, ChangeUnitOperation,
    ChangeUnitUpdate, CloseAssessmentInput, CloseIntent, CloseReason, CloseTaskRequest,
    IdempotencyKey, InitialScope, IntakeRequest, JudgmentKind, JudgmentPresentation,
    JudgmentRationale, JudgmentRequiredFor, ObservedChanges, OperationCategory,
    PrepareWriteRequest, ProjectId, ReconcileChangesRequest, RecordId, RecordRunRequest,
    RecordUserJudgmentRequest, RequestId, RequestUserJudgmentRequest, RequestedMode, ResumePolicy,
    RunKind, ScopeUpdate, StateRecordKind, StateRecordRef, TaskId, ToolEnvelope,
    UpdateScopeRequest, UserJudgmentContext, UserJudgmentOptionId, WriteCheckId,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING, VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

const PROMPT_CAPTURE_TEST_HOST_KIND: &str = "prompt_capture_test_host";

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
        ["guard", "session-start", "--repo", fixture.repo_arg()],
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

    let stored = guard_event(
        fixture.runtime_home(),
        fixture.project_id(),
        "guard_session_start_event",
    )?
    .expect("guard event should be stored");
    assert_eq!(stored.decision, "inject_context");
    assert_eq!(stored.event_kind, "session_start");
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
        ["guard", "session-start", "--repo", fixture.repo_arg()],
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
fn guard_pre_tool_denies_product_write_without_active_task() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-no-task")?;
    let event = json!({
        "event_id": "guard_pre_no_task",
        "session_id": "guard_session_pre_no_task",
        "connection_id": fixture.connection_id(),
        "host": {"kind": "claude_code"},
        "tool_name": "shell",
        "command": "touch src/lib.rs",
        "paths": ["src/lib.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "deny");
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
        "tool_name": "shell",
        "command": "git status --short"
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(value["allowed"], true);
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
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
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
    .expect("outside-project guard event should be stored");
    assert_eq!(stored.decision, "deny");
    assert_eq!(stored.event_kind, "pre_tool");
    Ok(())
}

#[test]
fn guard_pre_tool_requires_current_write_readiness() -> Result<(), Box<dyn Error>> {
    let fixture = GuardCliFixture::new("guard-pre-write-ready")?;
    let task_id = fixture.create_active_task()?;
    let denied_event = json!({
        "event_id": "guard_pre_missing_write_check",
        "session_id": "guard_session_write_ready",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"]
    });

    let denied = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
        &denied_event,
    )?;
    assert_eq!(denied.status.code(), Some(1));
    assert_reason(&json_stdout(&denied)?, "write_readiness_missing");

    fixture.prepare_write(&task_id)?;
    let allowed_event = json!({
        "event_id": "guard_pre_with_write_check",
        "session_id": "guard_session_write_ready",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"]
    });
    let allowed = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
        &allowed_event,
    )?;
    assert_success(&allowed);
    let value = json_stdout(&allowed)?;
    assert_eq!(value["decision"], "allow");
    assert_eq!(value["allowed"], true);
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
        "tool_name": "shell",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"]
    });

    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "post-tool", "--repo", fixture.repo_arg()],
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
    .expect("post-tool guard event should be stored");
    assert_eq!(stored.decision, "warn");
    assert_eq!(stored.event_kind, "post_tool");
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
        "tool_name": "shell",
        "tool_call_id": "tool_call_expected",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:00:00Z"
    });

    let pre_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
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
        "tool_name": "shell",
        "tool_call_id": "tool_call_expected",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:01:00Z"
    });
    let post_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "post-tool", "--repo", fixture.repo_arg()],
        &post,
    )?;
    assert_success(&post_output);
    let post_value = json_stdout(&post_output)?;
    assert_eq!(post_value["decision"], "allow");
    assert_eq!(
        post_value["result"]["matched_expected_writes"][0]["expected_write_id"],
        expected_id
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
        "tool_name": "shell",
        "tool_call_id": "tool_call_scope_expected",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:10:00Z"
    });
    assert_success(&run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
        &pre,
    )?);

    let post = json!({
        "event_id": "guard_post_scope_changed",
        "session_id": "guard_session_scope_expected",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "tool_call_id": "tool_call_scope_expected",
        "command": "touch src/other.rs",
        "success": true,
        "changed_paths": ["src/other.rs"],
        "timestamp": "2026-06-30T05:11:00Z"
    });
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "post-tool", "--repo", fixture.repo_arg()],
        &post,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["decision"], "warn");
    assert_eq!(
        value["result"]["unrecorded_changes"][0]["observed_paths"][0],
        "src/other.rs"
    );
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
        "tool_name": "shell",
        "command": "python scripts/rewrite.py src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:20:00Z"
    });
    let pre_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
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

    let post = json!({
        "event_id": "guard_post_ambiguous_shell",
        "session_id": "guard_session_ambiguous_shell",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "command": "python scripts/rewrite.py src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:21:00Z"
    });
    let post_output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "post-tool", "--repo", fixture.repo_arg()],
        &post,
    )?;
    assert_success(&post_output);
    assert_eq!(json_stdout(&post_output)?["decision"], "warn");
    assert_eq!(
        list_unresolved_unrecorded_changes(
            fixture.runtime_home(),
            fixture.project_id(),
            Some(fixture.connection_id()),
        )?
        .len(),
        1
    );
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
        "tool_name": "shell",
        "tool_call_id": "shared_tool_call",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:30:00Z"
    });
    assert_success(&run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "pre-tool", "--repo", fixture.repo_arg()],
        &pre,
    )?);

    let post_other_session = json!({
        "event_id": "guard_post_session_b",
        "session_id": "guard_session_b",
        "connection_id": fixture.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "tool_call_id": "shared_tool_call",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:31:00Z"
    });
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "post-tool", "--repo", fixture.repo_arg()],
        &post_other_session,
    )?;
    assert_success(&output);
    assert_eq!(json_stdout(&output)?["decision"], "warn");
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
fn guard_expected_write_does_not_leak_between_projects() -> Result<(), Box<dyn Error>> {
    let first = GuardCliFixture::new("guard-project-isolation-a")?;
    let task_id = first.create_active_task()?;
    first.prepare_write(&task_id)?;
    let pre = json!({
        "event_id": "guard_pre_project_a",
        "session_id": "guard_session_project",
        "connection_id": first.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "tool_call_id": "tool_call_project",
        "command": "touch src/export.rs",
        "paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:40:00Z"
    });
    assert_success(&run_guard(
        first.runtime_home(),
        first.repo_root(),
        ["guard", "pre-tool", "--repo", first.repo_arg()],
        &pre,
    )?);

    let second = GuardCliFixture::new("guard-project-isolation-b")?;
    let task_id = second.create_active_task()?;
    let post = json!({
        "event_id": "guard_post_project_b",
        "session_id": "guard_session_project",
        "connection_id": second.connection_id(),
        "host_kind": "codex",
        "tool_name": "shell",
        "tool_call_id": "tool_call_project",
        "command": "touch src/export.rs",
        "success": true,
        "changed_paths": ["src/export.rs"],
        "timestamp": "2026-06-30T05:41:00Z"
    });
    let output = run_guard(
        second.runtime_home(),
        second.repo_root(),
        ["guard", "post-tool", "--repo", second.repo_arg()],
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
            "guard",
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
        ["guard", "session-start", "--repo", fixture.repo_arg()],
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
        ["guard", "session-start", "--repo", fixture.repo_arg()],
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
        ["guard", "session-start", "--repo", fixture.repo_arg()],
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
        ["guard", "session-start", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
            "mode": "guarded",
            "guard_mode": "guarded",
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
        &event,
    )?;
    assert_eq!(output.status.code(), Some(1));
    let value = json_stdout(&output)?;
    assert_reason(&value, "malformed_judgment_command");
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
        &first,
    )?);
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
        &first,
    )?);
    let output = run_guard(
        fixture.runtime_home(),
        fixture.repo_root(),
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "prompt-capture", "--repo", fixture.repo_arg()],
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
        ["guard", "stop", "--repo", fixture.repo_arg()],
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

#[cfg(unix)]
#[test]
fn guarded_init_hook_write_prompt_lifecycle_closes() -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-close", "guarded")?;
    assert_guard_init_state_is_installed_or_degraded(&fixture.init_output);
    fixture.mark_required_hooks_supported()?;
    fixture.activate_guard("guard_lifecycle_session_start")?;

    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("happy")?;
    let write_check_id = fixture.prepare_write(&task_id, &change_unit_id, "happy")?;

    let pre = json!({
        "event_id": "guard_lifecycle_pre",
        "session_id": fixture.session_id(),
        "connection_id": fixture.connection_id(),
        "guard_installation_id": fixture.guard_installation_id(),
        "host_kind": "codex",
        "tool_name": "shell",
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
        "tool_name": "shell",
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
        &write_check_id,
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
        close.response_value["guard_health"]["guard_mode"],
        "guarded"
    );
    assert_eq!(
        close.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn guarded_bypass_reconcile_prompt_acceptance_unblocks_close() -> Result<(), Box<dyn Error>> {
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-bypass", "guarded")?;
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
        "tool_name": "shell",
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
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-health", "guarded")?;
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
    let fixture = GuardedLifecycleFixture::init("guarded-lifecycle-mcp-only", "mcp-only")?;
    assert_eq!(
        fixture.init_output["states"]["guard_installation"],
        "configured"
    );
    let (task_id, change_unit_id) = fixture.create_task_with_change_unit("mcp_only")?;
    fixture.record_non_write_close_basis(&task_id, &change_unit_id, "mcp_only")?;
    fixture.request_final_acceptance(&task_id, &change_unit_id, "mcp_only")?;

    let check = fixture.check_close(&task_id)?;
    assert_eq!(check.response_value["close_state"], "blocked");
    assert_close_blocker(&check.response_value, "pending_user_judgment");
    assert_no_close_blocker(&check.response_value, "guard_not_observed");
    assert_eq!(
        check.response_value["guard_health"]["guard_mode"],
        "mcp_only"
    );
    assert_eq!(
        check.response_value["guard_health"]["guard_hook_observed"],
        false
    );
    Ok(())
}

struct GuardCliFixture {
    inner: CoreFixture,
    repo_root: std::path::PathBuf,
    repo_arg: String,
}

impl GuardCliFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let inner = CoreFixture::new(prefix)?;
        let repo_root = inner.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let repo_arg = repo_root.display().to_string();
        Ok(Self {
            inner,
            repo_root,
            repo_arg,
        })
    }

    fn with_prompt_capture(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let fixture = Self::new(prefix)?;
        fixture.install_guard_policy()?;
        Ok(fixture)
    }

    fn runtime_home(&self) -> &Path {
        self.inner.runtime_home_path()
    }

    fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    fn repo_arg(&self) -> &str {
        &self.repo_arg
    }

    fn project_id(&self) -> &str {
        self.inner.project_id()
    }

    fn connection_id(&self) -> &str {
        self.inner.connection_id()
    }

    fn guard_installation_id(&self) -> String {
        format!("guard_installation_cli_activation_{}", self.connection_id())
    }

    fn create_active_task(&self) -> Result<String, Box<dyn Error>> {
        let service = CoreService::new(self.runtime_home());
        let response = service.intake(
            self.inner
                .intake_request("req_guard_intake", "idem_guard_intake", false, Some(0)),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let task_id = record_id(&response.response_value["task_ref"])?;
        service.update_scope(
            self.inner.update_scope_request(UpdateScopeFixture {
                request_id: "req_guard_scope",
                idempotency_key: "idem_guard_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: &task_id,
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Guard fixture scope for src/export.rs.",
            }),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        Ok(task_id)
    }

    fn prepare_write(&self, task_id: &str) -> Result<(), Box<dyn Error>> {
        let service = CoreService::new(self.runtime_home());
        let state_version = self.inner.store()?.project_state()?.state_version;
        let response = service.prepare_write(
            self.inner.prepare_write_request(
                "req_guard_prepare_write",
                "idem_guard_prepare_write",
                Some(state_version),
                Some(task_id),
                None,
            ),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(response.response_value["decision"], "allowed");
        Ok(())
    }

    fn create_pending_authority_judgment(&self, suffix: &str) -> Result<String, Box<dyn Error>> {
        let task_id = self.create_active_task()?;
        let state_version = self.inner.store()?.project_state()?.state_version;
        let service = CoreService::new(self.runtime_home());
        let request_id = format!("req_guard_chat_judgment_{suffix}");
        let idempotency_key = format!("idem_guard_chat_judgment_{suffix}");
        let response = service.request_user_judgment(
            self.inner.user_judgment_request(UserJudgmentFixture {
                request_id: &request_id,
                idempotency_key: &idempotency_key,
                dry_run: false,
                expected_state_version: Some(state_version),
                task_id: &task_id,
                change_unit_id: None,
                judgment_kind: JudgmentKind::Cancellation,
            }),
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        record_id(&response.response_value["user_judgment_ref"])
    }

    fn prompt_verification_code(&self, judgment_id: &str) -> Result<String, Box<dyn Error>> {
        let record = self
            .inner
            .store()?
            .user_judgment_record(judgment_id)?
            .expect("judgment should be stored");
        Ok(chat_judgment_verification_code(
            &record.project_id,
            &record.task_id,
            &record.judgment_id,
            &record.requested_at,
            self.connection_id(),
        ))
    }

    fn assert_recorded_prompt_judgment(
        &self,
        judgment_id: &str,
        expected_outcome: &str,
        expected_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        let record = self
            .inner
            .store()?
            .user_judgment_record(judgment_id)?
            .expect("judgment should be stored");
        assert_eq!(record.status, "resolved");
        assert_eq!(record.resolution_outcome.as_deref(), Some(expected_outcome));
        assert_eq!(
            record.resolution_machine_action.as_deref(),
            Some(expected_action)
        );
        assert_eq!(
            record.resolved_by_actor_source.as_deref(),
            Some("local_user")
        );
        assert_eq!(
            record.resolved_verification_basis.as_deref(),
            Some(VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK)
        );
        assert_eq!(
            record.resolved_assurance_level.as_deref(),
            Some("local_user_channel")
        );
        Ok(())
    }

    fn judgment_status(&self, judgment_id: &str) -> Result<String, Box<dyn Error>> {
        Ok(self.inner.user_judgment_status(judgment_id)?)
    }

    fn judgment_resolution(&self, judgment_id: &str) -> Result<Value, Box<dyn Error>> {
        self.inner.user_judgment_resolution(judgment_id)
    }

    fn set_judgment_basis_status(
        &self,
        judgment_id: &str,
        basis_status: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.inner.conn()?.execute(
            "UPDATE user_judgments
                SET basis_status = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![self.project_id(), judgment_id, basis_status],
        )?;
        Ok(())
    }

    fn set_judgment_expires_at(
        &self,
        judgment_id: &str,
        expires_at: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut request_json: Value = serde_json::from_str(
            &self
                .inner
                .store()?
                .user_judgment_record(judgment_id)?
                .expect("judgment should be stored")
                .request_json,
        )?;
        request_json["expires_at"] = json!(expires_at);
        self.inner.conn()?.execute(
            "UPDATE user_judgments
                SET request_json = ?3
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![self.project_id(), judgment_id, request_json.to_string()],
        )?;
        Ok(())
    }

    fn register_extra_connection(&self, connection_id: &str) -> Result<(), Box<dyn Error>> {
        ensure_agent_connection(
            self.runtime_home(),
            AgentConnectionRegistration {
                connection_internal_id: connection_id.to_owned(),
                host_kind: HOST_KIND_CODEX.to_owned(),
                intent: CONNECTION_INTENT_SHARED.to_owned(),
                host_scope: HOST_SCOPE_PROJECT.to_owned(),
                server_name: format!("volicord-test-{connection_id}"),
                config_target: self
                    .runtime_home()
                    .join("agent-connections")
                    .join(connection_id)
                    .to_string_lossy()
                    .into_owned(),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: format!("fixture:{connection_id}"),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_connection_project(
            self.runtime_home(),
            ConnectionProjectRegistration {
                connection_internal_id: connection_id.to_owned(),
                project_id: self.project_id().to_owned(),
            },
        )?;
        Ok(())
    }

    fn install_guard_policy(&self) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_with(true, true, "configured")
    }

    fn install_guard_policy_with(
        &self,
        host_supports_prompt_capture: bool,
        prompt_capture_configured: bool,
        installation_status: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        self.install_guard_policy_for_connection(
            self.connection_id(),
            host_supports_prompt_capture,
            prompt_capture_configured,
            installation_status,
        )
    }

    fn install_guard_policy_for_connection(
        &self,
        connection_id: &str,
        host_supports_prompt_capture: bool,
        prompt_capture_configured: bool,
        installation_status: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let guard_installation_id = format!("guard_installation_cli_activation_{connection_id}");
        let policy = json!({
            "schema": "volicord-policy-v1",
            "managed_by": "volicord",
            "host": PROMPT_CAPTURE_TEST_HOST_KIND,
            "mode": "guarded",
            "guard_mode": "guarded",
            "connection_id": connection_id,
            "guard_installation_id": guard_installation_id
        });
        let policy_hash = sha256_text(&serde_json::to_string(&policy)?);
        let policy_dir = self.repo_root.join(".volicord");
        fs::create_dir_all(&policy_dir)?;
        fs::write(
            policy_dir.join("policy.json"),
            serde_json::to_string_pretty(&policy)?,
        )?;
        upsert_guard_installation(
            self.runtime_home(),
            GuardInstallationUpsert {
                guard_installation_id: guard_installation_id.clone(),
                connection_internal_id: connection_id.to_owned(),
                project_id: Some(self.project_id().to_owned()),
                host_kind: PROMPT_CAPTURE_TEST_HOST_KIND.to_owned(),
                guard_mode: "guarded".to_owned(),
                host_capability_json: json!({
                    "schema": "volicord-guard-capability-v1",
                    "policy_hash": policy_hash.clone(),
                    "host_capabilities": {
                        "user_prompt_submit_hook": host_supports_prompt_capture
                    },
                    "required_guard_phases": [
                        "session_start_hook",
                        "pre_tool_hook",
                        "post_tool_hook",
                        "user_prompt_submit_hook",
                        "stop_hook"
                    ],
                    "missing_required_hooks": [],
                    "prompt_capture": prompt_capture_configured
                })
                .to_string(),
                installation_status: installation_status.to_owned(),
                installed_at: Some("2026-06-30T03:59:00Z".to_owned()),
                last_checked_at: "2026-06-30T03:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok((guard_installation_id, policy_hash))
    }

    fn invocation(&self, operation_category: OperationCategory) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(self.project_id()),
            ActorSource::agent_connection(self.connection_id().to_owned()),
            operation_category,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
    }
}

#[cfg(unix)]
struct GuardedLifecycleFixture {
    _runtime_home: TempRuntimeHome,
    repo_root: PathBuf,
    repo_arg: String,
    project_id: String,
    connection_id: String,
    guard_installation_id: String,
    init_output: Value,
}

#[cfg(unix)]
impl GuardedLifecycleFixture {
    fn init(prefix: &str, mode: &str) -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let repo_root = runtime_home.create_product_repo("product-repo")?;
        fs::create_dir_all(repo_root.join(".git"))?;
        let repo_arg = repo_root
            .to_str()
            .ok_or("fixture product repo path should be UTF-8")?
            .to_owned();
        let bin_dir = runtime_home.path().join("bin");
        write_fake_codex(&bin_dir)?;
        write_fake_mcp(&bin_dir)?;

        let mut args = vec![
            "init",
            "--host",
            "codex",
            "--repo",
            repo_arg.as_str(),
            "--mode",
            mode,
        ];
        if mode != "mcp-only" {
            args.push("--allow-degraded");
        }
        args.push("--json");

        let output = Command::new(volicord_bin())
            .args(args)
            .env("VOLICORD_HOME", runtime_home.path())
            .env("PATH", path_env(&[bin_dir.as_path()]))
            .env("VOLICORD_TEST_CONNECTION_MODE", "workflow")
            .output()?;
        assert_success(&output);
        let init_output = json_stdout(&output)?;
        let connection_id = init_output["connection"]["connection_id"]
            .as_str()
            .expect("init should report connection_id")
            .to_owned();
        let projects = list_projects(runtime_home.path())?;
        assert_eq!(projects.len(), 1);
        let project_id = projects[0].project_id.clone();
        let guard_installations =
            list_guard_installations(runtime_home.path(), &connection_id, Some(&project_id))?;
        assert_eq!(guard_installations.len(), 1);
        let guard_installation_id = guard_installations[0].guard_installation_id.clone();
        mark_connection_verified(runtime_home.path(), &connection_id)?;

        Ok(Self {
            _runtime_home: runtime_home,
            repo_root,
            repo_arg,
            project_id,
            connection_id,
            guard_installation_id,
            init_output,
        })
    }

    fn runtime_home(&self) -> &Path {
        self._runtime_home.path()
    }

    fn repo_arg(&self) -> &str {
        &self.repo_arg
    }

    fn project_id(&self) -> &str {
        &self.project_id
    }

    fn connection_id(&self) -> &str {
        &self.connection_id
    }

    fn guard_installation_id(&self) -> &str {
        &self.guard_installation_id
    }

    fn session_id(&self) -> &str {
        "guard_lifecycle_session"
    }

    fn store(&self) -> Result<CoreProjectStore, Box<dyn Error>> {
        Ok(CoreProjectStore::open(
            self.runtime_home(),
            &ProjectId::new(&self.project_id),
        )?)
    }

    fn state_version(&self) -> Result<u64, Box<dyn Error>> {
        Ok(self.store()?.project_state()?.state_version)
    }

    fn service(&self) -> CoreService {
        CoreService::new(self.runtime_home())
    }

    fn invocation(&self, operation_category: OperationCategory) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(&self.project_id),
            ActorSource::agent_connection(self.connection_id.clone()),
            operation_category,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
    }

    fn user_invocation(&self) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(&self.project_id),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
    }

    fn envelope(
        &self,
        request_id: &str,
        idempotency_key: Option<&str>,
        expected_state_version: Option<u64>,
        task_id: Option<&str>,
    ) -> ToolEnvelope {
        ToolEnvelope {
            project_id: ProjectId::new(&self.project_id),
            task_id: task_id.map(TaskId::new).into(),
            request_id: RequestId::new(request_id),
            idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
            expected_state_version: expected_state_version.into(),
            dry_run: false,
            locale: Some("en-US".to_owned()).into(),
        }
    }

    fn activate_guard(&self, event_id: &str) -> Result<(), Box<dyn Error>> {
        let event = json!({
            "event_id": event_id,
            "session_id": self.session_id(),
            "connection_id": self.connection_id(),
            "guard_installation_id": self.guard_installation_id(),
            "host_kind": "codex",
            "timestamp": "2026-06-30T06:00:00Z"
        });
        let output = self.run_guard_event("session-start", &event)?;
        assert_success(&output);
        let value = json_stdout(&output)?;
        assert_eq!(value["decision"], "inject_context");
        let stored = guard_installation(self.runtime_home(), self.guard_installation_id())?
            .expect("guard installation should be stored");
        assert_eq!(stored.last_seen_phase.as_deref(), Some("session_start"));
        Ok(())
    }

    fn mark_required_hooks_supported(&self) -> Result<(), Box<dyn Error>> {
        let stored = guard_installation(self.runtime_home(), self.guard_installation_id())?
            .expect("guard installation should be stored");
        let mut capability = serde_json::from_str::<Value>(&stored.host_capability_json)?;
        capability["required_guard_phases"] = json!([
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ]);
        capability["missing_required_hooks"] = json!([]);
        capability["host_capabilities"] = json!({
            "user_prompt_submit_hook": true
        });
        capability["prompt_capture"] = json!(true);
        upsert_guard_installation(
            self.runtime_home(),
            GuardInstallationUpsert {
                guard_installation_id: stored.guard_installation_id,
                connection_internal_id: stored.connection_internal_id,
                project_id: Some(self.project_id.clone()),
                host_kind: stored.host_kind,
                guard_mode: stored.guard_mode,
                host_capability_json: capability.to_string(),
                installation_status: "reload_required".to_owned(),
                installed_at: stored.installed_at,
                last_checked_at: "2026-06-30T05:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: stored.metadata_json,
            },
        )?;
        Ok(())
    }

    fn run_guard_event(&self, phase: &str, event: &Value) -> Result<Output, Box<dyn Error>> {
        run_guard(
            self.runtime_home(),
            &self.repo_root,
            ["guard", phase, "--repo", self.repo_arg()],
            event,
        )
    }

    fn create_task_with_change_unit(
        &self,
        suffix: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let service = self.service();
        let intake = service.intake(
            IntakeRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_intake"),
                    Some(&format!("idem_{suffix}_intake")),
                    Some(0),
                    None,
                ),
                plain_language_request: "Create a guarded lifecycle fixture task.".to_owned(),
                requested_mode: RequestedMode::Work,
                resume_policy: ResumePolicy::CreateNew,
                initial_scope: InitialScope {
                    boundary: "Exercise guarded lifecycle behavior in a temp repository."
                        .to_owned(),
                    non_goals: vec!["Changing unrelated files.".to_owned()],
                    acceptance_criteria: vec![
                        "The guarded lifecycle reaches the expected close state.".to_owned(),
                    ],
                },
                initial_context_refs: Vec::new(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let task_id = record_id(&intake.response_value["task_ref"])?;
        let after_intake = self.state_version()?;
        let mut fields = serde_json::Map::new();
        fields.insert(
            "scope_summary".to_owned(),
            Value::String("Guarded lifecycle scope for src/export.rs.".to_owned()),
        );
        fields.insert("affected_paths".to_owned(), json!([DEFAULT_PRODUCT_PATH]));
        let scope = service.update_scope(
            UpdateScopeRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_scope"),
                    Some(&format!("idem_{suffix}_scope")),
                    Some(after_intake),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                goal_summary: Some("Guarded lifecycle task.".to_owned()).into(),
                scope_update: Some(ScopeUpdate {
                    include: vec!["Guarded lifecycle fixture behavior.".to_owned()],
                    exclude: vec!["Unrelated repository behavior.".to_owned()],
                })
                .into(),
                scope_boundary: Some("Stay within the temp Product Repository.".to_owned()).into(),
                non_goals: Some(vec!["Do not touch external user files.".to_owned()]).into(),
                acceptance_criteria: Some(vec![
                    "The fixture close check reports the expected state.".to_owned(),
                ])
                .into(),
                autonomy_boundary: Some("Use only fixture inputs.".to_owned()).into(),
                baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
                change_unit: ChangeUnitUpdate {
                    operation: ChangeUnitOperation::CreateCurrent,
                    effect_contract: None,
                    fields,
                },
                related_scope_decision_refs: Vec::new(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        let change_unit_id = record_id(&scope.response_value["change_unit_ref"])?;
        Ok((task_id, change_unit_id))
    }

    fn prepare_write(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let response = self.service().prepare_write(
            PrepareWriteRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_prepare"),
                    Some(&format!("idem_{suffix}_prepare")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                task_id: Some(TaskId::new(task_id)).into(),
                change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
                intended_operation: "local_product_file_update".to_owned(),
                intended_paths: vec![DEFAULT_PRODUCT_PATH.to_owned()],
                product_file_write_intended: true,
                sensitive_categories: Vec::new(),
                baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        assert_eq!(response.response_value["decision"], "allowed");
        Ok(response.response_value["write_check_ref"]["record_id"]
            .as_str()
            .expect("write check id should be present")
            .to_owned())
    }

    fn apply_product_change(&self, contents: &str) -> Result<(), Box<dyn Error>> {
        let path = self.repo_root.join(DEFAULT_PRODUCT_PATH);
        fs::create_dir_all(path.parent().expect("fixture path should have a parent"))?;
        fs::write(path, format!("{contents}\n"))?;
        Ok(())
    }

    fn record_product_write_close_basis(
        &self,
        task_id: &str,
        change_unit_id: &str,
        write_check_id: &str,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        self.record_close_basis(task_id, change_unit_id, Some(write_check_id), true, suffix)
    }

    fn record_non_write_close_basis(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        self.record_close_basis(task_id, change_unit_id, None, false, suffix)
    }

    fn record_close_basis(
        &self,
        task_id: &str,
        change_unit_id: &str,
        write_check_id: Option<&str>,
        product_write_observed: bool,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let request = RecordRunRequest {
            envelope: self.envelope(
                &format!("req_{suffix}_run"),
                Some(&format!("idem_{suffix}_run")),
                Some(self.state_version()?),
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            change_unit_id: ChangeUnitId::new(change_unit_id),
            kind: RunKind::Implementation,
            run_id: None.into(),
            baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
            write_check_id: write_check_id.map(WriteCheckId::new).into(),
            summary: "Recorded guarded lifecycle fixture run.".to_owned(),
            observed_changes: ObservedChanges {
                changed_paths: if product_write_observed {
                    vec![DEFAULT_PRODUCT_PATH.to_owned()]
                } else {
                    Vec::new()
                },
                product_file_write_observed: product_write_observed,
                sensitive_categories: Vec::new(),
                baseline_ref: Some(BaselineRef::new(DEFAULT_BASELINE_REF)).into(),
            },
            artifact_inputs: Vec::new(),
            evidence_updates: vec![supported_evidence_update(
                "Lifecycle close claim supported.",
            )],
            evidence_observations: Vec::new(),
            close_assessment: Some(CloseAssessmentInput {
                result_summary: "Lifecycle close claim supported.".to_owned(),
                result_refs: Vec::new(),
                residual_risks: Vec::new(),
                sensitive_categories: Vec::new(),
                recovery_constraints: Vec::new(),
            })
            .into(),
        };
        let response = self
            .service()
            .record_run(request, self.invocation(OperationCategory::AgentWorkflow))?;
        Ok(response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"))
    }

    fn request_final_acceptance(
        &self,
        task_id: &str,
        change_unit_id: &str,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let state_version = self.state_version()?;
        let response = self.service().request_user_judgment(
            RequestUserJudgmentRequest {
                envelope: self.envelope(
                    &format!("req_{suffix}_final"),
                    Some(&format!("idem_{suffix}_final")),
                    Some(state_version),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                change_unit_id: Some(ChangeUnitId::new(change_unit_id)).into(),
                judgment_kind: JudgmentKind::FinalAcceptance,
                presentation: JudgmentPresentation::Short,
                question: "Does the user accept the current close basis?".to_owned(),
                options: None.into(),
                context: UserJudgmentContext {
                    summary: "The guarded lifecycle fixture is ready for final acceptance."
                        .to_owned(),
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                    visible_risks: Vec::new(),
                    constraints: vec![
                        "This answer applies only to the current fixture close basis.".to_owned(),
                    ],
                },
                affected_refs: vec![self.state_ref(
                    StateRecordKind::Task,
                    task_id,
                    Some(task_id),
                    Some(state_version),
                )],
                sensitive_action_scope: None.into(),
                required_for: vec![JudgmentRequiredFor::CloseComplete],
                expires_at: None.into(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?;
        record_id(&response.response_value["user_judgment_ref"])
    }

    fn answer_pending_judgment_through_prompt(
        &self,
        task_id: &str,
        judgment_id: &str,
        event_id: &str,
        capture_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let records = self
            .store()?
            .user_judgment_records_for_task(&TaskId::new(task_id))?;
        let (index, record) = records
            .iter()
            .enumerate()
            .find(|(_, record)| record.judgment_id == judgment_id)
            .ok_or("pending judgment should be stored for task")?;
        let verification_code = chat_judgment_verification_code(
            &record.project_id,
            &record.task_id,
            &record.judgment_id,
            &record.requested_at,
            self.connection_id(),
        );
        let message = format!("Volicord: answer J-{} 1 {verification_code}", index + 1);
        let event = json!({
            "event_id": event_id,
            "prompt_capture_id": capture_id,
            "session_id": self.session_id(),
            "connection_id": self.connection_id(),
            "guard_installation_id": self.guard_installation_id(),
            "host_kind": "codex",
            "message": message,
            "timestamp": "2026-06-30T06:10:00Z"
        });
        let output = self.run_guard_event("prompt-capture", &event)?;
        assert_success(&output);
        let value = json_stdout(&output)?;
        assert_eq!(value["decision"], "inject_context");
        assert_eq!(
            value["result"]["recognized_judgment_command"]["resolution_outcome"],
            "accepted"
        );
        Ok(())
    }

    fn record_judgment_direct(
        &self,
        task_id: &str,
        judgment_id: &str,
        judgment_kind: JudgmentKind,
    ) -> Result<u64, Box<dyn Error>> {
        let response = self.service().record_user_judgment(
            RecordUserJudgmentRequest {
                envelope: self.envelope(
                    &format!("req_direct_record_{judgment_id}"),
                    Some(&format!("idem_direct_record_{judgment_id}")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                user_judgment_id: volicord_types::UserJudgmentId::new(judgment_id),
                judgment_kind,
                selected_option_id: UserJudgmentOptionId::new("accept"),
                answer: answer_payload(judgment_kind),
                rationale: JudgmentRationale {
                    summary: "The local user accepted the fixture judgment.".to_owned(),
                    selected_reason: Some(
                        "The fixture close basis was visible to the test user channel.".to_owned(),
                    )
                    .into(),
                    considered_alternatives: Vec::new(),
                    rejected_alternatives: Vec::new(),
                    assumptions: vec![
                        "This direct fixture answer covers only the pending judgment.".to_owned(),
                    ],
                    tradeoffs: vec![
                        "The fixture records acceptance only after the close basis is current."
                            .to_owned(),
                    ],
                    uncertainties: Vec::new(),
                    review_triggers: vec![
                        "Review if the fixture close basis changes before close.".to_owned(),
                    ],
                    related_refs: Vec::new(),
                    artifact_refs: Vec::new(),
                },
                note: Some("Recorded by guarded lifecycle fixture.".to_owned()).into(),
                accepted_risks: Vec::new(),
            },
            self.user_invocation(),
        )?;
        assert_eq!(
            response.response_value["base"]["response_kind"], "result",
            "{}",
            response.response_value
        );
        Ok(response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state version should be present"))
    }

    fn check_close(
        &self,
        task_id: &str,
    ) -> Result<volicord_core::PipelineResponse, Box<dyn Error>> {
        Ok(self.service().close_task(
            CloseTaskRequest {
                envelope: self.envelope(&format!("req_check_{task_id}"), None, None, Some(task_id)),
                task_id: TaskId::new(task_id),
                intent: CloseIntent::Check,
                close_reason: None.into(),
                superseding_task_id: None.into(),
                user_note: Some("Guarded lifecycle close check.".to_owned()).into(),
            },
            self.invocation(OperationCategory::Read),
        )?)
    }

    fn close_task(
        &self,
        task_id: &str,
        suffix: &str,
    ) -> Result<volicord_core::PipelineResponse, Box<dyn Error>> {
        Ok(self.service().close_task(
            CloseTaskRequest {
                envelope: self.envelope(
                    &format!("req_close_{suffix}"),
                    Some(&format!("idem_close_{suffix}")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked).into(),
                superseding_task_id: None.into(),
                user_note: Some("Guarded lifecycle close.".to_owned()).into(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?)
    }

    fn reconcile_changes(
        &self,
        task_id: &str,
        suffix: &str,
    ) -> Result<volicord_core::PipelineResponse, Box<dyn Error>> {
        Ok(self.service().reconcile_changes(
            ReconcileChangesRequest {
                envelope: self.envelope(
                    &format!("req_reconcile_{suffix}"),
                    Some(&format!("idem_reconcile_{suffix}")),
                    Some(self.state_version()?),
                    Some(task_id),
                ),
                task_id: TaskId::new(task_id),
                resolution_requests: Vec::new(),
            },
            self.invocation(OperationCategory::AgentWorkflow),
        )?)
    }

    fn state_ref(
        &self,
        record_kind: StateRecordKind,
        record_id: &str,
        task_id: Option<&str>,
        state_version: Option<u64>,
    ) -> StateRecordRef {
        StateRecordRef {
            record_kind,
            record_id: RecordId::new(record_id),
            project_id: ProjectId::new(&self.project_id),
            task_id: task_id.map(TaskId::new).into(),
            state_version: state_version.into(),
        }
    }
}

fn prompt_event(
    fixture: &GuardCliFixture,
    event_id: &str,
    capture_id: &str,
    message: &str,
) -> Value {
    json!({
        "event_id": event_id,
        "prompt_capture_id": capture_id,
        "session_id": "guard_session_chat",
        "connection_id": fixture.connection_id(),
        "host_kind": PROMPT_CAPTURE_TEST_HOST_KIND,
        "message": message
    })
}

fn run_guard<const N: usize>(
    runtime_home: &Path,
    current_dir: &Path,
    args: [&str; N],
    event: &Value,
) -> Result<Output, Box<dyn Error>> {
    let mut child = Command::new(volicord_bin())
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(event.to_string().as_bytes())?;
    Ok(child.wait_with_output()?)
}

fn run_guard_file<const N: usize>(
    runtime_home: &Path,
    current_dir: &Path,
    args: [&str; N],
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(volicord_bin())
        .args(args)
        .env("VOLICORD_HOME", runtime_home)
        .current_dir(current_dir)
        .output()?)
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

fn json_stdout(output: &Output) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&stdout(output))?)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn sha256_text(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("sha256:{digest:x}")
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
fn mark_connection_verified(
    runtime_home: &Path,
    connection_id: &str,
) -> Result<(), Box<dyn Error>> {
    let existing = agent_connection_record(runtime_home, connection_id)?
        .ok_or("initialized Agent Connection should be stored")?;
    ensure_agent_connection(
        runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: existing.connection_internal_id,
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: existing.config_target,
            mode: existing.mode,
            enabled: existing.enabled,
            managed_fingerprint: existing.managed_fingerprint,
            last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    Ok(())
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
fn assert_guard_init_state_is_installed_or_degraded(value: &Value) {
    let state = value["states"]["guard_installation"]
        .as_str()
        .expect("init output should include guard installation state");
    assert!(
        matches!(state, "configured" | "reload_required" | "degraded"),
        "unexpected guarded init state: {state}"
    );
}

fn assert_reason(value: &Value, code: &str) {
    assert!(
        value["result"]["reasons"]
            .as_array()
            .expect("reasons should be an array")
            .iter()
            .any(|reason| reason["code"] == code),
        "expected reason {code}, got {}",
        value["result"]["reasons"]
    );
}

fn assert_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().any(|candidate| candidate == code),
        "expected close blocker code {code}, got {codes:?}"
    );
}

fn assert_no_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().all(|candidate| candidate != code),
        "did not expect close blocker code {code}, got {codes:?}"
    );
}

fn close_blocker_codes(response_value: &Value) -> Vec<String> {
    response_value
        .get("blockers")
        .or_else(|| response_value.get("close_blockers"))
        .and_then(Value::as_array)
        .expect("blockers or close_blockers should be present")
        .iter()
        .filter_map(|blocker| blocker["code"].as_str().map(str::to_owned))
        .collect()
}

fn record_id(value: &Value) -> Result<String, Box<dyn Error>> {
    value["record_id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "record_id should be present".into())
}
