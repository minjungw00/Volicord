#![forbid(unsafe_code)]

mod support;

use std::{collections::BTreeSet, error::Error, fs, io::Read, path::Path};

#[cfg(unix)]
use std::{
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
    time::{Duration, SystemTime},
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::time::Instant;

#[cfg(unix)]
use chrono::{DateTime, SecondsFormat, Utc};

use serde_json::Value;
use sha2::{Digest, Sha256};
#[cfg(unix)]
use volicord_cli::host_integration::{
    managed_fingerprint, HostKind, HostScope, ManagedServerEntry, DEFAULT_SERVER_NAME,
    MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1,
};
use volicord_core::{CoreService, GitWorkspaceContext, InvocationContext};
use volicord_platform_fs::capture_git_workspace_snapshot;
use volicord_store::agent_connections::{
    add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
    ConnectionProjectRegistration, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
    VERIFIED_STATUS_COMPLETE,
};
use volicord_store::guards::{insert_unrecorded_change, UnrecordedChangeInsert};
use volicord_store::{
    bootstrap::{
        initialize_runtime_home, installation_profile, list_projects, register_project,
        write_installation_profile, InstallationProfileRegistration, ProjectRegistration,
        ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::CoreProjectStore,
    diagnostics::read_workflow_metric_aggregates,
};
use volicord_test_support::{
    core_fixtures::{CoreFixture, UpdateScopeFixture, DEFAULT_BASELINE_REF},
    TempRuntimeHome,
};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_bytes, AcceptanceCriterionInput, ActorSource,
    BaselineRef, ChangeUnitOperation, ConnectionObservationGuardEventKind,
    ConnectionObservationSourceSelector, EvidenceCaptureSpec, EvidenceRequirement, EvidenceTarget,
    IdempotencyKey, InitialScope, JudgmentKind, JudgmentPresentation, OperationCategory,
    PrepareEvidenceCaptureRequest, ProjectId, RequestId, RequestedControlLevel, RequestedMode,
    RequiredNullable, ResumePolicy, StateRecordKind, StateRecordRef, TaskId, ToolEnvelope,
    UserActionChoiceDraft, UserActionContext, UserActionDraft, UserActionOptionId,
    UserActionOptionInput, UserActionRequiredFor, UtcTimestamp,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

use support::{
    assertions::{assert_non_guarantees, assert_success, json_stdout, stderr, stdout},
    binary_fixture::{
        create_git_repo, path_text, run_with_home_env, run_with_home_env_in_dir, run_without_home,
        write_test_installation_profile,
    },
    json::record_id,
};

#[cfg(unix)]
use serde_json::json;

#[cfg(unix)]
use volicord_store::{
    agent_connections::{
        agent_connection_record, list_agent_connections, list_connection_projects,
        set_connection_enabled, update_agent_connection_verification_report,
        CONNECTION_MODE_READ_ONLY, VERIFIED_STATUS_ACTION_REQUIRED,
    },
    guards::{
        guard_event, insert_agent_session, insert_guard_event, list_guard_installations,
        upsert_guard_installation, AgentSessionInsert, GuardEventInsert, GuardInstallationUpsert,
    },
    session_watch::{
        compare_watch_snapshots, create_watch_baseline, record_watch_observation,
        snapshot_product_repository, watch_baseline, watch_observation, SessionWatchStatus,
        WatchBaselineCreate, WatchObservationInsert, WatchSnapshotOptions,
    },
};

#[cfg(unix)]
use volicord_types::RECONCILE_CHANGES_TOOL_NAME;

#[cfg(unix)]
use support::{
    binary_fixture::{create_real_git_repo, prepare_runtime_home, volicord_bin},
    fake_hosts::{
        hook_execution_path_env, is_executable, path_env, path_env_with_existing,
        write_fake_claude_code, write_fake_codex, write_fake_codex_with_version,
    },
    fake_mcp::{write_fake_mcp, write_fake_mcp_missing_workflow_reconcile},
    guard_fixture::{
        expand_claude_project_command, pre_tool_write_event, run_executable_hook_command,
        run_shell_hook_command,
    },
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
    assert!(text.contains("volicord evidence capture-command --intent ID"));
    assert!(text.contains("volicord connection status [HOST]"));
    assert!(text.contains("volicord changes reconcile"));
    assert!(text.contains("volicord serve --transport local-http"));
    assert!(text.contains("volicord mcp --stdio"));
    assert!(text.contains("volicord inbox resolve <user-action-request-id> --choice <choice>"));
    assert!(text.contains("User Channel"));

    let init_help = run_without_home(["init", "--help"])?;
    assert_success(&init_help);
    let init_text = stdout(&init_help);
    assert!(init_text.contains("volicord init --host codex|claude-code --repo PATH"));
    assert!(init_text.contains("--shared"));
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
            "--discover-repository",
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
            "--session",
            "--criterion",
            "--claim",
            "--artifact",
            "--summary",
            "--contradicted",
            "--intent",
            "--pre-event",
            "--post-event",
            "--guard-event",
            "--watch-observation",
            "--file",
            "--source-home",
            "--destination-home",
        ],
    )?;
    assert_help_options(
        ["mcp", "--help"],
        &[
            "--stdio",
            "--check",
            "--discover-repository",
            "--host",
            "--connection",
            "--project",
        ],
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
    assert_help_options(
        ["diagnostics", "--help"],
        &["--session", "--repo", "--json"],
    )?;
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
            "--shared",
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
        ["evidence", "--help"],
        &[
            "--intent",
            "--repo",
            "--json",
            "--pre-event",
            "--post-event",
            "--guard-event",
            "--watch-observation",
        ],
    )?;
    assert_help_options(
        ["inbox", "--help"],
        &[
            "--repo",
            "--task",
            "--choice",
            "--note",
            "--criterion",
            "--claim",
            "--artifact",
            "--summary",
            "--contradicted",
            "--json",
        ],
    )?;
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_records_complete_nonzero_receipt_without_raw_data_and_is_one_time(
) -> Result<(), Box<dyn Error>> {
    let marker_name = "command-executions.txt";
    let script = format!(
        "printf 'executed\\n' >> {marker_name}; printf \"$CAPTURE_SECRET\"; printf 'stderr-secret' >&2; exit 7"
    );
    let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), script.clone()];
    let (fixture, intent_id) = prepared_command_capture("cli-evidence-command", &argv)?;
    let before = fixture.counts()?;

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--json",
            "--",
            "/bin/sh",
            "-c",
            &script,
        ],
        &[("CAPTURE_SECRET", "environment-secret".to_owned())],
        &fixture.product_repo_path(),
    )?;
    assert_success(&output);
    let rendered = json_stdout(&output)?;
    assert_eq!(rendered["complete"], true);
    assert_eq!(rendered["observed_outcome"]["exit_code"], 7);
    let after = fixture.counts()?;
    assert_eq!(after.state_version, before.state_version);
    assert_eq!(
        after.evidence_capture_receipts,
        before.evidence_capture_receipts + 1
    );
    assert_eq!(after.artifact_staging, before.artifact_staging + 1);

    let receipt = fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .expect("receipt should exist");
    for forbidden in [
        script.as_str(),
        "environment-secret",
        "stderr-secret",
        marker_name,
    ] {
        assert!(!receipt.safe_receipt_json.contains(forbidden));
    }
    assert!(receipt.safe_receipt_json.contains("environment_not_bound"));
    assert_eq!(
        fs::read_to_string(fixture.product_repo_path().join(marker_name))?,
        "executed\n"
    );

    let duplicate = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--json",
            "--",
            "/bin/sh",
            "-c",
            &script,
        ],
        &[("CAPTURE_SECRET", "environment-secret".to_owned())],
        &fixture.product_repo_path(),
    )?;
    assert!(!duplicate.status.success());
    assert!(stderr(&duplicate).contains("already fulfilled"));
    assert_eq!(
        fs::read_to_string(fixture.product_repo_path().join(marker_name))?,
        "executed\n"
    );
    assert_eq!(fixture.counts()?, after);
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_uses_future_persisted_project_clock_for_receipt_times(
) -> Result<(), Box<dyn Error>> {
    let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), "exit 0".to_owned()];
    let (fixture, intent_id) =
        prepared_command_capture("cli-evidence-project-clock-receipt", &argv)?;
    let future_floor = "2999-07-13T12:34:56.789Z";
    let future_expiry = "2999-07-13T12:49:56.789Z";
    set_capture_intent_clock(
        &fixture,
        &intent_id,
        future_floor,
        future_expiry,
        future_floor,
    )?;

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--json",
            "--",
            "/bin/sh",
            "-c",
            "exit 0",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert_success(&output);

    let receipt = fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .expect("receipt should exist");
    assert_eq!(receipt.observed_at, future_floor);
    assert_eq!(receipt.created_at, future_floor);
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_rejects_project_clock_at_expiry_before_execution(
) -> Result<(), Box<dyn Error>> {
    let script = "printf 'ran' > expired-command-marker.txt";
    let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), script.to_owned()];
    let (fixture, intent_id) =
        prepared_command_capture("cli-evidence-project-clock-expired", &argv)?;
    let created_at = capture_intent_timestamp(&fixture, &intent_id, "created_at")?;
    let expires_at = capture_intent_timestamp(&fixture, &intent_id, "expires_at")?;
    set_capture_intent_clock(&fixture, &intent_id, &created_at, &expires_at, &expires_at)?;
    let before = fixture.counts()?;

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--",
            "/bin/sh",
            "-c",
            script,
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("intent has expired"));
    assert!(!fixture
        .product_repo_path()
        .join("expired-command-marker.txt")
        .exists());
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_rejects_corrupt_intent_window_before_execution(
) -> Result<(), Box<dyn Error>> {
    for case in [
        "unrepresentable_created_at",
        "unrepresentable_expires_at",
        "reversed",
        "extended_ttl",
    ] {
        let marker_name = format!("corrupt-intent-{case}-marker.txt");
        let script = format!("printf 'ran' > {marker_name}");
        let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), script.clone()];
        let (fixture, intent_id) =
            prepared_command_capture(&format!("cli-evidence-corrupt-intent-{case}"), &argv)?;
        let original_created_at = capture_intent_timestamp(&fixture, &intent_id, "created_at")?;
        let original_expires_at = capture_intent_timestamp(&fixture, &intent_id, "expires_at")?;
        let extended_expires_at = UtcTimestamp::parse(&original_expires_at)?
            .checked_add(chrono::Duration::minutes(1))?
            .to_string();
        let (created_at, expires_at) = match case {
            "unrepresentable_created_at" => {
                ("9999-12-31T23:59:59-23:59", original_expires_at.as_str())
            }
            "unrepresentable_expires_at" => {
                (original_created_at.as_str(), "9999-12-31T23:59:59-23:59")
            }
            "reversed" => (original_expires_at.as_str(), original_created_at.as_str()),
            "extended_ttl" => (original_created_at.as_str(), extended_expires_at.as_str()),
            _ => unreachable!(),
        };
        set_capture_intent_clock(
            &fixture,
            &intent_id,
            created_at,
            expires_at,
            &original_created_at,
        )?;
        let before = fixture.counts()?;

        let output = run_with_home_env_in_dir(
            fixture.runtime_home_path(),
            [
                "evidence",
                "capture-command",
                "--intent",
                &intent_id,
                "--",
                "/bin/sh",
                "-c",
                &script,
            ],
            &[],
            &fixture.product_repo_path(),
        )?;
        assert!(
            !output.status.success(),
            "case {case} unexpectedly succeeded"
        );
        assert!(
            stderr(&output).contains("is invalid"),
            "case {case} returned an unexpected error: {}",
            stderr(&output)
        );
        assert!(!fixture.product_repo_path().join(&marker_name).exists());
        assert!(fixture
            .store()?
            .evidence_capture_receipt_for_intent(&intent_id)?
            .is_none());
        assert_eq!(fixture.counts()?, before);
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_caps_runtime_with_future_project_clock_remaining_ttl(
) -> Result<(), Box<dyn Error>> {
    let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), "sleep 5".to_owned()];
    let (fixture, intent_id) =
        prepared_command_capture("cli-evidence-project-clock-deadline", &argv)?;
    set_capture_intent_clock(
        &fixture,
        &intent_id,
        "2999-07-13T12:34:56.000Z",
        "2999-07-13T12:49:56.000Z",
        "2999-07-13T12:49:55.900Z",
    )?;
    let before = fixture.counts()?;

    let started = Instant::now();
    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--",
            "/bin/sh",
            "-c",
            "sleep 5",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("did not finish before"));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(fixture.counts()?, before);
    assert!(fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .is_none());
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_digest_mismatch_has_no_source_effects_or_execution(
) -> Result<(), Box<dyn Error>> {
    let intended_script = "printf 'should-not-run' > intended-marker.txt".to_owned();
    let intended_argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), intended_script];
    let (fixture, intent_id) = prepared_command_capture("cli-evidence-digest", &intended_argv)?;
    let before = fixture.counts()?;
    let mismatched_script = "printf 'mismatch-ran' > mismatch-marker.txt";

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--",
            "/bin/sh",
            "-c",
            mismatched_script,
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("digest does not match"));
    assert!(!fixture
        .product_repo_path()
        .join("mismatch-marker.txt")
        .exists());
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_output_budget_rejects_without_receipt() -> Result<(), Box<dyn Error>> {
    let argv = vec![
        "/usr/bin/head".to_owned(),
        "-c".to_owned(),
        (16 * 1024 * 1024 + 1_u64).to_string(),
        "/dev/zero".to_owned(),
    ];
    let (fixture, intent_id) = prepared_command_capture("cli-evidence-output-budget", &argv)?;
    let before = fixture.counts()?;
    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &intent_id,
            "--",
            &argv[0],
            &argv[1],
            &argv[2],
            &argv[3],
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("output exceeded"));
    assert_eq!(fixture.counts()?, before);
    assert!(fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .is_none());
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn evidence_capture_command_rejects_disabled_connection_and_stale_workspace_before_execution(
) -> Result<(), Box<dyn Error>> {
    let script = "printf 'ran' > should-not-run.txt".to_owned();
    let argv = vec!["/bin/sh".to_owned(), "-c".to_owned(), script.clone()];

    let (disabled_fixture, disabled_intent) =
        prepared_command_capture("cli-evidence-disabled", &argv)?;
    set_connection_enabled(
        disabled_fixture.runtime_home_path(),
        disabled_fixture.connection_id(),
        false,
    )?;
    let disabled_before = disabled_fixture.counts()?;
    let disabled = run_with_home_env_in_dir(
        disabled_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &disabled_intent,
            "--",
            "/bin/sh",
            "-c",
            &script,
        ],
        &[],
        &disabled_fixture.product_repo_path(),
    )?;
    assert!(!disabled.status.success());
    assert!(stderr(&disabled).contains("disabled"));
    assert!(!disabled_fixture
        .product_repo_path()
        .join("should-not-run.txt")
        .exists());
    assert_eq!(disabled_fixture.counts()?, disabled_before);

    let (stale_fixture, stale_intent) =
        prepared_command_capture("cli-evidence-stale-workspace", &argv)?;
    fs::write(
        stale_fixture.product_repo_path().join(".git/HEAD"),
        "ref: refs/heads/other\n",
    )?;
    let stale_before = stale_fixture.counts()?;
    let stale = run_with_home_env_in_dir(
        stale_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-command",
            "--intent",
            &stale_intent,
            "--",
            "/bin/sh",
            "-c",
            &script,
        ],
        &[],
        &stale_fixture.product_repo_path(),
    )?;
    assert!(!stale.status.success());
    assert!(stderr(&stale).contains("workspace context changed"));
    assert!(!stale_fixture
        .product_repo_path()
        .join("should-not-run.txt")
        .exists());
    assert_eq!(stale_fixture.counts()?, stale_before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_tool_requires_exact_complete_pre_post_pair_and_keeps_raw_result_out(
) -> Result<(), Box<dyn Error>> {
    let tool_input = json!({"command": "raw-tool-input-secret"});
    let input_sha256 = bare_canonical_sha256(&tool_input)?;
    let (fixture, intent_id) = prepared_capture(
        "cli-evidence-tool",
        EvidenceCaptureSpec::VerifiedToolInvocation {
            tool_name: "Bash".to_owned(),
            tool_input_sha256: input_sha256.clone(),
            expected_success: RequiredNullable::null(),
        },
        Some("session_evidence_tool"),
        Some("guard_evidence_tool"),
    )?;
    let intent_created_at = volicord_types::UtcTimestamp::parse(&capture_intent_timestamp(
        &fixture,
        &intent_id,
        "created_at",
    )?)?;
    let source_timestamp = volicord_types::UtcTimestamp::from_datetime(
        *intent_created_at.as_datetime() + chrono::Duration::microseconds(500),
    )
    .to_string();
    insert_tool_guard_event(
        &fixture,
        "guard_event_tool_pre",
        "pre_tool",
        "session_evidence_tool",
        "guard_evidence_tool",
        json!({
            "tool_name": "Bash",
            "tool_use_id": "tool-use-exact",
            "tool_input": tool_input,
        }),
        &input_sha256,
        None,
        &source_timestamp,
    )?;
    let tool_response = json!({
        "success": false,
        "exit_code": 3,
        "stdout": "raw-tool-output-secret"
    });
    insert_tool_guard_event(
        &fixture,
        "guard_event_tool_post",
        "post_tool",
        "session_evidence_tool",
        "guard_evidence_tool",
        json!({
            "tool_name": "Bash",
            "tool_use_id": "tool-use-exact",
            "tool_input": {"command": "raw-tool-input-secret"},
            "tool_response": tool_response,
        }),
        &input_sha256,
        Some(&bare_canonical_sha256(&tool_response)?),
        &source_timestamp,
    )?;

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-tool",
            "--intent",
            &intent_id,
            "--pre-event",
            "guard_event_tool_pre",
            "--post-event",
            "guard_event_tool_post",
            "--json",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert_success(&output);
    let rendered = json_stdout(&output)?;
    assert_eq!(rendered["observed_outcome"]["success"], false);
    assert_eq!(rendered["observed_outcome"]["exit_code"], 3);
    assert_eq!(rendered["observed_at"], source_timestamp);
    assert_eq!(
        rendered["observed_outcome"]["tool_result_size_bytes"],
        canonical_json_bytes(&tool_response)?.len() as u64
    );
    let receipt = fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .expect("tool receipt");
    assert!(!receipt.safe_receipt_json.contains("raw-tool-input-secret"));
    assert!(!receipt.safe_receipt_json.contains("raw-tool-output-secret"));
    assert!(receipt.safe_receipt_json.contains("tool-use-exact"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_tool_rejects_persisted_v1_guard_capability_without_receipt(
) -> Result<(), Box<dyn Error>> {
    let tool_input = json!({"path": "src/lib.rs"});
    let input_sha256 = bare_canonical_sha256(&tool_input)?;
    let (fixture, intent_id) = prepared_capture(
        "cli-evidence-tool-v1-capability",
        EvidenceCaptureSpec::VerifiedToolInvocation {
            tool_name: "Read".to_owned(),
            tool_input_sha256: input_sha256.clone(),
            expected_success: RequiredNullable::null(),
        },
        Some("session_evidence_tool_v1"),
        Some("guard_evidence_tool_v1"),
    )?;
    let source_timestamp = capture_intent_timestamp(&fixture, &intent_id, "created_at")?;
    insert_tool_guard_event(
        &fixture,
        "guard_event_tool_v1_pre",
        "pre_tool",
        "session_evidence_tool_v1",
        "guard_evidence_tool_v1",
        json!({
            "tool_name": "Read",
            "tool_use_id": "tool-use-v1",
            "tool_input": tool_input,
        }),
        &input_sha256,
        None,
        &source_timestamp,
    )?;
    let tool_response = json!({"success": true, "exit_code": 0});
    insert_tool_guard_event(
        &fixture,
        "guard_event_tool_v1_post",
        "post_tool",
        "session_evidence_tool_v1",
        "guard_evidence_tool_v1",
        json!({
            "tool_name": "Read",
            "tool_use_id": "tool-use-v1",
            "tool_input": {"path": "src/lib.rs"},
            "tool_response": tool_response,
        }),
        &input_sha256,
        Some(&bare_canonical_sha256(&tool_response)?),
        &source_timestamp,
    )?;
    let mut capability = evidence_guard_capability(&fixture, "guard_evidence_tool_v1", "shared");
    capability["schema"] = json!("volicord-host-hook-capability-v1");
    overwrite_guard_capability(&fixture, "guard_evidence_tool_v1", &capability)?;
    let before = fixture.counts()?;

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-tool",
            "--intent",
            &intent_id,
            "--pre-event",
            "guard_event_tool_v1_pre",
            "--post-event",
            "guard_event_tool_v1_post",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("host_capability_json"));
    assert_eq!(fixture.counts()?, before);
    assert!(fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_tool_rejects_post_only_and_mismatched_invocation_without_receipt(
) -> Result<(), Box<dyn Error>> {
    let tool_input = json!({"path": "src/lib.rs"});
    let input_sha256 = bare_canonical_sha256(&tool_input)?;
    let (fixture, intent_id) = prepared_capture(
        "cli-evidence-tool-mismatch",
        EvidenceCaptureSpec::VerifiedToolInvocation {
            tool_name: "Read".to_owned(),
            tool_input_sha256: input_sha256.clone(),
            expected_success: RequiredNullable::null(),
        },
        Some("session_evidence_tool_mismatch"),
        Some("guard_evidence_tool_mismatch"),
    )?;
    let source_timestamp = capture_intent_timestamp(&fixture, &intent_id, "created_at")?;
    insert_tool_guard_event(
        &fixture,
        "guard_event_only_post",
        "post_tool",
        "session_evidence_tool_mismatch",
        "guard_evidence_tool_mismatch",
        json!({
            "tool_name": "Read",
            "tool_use_id": "tool-use-post",
            "tool_input": tool_input,
            "tool_response": {"success": true}
        }),
        &input_sha256,
        None,
        &source_timestamp,
    )?;
    let before = fixture.counts()?;
    let post_only = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-tool",
            "--intent",
            &intent_id,
            "--pre-event",
            "missing-pre-event",
            "--post-event",
            "guard_event_only_post",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!post_only.status.success());
    assert_eq!(fixture.counts()?, before);

    insert_tool_guard_event(
        &fixture,
        "guard_event_mismatch_pre",
        "pre_tool",
        "session_evidence_tool_mismatch",
        "guard_evidence_tool_mismatch",
        json!({
            "tool_name": "Read",
            "tool_use_id": "tool-use-pre",
            "tool_input": {"path": "src/lib.rs"}
        }),
        &input_sha256,
        None,
        &source_timestamp,
    )?;
    let mismatched = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-tool",
            "--intent",
            &intent_id,
            "--pre-event",
            "guard_event_mismatch_pre",
            "--post-event",
            "guard_event_only_post",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!mismatched.status.success());
    assert!(stderr(&mismatched).contains("invocation IDs do not match"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_tool_rejects_sources_outside_intent_window() -> Result<(), Box<dyn Error>> {
    let tool_input = json!({"path": "src/lib.rs"});
    let input_sha256 = bare_canonical_sha256(&tool_input)?;
    let (fixture, intent_id) = prepared_capture(
        "cli-evidence-tool-source-window",
        EvidenceCaptureSpec::VerifiedToolInvocation {
            tool_name: "Read".to_owned(),
            tool_input_sha256: input_sha256.clone(),
            expected_success: RequiredNullable::null(),
        },
        Some("session_evidence_tool_window"),
        Some("guard_evidence_tool_window"),
    )?;

    for (suffix, occurred_at) in [
        ("old", "2000-01-01T00:00:00Z".to_owned()),
        (
            "expiry",
            capture_intent_timestamp(&fixture, &intent_id, "expires_at")?,
        ),
    ] {
        let invocation_id = format!("tool-use-{suffix}");
        let pre_id = format!("guard_event_window_{suffix}_pre");
        let post_id = format!("guard_event_window_{suffix}_post");
        insert_tool_guard_event(
            &fixture,
            &pre_id,
            "pre_tool",
            "session_evidence_tool_window",
            "guard_evidence_tool_window",
            json!({
                "tool_name": "Read",
                "tool_use_id": invocation_id,
                "tool_input": tool_input.clone(),
            }),
            &input_sha256,
            None,
            &occurred_at,
        )?;
        let tool_response = json!({"success": true, "exit_code": 0});
        insert_tool_guard_event(
            &fixture,
            &post_id,
            "post_tool",
            "session_evidence_tool_window",
            "guard_evidence_tool_window",
            json!({
                "tool_name": "Read",
                "tool_use_id": format!("tool-use-{suffix}"),
                "tool_input": {"path": "src/lib.rs"},
                "tool_response": tool_response,
            }),
            &input_sha256,
            Some(&bare_canonical_sha256(&tool_response)?),
            &occurred_at,
        )?;
        let before = fixture.counts()?;
        let output = run_with_home_env_in_dir(
            fixture.runtime_home_path(),
            [
                "evidence",
                "capture-tool",
                "--intent",
                &intent_id,
                "--pre-event",
                &pre_id,
                "--post-event",
                &post_id,
            ],
            &[],
            &fixture.product_repo_path(),
        )?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("outside the capture-intent source window"));
        assert_eq!(fixture.counts()?, before);
        assert!(fixture
            .store()?
            .evidence_capture_receipt_for_intent(&intent_id)?
            .is_none());
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_connection_accepts_exact_registered_guard_event() -> Result<(), Box<dyn Error>>
{
    let redacted_event = json!({
        "hook_event_name": "Stop",
        "status": "complete",
        "content": {"omitted": true, "sha256": "sha256:redacted"}
    });
    let expected_observation_sha256 = bare_canonical_sha256(&redacted_event)?;
    let (fixture, intent_id) = prepared_capture(
        "cli-evidence-connection-guard",
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_selector: ConnectionObservationSourceSelector::GuardEvent {
                event_kind: ConnectionObservationGuardEventKind::Stop,
            },
            expected_complete: RequiredNullable::null(),
        },
        Some("session_evidence_connection"),
        Some("guard_evidence_connection"),
    )?;
    let source_timestamp = capture_intent_timestamp(&fixture, &intent_id, "created_at")?;
    insert_tool_guard_event(
        &fixture,
        "guard_event_connection_source",
        "stop",
        "session_evidence_connection",
        "guard_evidence_connection",
        redacted_event.clone(),
        &"0".repeat(64),
        None,
        &source_timestamp,
    )?;
    insert_tool_guard_event(
        &fixture,
        "guard_event_connection_wrong_kind",
        "pre_tool",
        "session_evidence_connection",
        "guard_evidence_connection",
        redacted_event,
        &"0".repeat(64),
        None,
        &source_timestamp,
    )?;

    let before_wrong_kind = fixture.counts()?;
    let wrong_kind = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &intent_id,
            "--guard-event",
            "guard_event_connection_wrong_kind",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert!(!wrong_kind.status.success());
    assert!(stderr(&wrong_kind).contains("does not match the intent source selector"));
    assert_eq!(fixture.counts()?, before_wrong_kind);

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &intent_id,
            "--guard-event",
            "guard_event_connection_source",
            "--json",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;
    assert_success(&output);
    let rendered = json_stdout(&output)?;
    let receipt = fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .ok_or("guard capture receipt should exist")?;
    let stored_outcome = serde_json::from_str::<Value>(&receipt.observed_outcome_json)?;
    let stored_body = serde_json::from_str::<Value>(&receipt.safe_receipt_json)?;
    assert_eq!(rendered["observed_outcome"], stored_outcome);
    assert_eq!(
        stored_outcome,
        json!({
            "complete": true,
            "guard_event_kind": "stop",
            "guard_decision": "allow",
            "observation_sha256": expected_observation_sha256,
        })
    );
    assert_eq!(
        receipt.result_sha256,
        bare_canonical_sha256(&stored_outcome)?
    );
    assert_eq!(
        stored_body["source"],
        json!({
            "connection_id": fixture.connection_id(),
            "session_id": "session_evidence_connection",
            "guard_installation_id": "guard_evidence_connection",
            "guard_event_ids": ["guard_event_connection_source"],
            "watch_observation_refs": [],
            "host_invocation_id": null,
        })
    );
    assert_eq!(rendered["observed_at"], source_timestamp);
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_connection_rejects_persisted_guard_owner_intent_mismatch(
) -> Result<(), Box<dyn Error>> {
    let (fixture, intent_id) = prepared_capture(
        "cli-evidence-connection-binding-mismatch",
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_selector: ConnectionObservationSourceSelector::GuardEvent {
                event_kind: ConnectionObservationGuardEventKind::Stop,
            },
            expected_complete: RequiredNullable::null(),
        },
        Some("session_evidence_connection_binding"),
        Some("guard_evidence_connection_binding"),
    )?;
    let source_timestamp = capture_intent_timestamp(&fixture, &intent_id, "created_at")?;
    insert_tool_guard_event(
        &fixture,
        "guard_event_connection_binding",
        "stop",
        "session_evidence_connection_binding",
        "guard_evidence_connection_binding",
        json!({
            "hook_event_name": "Stop",
            "status": "complete",
            "content": {"omitted": true, "sha256": "sha256:redacted"}
        }),
        &"0".repeat(64),
        None,
        &source_timestamp,
    )?;
    let capability =
        evidence_guard_capability(&fixture, "guard_evidence_connection_binding", "personal");
    assert!(volicord_types::host_hook_capability_has_exact_v2_shape(
        &capability
    ));
    overwrite_guard_capability(&fixture, "guard_evidence_connection_binding", &capability)?;
    let before = fixture.counts()?;

    let output = run_with_home_env_in_dir(
        fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &intent_id,
            "--guard-event",
            "guard_event_connection_binding",
        ],
        &[],
        &fixture.product_repo_path(),
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("host_capability_json"));
    assert_eq!(fixture.counts()?, before);
    assert!(fixture
        .store()?
        .evidence_capture_receipt_for_intent(&intent_id)?
        .is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn evidence_capture_connection_accepts_complete_watcher_and_rejects_degraded_scan(
) -> Result<(), Box<dyn Error>> {
    let (complete_fixture, complete_intent, complete_observation) =
        prepared_watch_capture("cli-evidence-watch-complete", false)?;
    let selected_observation = watch_observation(
        complete_fixture.runtime_home_path(),
        complete_fixture.project_id(),
        &complete_observation,
    )?
    .ok_or("watch fixture observation should exist")?;
    let selection = json!({
        "watch_observation_id": &selected_observation.watch_observation_id,
        "watch_baseline_id": &selected_observation.watch_baseline_id,
        "session_id": &selected_observation.session_id,
        "connection_id": &selected_observation.connection_internal_id,
        "snapshot_algorithm": &selected_observation.snapshot_algorithm,
        "snapshot_digest": &selected_observation.snapshot_digest,
        "snapshot_entries": serde_json::from_str::<Value>(
            &selected_observation.snapshot_entries_json,
        )?,
        "observed_paths": serde_json::from_str::<Value>(
            &selected_observation.observed_paths_json,
        )?,
        "change_summary": serde_json::from_str::<Value>(
            &selected_observation.change_summary_json,
        )?,
        "observed_at": &selected_observation.observed_at,
    });
    let expected_observation_sha256 = bare_canonical_sha256(&selection)?;
    let complete = run_with_home_env_in_dir(
        complete_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &complete_intent,
            "--watch-observation",
            &complete_observation,
            "--json",
        ],
        &[],
        &complete_fixture.product_repo_path(),
    )?;
    assert_success(&complete);
    let rendered = json_stdout(&complete)?;
    let receipt = complete_fixture
        .store()?
        .evidence_capture_receipt_for_intent(&complete_intent)?
        .ok_or("watcher capture receipt should exist")?;
    let stored_outcome = serde_json::from_str::<Value>(&receipt.observed_outcome_json)?;
    let stored_body = serde_json::from_str::<Value>(&receipt.safe_receipt_json)?;
    assert_eq!(
        rendered["observed_outcome"], stored_outcome,
        "rendered watcher outcome must be the stored receipt outcome"
    );
    assert_eq!(
        stored_outcome,
        json!({
            "complete": true,
            "snapshot_algorithm": selected_observation.snapshot_algorithm,
            "snapshot_digest": selected_observation.snapshot_digest,
            "observation_sha256": expected_observation_sha256,
        })
    );
    assert_eq!(
        receipt.result_sha256,
        bare_canonical_sha256(&stored_outcome)?
    );
    assert_eq!(
        stored_body["source"],
        json!({
            "connection_id": complete_fixture.connection_id(),
            "session_id": "session_evidence_watch",
            "guard_installation_id": null,
            "guard_event_ids": [],
            "watch_observation_refs": [&complete_observation],
            "host_invocation_id": null,
        })
    );

    let (superseded_fixture, superseded_intent, superseded_observation) =
        prepared_watch_capture("cli-evidence-watch-superseded-baseline", false)?;
    let original_baseline = watch_baseline(
        superseded_fixture.runtime_home_path(),
        superseded_fixture.project_id(),
        "watch_baseline_evidence",
    )?
    .ok_or("watch fixture baseline should exist")?;
    let later_baseline_at = UtcTimestamp::from_datetime(
        *UtcTimestamp::parse(&original_baseline.updated_at)?.as_datetime()
            + chrono::Duration::nanoseconds(1),
    )
    .to_canonical_string();
    let current_snapshot = snapshot_product_repository(
        superseded_fixture.runtime_home_path(),
        superseded_fixture.product_repo_path(),
        WatchSnapshotOptions {
            watch_paths: vec!["watch.txt".into()],
            ..WatchSnapshotOptions::default()
        },
    )?;
    create_watch_baseline(
        superseded_fixture.runtime_home_path(),
        superseded_fixture.project_id(),
        WatchBaselineCreate {
            watch_baseline_id: "watch_baseline_new_current".to_owned(),
            session_id: "session_evidence_watch".to_owned(),
            connection_internal_id: superseded_fixture.connection_id().to_owned(),
            guard_installation_id: None,
            status: SessionWatchStatus::Active,
            snapshot: current_snapshot,
            created_at: later_baseline_at,
            metadata_json: "{}".to_owned(),
        },
    )?;
    let superseded_before = superseded_fixture.counts()?;
    let superseded = run_with_home_env_in_dir(
        superseded_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &superseded_intent,
            "--watch-observation",
            &superseded_observation,
        ],
        &[],
        &superseded_fixture.product_repo_path(),
    )?;
    assert!(!superseded.status.success());
    assert!(stderr(&superseded).contains("does not belong to the current baseline"));
    assert_eq!(superseded_fixture.counts()?, superseded_before);

    let (degraded_fixture, degraded_intent, degraded_observation) =
        prepared_watch_capture("cli-evidence-watch-degraded", true)?;
    let before = degraded_fixture.counts()?;
    let degraded = run_with_home_env_in_dir(
        degraded_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &degraded_intent,
            "--watch-observation",
            &degraded_observation,
        ],
        &[],
        &degraded_fixture.product_repo_path(),
    )?;
    assert!(!degraded.status.success());
    assert!(stderr(&degraded).contains("incomplete or degraded"));
    assert_eq!(degraded_fixture.counts()?, before);

    let (baseline_fixture, baseline_intent, baseline_observation) =
        prepared_watch_capture_with_degradation(
            "cli-evidence-watch-baseline-degraded",
            true,
            false,
        )?;
    let baseline_before = baseline_fixture.counts()?;
    let baseline_degraded = run_with_home_env_in_dir(
        baseline_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &baseline_intent,
            "--watch-observation",
            &baseline_observation,
        ],
        &[],
        &baseline_fixture.product_repo_path(),
    )?;
    assert!(!baseline_degraded.status.success());
    assert!(stderr(&baseline_degraded).contains("baseline is incomplete"));
    assert_eq!(baseline_fixture.counts()?, baseline_before);

    let (tampered_fixture, tampered_intent, tampered_observation) =
        prepared_watch_capture("cli-evidence-watch-integrity", false)?;
    tampered_fixture.conn()?.execute(
        "UPDATE session_watch_observations
            SET snapshot_digest = ?3,
                observed_paths_json = '[\"not-the-derived-path\"]'
          WHERE project_id = ?1 AND watch_observation_id = ?2",
        rusqlite::params![
            tampered_fixture.project_id(),
            tampered_observation,
            "f".repeat(64)
        ],
    )?;
    let tampered_before = tampered_fixture.counts()?;
    let tampered = run_with_home_env_in_dir(
        tampered_fixture.runtime_home_path(),
        [
            "evidence",
            "capture-connection",
            "--intent",
            &tampered_intent,
            "--watch-observation",
            &tampered_observation,
        ],
        &[],
        &tampered_fixture.product_repo_path(),
    )?;
    assert!(!tampered.status.success());
    assert!(stderr(&tampered).contains("integrity validation failed"));
    assert_eq!(tampered_fixture.counts()?, tampered_before);
    Ok(())
}

#[test]
fn export_help_lists_authority_bundle() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(["export", "--help"])?;
    assert_success(&output);
    let text = stdout(&output);

    assert!(text.contains("volicord export authority-bundle --output PATH"));
    Ok(())
}

#[test]
fn export_authority_bundle_help_shows_authority_bundle_usage() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(["export", "authority-bundle", "--help"])?;
    assert_success(&output);
    let text = stdout(&output);

    assert!(text.contains("volicord export authority-bundle --output PATH"));
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
    assert_eq!(value["build_id"], volicord_mcp::build_id());
    assert_eq!(
        value["build"],
        serde_json::to_value(volicord_mcp::build_info())?
    );
    assert_eq!(value["states"]["build_id"], volicord_mcp::build_id());
    let build_check = value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "build_identity")
        .expect("build identity check");
    let build = volicord_mcp::build_info();
    let exact_clean = build.git_commit != "unknown"
        && build.git_dirty == Some(false)
        && build.metadata_source != "unknown"
        && build.profile_exact
        && build.build_profile.is_some()
        && build.target_triple != "unknown"
        && build.opt_level != "unknown"
        && build.debug.is_some();
    assert_eq!(
        build_check["status"],
        if exact_clean { "passed" } else { "warning" }
    );
    assert_eq!(
        build_check["details"]["metadata_source"],
        build.metadata_source
    );
    assert_eq!(build_check["details"]["profile_exact"], build.profile_exact);
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
        "Initialize the primary host connection from the Product Repository."
    );
    assert_eq!(
        value["primary_next_action"]["command"],
        "volicord init --host <host> --repo <path>"
    );
    assert_eq!(value["states"]["prompt_capture_status"], "not_checked");
    let doctor_text = run_with_home_env(runtime_home.path(), ["doctor"], &[])?;
    assert_success(&doctor_text);
    let text = stdout(&doctor_text);
    assert!(text.contains("Volicord doctor action_required"));
    assert!(text.contains(&format!("Build:\n  {}", volicord_mcp::build_id())));
    assert!(text.contains("Task lifecycle: not shown in this view"));
    assert!(
        text.contains("Volicord record effect for this command: local diagnostic observation only")
    );
    assert!(text.contains("Pending user actions: not shown in this view"));
    assert!(text.contains("Close readiness: not shown in this view"));
    assert!(text.contains(
        "Primary next action: Initialize the primary host connection from the Product Repository"
    ));
    assert!(text.contains("Status:\n  Installation profile: local init or profile repair is required before Volicord workflows are usable"));
    assert!(text.contains("Runtime Home: ready"));
    assert!(text.contains("Installation profile: missing or invalid"));
    assert!(text.contains("MCP configuration: unknown"));
    assert!(text.contains("Prompt capture: not checked"));
    assert!(text.contains(
        "Next:\n  1. Initialize the primary host connection from the Product Repository"
    ));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &text,
        &[
            "volicord init --host <host> --repo <path>",
            "volicord doctor --json",
        ],
    );
    assert_non_connection_text_omits_diagnostic_dump_fields(&text);
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
fn init_defaults_to_personal_codex_connection() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-personal-codex")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

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
            "--mcp-command",
            path_text(&mcp_command).as_str(),
        ],
        &env,
    )?;
    assert_success(&text_output);
    let text = stdout(&text_output);
    assert!(text.contains("Connection:\n  intent: personal\n  host scope: user"));
    assert!(text.contains(&format!(
        "volicord connection verify codex --repo {}",
        repo_root.display()
    )));
    assert!(text.contains(&format!(
        "volicord connection status codex --repo {} --json",
        repo_root.display()
    )));
    assert!(!text.contains("connection verify codex --shared"));
    assert!(!text.contains("connection status codex --shared"));
    fs::set_permissions(
        repo_root.join(".volicord/policy.json"),
        fs::Permissions::from_mode(0o644),
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
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["connection"]["connection_intent"], "personal");
    assert_eq!(value["connection"]["host_scope"], "user");
    assert_eq!(
        value["connection"]["config_target"],
        path_text(&codex_home.join("config.toml"))
    );
    assert!(codex_home.join("config.toml").exists());
    assert!(!repo_root.join(".codex/config.toml").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    let exclude = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert_eq!(
        exclude
            .matches("# BEGIN VOLICORD MANAGED LOCAL EXCLUDES")
            .count(),
        1
    );
    assert!(exclude.contains("/.volicord/"));
    assert!(exclude.contains("/.codex/hooks/volicord-pre-tool.sh"));
    assert!(!exclude.contains("/.codex/\n"));
    assert!(!exclude.contains("/.gitignore"));
    assert_eq!(
        fs::metadata(repo_root.join(".volicord/policy.json"))?
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        value["primary_next_action"]["command"],
        format!(
            "volicord connection verify codex --repo {}",
            repo_root.display()
        )
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn doctor_warns_when_personal_local_files_are_unignored_or_tracked() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-doctor-personal-git-index")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let test_path = path_env_with_existing(&[bin_dir.as_path()])?;
    let env = [
        ("PATH", test_path),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);

    let protected = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&protected);
    let protected = json_stdout(&protected)?;
    let protected_check = protected["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "personal_local_git_tracking")
        .expect("personal Git tracking check");
    assert_eq!(
        protected_check["status"], "passed",
        "unexpected protected-path diagnostic: {protected_check:#}"
    );
    assert_eq!(protected_check["details"]["tracked_paths"], json!([]));
    assert_eq!(
        protected_check["details"]["unignored_existing_paths"],
        json!([])
    );

    let exclude_path = repo_root.join(".git/info/exclude");
    let managed_excludes = fs::read_to_string(&exclude_path)?;
    fs::write(&exclude_path, "")?;
    let unignored = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&unignored);
    let unignored = json_stdout(&unignored)?;
    let unignored_check = unignored["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "personal_local_git_tracking")
        .expect("personal Git tracking check");
    assert_eq!(unignored_check["status"], "warning");
    assert!(unignored_check["details"]["unignored_existing_paths"]
        .as_array()
        .expect("unignored paths")
        .iter()
        .any(|finding| finding["path"] == "/.volicord/"));
    fs::write(&exclude_path, managed_excludes)?;

    let add = Command::new("git")
        .args(["add", "-f", "--", ".volicord/policy.json"])
        .current_dir(&repo_root)
        .output()?;
    assert!(
        add.status.success(),
        "git add failed\nstdout:\n{}\nstderr:\n{}",
        stdout(&add),
        stderr(&add)
    );

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&doctor);
    let value = json_stdout(&doctor)?;
    let check = value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "personal_local_git_tracking")
        .expect("personal Git tracking check");
    assert_eq!(check["status"], "warning");
    assert!(check["details"]["tracked_paths"]
        .as_array()
        .expect("tracked paths")
        .iter()
        .any(|finding| finding["path"] == "/.volicord/"));
    assert_eq!(check["details"]["reads_local_policy_file"], true);
    assert_eq!(
        check["details"]["does_not_read_other_local_integration_file_contents"],
        true
    );
    assert!(value["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .any(|action| action["id"] == "protect_personal_local_files"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn doctor_uses_effective_local_policy_intent_after_personal_to_shared_transition(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-doctor-effective-local-intent")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let test_path = path_env_with_existing(&[bin_dir.as_path()])?;
    let env = [
        ("PATH", test_path),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let personal = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&personal);

    let personal_only_path = repo_root.join(".codex/rules/volicord.rules");
    fs::create_dir_all(
        personal_only_path
            .parent()
            .expect("personal-only path should have a parent"),
    )?;
    fs::write(&personal_only_path, "{}\n")?;

    let shared = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&shared);

    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["storage_scope"], "local_overlay");
    assert_eq!(policy["connection_intent"], "shared");
    let exclude_path = repo_root.join(".git/info/exclude");
    let shared_excludes = fs::read_to_string(&exclude_path)?;
    assert!(shared_excludes.contains("/.volicord/"));
    assert!(shared_excludes.contains("/.codex/hooks/volicord-pre-tool.sh"));
    assert!(!shared_excludes.contains("/.codex/rules/volicord.rules"));

    let protected = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&protected);
    let protected = json_stdout(&protected)?;
    let protected_check = protected["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "personal_local_git_tracking")
        .expect("local Git tracking check");
    assert_eq!(protected_check["status"], "warning");
    assert_eq!(
        protected_check["details"]["effective_personal_project_count"],
        0
    );
    assert!(protected_check["details"]["unignored_existing_paths"]
        .as_array()
        .expect("unignored paths")
        .iter()
        .any(|finding| finding["path"] == "/.codex/rules/volicord.rules"));

    fs::write(&exclude_path, "")?;
    let unprotected = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&unprotected);
    let unprotected = json_stdout(&unprotected)?;
    let unprotected_check = unprotected["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .find(|check| check["id"] == "personal_local_git_tracking")
        .expect("local Git tracking check");
    assert_eq!(unprotected_check["status"], "warning");
    let unignored_paths = unprotected_check["details"]["unignored_existing_paths"]
        .as_array()
        .expect("unignored paths");
    assert!(unignored_paths
        .iter()
        .any(|finding| finding["path"] == "/.volicord/"));
    assert!(unignored_paths
        .iter()
        .any(|finding| finding["path"] == "/.codex/rules/volicord.rules"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn claude_detective_personal_to_shared_retires_only_owned_projection_and_converges(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-claude-personal-shared-migration")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let personal = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&personal);
    let local_settings_path = repo_root.join(".claude/settings.local.json");
    let mut local_settings: Value =
        serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    local_settings["theme"] = Value::String("user-owned-dark".to_owned());
    fs::write(
        &local_settings_path,
        serde_json::to_string_pretty(&local_settings)? + "\n",
    )?;

    let shared = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&shared);
    let shared_output = json_stdout(&shared)?;
    assert!(shared_output["retired_files"]
        .as_array()
        .expect("retired files")
        .iter()
        .any(|file| {
            file["path"] == path_text(&local_settings_path) && file["status"] == "updated"
        }));
    let preserved: Value = serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    assert_eq!(preserved["theme"], "user-owned-dark");
    assert!(preserved.get("hooks").is_none());
    assert!(repo_root.join(".claude/settings.json").exists());
    assert!(repo_root.join(".mcp.json").exists());
    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["connection_intent"], "shared");
    let excludes = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(!excludes.contains("/.claude/settings.local.json"));

    let rerun = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&rerun);
    let preserved_again: Value = serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    assert_eq!(preserved_again, preserved);

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&doctor);
    let doctor = json_stdout(&doctor)?;
    let intent_check = doctor["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == "integration_intent_drift")
        .expect("intent drift check");
    assert_eq!(intent_check["status"], "passed", "{intent_check:#}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn claude_personal_to_codex_shared_retires_prior_host_and_converges() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-cross-host-migration")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let claude = write_fake_claude_code(&bin_dir)?;
    write_fake_codex(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let codex_home = runtime_home.path().join("codex-home");
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let personal = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&personal);
    let personal = json_stdout(&personal)?;
    let prior_connection_id = personal["connection"]["connection_id"]
        .as_str()
        .expect("prior connection id")
        .to_owned();
    let local_settings_path = repo_root.join(".claude/settings.local.json");
    let mut local_settings: Value =
        serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    local_settings["theme"] = Value::String("user-owned-dark".to_owned());
    fs::write(
        &local_settings_path,
        serde_json::to_string_pretty(&local_settings)? + "\n",
    )?;
    assert!(claude.with_extension("state").exists());
    assert!(repo_root.join(".claude/rules/volicord.md").exists());
    assert!(fs::read_to_string(repo_root.join(".git/info/exclude"))?
        .contains("/.claude/settings.local.json"));

    let shared = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&shared);
    let shared = json_stdout(&shared)?;
    assert_ne!(shared["connection"]["connection_id"], prior_connection_id);
    assert!(shared["retired_files"]
        .as_array()
        .expect("retired files")
        .iter()
        .any(|file| {
            file["path"] == path_text(&local_settings_path) && file["status"] == "updated"
        }));

    let preserved: Value = serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    assert_eq!(preserved["theme"], "user-owned-dark");
    assert!(preserved.get("hooks").is_none());
    assert!(!repo_root.join(".claude/rules/volicord.md").exists());
    assert!(!repo_root
        .join(".claude/hooks/volicord-pre-tool.sh")
        .exists());
    assert!(!claude.with_extension("state").exists());
    assert!(repo_root.join(".codex/config.toml").exists());
    assert!(repo_root.join(".codex/hooks.json").exists());
    assert!(repo_root.join(".codex/rules/volicord.rules").exists());

    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["host"], "codex");
    assert_eq!(policy["connection_intent"], "shared");
    assert_eq!(policy["selected_profile"], "detective");
    let excludes = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(!excludes.contains("/.claude/settings.local.json"));
    assert!(!excludes.contains("/.codex/hooks.json"));

    let prior_connection = agent_connection_record(runtime_home.path(), &prior_connection_id)?
        .expect("prior connection remains as disabled history");
    assert!(!prior_connection.enabled);
    assert!(list_connection_projects(runtime_home.path(), &prior_connection_id)?.is_empty());

    let rerun = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&rerun);
    let preserved_again: Value = serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    assert_eq!(preserved_again, preserved);
    assert!(!claude.with_extension("state").exists());

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&doctor);
    let doctor = json_stdout(&doctor)?;
    let intent_check = doctor["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == "integration_intent_drift")
        .expect("intent drift check");
    assert_eq!(intent_check["status"], "passed", "{intent_check:#}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn migration_preserves_superseded_connection_used_by_another_project() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-migration-multi-project-prior")?;
    let repo_a = create_real_git_repo(&runtime_home, "product-repo-a")?;
    let repo_b = create_real_git_repo(&runtime_home, "product-repo-b")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let codex_home = runtime_home.path().join("codex-home");
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let first = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_a).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&first);
    let first = json_stdout(&first)?;
    let prior_connection_id = first["connection"]["connection_id"]
        .as_str()
        .expect("Codex connection id")
        .to_owned();
    let second = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&second);
    let second = json_stdout(&second)?;
    assert_eq!(second["connection"]["connection_id"], prior_connection_id);
    let codex_config_path = codex_home.join("config.toml");
    let codex_config_before = fs::read_to_string(&codex_config_path)?;

    let migrated = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_a).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&migrated);
    assert_eq!(fs::read_to_string(&codex_config_path)?, codex_config_before);

    let prior = agent_connection_record(runtime_home.path(), &prior_connection_id)?
        .expect("shared Codex connection remains");
    assert!(prior.enabled);
    let remaining_projects = list_connection_projects(runtime_home.path(), &prior_connection_id)?;
    assert_eq!(remaining_projects.len(), 1);
    assert_eq!(remaining_projects[0].project.repo_root, repo_b);
    assert!(repo_a.join(".mcp.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn migration_retires_disabled_prior_policy_connection_but_preserves_disabled_alternative(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-migration-disabled-prior-policy")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let codex_home = runtime_home.path().join("codex-home");
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let prior = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&prior);
    let prior = json_stdout(&prior)?;
    let prior_connection_id = prior["connection"]["connection_id"]
        .as_str()
        .expect("prior connection id")
        .to_owned();
    let project_id = prior["connection"]["project_id"]
        .as_str()
        .expect("project id")
        .to_owned();
    set_connection_enabled(runtime_home.path(), &prior_connection_id, false)?;

    let unrelated_connection_id = "conn_disabled_unrelated";
    ensure_agent_connection(
        runtime_home.path(),
        AgentConnectionRegistration {
            connection_internal_id: unrelated_connection_id.to_owned(),
            host_kind: "claude_code".to_owned(),
            intent: "personal".to_owned(),
            host_scope: "user".to_owned(),
            server_name: "volicord-unrelated".to_owned(),
            config_target: path_text(&runtime_home.path().join("unrelated-host-target")),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: false,
            managed_fingerprint: "unrelated-disabled-fixture".to_owned(),
            last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
            last_verification_report_json: "{}".to_owned(),
            last_user_actions_json: "[]".to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_connection_project(
        runtime_home.path(),
        ConnectionProjectRegistration {
            connection_internal_id: unrelated_connection_id.to_owned(),
            project_id,
        },
    )?;

    let migrated = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&migrated);

    assert!(list_connection_projects(runtime_home.path(), &prior_connection_id)?.is_empty());
    let unrelated = agent_connection_record(runtime_home.path(), unrelated_connection_id)?
        .expect("unrelated disabled alternative remains");
    assert!(!unrelated.enabled);
    assert_eq!(
        list_connection_projects(runtime_home.path(), unrelated_connection_id)?.len(),
        1
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn migration_reuses_enabled_requested_connection_without_disrupting_existing_project(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-migration-reused-target")?;
    let repo_a = create_real_git_repo(&runtime_home, "product-repo-a")?;
    let repo_b = create_real_git_repo(&runtime_home, "product-repo-b")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let codex_home = runtime_home.path().join("codex-home");
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let requested = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&requested);
    let requested = json_stdout(&requested)?;
    let requested_connection_id = requested["connection"]["connection_id"]
        .as_str()
        .expect("requested Codex connection id")
        .to_owned();
    let codex_config_path = codex_home.join("config.toml");
    let codex_config_before = fs::read_to_string(&codex_config_path)?;

    let prior = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_a).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&prior);
    let prior = json_stdout(&prior)?;
    let prior_connection_id = prior["connection"]["connection_id"]
        .as_str()
        .expect("prior Claude connection id")
        .to_owned();

    let migrated = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_a).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert_success(&migrated);
    let migrated = json_stdout(&migrated)?;
    assert_eq!(
        migrated["connection"]["connection_id"],
        requested_connection_id
    );
    assert_eq!(fs::read_to_string(&codex_config_path)?, codex_config_before);

    let requested = agent_connection_record(runtime_home.path(), &requested_connection_id)?
        .expect("requested connection remains active");
    assert!(requested.enabled);
    let requested_projects =
        list_connection_projects(runtime_home.path(), &requested_connection_id)?;
    assert_eq!(requested_projects.len(), 2);
    assert!(requested_projects
        .iter()
        .any(|membership| membership.project.repo_root == repo_a));
    assert!(requested_projects
        .iter()
        .any(|membership| membership.project.repo_root == repo_b));
    let prior = agent_connection_record(runtime_home.path(), &prior_connection_id)?
        .expect("prior connection remains as disabled history");
    assert!(!prior.enabled);
    assert!(list_connection_projects(runtime_home.path(), &prior_connection_id)?.is_empty());
    let retired_project_mcp: Value =
        serde_json::from_str(&fs::read_to_string(repo_a.join(".mcp.json"))?)?;
    assert!(retired_project_mcp["mcpServers"].get("volicord").is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_legacy_projection_publishes_fingerprint_after_host_write_and_retries(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-codex-legacy-retry-safe")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let repo_arg = path_text(&repo_root);
    let init_args = [
        "init",
        "--shared",
        "--host",
        "codex",
        "--repo",
        repo_arg.as_str(),
        "--profile",
        "record",
        "--json",
    ];
    let initial = run_with_home_env(runtime_home.path(), init_args, &env)?;
    assert_success(&initial);
    let initial = json_stdout(&initial)?;
    let connection_id = initial["connection"]["connection_id"]
        .as_str()
        .expect("Codex connection id")
        .to_owned();

    let mut legacy =
        ManagedServerEntry::new_repository_discovery(volicord_mcp::RepositoryDiscoveryHost::Codex);
    legacy.env_vars.clear();
    let legacy_fingerprint = managed_fingerprint(
        HostKind::Codex,
        HostScope::Project,
        DEFAULT_SERVER_NAME,
        &legacy,
    );
    let config_path = repo_root.join(".codex/config.toml");
    fs::write(
        &config_path,
        "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]\n",
    )?;
    replace_connection_managed_fingerprint(
        runtime_home.path(),
        &connection_id,
        &legacy_fingerprint,
    )?;

    let config_parent = config_path.parent().expect("Codex config parent");
    let original_permissions = fs::metadata(config_parent)?.permissions();
    fs::set_permissions(config_parent, fs::Permissions::from_mode(0o555))?;
    let failed_result = run_with_home_env(runtime_home.path(), init_args, &env);
    fs::set_permissions(config_parent, original_permissions)?;
    let failed = failed_result?;
    assert!(
        !failed.status.success(),
        "read-only host write unexpectedly succeeded"
    );
    assert_eq!(
        agent_connection_record(runtime_home.path(), &connection_id)?
            .expect("Codex connection remains registered")
            .managed_fingerprint,
        legacy_fingerprint
    );

    let recovered = run_with_home_env(runtime_home.path(), init_args, &env)?;
    assert_success(&recovered);
    assert!(fs::read_to_string(&config_path)?.contains("env_vars = [\"VOLICORD_HOME\"]"));
    let current =
        ManagedServerEntry::new_repository_discovery(volicord_mcp::RepositoryDiscoveryHost::Codex);
    assert_eq!(
        agent_connection_record(runtime_home.path(), &connection_id)?
            .expect("Codex connection converged")
            .managed_fingerprint,
        managed_fingerprint(
            HostKind::Codex,
            HostScope::Project,
            DEFAULT_SERVER_NAME,
            &current,
        )
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_add_claude_legacy_projection_publishes_fingerprint_after_host_write_and_retries(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-claude-legacy-retry-safe")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let initial = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&initial);
    let initial = json_stdout(&initial)?;
    let connection_id = initial["connection"]["connection_id"]
        .as_str()
        .expect("Claude Code connection id")
        .to_owned();

    let mut legacy = ManagedServerEntry::new_repository_discovery(
        volicord_mcp::RepositoryDiscoveryHost::ClaudeCode,
    );
    legacy.env.clear();
    let legacy_fingerprint = managed_fingerprint(
        HostKind::ClaudeCode,
        HostScope::Project,
        DEFAULT_SERVER_NAME,
        &legacy,
    );
    let config_path = repo_root.join(".mcp.json");
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&json!({
            "mcpServers": {"volicord": legacy.to_json_value()}
        }))? + "\n",
    )?;
    replace_connection_managed_fingerprint(
        runtime_home.path(),
        &connection_id,
        &legacy_fingerprint,
    )?;

    let repo_arg = path_text(&repo_root);
    let add_args = [
        "connection",
        "add",
        "claude-code",
        "--shared",
        "--repo",
        repo_arg.as_str(),
        "--json",
    ];
    let original_permissions = fs::metadata(&repo_root)?.permissions();
    fs::set_permissions(&repo_root, fs::Permissions::from_mode(0o555))?;
    let failed_result = run_with_home_env(runtime_home.path(), add_args, &env);
    fs::set_permissions(&repo_root, original_permissions)?;
    let failed = failed_result?;
    assert!(
        !failed.status.success(),
        "read-only host write unexpectedly succeeded"
    );
    assert_eq!(
        agent_connection_record(runtime_home.path(), &connection_id)?
            .expect("Claude Code connection remains registered")
            .managed_fingerprint,
        legacy_fingerprint
    );

    let recovered = run_with_home_env(runtime_home.path(), add_args, &env)?;
    assert_success(&recovered);
    let current_config: Value = serde_json::from_str(&fs::read_to_string(&config_path)?)?;
    assert_eq!(
        current_config["mcpServers"]["volicord"]["env"]["VOLICORD_HOME"],
        "${VOLICORD_HOME}"
    );
    let current = ManagedServerEntry::new_repository_discovery(
        volicord_mcp::RepositoryDiscoveryHost::ClaudeCode,
    );
    assert_eq!(
        agent_connection_record(runtime_home.path(), &connection_id)?
            .expect("Claude Code connection converged")
            .managed_fingerprint,
        managed_fingerprint(
            HostKind::ClaudeCode,
            HostScope::Project,
            DEFAULT_SERVER_NAME,
            &current,
        )
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_repairs_stale_profile_command_and_doctor_points_to_the_repair_path(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-stale-profile-command-repair")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let repo_arg = path_text(&repo_root);
    let init_args = [
        "init",
        "--shared",
        "--host",
        "codex",
        "--repo",
        repo_arg.as_str(),
        "--profile",
        "record",
        "--json",
    ];
    let initial = run_with_home_env(runtime_home.path(), init_args, &env)?;
    assert_success(&initial);
    let profile = installation_profile(runtime_home.path())?.expect("installation profile");
    let stale_command = runtime_home.path().join("removed-bin/volicord");
    write_installation_profile(
        runtime_home.path(),
        InstallationProfileRegistration {
            installation_id: profile.installation_id,
            volicord_command: path_text(&stale_command),
            volicord_mcp_command: profile.volicord_mcp_command,
            bin_dir: stale_command
                .parent()
                .expect("stale command parent")
                .to_path_buf(),
            default_connection_mode: profile.default_connection_mode,
            metadata_json: profile.metadata_json,
        },
    )?;

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert!(
        !doctor.status.success(),
        "stale profile Doctor unexpectedly passed"
    );
    let doctor = json_stdout(&doctor)?;
    assert_eq!(doctor["status"], "failed");
    let repair = doctor["actions"]
        .as_array()
        .expect("Doctor actions")
        .iter()
        .find(|action| action["id"] == "repair_volicord_command")
        .expect("stale profile command repair action");
    assert_eq!(
        repair["command"],
        "volicord init --host <host> --repo <path>"
    );
    assert!(!repair["command"]
        .as_str()
        .expect("repair command")
        .contains("--mcp-command"));

    let repaired = run_with_home_env(runtime_home.path(), init_args, &env)?;
    assert_success(&repaired);
    let repaired_profile =
        installation_profile(runtime_home.path())?.expect("repaired installation profile");
    assert_eq!(
        repaired_profile.volicord_command,
        canonical_volicord_command()
    );
    let stop_wrapper = fs::read_to_string(repo_root.join(".codex/hooks/volicord-stop.sh"))?;
    assert_generated_wrapper_binding(&stop_wrapper, runtime_home.path(), "_final-output");
    Ok(())
}

#[cfg(unix)]
#[test]
fn interrupted_cross_host_migration_keeps_personal_protection_and_converges(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-cross-host-migration-fail-safe")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let claude = write_fake_claude_code(&bin_dir)?;
    write_fake_codex(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let codex_home = runtime_home.path().join("codex-home");
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let personal = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&personal);
    let personal = json_stdout(&personal)?;
    let prior_connection_id = personal["connection"]["connection_id"]
        .as_str()
        .expect("prior connection id")
        .to_owned();

    let local_settings_path = repo_root.join(".claude/settings.local.json");
    let claude_state_path = claude.with_extension("state");
    let original_claude_state = fs::read_to_string(&claude_state_path)?;
    let changed_claude_state = original_claude_state.replace(
        &format!("Command: {}", mcp_command.display()),
        "Command: user-managed-command",
    );
    assert_ne!(changed_claude_state, original_claude_state);
    fs::write(&claude_state_path, &changed_claude_state)?;

    let shared = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert!(!shared.status.success(), "migration unexpectedly succeeded");
    let failed_migration = json_stdout(&shared)?;
    assert_eq!(failed_migration["status"], "failed");
    assert_eq!(
        failed_migration["migration"]["state"],
        "partial_application"
    );
    let migration_id = failed_migration["migration"]["migration_id"]
        .as_str()
        .expect("stable migration id");
    assert!(migration_id.starts_with("migration_"));
    assert_ne!(
        migration_id,
        failed_migration["migration"]["requested_connection_id"]
            .as_str()
            .expect("requested connection id")
    );
    assert_eq!(
        failed_migration["migration"]["requested_connection_enabled"],
        true
    );
    assert_eq!(
        failed_migration["migration"]["registry_transition"],
        "applied"
    );
    assert_eq!(
        failed_migration["migration"]["requested_project_membership_active"],
        true
    );
    assert_eq!(
        failed_migration["migration"]["prior_connection_inventory"],
        "disabled_pending_host_cleanup"
    );
    assert_eq!(
        failed_migration["migration"]["host_projection"],
        "partially_applied_after_registry_transition"
    );
    assert!(failed_migration["error"]
        .as_str()
        .expect("migration error")
        .contains("Claude Code MCP entry changed"));
    assert!(failed_migration["retry_arguments"]
        .as_array()
        .expect("retry arguments")
        .iter()
        .any(|argument| argument == "--shared"));
    assert!(failed_migration["retry_arguments"]
        .as_array()
        .expect("retry arguments")
        .windows(2)
        .any(|arguments| {
            arguments[0] == "--home" && arguments[1] == path_text(runtime_home.path())
        }));
    assert!(repo_root.join(".codex/config.toml").exists());
    assert!(repo_root.join(".codex/hooks.json").exists());
    assert_eq!(
        fs::read_to_string(&claude_state_path)?,
        changed_claude_state
    );
    assert!(!local_settings_path.exists());
    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["host"], "codex");
    assert_eq!(policy["connection_intent"], "shared");
    assert!(!fs::read_to_string(repo_root.join(".git/info/exclude"))?
        .contains("/.claude/settings.local.json"));
    let connections = list_agent_connections(runtime_home.path())?;
    assert_eq!(connections.len(), 2);
    assert_eq!(
        connections
            .iter()
            .filter(|connection| connection.enabled)
            .count(),
        1
    );
    let staged_connection = connections
        .iter()
        .find(|connection| connection.connection_internal_id != prior_connection_id)
        .expect("requested connection is active while host cleanup remains pending");
    assert!(staged_connection.enabled);
    assert_eq!(
        failed_migration["migration"]["requested_connection_id"],
        staged_connection.connection_internal_id
    );
    assert_eq!(
        list_connection_projects(
            runtime_home.path(),
            &staged_connection.connection_internal_id
        )?
        .len(),
        1
    );
    let prior_connection = agent_connection_record(runtime_home.path(), &prior_connection_id)?
        .expect("prior connection remains as pending cleanup inventory");
    assert!(!prior_connection.enabled);
    assert_eq!(
        list_connection_projects(runtime_home.path(), &prior_connection_id)?.len(),
        1
    );

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&doctor);
    let doctor = json_stdout(&doctor)?;
    let intent_check = doctor["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == "integration_intent_drift")
        .expect("intent drift check");
    assert_eq!(intent_check["status"], "warning", "{intent_check:#}");
    assert!(
        intent_check["details"]["findings"]
            .as_array()
            .expect("intent drift findings")
            .iter()
            .any(|finding| finding["kind"] == "pending_host_cleanup"),
        "{intent_check:#}"
    );

    let middle_connection_id = staged_connection.connection_internal_id.clone();
    let chained = run_with_home_env(
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
        &env,
    )?;
    assert!(
        !chained.status.success(),
        "chained migration unexpectedly hid the older cleanup failure"
    );
    let chained = json_stdout(&chained)?;
    assert_eq!(
        chained["migration"]["prior_connection_inventory"],
        "disabled_pending_host_cleanup"
    );
    let chained_connection_id = chained["migration"]["requested_connection_id"]
        .as_str()
        .expect("chained requested connection id")
        .to_owned();
    assert_ne!(chained_connection_id, middle_connection_id);
    let chained_prior_ids = chained["migration"]["prior_connection_ids"]
        .as_array()
        .expect("chained prior connection ids")
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        chained_prior_ids,
        BTreeSet::from([middle_connection_id.as_str(), prior_connection_id.as_str(),])
    );
    for superseded_connection_id in [&prior_connection_id, &middle_connection_id] {
        let superseded = agent_connection_record(runtime_home.path(), superseded_connection_id)?
            .expect("chained superseded connection remains durable");
        let metadata: Value = serde_json::from_str(&superseded.metadata_json)?;
        assert_eq!(
            metadata["pending_host_cleanup"]["replacement_connection_id"],
            chained_connection_id
        );
        assert_eq!(
            list_connection_projects(runtime_home.path(), superseded_connection_id)?.len(),
            1
        );
    }

    fs::write(&claude_state_path, original_claude_state)?;
    let recovered = run_with_home_env(
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
        &env,
    )?;
    assert_success(&recovered);
    assert!(!claude_state_path.exists());
    assert!(!local_settings_path.exists());
    let recovered_policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(recovered_policy["host"], "codex");
    assert_eq!(recovered_policy["connection_intent"], "personal");
    let recovered_excludes = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(recovered_excludes.contains("/.codex/hooks.json"));
    let prior_connection = agent_connection_record(runtime_home.path(), &prior_connection_id)?
        .expect("prior connection remains as disabled history");
    assert!(!prior_connection.enabled);
    assert!(list_connection_projects(runtime_home.path(), &prior_connection_id)?.is_empty());
    assert!(list_connection_projects(runtime_home.path(), &middle_connection_id)?.is_empty());
    let recovered_connection =
        agent_connection_record(runtime_home.path(), &chained_connection_id)?
            .expect("chained requested connection remains active");
    assert!(recovered_connection.enabled);
    Ok(())
}

#[cfg(unix)]
#[test]
fn claude_detective_shared_to_personal_preserves_mixed_json_and_converges(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-claude-shared-personal-migration")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let shared = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&shared);
    let shared_settings_path = repo_root.join(".claude/settings.json");
    let mut shared_settings: Value =
        serde_json::from_str(&fs::read_to_string(&shared_settings_path)?)?;
    shared_settings["theme"] = Value::String("user-owned-light".to_owned());
    fs::write(
        &shared_settings_path,
        serde_json::to_string_pretty(&shared_settings)? + "\n",
    )?;
    let mcp_path = repo_root.join(".mcp.json");
    let mut mcp: Value = serde_json::from_str(&fs::read_to_string(&mcp_path)?)?;
    mcp["mcpServers"]["other"] = json!({ "command": "other-mcp", "args": [] });
    fs::write(&mcp_path, serde_json::to_string_pretty(&mcp)? + "\n")?;

    let personal = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&personal);
    let preserved: Value = serde_json::from_str(&fs::read_to_string(&shared_settings_path)?)?;
    assert_eq!(preserved["theme"], "user-owned-light");
    assert!(preserved.get("hooks").is_none());
    assert!(repo_root.join(".claude/settings.local.json").exists());
    let preserved_mcp: Value = serde_json::from_str(&fs::read_to_string(&mcp_path)?)?;
    assert!(preserved_mcp["mcpServers"].get("volicord").is_none());
    assert_eq!(preserved_mcp["mcpServers"]["other"]["command"], "other-mcp");
    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["connection_intent"], "personal");
    let excludes = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(excludes.contains("/.claude/settings.local.json"));

    let rerun = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&rerun);
    let preserved_again: Value = serde_json::from_str(&fs::read_to_string(&shared_settings_path)?)?;
    assert_eq!(preserved_again, preserved);

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &env)?;
    assert_success(&doctor);
    let doctor = json_stdout(&doctor)?;
    let intent_check = doctor["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == "integration_intent_drift")
        .expect("intent drift check");
    assert_eq!(intent_check["status"], "passed", "{intent_check:#}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn intent_migration_fails_closed_and_keeps_personal_excludes_on_owned_projection_drift(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-claude-migration-fail-safe")?;
    let repo_root = create_real_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env_with_existing(&[bin_dir.as_path()])?),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let personal = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&personal);
    let local_settings_path = repo_root.join(".claude/settings.local.json");
    let mut changed: Value = serde_json::from_str(&fs::read_to_string(&local_settings_path)?)?;
    changed["hooks"]["PreToolUse"][0]["hooks"][0]["timeout"] = json!(99);
    fs::write(
        &local_settings_path,
        serde_json::to_string_pretty(&changed)? + "\n",
    )?;

    let shared = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--json",
        ],
        &env,
    )?;
    assert!(!shared.status.success(), "migration unexpectedly succeeded");
    assert!(stderr(&shared).contains("no longer matches Volicord ownership"));
    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["connection_intent"], "personal");
    let excludes = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(excludes.contains("/.claude/settings.local.json"));
    assert_eq!(
        serde_json::from_str::<Value>(&fs::read_to_string(&local_settings_path)?)?,
        changed
    );
    assert!(!repo_root.join(".mcp.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_defaults_to_personal_claude_code_connection() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-personal-claude")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let claude = write_fake_claude_code(&bin_dir)?;
    let mcp_command = write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--host",
            "claude-code",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--mcp-command",
            path_text(&mcp_command).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(value["connection"]["connection_intent"], "personal");
    assert_eq!(value["connection"]["host_scope"], "local");
    assert_eq!(
        value["connection"]["config_target"],
        format!("claude cwd={}", repo_root.display())
    );
    assert!(!repo_root.join(".mcp.json").exists());
    assert_eq!(value["states"]["hook_config"], "disabled");
    assert_eq!(
        value["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        value["states"]["control_surface"]["session_watcher_active"],
        false
    );
    assert_eq!(
        value["states"]["final_output_authority_disclosure"],
        json!({
            "support_status": "implemented_unverified",
            "configured": true,
            "configuration_verified": true,
            "required_subcapabilities": ["authority_display", "authenticated_exact_replay"],
            "subcapabilities": {
                "authority_display": "implemented_unverified",
                "authenticated_exact_replay": "implemented_unverified"
            }
        })
    );
    assert_complete_host_feature_support(&value, HostKind::ClaudeCode);
    let settings: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".claude/settings.local.json"),
    )?)?;
    assert_eq!(
        settings["hooks"]
            .as_object()
            .expect("hooks should be an object")
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["Stop".to_owned()]
    );
    let stop_wrapper = fs::read_to_string(repo_root.join(".claude/hooks/volicord-stop.sh"))?;
    assert!(stop_wrapper.contains("# purpose=final_output_authority_disclosure"));
    assert_generated_wrapper_binding(&stop_wrapper, runtime_home.path(), "_final-output");
    assert!(stop_wrapper.contains("--integration-profile record"));
    assert!(stop_wrapper.contains("--host-output claude-code"));
    assert!(!repo_root
        .join(".claude/hooks/volicord-pre-tool.sh")
        .exists());
    assert!(!repo_root.join(".claude/rules/volicord.md").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    let exclude = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(exclude.contains("/.volicord/"));
    assert!(exclude.contains("/.claude/rules/volicord.md"));
    assert!(!exclude.contains("/.mcp.json"));
    let host_state = fs::read_to_string(claude.with_extension("state"))?;
    assert!(host_state.contains("Scope: local"));
    assert!(host_state.contains(&format!("Command: {}", mcp_command.display())));
    assert_eq!(
        value["primary_next_action"]["command"],
        format!(
            "volicord connection verify claude-code --repo {}",
            repo_root.display()
        )
    );
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
            "--shared",
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
    assert_eq!(value["connection"]["connection_intent"], "shared");
    assert_eq!(value["connection"]["host_scope"], "project");
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["required_hook_phases"], "configured");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["hook_root_resolution"]["basis"], "git_work_tree");
    assert_eq!(value["hook_root_resolution"]["all_cwd_independent"], true);
    assert_eq!(value["states"]["hook_path_safety"], "ok");
    assert_eq!(value["states"]["hook_commands_cwd_independent"], true);
    assert_eq!(value["states"]["hook_commands_subdirectory_safe"], true);
    assert_eq!(
        value["states"]["final_output_authority_disclosure"],
        json!({
            "support_status": "implemented_unverified",
            "configured": true,
            "configuration_verified": true,
            "required_subcapabilities": [
                "authority_display",
                "authenticated_exact_replay",
                "block_finalization"
            ],
            "subcapabilities": {
                "authority_display": "implemented_unverified",
                "authenticated_exact_replay": "implemented_unverified",
                "block_finalization": "implemented_unverified"
            }
        })
    );
    assert_complete_host_feature_support(&value, HostKind::Codex);
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
    assert_generated_wrapper_binding(&wrapper_text, runtime_home.path(), "_hook pre-tool");
    assert!(wrapper_text.contains(&format!("--connection {connection_id}")));
    assert!(wrapper_text.contains("--guard-installation"));
    assert!(wrapper_text.contains("--host codex"));
    assert!(wrapper_text.contains("--policy-hash"));
    assert!(wrapper_text.contains("--host-output codex"));
    assert!(is_executable(&wrapper)?);
    assert!(repo_root.join(".codex/rules/volicord.rules").exists());
    let exclude = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(exclude.contains("/.volicord/"));
    assert!(exclude.contains("/.codex/hooks/volicord-pre-tool.sh"));
    assert!(!exclude.contains("/.codex/hooks.json"));
    assert!(!exclude.contains("/.codex/rules/volicord.rules"));
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
            "--shared",
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
    assert_eq!(value["connection"]["connection_intent"], "shared");
    assert_eq!(value["connection"]["host_scope"], "project");
    assert_eq!(value["states"]["hook_config"], "created");
    assert_eq!(value["states"]["guard_installation"], "reload_required");
    assert_eq!(value["states"]["prompt_capture"], "reload_required");
    assert_eq!(value["hook_root_resolution"]["basis"], "claude_project_dir");
    assert_eq!(value["hook_root_resolution"]["all_cwd_independent"], true);
    assert_eq!(value["states"]["hook_path_safety"], "ok");
    assert_eq!(value["states"]["hook_commands_cwd_independent"], true);
    assert_eq!(value["states"]["hook_commands_subdirectory_safe"], true);
    assert_eq!(
        value["states"]["final_output_authority_disclosure"],
        json!({
            "support_status": "implemented_unverified",
            "configured": true,
            "configuration_verified": true,
            "required_subcapabilities": [
                "authority_display",
                "authenticated_exact_replay",
                "block_finalization"
            ],
            "subcapabilities": {
                "authority_display": "implemented_unverified",
                "authenticated_exact_replay": "implemented_unverified",
                "block_finalization": "implemented_unverified"
            }
        })
    );
    assert_complete_host_feature_support(&value, HostKind::ClaudeCode);
    assert!(repo_root.join(".mcp.json").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    let exclude = fs::read_to_string(repo_root.join(".git/info/exclude"))?;
    assert!(exclude.contains("/.volicord/"));
    assert!(exclude.contains("/.claude/hooks/volicord-pre-tool.sh"));
    assert!(!exclude.contains("/.claude/settings.local.json"));
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
    assert_generated_wrapper_binding(&wrapper_text, runtime_home.path(), "_hook pre-tool");
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
            "--shared",
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

    let opaque_event_id =
        unique_guard_event_id_for_connection(&runtime_home, project_id, connection_id, "pre_tool")?;
    assert_ne!(opaque_event_id, event_id);
    let stored = guard_event(runtime_home.path(), project_id, &opaque_event_id)?
        .expect("generated Codex hook command should invoke volicord _hook");
    assert_eq!(stored.connection_internal_id, connection_id);
    assert_eq!(stored.event_kind, "pre_tool");
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
            "--shared",
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

    let opaque_event_id =
        unique_guard_event_id_for_connection(&runtime_home, project_id, connection_id, "pre_tool")?;
    assert_ne!(opaque_event_id, event_id);
    let stored = guard_event(runtime_home.path(), project_id, &opaque_event_id)?
        .expect("generated Claude Code hook command should invoke volicord _hook");
    assert_eq!(stored.connection_internal_id, connection_id);
    assert_eq!(stored.event_kind, "pre_tool");
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
            "--shared",
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
    assert_eq!(
        value["summary_card"]["next"],
        "Regenerate cwd-independent detective host-hook commands, then rerun verification."
    );
    assert!(!value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord init --host codex"));
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
fn init_codex_record_profile_installs_only_final_output_handler() -> Result<(), Box<dyn Error>> {
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
            "--shared",
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
    assert!(init_text.contains("Connection:\n  intent: shared\n  host scope: project"));
    assert!(init_text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(init_text.contains("Repo file changes:"));
    assert!(init_text.contains("created .codex/config.toml"));
    assert!(init_text.contains("created .codex/hooks.json"));
    assert!(init_text.contains("created .codex/hooks/volicord-stop.sh"));
    assert!(init_text.contains("created .volicord/policy.json"));
    assert!(init_text.contains("updated AGENTS.md"));
    assert!(init_text.contains(&format!(
        "Stored local Volicord state:\n  {}",
        runtime_home.path().display()
    )));
    assert!(init_text.contains("Next:"));
    assert!(init_text.contains("Open, restart, or reload Codex in this repository."));
    assert!(!init_text.contains("Trust or approve the project configuration if Codex asks."));
    let verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );
    assert!(init_text.contains(&format!("  2. Run:\n     {verify_command}\n")));
    assert!(!init_text.contains(&format!("Run {verify_command}.")));
    assert!(init_text.contains("Limits:"));
    assert!(init_text.contains(
        "The record profile supports cooperative Volicord workflow recording through MCP."
    ));
    assert!(init_text.contains("OS sandboxing, network isolation, malware defense"));
    assert!(init_text.contains("full write prevention, actor identity proof"));
    assert!(
        init_text.contains("correctness proof, test sufficiency proof, or human review completion")
    );
    assert!(init_text.contains("Diagnostics:"));
    let diagnostics_command = format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    );
    assert!(init_text.contains(&format!(
        "Diagnostics:\n  Run:\n    {diagnostics_command}\n"
    )));
    assert!(!init_text.contains(&format!("Detailed diagnostics: {diagnostics_command}")));
    assert_init_text_omits_internal_diagnostics(&init_text);

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
    assert_eq!(value["connection"]["connection_intent"], "shared");
    assert_eq!(value["connection"]["host_scope"], "project");
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
        value["states"]["final_output_authority_disclosure"],
        json!({
            "support_status": "implemented_unverified",
            "configured": true,
            "configuration_verified": true,
            "required_subcapabilities": ["authority_display", "authenticated_exact_replay"],
            "subcapabilities": {
                "authority_display": "implemented_unverified",
                "authenticated_exact_replay": "implemented_unverified"
            }
        })
    );
    assert_complete_host_feature_support(&value, HostKind::Codex);
    assert_eq!(
        value["states"]["cooperative_pre_tool_denial_available"],
        false
    );
    assert_eq!(value["states"]["post_tool_correlation_available"], false);
    assert_eq!(value["states"]["bypass_detection_active"], false);
    assert!(repo_root.join(".codex/config.toml").exists());
    assert!(repo_root.join("AGENTS.md").exists());
    assert!(repo_root.join(".volicord/policy.json").exists());
    let hooks: Value =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".codex/hooks.json"))?)?;
    assert_eq!(
        hooks["hooks"]
            .as_object()
            .expect("hooks should be an object")
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["Stop".to_owned()]
    );
    assert!(hooks.to_string().contains("volicord-stop.sh"));
    assert!(!hooks.to_string().contains("volicord-dispatch.sh"));
    let stop_wrapper = fs::read_to_string(repo_root.join(".codex/hooks/volicord-stop.sh"))?;
    assert!(stop_wrapper.contains("# purpose=final_output_authority_disclosure"));
    assert_generated_wrapper_binding(&stop_wrapper, runtime_home.path(), "_final-output");
    assert!(stop_wrapper.contains("--guard-installation"));
    assert!(stop_wrapper.contains("--integration-profile record"));
    assert!(stop_wrapper.contains("--policy-hash"));
    assert!(stop_wrapper.contains("--host-output codex"));
    assert!(!repo_root.join(".codex/hooks/volicord-dispatch.sh").exists());
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
    assert_eq!(capability["native_host_output_adapter"], "codex");
    assert_eq!(
        capability["final_output_authority_disclosure_implementation_available"],
        true
    );
    assert_eq!(
        capability["host_hook_commands"]
            .as_array()
            .expect("host command inventory should be an array")
            .len(),
        1
    );
    assert_eq!(
        capability["host_hook_commands"][0]["purpose"],
        "final_output_authority_disclosure"
    );
    assert_eq!(capability["prompt_capture"], false);
    assert!(capability["missing_required_hooks"]
        .as_array()
        .expect("missing hooks should be an array")
        .is_empty());

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_success(&doctor);
    let doctor_json = json_stdout(&doctor)?;
    assert_eq!(doctor_json["states"]["selected_profile"], "record");
    assert_eq!(doctor_json["states"]["guard_files"], "not_checked");
    assert_eq!(doctor_json["states"]["hook_path_safety"], "not_checked");
    let doctor_checks = doctor_json["checks"]
        .as_array()
        .expect("doctor checks should be an array");
    let doctor_guard_files_check = doctor_checks
        .iter()
        .find(|check| check["id"] == "guard_files_installed")
        .expect("doctor guard files check should be present");
    assert_eq!(doctor_guard_files_check["status"], "skipped");
    assert_eq!(
        doctor_guard_files_check["summary"],
        "detective host-hook files are not applicable to record-profile installations"
    );
    assert!(
        !doctor_json["actions"]
            .as_array()
            .expect("doctor actions should be an array")
            .iter()
            .any(|action| matches!(
                action["id"].as_str(),
                Some(
                    "repair_guard_files" | "repair_guard_hook_path_safety" | "repair_guard_status"
                )
            )),
        "record-profile doctor should not offer detective repair: {doctor_json}"
    );

    let rerun_text_output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
fn init_codex_record_profile_succeeds_without_detective_hooks_or_watcher(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-record-without-detective-prereqs")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
    assert_eq!(
        value["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        value["states"]["control_surface"]["session_watcher_active"],
        false
    );
    assert_eq!(
        value["states"]["final_output_authority_disclosure"],
        json!({
            "support_status": "implemented_unverified",
            "configured": true,
            "configuration_verified": true,
            "required_subcapabilities": ["authority_display", "authenticated_exact_replay"],
            "subcapabilities": {
                "authority_display": "implemented_unverified",
                "authenticated_exact_replay": "implemented_unverified"
            }
        })
    );
    assert_complete_host_feature_support(&value, HostKind::Codex);
    assert!(repo_root.join(".codex/config.toml").exists());
    assert!(repo_root.join(".codex/hooks.json").exists());
    assert!(repo_root.join(".codex/hooks/volicord-stop.sh").exists());
    assert!(!repo_root.join(".codex/hooks/volicord-pre-tool.sh").exists());
    assert!(!repo_root.join(".codex/rules/volicord.rules").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn codex_record_detective_profile_migration_reconciles_managed_phases() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-codex-profile-migration")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    for profile in ["record", "detective", "record"] {
        let output = run_with_home_env(
            runtime_home.path(),
            [
                "init",
                "--shared",
                "--host",
                "codex",
                "--repo",
                path_text(&repo_root).as_str(),
                "--profile",
                profile,
                "--json",
            ],
            &env,
        )?;
        assert_success(&output);
    }

    let hooks: Value =
        serde_json::from_str(&fs::read_to_string(repo_root.join(".codex/hooks.json"))?)?;
    assert_eq!(
        hooks["hooks"]
            .as_object()
            .expect("hooks should be an object")
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["Stop".to_owned()]
    );
    assert!(
        fs::read_to_string(repo_root.join(".codex/hooks/volicord-stop.sh"))?
            .contains("volicord _final-output")
    );
    for retired in [
        ".codex/hooks/volicord-dispatch.sh",
        ".codex/hooks/volicord-session-start.sh",
        ".codex/hooks/volicord-pre-tool.sh",
        ".codex/hooks/volicord-post-tool.sh",
        ".codex/hooks/volicord-prompt-capture.sh",
        ".codex/rules/volicord.rules",
    ] {
        assert!(
            !repo_root.join(retired).exists(),
            "{retired} should be retired"
        );
    }
    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["selected_profile"], "record");
    assert_eq!(policy["host_hook"]["enabled"], false);
    Ok(())
}

#[cfg(unix)]
#[test]
fn claude_record_detective_profile_migration_preserves_unrelated_hooks(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-claude-profile-migration")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_claude_code(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let settings_path = repo_root.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))?;
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&json!({
            "theme": "dark",
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "./user-owned-pre-tool.sh"
                    }]
                }]
            }
        }))? + "\n",
    )?;

    for profile in ["record", "detective", "record"] {
        let output = run_with_home_env(
            runtime_home.path(),
            [
                "init",
                "--shared",
                "--host",
                "claude-code",
                "--repo",
                path_text(&repo_root).as_str(),
                "--profile",
                profile,
                "--json",
            ],
            &env,
        )?;
        assert_success(&output);
    }

    let settings: Value = serde_json::from_str(&fs::read_to_string(&settings_path)?)?;
    assert_eq!(settings["theme"], "dark");
    assert_eq!(
        settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "./user-owned-pre-tool.sh"
    );
    assert_eq!(
        settings["hooks"]["Stop"]
            .as_array()
            .expect("Stop groups should be an array")
            .len(),
        1
    );
    assert!(!settings.to_string().contains("volicord-pre-tool"));
    assert!(!settings.to_string().contains("volicord-session-start"));
    assert!(
        fs::read_to_string(repo_root.join(".claude/hooks/volicord-stop.sh"))?
            .contains("volicord _final-output")
    );
    assert!(!repo_root.join(".claude/rules/volicord.md").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn record_final_output_status_degrades_without_activating_detective_hooks(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-record-final-output-degraded")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];
    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);
    fs::remove_file(repo_root.join(".codex/hooks/volicord-stop.sh"))?;

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
        &env,
    )?;
    assert_success(&status);
    let value = json_stdout(&status)?;
    assert_eq!(
        value["states"]["final_output_authority_disclosure"],
        json!({
            "support_status": "implemented_unverified",
            "configured": false,
            "configuration_verified": false,
            "required_subcapabilities": ["authority_display", "authenticated_exact_replay"],
            "subcapabilities": {
                "authority_display": "implemented_unverified",
                "authenticated_exact_replay": "implemented_unverified"
            }
        })
    );
    assert_complete_host_feature_support(&value, HostKind::Codex);
    assert_eq!(value["states"]["hook_config"], "disabled");
    assert_eq!(
        value["states"]["control_surface"]["host_hooks_active"],
        false
    );
    assert_eq!(
        value["states"]["control_surface"]["session_watcher_active"],
        false
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_verify_cli_handshake_without_managed_host_remains_action_required(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-record-verify-compact")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "trusted")?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &env,
    )?;
    assert_success(&verify);
    let text = stdout(&verify);
    let verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );
    let diagnostics_command = format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    );

    assert!(text.contains("Agent Connection checked for Codex"));
    assert!(!text.contains("Agent Connection verified for Codex"));
    assert!(text.contains("Status:\n  Verification: action required"));
    assert!(text.contains("Profile:\n  record"));
    assert!(text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(text.contains("Checks:\n  MCP configuration: match"));
    assert!(text.contains("  Codex project trust: trusted"));
    assert!(text.contains("  CLI MCP preflight: passed"));
    assert!(text.contains("  CLI MCP handshake: passed"));
    assert!(text.contains("  CLI MCP storage read: passed"));
    assert!(text.contains("  CLI MCP storage write: passed"));
    assert!(text.contains("  CLI MCP effective tools: workflow"));
    assert!(text.contains("  Managed Codex MCP startup: not observed"));
    assert!(text.contains("  Managed Codex tools/list: not observed"));
    assert!(text.contains("  Managed Codex tool call: not observed"));
    assert!(text.contains("  Active Codex tool exposure: unconfirmed"));
    assert!(text.contains("  Host MCP command: uses volicord from the Codex host PATH"));
    assert!(text.contains("  Host follow-up: action required"));
    assert!(text.contains("Next:"));
    assert!(!text.contains("Trust or approve the project configuration if Codex asks."));
    assert!(
        text.contains("Restart, reload, resume, or start a new Codex session in this repository.")
    );
    assert!(text.contains("Confirm that Volicord tools are exposed in the active Codex session."));
    assert!(text.contains("If tools are not exposed, check Codex MCP startup/tool-list logs."));
    assert!(!text.contains("Also ensure `volicord` is launchable by the Codex host process."));
    assert!(!text.contains("has started the Volicord MCP server"));
    assert_order(
        &text,
        "Restart, reload, resume, or start a new Codex session in this repository.",
        "Confirm that Volicord tools are exposed in the active Codex session.",
    );
    assert!(text.contains("Limits:"));
    assert!(text.contains(
        "The record profile supports cooperative Volicord workflow recording through MCP."
    ));
    assert!(text.contains("Diagnostics:"));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &text,
        &[&verify_command, &diagnostics_command],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&text);
    assert!(!text.contains("regenerate cwd-independent detective host-hook commands"));
    assert!(!text.contains("refresh stale detective host-hook files"));

    let verify_json = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&verify_json);
    let value = json_stdout(&verify_json)?;
    assert_eq!(
        value["primary_next_action"]["id"],
        "managed_host_startup_not_observed"
    );
    assert_eq!(
        value["primary_next_action"]["instruction"],
        "Restart, reload, resume, or start a new Codex session in this repository so Codex starts the managed Volicord MCP server."
    );
    assert_eq!(value["primary_next_action"]["command"], verify_command);
    assert_order(
        value["primary_next_action"]["instruction"]
            .as_str()
            .expect("primary action instruction should be text"),
        "Restart",
        "managed Volicord MCP server",
    );
    assert!(!value["primary_next_action"]["instruction"]
        .as_str()
        .expect("primary action instruction should be text")
        .contains("launchable by the Codex host process"));
    assert!(!value["primary_next_action"]["instruction"]
        .as_str()
        .expect("primary action instruction should be text")
        .contains("volicord connection verify"));
    assert_eq!(
        value["summary_card"]["next"],
        "Restart, reload, resume, or start a new Codex session in this repository, then confirm active Volicord tool exposure."
    );
    assert_order(
        value["summary_card"]["next"]
            .as_str()
            .expect("summary next should be text"),
        "Restart",
        "active Volicord tool exposure",
    );
    assert!(!value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("host command launchability"));
    assert!(!value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("project configuration"));
    assert!(!value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord connection verify"));
    assert_ne!(value["primary_next_action"]["id"], "host_trust_required");
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "managed_host_startup_not_observed"));
    assert!(!value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "host_trust_required"));
    assert!(!value["connection"]["user_actions"]
        .as_array()
        .expect("connection user actions should be an array")
        .iter()
        .any(|action| action["kind"] == "host_trust_required"));
    assert!(value["connection"]["user_actions"]
        .as_array()
        .expect("connection user actions should be an array")
        .iter()
        .any(|action| action["kind"] == "managed_host_startup_not_observed"));
    assert_eq!(value["verification"]["project_trust"]["status"], "trusted");
    assert_eq!(
        value["verification"]["host_runtime"]["status"],
        "not_observed"
    );
    assert_eq!(
        value["verification"]["managed_host_startup"],
        "not_observed"
    );
    assert_eq!(
        value["verification"]["managed_host_tools_list"],
        "not_observed"
    );
    assert_eq!(
        value["verification"]["managed_host_tool_call"],
        "not_observed"
    );
    assert_eq!(value["verification"]["active_tool_exposure"], "unconfirmed");
    assert_eq!(
        value["verification"]["host_mcp_command"]["mode"],
        "path_resolved"
    );
    assert_eq!(
        value["verification"]["host_mcp_command"]["risk"],
        "host_path_unconfirmed"
    );
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "host_mcp_command"
            && check["status"] == "warning"
            && check["details"]["risk"] == "host_path_unconfirmed"));
    assert_eq!(value["verification"]["host"]["managed_config"], "match");
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["status"],
        "passed"
    );
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["diagnostics"]["storage_read"],
        "passed"
    );
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["diagnostics"]["storage_write"],
        "passed"
    );
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["diagnostics"]["effective_tool_mode"],
        "workflow"
    );
    assert_eq!(
        value["verification"]["cli_mcp_handshake"]["status"],
        "passed"
    );
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "cli_mcp_storage_read"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "cli_mcp_storage_write"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "cli_mcp_effective_tools"
            && check["status"] == "passed"
            && check["details"]["value"] == "workflow"));
    assert_eq!(
        value["connection"]["verification_report"]["cli_mcp_handshake"]["status"],
        "passed"
    );
    assert_eq!(
        value["connection"]["verification_report"]["cli_mcp_preflight"]["diagnostics"]
            ["storage_write"],
        "passed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_trusted_project_mentions_storage_diagnostics() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-record-status-compact")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "trusted")?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let text = stdout(&status);
    let verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );
    let diagnostics_command = format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    );

    assert!(text.contains("Agent Connection status for Codex"));
    assert!(text.contains(
        "Status:\n  Connection: enabled\n  Mode: workflow\n  Last verification: action required"
    ));
    assert!(text.contains("Profile:\n  record"));
    assert!(text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(text.contains(
        "Checks:\n  Stored connection: enabled, mode workflow, last verification action required"
    ));
    assert!(text.contains("  Current MCP configuration: match"));
    assert!(text.contains("  Codex project trust: trusted"));
    assert!(text.contains("  Last CLI MCP preflight: passed"));
    assert!(text.contains("  Last CLI MCP handshake: passed"));
    assert!(text.contains("  CLI MCP storage read: passed"));
    assert!(text.contains("  CLI MCP storage write: passed"));
    assert!(text.contains("  CLI MCP effective tools: workflow"));
    assert!(text.contains("  Managed Codex MCP startup: not observed"));
    assert!(text.contains("  Managed Codex tools/list: not observed"));
    assert!(text.contains("  Managed Codex tool call: not observed"));
    assert!(text.contains("  Active Codex tool exposure: unconfirmed"));
    assert!(text.contains("  Host MCP command: uses volicord from the Codex host PATH"));
    assert!(text.contains("  Host follow-up: action required"));
    assert!(!text.contains("Trust or approve the project configuration if Codex asks."));
    assert!(
        text.contains("Restart, reload, resume, or start a new Codex session in this repository.")
    );
    assert!(text.contains("Confirm that Volicord tools are exposed in the active Codex session."));
    assert!(text.contains("If tools are not exposed, check Codex MCP startup/tool-list logs."));
    assert!(!text.contains("Also ensure `volicord` is launchable by the Codex host process."));
    assert!(!text.contains("has started the Volicord MCP server"));
    assert_order(
        &text,
        "CLI MCP storage read: passed",
        "Managed Codex MCP startup: not observed",
    );
    assert_text_renders_volicord_commands_as_standalone_lines(
        &text,
        &[&verify_command, &diagnostics_command],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&text);
    assert!(!text.contains("regenerate cwd-independent detective host-hook commands"));
    assert!(!text.contains("refresh stale detective host-hook files"));

    let status_json = run_with_home_env(
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
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status_json);
    let value = json_stdout(&status_json)?;
    assert_eq!(
        value["primary_next_action"]["id"],
        "managed_host_startup_not_observed"
    );
    assert_eq!(
        value["summary_card"]["next"],
        "Restart, reload, resume, or start a new Codex session in this repository, then confirm active Volicord tool exposure."
    );
    assert_eq!(
        value["primary_next_action"]["instruction"],
        "Restart, reload, resume, or start a new Codex session in this repository so Codex starts the managed Volicord MCP server."
    );
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "managed_host_startup_not_observed"));
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "cli_mcp_storage_read"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "cli_mcp_storage_write"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "cli_mcp_effective_tools"
            && check["status"] == "passed"
            && check["details"]["value"] == "workflow"));
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "host_mcp_command"
            && check["status"] == "warning"
            && check["details"]["risk"] == "host_path_unconfirmed"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_verify_untrusted_project_keeps_trust_guidance() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-codex-untrusted")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "untrusted")?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&verify);
    let value = json_stdout(&verify)?;
    let verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );

    assert_eq!(
        value["verification"]["project_trust"]["status"],
        "untrusted"
    );
    assert_eq!(value["primary_next_action"]["id"], "host_trust_required");
    assert_eq!(
        value["primary_next_action"]["instruction"],
        "Codex project trust is untrusted in the Codex user configuration"
    );
    assert_eq!(value["primary_next_action"]["command"], verify_command);
    assert_eq!(
        value["summary_card"]["next"],
        "The project must be trusted before project-scoped Codex configuration loads; then rerun verification."
    );
    assert!(!value["primary_next_action"]["instruction"]
        .as_str()
        .expect("primary action instruction should be text")
        .contains("volicord connection verify"));
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "host_trust_required"));
    assert!(value["connection"]["user_actions"]
        .as_array()
        .expect("connection user actions should be an array")
        .iter()
        .any(|action| action["kind"] == "host_trust_required"));

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
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let value = json_stdout(&status)?;
    assert_eq!(value["status"], "action_required");
    assert_eq!(value["primary_next_action"]["id"], "host_trust_required");
    assert_eq!(
        value["summary_card"]["next"],
        "The project must be trusted before project-scoped Codex configuration loads; then rerun verification."
    );
    assert_eq!(value["checks"][1]["id"], "codex_project_trust");
    assert_eq!(value["checks"][1]["details"]["status"], "untrusted");
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_verify_record_profile_does_not_offer_detective_hook_repair(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-record-verify-no-detective-repair")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);

    let verify_json = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&verify_json);
    let value = json_stdout(&verify_json)?;
    assert_eq!(value["states"]["selected_profile"], "record");
    assert_eq!(value["states"]["guard_files"], "disabled");
    assert_eq!(value["states"]["hook_config"], "disabled");
    assert_eq!(value["states"]["required_hook_phases"], "disabled");
    assert_eq!(value["states"]["hook_path_safety"], "not_applicable");
    assert_eq!(value["host_hook"]["hook_path_safety"], "not_applicable");
    assert!(value["host_hook"]["hook_path_safety_details"]
        .as_array()
        .expect("hook path details should be an array")
        .is_empty());
    let checks = value["checks"]
        .as_array()
        .expect("checks should be an array");
    let guard_files_check = checks
        .iter()
        .find(|check| check["id"] == "guard_files_installed")
        .expect("guard files check should be present");
    assert_eq!(guard_files_check["status"], "skipped");
    assert_eq!(
        guard_files_check["summary"],
        "detective host-hook files are not applicable for the record profile"
    );
    assert!(checks.iter().all(|check| {
        check["summary"]
            .as_str()
            .is_none_or(|summary| summary != "detective host-hook files are stale")
    }));
    let prompt_capture_check = checks
        .iter()
        .find(|check| check["id"] == "prompt_capture_available")
        .expect("prompt capture check should be present");
    assert_eq!(prompt_capture_check["status"], "skipped");
    let primary_id = value["primary_next_action"]["id"].as_str();
    assert!(
        !matches!(
            primary_id,
            Some(
                "guard_hook_path_safety"
                    | "guard_files_missing"
                    | "guard_files_stale"
                    | "guard_files_broken"
                    | "guard_capability_degraded"
            )
        ),
        "record profile should not promote detective hook repair: {value}"
    );
    let summary_next = value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text");
    assert!(!summary_next.contains("regenerate cwd-independent detective host-hook commands"));
    assert!(!summary_next.contains("refresh stale detective host-hook files"));

    let verify_text = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
        ],
        &env,
    )?;
    assert_success(&verify_text);
    let text = stdout(&verify_text);
    assert!(text.contains("Agent Connection checked for Codex"));
    assert!(text.contains("Profile:\n  record"));
    assert_connection_text_omits_diagnostic_dump_fields(&text);
    assert!(
        !text.contains("Detective hook commands are not in the supported cwd-independent shape")
    );
    assert!(!text.contains("regenerate cwd-independent detective host-hook commands"));
    assert!(!text.contains("refresh stale detective host-hook files"));
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
            "--shared",
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
    assert!(diagnostic.contains("meet every Detective prerequisite"));
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
            "--shared",
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
            "--shared",
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
    assert_eq!(value["connection"]["connection_intent"], "shared");
    assert_eq!(value["connection"]["host_scope"], "project");
    assert_eq!(value["profile"]["status"], "planned");
    assert_eq!(value["mcp"]["command"], "volicord");
    assert_eq!(
        value["mcp"]["args"],
        serde_json::json!(["mcp", "--stdio", "--discover-repository", "--host", "codex"])
    );
    assert_eq!(value["mcp"]["env"], serde_json::json!({}));
    assert_eq!(value["generated_files"][0]["kind"], "git_info_exclude");
    assert_eq!(value["generated_files"][0]["status"], "planned_create");
    assert_eq!(value["generated_files"][1]["kind"], "agents_managed_block");
    assert_eq!(value["generated_files"][1]["status"], "planned_create");
    assert_eq!(value["generated_files"][2]["kind"], "volicord_policy");
    assert_eq!(value["generated_files"][2]["status"], "planned_create");
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
    assert!(!repo_root.join(".git/info/exclude").exists());

    let text_output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "detective",
            "--dry-run",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;
    assert_success(&text_output);
    let text = stdout(&text_output);
    assert!(text.contains("Connection:\n  intent: shared\n  host scope: project"));
    assert!(text.contains(&format!(
        "volicord init --host codex --shared --repo {} --profile detective --dry-run --json",
        repo_root.display()
    )));
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
            "--shared",
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
fn init_codex_record_rejects_unmanaged_stop_config() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-init-codex-record-hook-conflict")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let hooks_path = repo_root.join(".codex/hooks.json");
    fs::create_dir_all(hooks_path.parent().expect("hook path should have parent"))?;
    let unmanaged = json!({
        "hooks": {
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": "echo user-owned-stop"
                }]
            }]
        }
    });
    fs::write(
        &hooks_path,
        serde_json::to_string_pretty(&unmanaged)? + "\n",
    )?;

    let output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("host_hook_config already exists with unmanaged content"));
    assert_eq!(
        serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)?,
        unmanaged
    );
    assert!(!runtime_home.registry_db_path().exists());
    assert!(!repo_root.join(".volicord/policy.json").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_codex_guarded_writes_policy_mcp_and_guard_status_idempotently() -> Result<(), Box<dyn Error>>
{
    const START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE -->";
    const END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE -->";

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
            "--shared",
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
    let verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );
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
    assert_eq!(
        value["primary_next_action"]["instruction"],
        "Restart or reload codex so it loads the Volicord MCP and host hook configuration"
    );
    assert_eq!(value["primary_next_action"]["command"], verify_command);
    assert!(!value["primary_next_action"]["instruction"]
        .as_str()
        .expect("primary action instruction should be text")
        .contains("volicord connection verify"));
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
        serde_json::json!(["mcp", "--stdio", "--discover-repository", "--host", "codex"])
    );
    assert_eq!(value["mcp"]["env"], serde_json::json!({}));
    let mcp_config = fs::read_to_string(repo_root.join(".codex/config.toml"))?;
    assert!(mcp_config.contains(
        "args = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]"
    ));
    assert!(!mcp_config.contains(&connection_id));
    assert!(!mcp_config.contains(project_id));
    assert!(!mcp_config.contains(path_text(runtime_home.path()).as_str()));
    assert!(value["actions"]
        .as_array()
        .expect("actions should be an array")
        .iter()
        .any(|action| action["id"] == "reload_required"));

    let text_output = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
    assert!(init_text.contains(&format!("  3. Run:\n     {verify_command}\n")));
    assert!(!init_text.contains(&format!("Run {verify_command}.")));
    assert!(init_text.contains("Limits:"));
    assert!(init_text.contains("The detective profile adds cooperative host observation"));
    assert!(init_text.contains("OS sandboxing, network isolation, malware defense"));
    assert!(init_text.contains("Diagnostics:"));
    let diagnostics_command = format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    );
    assert!(init_text.contains(&format!(
        "Diagnostics:\n  Run:\n    {diagnostics_command}\n"
    )));
    assert!(!init_text.contains(&format!("Detailed diagnostics: {diagnostics_command}")));
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
        "managed_host_startup_not_observed"
    );
    assert_eq!(
        status_without_intent_json["summary_card"]["next"],
        "Restart, reload, resume, or start a new Codex session in this repository, then confirm active Volicord tool exposure."
    );
    assert!(!status_without_intent_json["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord connection verify"));
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
    assert!(status_text.contains("Agent Connection status for Codex"));
    assert!(status_text.contains(
        "Status:\n  Connection: enabled\n  Mode: workflow\n  Last verification: action required"
    ));
    assert!(status_text.contains("  Host follow-up: action required"));
    assert!(status_text
        .contains("Restart, reload, resume, or start a new Codex session in this repository."));
    assert!(status_text
        .contains("Confirm that Volicord tools are exposed in the active Codex session."));
    assert!(
        status_text.contains("If tools are not exposed, check Codex MCP startup/tool-list logs.")
    );
    let verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );
    let diagnostics_command = format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    );
    assert_text_renders_volicord_commands_as_standalone_lines(
        &status_text,
        &[&verify_command, &diagnostics_command],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&status_text);

    let config = fs::read_to_string(repo_root.join(".codex/config.toml"))?;
    assert!(config.contains(
        "args = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]"
    ));
    assert!(!config.contains("[mcp_servers.volicord.env]"));
    assert!(!config.contains(&connection_id));
    assert!(!config.contains(project_id));
    assert!(config.contains("env_vars = [\"VOLICORD_HOME\"]"));
    assert!(!config.contains(path_text(runtime_home.path()).as_str()));
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
    assert!(rules.contains("# BEGIN VOLICORD MANAGED CODEX RULES"));
    assert!(rules.contains("prefix_rule("));
    assert!(rules.contains("pattern = [\"sh\", \"-c\", ["));
    assert!(!rules.contains("pattern = [\".codex\", \"hooks\"]"));
    assert!(rules.contains("not_match = ["));
    assert!(rules.contains("sh -c 'echo unrelated'"));
    assert!(rules.contains("volicord status"));
    assert!(rules.contains("git rev-parse --show-toplevel"));
    assert!(rules.contains(".codex/hooks/volicord-dispatch.sh"));
    assert!(rules.contains("session-start"));
    assert!(rules.contains("stop"));

    let agents = fs::read_to_string(repo_root.join("AGENTS.md"))?;
    assert_eq!(count_occurrences(&agents, START_MARKER), 1);
    assert!(agents.contains("Existing top"));
    assert!(agents.contains("Existing bottom"));
    assert!(agents.contains("Treat Volicord's recorded scope"));
    assert!(agents.contains("outside an active compatible write authorization"));
    assert!(agents.contains("Do not infer, resolve, or record user-owned judgments"));
    assert!(agents.contains("Follow the `next_action` returned by Volicord"));
    assert!(agents.contains("Call `volicord.status` only when"));
    assert!(agents.contains("Do not claim completion while Volicord reports close blockers"));
    assert!(agents.contains("If Volicord is unavailable"));
    assert!(!agents.contains("Check Volicord status before planning"));
    assert!(!agents.contains("Start a task before planning implementation"));
    assert!(!agents.contains("Check close before claiming completion"));
    assert!(!agents.contains("old managed text"));

    let policy_path = repo_root.join(".volicord/policy.json");
    let policy: Value = serde_json::from_str(&fs::read_to_string(&policy_path)?)?;
    assert_eq!(policy["schema"], "volicord-policy-v2");
    assert_eq!(policy["managed_by"], "volicord");
    assert_eq!(policy["storage_scope"], "local_overlay");
    assert_eq!(policy["connection_intent"], "shared");
    assert_eq!(policy["host"], "codex");
    assert_eq!(policy["selected_profile"], "detective");
    assert_eq!(policy["mcp"]["command"], "volicord");
    assert_eq!(
        policy["mcp"]["args"],
        serde_json::json!(["mcp", "--stdio", "--discover-repository", "--host", "codex"])
    );
    assert_eq!(policy["host_hook"]["enabled"], true);
    assert_eq!(policy["workflow"]["default_direct_control"], "tracked");
    assert_eq!(policy["workflow"]["default_work_control"], "tracked");
    assert_eq!(policy["workflow"]["light"]["enabled"], false);
    assert_guard_policy_invokes_required_phases(&policy, &connection_id);
    assert_eq!(
        policy["host_hook"]["commands"]["pre_tool"]["command"],
        canonical_volicord_command()
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
    assert_eq!(capability["schema"], "volicord-host-hook-capability-v2");
    assert_eq!(
        capability["policy_hash"],
        value["guard_installation"]["policy_hash"]
    );
    assert_eq!(capability["prompt_capture"], true);
    assert_eq!(capability["selected_profile"], "detective");
    assert_eq!(capability["connection_intent"], "shared");
    assert_eq!(capability["native_host_output_adapter"], "codex");
    assert_eq!(
        capability["final_output_authority_disclosure_implementation_available"],
        true
    );
    assert_eq!(
        capability["native_host_output_adapter_config_verified"],
        true
    );
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
    assert_generated_wrapper_binding(&wrapper, runtime_home.path(), "_hook pre-tool");
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
            "--shared",
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
            "--shared",
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
            "--discover-repository",
            "--host",
            "claude-code"
        ])
    );
    assert_eq!(
        server["env"],
        serde_json::json!({"VOLICORD_HOME": "${VOLICORD_HOME}"})
    );
    let mcp_text = fs::read_to_string(repo_root.join(".mcp.json"))?;
    assert!(!mcp_text.contains(connection_id));
    assert!(!mcp_text.contains(project_id));
    assert!(!mcp_text.contains(path_text(runtime_home.path()).as_str()));

    let policy: Value = serde_json::from_str(&fs::read_to_string(
        repo_root.join(".volicord/policy.json"),
    )?)?;
    assert_eq!(policy["host"], "claude-code");
    assert_eq!(policy["host_hook"]["enabled"], true);
    assert_guard_policy_invokes_required_phases(&policy, connection_id);
    assert_eq!(
        policy["host_hook"]["commands"]["session_start"]["command"],
        canonical_volicord_command()
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
    assert_generated_wrapper_binding(&wrapper, runtime_home.path(), "_hook pre-tool");
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
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("Run from the Product Repository:"));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &diagnostic,
        &["volicord init --host <host> --repo <path>"],
    );
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

    let dry_run_text = run_with_home_env(
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
        ],
        &[("PATH", path_env(&[bin_dir.as_path()]))],
    )?;
    assert_success(&dry_run_text);
    let dry_text = stdout(&dry_run_text);
    let dry_apply_command = format!(
        "volicord connection add codex --shared --read-only --repo {}",
        repo_root.display()
    );
    let dry_verify_command = format!(
        "volicord connection verify codex --shared --repo {}",
        repo_root.display()
    );
    let dry_diagnostics_command = format!(
        "volicord connection add codex --shared --read-only --repo {} --dry-run --json",
        repo_root.display()
    );
    assert!(dry_text.contains("Agent Connection plan for Codex"));
    assert!(dry_text.contains("Status:\n  Plan: dry run\n  Mode: read-only\n  Intent: shared"));
    assert!(dry_text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(dry_text.contains("Planned changes:\n  would create .codex/config.toml"));
    assert!(dry_text.contains("After applying, open, restart, or reload Codex"));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &dry_text,
        &[
            &dry_apply_command,
            &dry_verify_command,
            &dry_diagnostics_command,
        ],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&dry_text);

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
    assert_eq!(dry_run_json["states"]["selected_profile"], "not_configured");
    assert_eq!(
        dry_run_json["states"]["control_surface"]["selected_profile"],
        "not_configured"
    );
    assert_complete_host_feature_support(&dry_run_json, HostKind::Codex);
    assert!(dry_run_json["states"]["final_output_authority_disclosure"].is_null());
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
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["status"],
        "passed"
    );
    assert_eq!(
        value["verification"]["cli_mcp_handshake"]["status"],
        "passed"
    );
    assert_eq!(
        connection["verification_report"]["status"],
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    assert_eq!(
        connection["verification_report"]["cli_mcp_preflight"]["status"],
        "passed"
    );
    assert_eq!(
        connection["verification_report"]["cli_mcp_handshake"]["status"],
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
    assert_eq!(stored_report["cli_mcp_preflight"]["status"], "passed");
    assert_eq!(stored_report["cli_mcp_handshake"]["status"], "passed");
    let projects = list_connection_projects(runtime_home.path(), connection_id)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project.repo_root, repo_root);

    let config = fs::read_to_string(repo_root.join(".codex").join("config.toml"))?;
    assert!(config.contains(
        "args = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]"
    ));
    assert!(!config.contains("[mcp_servers.volicord.env]"));
    assert!(!config.contains(connection_id));
    assert!(!config.contains(&projects[0].project_id));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_verify_complete_when_active_codex_tools_are_confirmed() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-verification-watch-skip")?;
    let repo_root = runtime_home.create_product_repo("product-repo")?;
    fs::create_dir_all(repo_root.join(".git"))?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "trusted")?;
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
        &[
            ("PATH", path_env(&[bin_dir.as_path(), volicord_dir])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&output);
    let value = json_stdout(&output)?;
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["status"], "passed",
        "{value}"
    );
    assert_eq!(
        value["verification"]["cli_mcp_handshake"]["status"], "passed",
        "{value}"
    );
    assert_eq!(
        value["verification"]["host_runtime"]["status"], "not_observed",
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

    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "legacy_source_less",
        "{}",
    )?;
    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "cli_verification_source",
        &json!({
            "lifecycle_events": [{
                "connection_id": connection_id,
                "project_id": projects[0].project_id,
                "host_kind": "codex",
                "launch_origin": "cli_verification",
                "lifecycle_event": "managed_host_startup",
                "timestamp": "2026-07-01T00:00:00Z",
                "storage_capability": "read_write",
                "effective_tool_mode": "workflow"
            }]
        })
        .to_string(),
    )?;
    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "invalid_managed_mismatch",
        &json!({
            "lifecycle_events": [{
                "connection_id": "conn_wrong",
                "project_id": projects[0].project_id,
                "host_kind": "codex",
                "launch_origin": "managed_host",
                "lifecycle_event": "managed_host_startup",
                "timestamp": "2026-07-01T00:00:01Z",
                "storage_capability": "read_write",
                "effective_tool_mode": "workflow"
            }]
        })
        .to_string(),
    )?;

    let legacy_verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path(), volicord_dir])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&legacy_verify);
    let legacy_value = json_stdout(&legacy_verify)?;
    assert_eq!(
        legacy_value["verification"]["host_runtime"]["status"], "not_observed",
        "{legacy_value}"
    );
    assert_eq!(legacy_value["status"], VERIFIED_STATUS_ACTION_REQUIRED);
    assert_eq!(
        legacy_value["connection"]["verification_status"],
        VERIFIED_STATUS_ACTION_REQUIRED
    );
    assert_eq!(
        legacy_value["primary_next_action"]["id"],
        "managed_host_startup_not_observed"
    );
    assert_eq!(
        legacy_value["verification"]["active_tool_exposure"],
        "unconfirmed"
    );
    assert_eq!(
        legacy_value["verification"]["host_runtime"]["managed_host_startup"], "not_observed",
        "{legacy_value}"
    );

    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "managed_startup_only",
        &json!({
            "lifecycle_events": [{
                "connection_id": connection_id,
                "project_id": projects[0].project_id,
                "host_kind": "codex",
                "launch_origin": "managed_host",
                "lifecycle_event": "managed_host_startup",
                "timestamp": "2026-07-01T00:01:00Z",
                "storage_capability": "read_write",
                "effective_tool_mode": "workflow"
            }]
        })
        .to_string(),
    )?;

    let startup_verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path(), volicord_dir])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&startup_verify);
    let startup_value = json_stdout(&startup_verify)?;
    assert_eq!(startup_value["status"], VERIFIED_STATUS_ACTION_REQUIRED);
    assert_eq!(
        startup_value["verification"]["managed_host_startup"],
        "observed"
    );
    assert_eq!(
        startup_value["verification"]["managed_host_tools_list"],
        "not_observed"
    );
    assert_eq!(
        startup_value["primary_next_action"]["id"],
        "managed_host_tools_list_not_observed"
    );

    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "managed_tools_list_only",
        &json!({
            "lifecycle_events": [
                {
                    "connection_id": connection_id,
                    "project_id": projects[0].project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_startup",
                    "timestamp": "2026-07-01T00:02:00Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": connection_id,
                    "project_id": projects[0].project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tools_list",
                    "timestamp": "2026-07-01T00:02:01Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                }
            ]
        })
        .to_string(),
    )?;

    let tools_list_verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path(), volicord_dir])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&tools_list_verify);
    let tools_list_value = json_stdout(&tools_list_verify)?;
    assert_eq!(tools_list_value["status"], VERIFIED_STATUS_ACTION_REQUIRED);
    assert_eq!(
        tools_list_value["verification"]["managed_host_tools_list"],
        "observed"
    );
    assert_eq!(
        tools_list_value["verification"]["managed_host_tool_call"],
        "not_observed"
    );
    assert_eq!(
        tools_list_value["verification"]["active_tool_exposure"],
        "unconfirmed"
    );
    assert_eq!(
        tools_list_value["primary_next_action"]["id"],
        "active_tool_exposure_unconfirmed"
    );
    assert!(tools_list_value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "active_tool_exposure"
            && check["status"] == "action_required"
            && check["details"]["value"] == "unconfirmed"));

    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "managed_tool_call",
        &json!({
            "lifecycle_events": [
                {
                    "connection_id": connection_id,
                    "project_id": projects[0].project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_startup",
                    "timestamp": "2026-07-01T00:03:00Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": connection_id,
                    "project_id": projects[0].project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tools_list",
                    "timestamp": "2026-07-01T00:03:01Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": connection_id,
                    "project_id": projects[0].project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tool_call",
                    "timestamp": "2026-07-01T00:03:02Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                }
            ]
        })
        .to_string(),
    )?;

    let managed_verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path(), volicord_dir])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&managed_verify);
    let managed_value = json_stdout(&managed_verify)?;
    assert_complete_codex_connection_json(&managed_value, "not_configured");
    assert!(managed_value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "managed_host_storage_write"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let status_value = json_stdout(&status)?;
    assert_complete_codex_connection_json(&status_value, "not_configured");
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_complete_when_managed_codex_tool_call_is_observed(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-status-managed-tool-call")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "trusted")?;
    let env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("CODEX_HOME", path_text(&codex_home)),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &env,
    )?;
    assert_success(&init);
    let init_value = json_stdout(&init)?;
    let connection_id = init_value["connection"]["connection_id"]
        .as_str()
        .expect("connection id should be present")
        .to_owned();
    let projects = list_connection_projects(runtime_home.path(), &connection_id)?;
    assert_eq!(projects.len(), 1);

    insert_managed_codex_tool_call_baseline(
        runtime_home.path(),
        &projects[0],
        "status_managed_tool_call",
    )?;

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &env,
    )?;
    assert_success(&verify);
    assert_complete_codex_connection_json(&json_stdout(&verify)?, "record");

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
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let status_value = json_stdout(&status)?;
    assert_complete_codex_connection_json(&status_value, "record");
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_does_not_report_complete_from_legacy_observation() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-status-legacy-complete")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "trusted")?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&init);
    let init_value = json_stdout(&init)?;
    let connection_id = init_value["connection"]["connection_id"]
        .as_str()
        .expect("connection id should be present")
        .to_owned();
    let record = agent_connection_record(runtime_home.path(), &connection_id)?
        .expect("connection should be stored");
    let projects = list_connection_projects(runtime_home.path(), &connection_id)?;
    assert_eq!(projects.len(), 1);

    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "legacy_source_less",
        "{}",
    )?;
    update_agent_connection_verification_report(
        runtime_home.path(),
        &connection_id,
        VERIFIED_STATUS_COMPLETE,
        &record.managed_fingerprint,
        &json!({
            "status": "complete",
            "host_runtime": {
                "status": "observed",
                "managed_host_startup": "not_observed",
                "managed_host_tools_list": "not_observed",
                "managed_host_tool_call": "not_observed",
                "details": "legacy source-less observation",
                "last_observed_at": null
            },
            "cli_mcp_preflight": { "status": "passed", "details": "CLI preflight" },
            "cli_mcp_handshake": { "status": "passed", "details": "CLI handshake" }
        })
        .to_string(),
        "[]",
    )?;

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--shared",
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let value = json_stdout(&status)?;
    assert_eq!(
        value["connection"]["verification_status"],
        VERIFIED_STATUS_COMPLETE
    );
    assert_eq!(value["status"], VERIFIED_STATUS_ACTION_REQUIRED);
    assert_eq!(
        value["primary_next_action"]["id"],
        "managed_host_startup_not_observed"
    );
    assert!(value["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "active_tool_exposure"
            && check["status"] == "action_required"
            && check["details"]["value"] == "unconfirmed"));
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
        ["connection", "add", "codex"],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
        &nested,
    )?;
    assert_success(&output);
    let add_text = stdout(&output);
    let diagnostics_command = format!(
        "volicord connection status codex --repo {} --json",
        repo_root.display()
    );
    assert!(add_text.contains("Agent Connection configured for Codex"));
    assert!(add_text
        .contains("Status:\n  Connection: enabled\n  Verification: complete\n  Mode: workflow"));
    assert!(add_text.contains("Profile:\n  not_configured"));
    assert!(add_text.contains(&format!("Repository:\n  {}", repo_root.display())));
    assert!(add_text.contains("Host configuration:"));
    assert!(add_text.contains("  Change: create"));
    assert!(add_text.contains("Checks:\n  MCP configuration: match"));
    assert!(add_text.contains("  Host follow-up: ready"));
    assert!(add_text.contains("Next:\n  none"));
    assert_text_renders_volicord_commands_as_standalone_lines(&add_text, &[&diagnostics_command]);
    assert_connection_text_omits_diagnostic_dump_fields(&add_text);

    let status_json = run_with_home_env_in_dir(
        runtime_home.path(),
        ["connection", "status", "codex", "--json"],
        &[("CODEX_HOME", path_text(&codex_home))],
        &nested,
    )?;
    assert_success(&status_json);
    let value = json_stdout(&status_json)?;
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
    assert!(status_text.contains("Agent Connection status for Codex"));
    assert!(status_text.contains(
        "Status:\n  Connection: enabled\n  Mode: workflow\n  Last verification: complete"
    ));
    assert!(status_text.contains("Profile:\n  not_configured"));
    assert!(status_text.contains("  Current MCP configuration: match"));
    assert!(status_text.contains("  Host follow-up: ready"));
    assert!(status_text.contains("Next:\n  none"));
    assert_connection_text_omits_diagnostic_dump_fields(&status_text);
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
            "--shared",
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
    assert_eq!(
        value["summary_card"]["next"],
        "Reinstall missing MCP configuration, then rerun verification."
    );
    assert_eq!(
        value["primary_next_action"]["command"],
        format!(
            "volicord init --host codex --shared --repo {}",
            path_text(&repo_root)
        )
    );
    assert_eq!(
        value["primary_next_action"]["instruction"],
        "Reinstall missing MCP configuration."
    );
    assert!(!value["primary_next_action"]["instruction"]
        .as_str()
        .expect("instruction should be text")
        .contains("volicord init"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_and_verify_reject_nonportable_shared_codex_binding(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-stale-mcp-env")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    let mcp = write_fake_mcp(&bin_dir)?;
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let add = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "add",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&add);
    let add_json = json_stdout(&add)?;
    let connection_id = add_json["connection"]["connection_id"]
        .as_str()
        .expect("connection id should be present")
        .to_owned();
    let project_id = add_json["connection"]["connected_projects"][0]
        .as_str()
        .expect("project id should be present")
        .to_owned();
    let config_path = repo_root.join(".codex/config.toml");
    let config = fs::read_to_string(&config_path)?;
    let nonportable_config = config.replace(
        "args = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]",
        &format!(
            "args = [\"mcp\", \"--stdio\", \"--connection\", \"{connection_id}\", \"--project\", \"{project_id}\"]"
        ),
    );
    fs::write(&config_path, nonportable_config)?;

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
    let status_json = json_stdout(&status)?;
    assert_eq!(status_json["states"]["mcp_config"], "unmanaged");
    assert_eq!(
        status_json["primary_next_action"]["id"],
        "mcp_config_changed"
    );
    assert_eq!(
        status_json["primary_next_action"]["command"],
        format!(
            "volicord init --host codex --shared --repo {}",
            path_text(&repo_root)
        )
    );

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
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
    let verify_json = json_stdout(&verify)?;
    assert_eq!(verify_json["states"]["mcp_config"], "unmanaged");
    assert_eq!(
        verify_json["verification"]["host"]["managed_config"],
        "unmanaged"
    );
    assert_eq!(
        verify_json["primary_next_action"]["id"],
        "mcp_config_changed"
    );

    let expected = format!(
        "args = [\"mcp\", \"--stdio\", \"--connection\", \"{connection_id}\", \"--project\", \"{project_id}\"]"
    );
    assert!(fs::read_to_string(config_path)?.contains(&expected));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_and_verify_reject_nonportable_shared_codex_command(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-command-drift")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
    assert_success(&init);
    let config_path = repo_root.join(".codex/config.toml");
    fs::write(
        &config_path,
        fs::read_to_string(&config_path)?.replace("command = \"volicord\"", "command = \"other\""),
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
    let status_json = json_stdout(&status)?;
    assert_eq!(status_json["states"]["mcp_config"], "unmanaged");
    assert_eq!(
        status_json["primary_next_action"]["id"],
        "mcp_config_changed"
    );
    assert_eq!(
        status_json["primary_next_action"]["command"],
        format!(
            "volicord init --host codex --shared --repo {}",
            path_text(&repo_root)
        )
    );

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
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
    let verify_json = json_stdout(&verify)?;
    assert_eq!(verify_json["states"]["mcp_config"], "unmanaged");
    assert_eq!(
        verify_json["verification"]["host"]["managed_config"],
        "unmanaged"
    );
    assert_eq!(
        verify_json["primary_next_action"]["id"],
        "mcp_config_changed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn codex_tool_approval_overlay_is_match_and_preserved_by_init() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-codex-tool-approval-overlay")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    write_codex_project_trust(&codex_home, &repo_root, "trusted")?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&init);
    let init_json = json_stdout(&init)?;
    let connection_id = init_json["connection"]["connection_id"]
        .as_str()
        .expect("connection id should be present")
        .to_owned();
    let projects = list_connection_projects(runtime_home.path(), &connection_id)?;
    assert_eq!(projects.len(), 1);
    let project_id = projects[0].project_id.clone();
    let config_path = repo_root.join(".codex/config.toml");
    let mut config = fs::read_to_string(&config_path)?;
    config.push_str(
        "\n[mcp_servers.volicord.tools.\"volicord.intake\"]\napproval_mode = \"approve\"\n",
    );
    fs::write(&config_path, config)?;

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
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let status_json = json_stdout(&status)?;
    assert_eq!(status_json["states"]["mcp_config"], "match");
    assert!(!status_json.to_string().contains("mcp_config_changed"));
    assert_ne!(
        status_json["primary_next_action"]["id"],
        "mcp_config_changed"
    );
    assert!(status_json["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .any(|check| check["id"] == "codex_tool_approval_policy"
            && check["status"] == "passed"
            && check["details"]["accepted"] == true
            && check["details"]["kind"] == "codex_tool_approval"
            && check["details"]["entries"][0]["tool"] == "volicord.intake"
            && check["details"]["entries"][0]["approval_mode"] == "approve"));
    assert!(!stdout(&status).contains("volicord init --host codex"));

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
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status_text);
    assert!(stdout(&status_text).contains("Codex tool approval policy: present"));
    assert!(!stdout(&status_text).contains("Review the changed MCP configuration."));

    let second_init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&second_init);
    let preserved = fs::read_to_string(&config_path)?;
    assert!(preserved.contains("[mcp_servers.volicord.tools.\"volicord.intake\"]"));
    assert!(preserved.contains("approval_mode = \"approve\""));

    insert_test_watch_baseline(
        runtime_home.path(),
        &projects[0],
        "codex_overlay_managed_tool_call",
        &json!({
            "lifecycle_events": [
                {
                    "connection_id": connection_id,
                    "project_id": project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_startup",
                    "timestamp": "2026-07-01T00:03:00Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": connection_id,
                    "project_id": project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tools_list",
                    "timestamp": "2026-07-01T00:03:01Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": connection_id,
                    "project_id": project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tool_call",
                    "timestamp": "2026-07-01T00:03:02Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                }
            ]
        })
        .to_string(),
    )?;

    let verify = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "verify",
            "codex",
            "--shared",
            "--repo",
            path_text(&repo_root).as_str(),
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
    assert_complete_codex_connection_json(&verify_json, "record");
    assert_eq!(verify_json["states"]["mcp_config"], "match");
    assert_eq!(
        verify_json["verification"]["host"]["managed_config"],
        "match"
    );
    assert_eq!(
        verify_json["verification"]["host"]["host_policy_overlay"]["present"],
        true
    );
    assert_eq!(
        verify_json["verification"]["host"]["host_policy_overlay"]["kind"],
        "codex_tool_approval"
    );
    assert_eq!(
        verify_json["verification"]["host"]["host_policy_overlay"]["accepted"],
        true
    );
    assert_eq!(
        verify_json["verification"]["host"]["host_policy_overlay"]["entries"][0]["tool"],
        "volicord.intake"
    );
    assert_eq!(
        verify_json["verification"]["host"]["host_policy_overlay"]["entries"][0]["approval_mode"],
        "approve"
    );
    assert_ne!(
        verify_json["primary_next_action"]["id"],
        "mcp_config_changed"
    );
    assert_eq!(verify_json["primary_next_action"], Value::Null);
    assert!(!stdout(&verify).contains("volicord init --host codex"));

    let complete_status = run_with_home_env(
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
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&complete_status);
    let complete_status_json = json_stdout(&complete_status)?;
    assert_complete_codex_connection_json(&complete_status_json, "record");
    assert_eq!(complete_status_json["states"]["mcp_config"], "match");
    assert!(!stdout(&complete_status).contains("volicord init --host codex"));
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
            "--shared",
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
    assert_eq!(
        value["verification"]["cli_mcp_preflight"]["status"],
        "passed"
    );
    assert_eq!(
        value["verification"]["cli_mcp_handshake"]["status"],
        "failed"
    );
    assert!(value["verification"]["cli_mcp_handshake"]["details"]
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
fn connection_status_reports_missing_authoritative_policy_as_broken() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-missing-guard")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
    assert_eq!(value["states"]["guard_installation"], "broken");
    assert_eq!(value["states"]["prompt_capture"], "degraded");
    assert_eq!(value["primary_next_action"]["id"], "guard_files_broken");
    assert_eq!(
        value["summary_card"]["next"],
        "Repair broken detective host-hook files, then rerun verification."
    );
    assert!(!value["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .contains("volicord init"));
    assert!(value["host_hook"]["missing_files"]
        .as_array()
        .expect("missing_files should be an array")
        .iter()
        .any(|path| path == &path_text(&repo_root.join(".volicord/policy.json"))));
    Ok(())
}

#[cfg(unix)]
#[test]
fn connection_status_reports_tampered_authoritative_policy_as_broken() -> Result<(), Box<dyn Error>>
{
    let runtime_home = TempRuntimeHome::new("cli-bin-connection-stale-guard")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;

    let init = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
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
    assert_eq!(value["states"]["guard_installation"], "broken");
    assert_eq!(value["primary_next_action"]["id"], "guard_files_broken");
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
    assert!(text.contains("Agent Connection status for Codex"));
    assert!(text.contains("Profile:\n  detective"));
    assert!(text.contains("  Host follow-up: action required"));
    assert!(text.contains("Repair broken detective host-hook files."));
    let repair_command = format!(
        "volicord init --host codex --shared --repo {}",
        repo_root.display()
    );
    let diagnostics_command = format!(
        "volicord connection status codex --shared --repo {} --json",
        repo_root.display()
    );
    assert_text_renders_volicord_commands_as_standalone_lines(
        &text,
        &[&repair_command, &diagnostics_command],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&text);

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_eq!(doctor.status.code(), Some(1));
    let doctor_json = json_stdout(&doctor)?;
    assert_eq!(doctor_json["status"], "failed");
    assert!(doctor_json["checks"]
        .as_array()
        .expect("Doctor checks should be an array")
        .iter()
        .any(|check| check["id"] == "project_policy_authority" && check["status"] == "failed"));
    assert_eq!(doctor_json["states"]["guard_files"], "action_recommended");
    assert_eq!(doctor_json["states"]["volicord_policy_file"], "stale");
    assert_eq!(
        doctor_json["primary_next_action"]["id"],
        "repair_guard_files"
    );
    assert_eq!(
        doctor_json["primary_next_action"]["command"],
        "volicord init --host HOST --repo PATH"
    );
    assert!(doctor_json["summary_card"]["next"]
        .as_str()
        .expect("summary next should be text")
        .starts_with("Reinstall or refresh detective host-hook files"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_repairs_v1_host_capability_after_current_audit_rejects_it() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-v1-capability-repair")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    write_fake_codex(&bin_dir)?;
    write_fake_mcp(&bin_dir)?;
    let repo_text = path_text(&repo_root);
    let init_args = [
        "init",
        "--shared",
        "--host",
        "codex",
        "--repo",
        repo_text.as_str(),
        "--profile",
        "detective",
        "--json",
    ];
    let init_env = [
        ("PATH", path_env(&[bin_dir.as_path()])),
        ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
    ];

    let init = run_with_home_env(runtime_home.path(), init_args, &init_env)?;
    assert_success(&init);
    let init_json = json_stdout(&init)?;
    let connection_id = init_json["connection"]["connection_id"]
        .as_str()
        .expect("connection id")
        .to_owned();
    let projects = list_connection_projects(runtime_home.path(), &connection_id)?;
    let installations = list_guard_installations(
        runtime_home.path(),
        &connection_id,
        Some(&projects[0].project_id),
    )?;
    let installation = installations
        .into_iter()
        .next()
        .expect("guard installation");
    let mut capability: Value = serde_json::from_str(&installation.host_capability_json)?;
    let capability_object = capability
        .as_object_mut()
        .expect("capability should be an object");
    capability_object.insert(
        "schema".to_owned(),
        Value::String("volicord-host-hook-capability-v1".to_owned()),
    );
    capability_object.remove("final_output_authority_disclosure_implementation_available");
    capability_object.insert(
        "final_output_authority_disclosure_supported".to_owned(),
        Value::Bool(true),
    );
    let registry = rusqlite::Connection::open(runtime_home.registry_db_path())?;
    let updated = registry.execute(
        "UPDATE guard_installations
            SET host_capability_json = ?1
          WHERE guard_installation_id = ?2",
        rusqlite::params![capability.to_string(), installation.guard_installation_id],
    )?;
    assert_eq!(updated, 1, "fixture capability row should exist");

    let stale_status = run_with_home_env(
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
    assert_success(&stale_status);
    let stale_json = json_stdout(&stale_status)?;
    assert_eq!(stale_json["states"]["generated_config_verified"], false);
    assert!(stale_json["host_hook"]["stale_files"]
        .as_array()
        .expect("stale files")
        .iter()
        .any(|value| value == "host_hook_capability_json:schema"));

    let refused_migration = run_with_home_env(
        runtime_home.path(),
        [
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            repo_text.as_str(),
            "--profile",
            "record",
            "--json",
        ],
        &init_env,
    )?;
    assert_ne!(refused_migration.status.code(), Some(0));
    assert!(stderr(&refused_migration).contains("INTEGRATION_MIGRATION_INVENTORY_INVALID"));

    let repaired = run_with_home_env(runtime_home.path(), init_args, &init_env)?;
    assert_success(&repaired);
    let repaired_installations = list_guard_installations(
        runtime_home.path(),
        &connection_id,
        Some(&projects[0].project_id),
    )?;
    assert_eq!(repaired_installations.len(), 1);
    let repaired_capability: Value =
        serde_json::from_str(&repaired_installations[0].host_capability_json)?;
    assert_eq!(
        repaired_capability["schema"],
        "volicord-host-hook-capability-v2"
    );
    assert_eq!(
        repaired_capability["final_output_authority_disclosure_implementation_available"],
        true
    );
    assert_eq!(
        repaired_capability["native_host_output_adapter_config_verified"],
        true
    );
    assert!(repaired_capability
        .get("final_output_authority_disclosure_supported")
        .is_none());
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
fn fresh_codex_version_drives_verify_but_stored_history_does_not_drive_status_or_doctor(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("cli-bin-codex-version-aware-support")?;
    let repo_root = create_git_repo(&runtime_home, "product-repo")?;
    let bin_dir = runtime_home.path().join("bin");
    let codex_home = runtime_home.path().join("codex-home");
    write_fake_codex_with_version(&bin_dir, "0.144.4")?;
    let mcp = write_fake_mcp(&bin_dir)?;
    prepare_runtime_home(runtime_home.path(), &mcp)?;

    let connected = run_with_home_env(
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
            ("CODEX_HOME", path_text(&codex_home)),
            ("VOLICORD_TEST_CONNECTION_MODE", "workflow".to_owned()),
        ],
    )?;
    assert_success(&connected);
    let connected = json_stdout(&connected)?;
    assert_eq!(connected["verification"]["host"]["host_version"], "0.144.4");
    assert_eq!(
        connected["states"]["host_feature_support"]["local_web_user_channel"],
        "implemented_unverified"
    );

    let status = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "status",
            "codex",
            "--repo",
            path_text(&repo_root).as_str(),
            "--json",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&status);
    let status = json_stdout(&status)?;
    assert_eq!(
        status["connection"]["verification_report"]["host"]["host_version"], "0.144.4",
        "the stored coordinate remains diagnostic history"
    );
    assert_eq!(
        status["states"]["host_feature_support"]["local_web_user_channel"],
        "implemented_unverified",
        "status must not treat a historical probe as current"
    );

    let doctor = run_with_home_env(runtime_home.path(), ["doctor", "--json"], &[])?;
    assert_eq!(doctor.status.code(), Some(1));
    let doctor = json_stdout(&doctor)?;
    assert_eq!(doctor["status"], "failed");
    assert!(doctor["checks"]
        .as_array()
        .expect("Doctor checks should be an array")
        .iter()
        .any(|check| check["id"] == "project_policy_authority" && check["status"] == "failed"));
    let row = doctor["states"]["host_feature_support_by_connection"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["host_kind"] == "codex"))
        .expect("Doctor should project the stored Codex connection");
    assert_eq!(
        row["host_feature_support"]["local_web_user_channel"], "implemented_unverified",
        "Doctor must use the no-current-probe fallback"
    );
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
    assert_eq!(status_json["states"]["selected_profile"], "not_configured");
    assert_eq!(
        status_json["states"]["control_surface"]["selected_profile"],
        "not_configured"
    );
    assert_eq!(
        status_json["host_hook"]["selected_profile"],
        "not_configured"
    );
    assert_eq!(
        status_json["host_hook"]["control_surface"]["selected_profile"],
        "not_configured"
    );
    assert_complete_host_feature_support(&status_json, HostKind::Codex);
    assert!(status_json["states"]["final_output_authority_disclosure"].is_null());

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

    let mode_text = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "mode",
            "codex",
            "read-only",
            "--repo",
            path_text(&repo_a).as_str(),
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&mode_text);
    let mode_text = stdout(&mode_text);
    let mode_verify_command = format!(
        "volicord connection verify codex --repo {}",
        repo_a.display()
    );
    let mode_diagnostics_command = format!(
        "volicord connection status codex --repo {} --json",
        repo_a.display()
    );
    assert!(mode_text.contains("Agent Connection mode updated for Codex"));
    assert!(mode_text.contains(
        "Status:\n  Connection: enabled\n  Mode: read-only\n  Last verification: complete"
    ));
    assert!(mode_text.contains("Open, restart, or reload Codex in this repository."));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &mode_text,
        &[&mode_verify_command, &mode_diagnostics_command],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&mode_text);

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

    let remove_dry_run_text = run_with_home_env(
        runtime_home.path(),
        [
            "connection",
            "remove",
            "codex",
            "--repo",
            path_text(&repo_b).as_str(),
            "--dry-run",
        ],
        &[("CODEX_HOME", path_text(&codex_home))],
    )?;
    assert_success(&remove_dry_run_text);
    let remove_dry_run_text = stdout(&remove_dry_run_text);
    let remove_apply_command = format!(
        "volicord connection remove codex --repo {}",
        repo_b.display()
    );
    let remove_diagnostics_command = format!(
        "volicord connection remove codex --repo {} --dry-run --json",
        repo_b.display()
    );
    assert!(remove_dry_run_text.contains("Agent Connection plan for Codex"));
    assert!(remove_dry_run_text.contains("Status:\n  Plan: dry run\n  Mode: read-only"));
    assert!(remove_dry_run_text.contains(&format!("Repository:\n  {}", repo_b.display())));
    assert!(remove_dry_run_text.contains("remove selected repository membership"));
    assert!(remove_dry_run_text
        .contains("keep host configuration for 1 remaining connected repository"));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &remove_dry_run_text,
        &[&remove_apply_command, &remove_diagnostics_command],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&remove_dry_run_text);

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
        ],
        &[
            ("PATH", path_env(&[bin_dir.as_path()])),
            ("CODEX_HOME", path_text(&codex_home)),
        ],
    )?;
    assert_success(&remove_last);
    let remove_last_text = stdout(&remove_last);
    assert!(remove_last_text.contains("Agent Connection removed for Codex"));
    assert!(remove_last_text.contains("Remaining repositories: 0"));
    assert!(remove_last_text.contains(&format!("Repository:\n  {}", repo_a.display())));
    assert!(remove_last_text.contains("Selected repository membership"));
    assert!(remove_last_text.contains("Matching managed host configuration"));
    assert!(remove_last_text
        .contains("Running host processes may keep cached configuration until they reload."));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &remove_last_text,
        &["volicord connection list --json"],
    );
    assert_connection_text_omits_diagnostic_dump_fields(&remove_last_text);
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
fn user_channel_resolves_pending_action_with_local_user_provenance() -> Result<(), Box<dyn Error>> {
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
    let user_action = service.request_user_action(
        request_user_action_request(
            "req_cli_user_action",
            "idem_cli_user_action",
            Some(1),
            &task_id,
        ),
        core_invocation(OperationCategory::AgentWorkflow),
    )?;
    let user_action_request_id = user_action.response_value["user_action_request_summary"]
        ["user_action_request_id"]
        .as_str()
        .expect("safe request summary should identify the pending request")
        .to_owned();

    let status = run_with_home_env_in_dir(runtime_home.path(), ["status"], &[], &repo_root)?;
    assert_success(&status);
    let status_text = stdout(&status);
    assert!(status_text.contains("User Channel status"));
    assert!(status_text.contains("Close readiness: blocked"));
    assert!(status_text.contains("Pending user actions: pending (1)"));
    assert!(status_text.contains(
        "Volicord record effect for this command: none (does not describe product-file writes or Runtime Home write capability)"
    ));
    assert!(!status_text.contains("Available resolve paths:"));
    assert!(status_text.contains("Primary next action:"));
    assert!(status_text.contains("Does not prove:"));
    assert!(status_text.contains("risk-free outcome"));

    let status_json =
        run_with_home_env_in_dir(runtime_home.path(), ["status", "--json"], &[], &repo_root)?;
    assert_success(&status_json);
    let status_value = json_stdout(&status_json)?;
    assert_eq!(status_value["summary_card"]["close_status"], "blocked");
    assert_eq!(status_value["summary_card"]["user_action"], "pending (1)");
    let close_blocker_count = status_value["close_blockers"]
        .as_array()
        .expect("close_blockers should be an array")
        .len();
    let next_action_count = status_value["next_actions"]
        .as_array()
        .expect("next_actions should be an array")
        .len();
    assert!(status_text.contains(&format!(
        "Close readiness blockers (total): {close_blocker_count}"
    )));
    assert!(status_text.contains(&format!(
        "Top-level next actions (total): {next_action_count}"
    )));
    assert!(status_value.get("user_channel_availability").is_none());
    let status_summaries = status_value["pending_user_action_summaries"]
        .as_array()
        .expect("status should expose safe pending user-action summaries");
    assert_eq!(status_summaries.len(), 1);
    assert_eq!(
        status_summaries[0]["user_action_request_id"],
        user_action_request_id.as_str()
    );
    assert_eq!(status_summaries[0]["status"], "pending");
    assert_eq!(status_summaries[0]["next_actor"], "user");

    let list = run_with_home_env_in_dir(runtime_home.path(), ["inbox"], &[], &repo_root)?;
    assert_success(&list);
    let list_text = stdout(&list);
    assert!(list_text.contains("User Action Inbox"));
    assert!(list_text.contains("Pending user actions: pending (1)"));
    assert!(list_text.contains("Profile: not shown in this view"));
    assert!(list_text.contains("1. Should the focused CLI user-channel choice be accepted?"));
    assert!(list_text.contains("id: "));
    assert!(list_text.contains("accept: Accept focused choice"));
    assert!(list_text.contains(
        "Available resolve paths: host prompt unavailable; chat capture unavailable; local consent unavailable; CLI inbox available"
    ));
    assert!(list_text.contains("volicord inbox resolve"));
    assert!(list_text.contains("Does not prove: approval"));
    assert!(!list_text.contains("project_user_channel"));
    assert_text_renders_volicord_commands_as_standalone_lines(
        &list_text,
        &[&format!(
            "volicord inbox resolve {} --choice <choice>",
            user_action_request_id
        )],
    );

    let list_json =
        run_with_home_env_in_dir(runtime_home.path(), ["inbox", "--json"], &[], &repo_root)?;
    assert_success(&list_json);
    let list_value = json_stdout(&list_json)?;
    assert_eq!(list_value["summary_card"]["user_action"], "pending (1)");
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
        .starts_with("resolve pending user action"));
    let first = &list_value["pending_user_action_inbox_items"][0];
    assert_eq!(
        first["user_action_request_id"],
        user_action_request_id.as_str()
    );
    assert_eq!(first["requirement_status"], "optional");
    assert_eq!(first["form"]["choices"][0]["choice_id"], "accept");
    assert_eq!(
        channel_path(&first["answer_path_availability"], "cli")["available"],
        true
    );
    assert_eq!(first["preferred_capture_path"]["kind"], "cli");
    assert!(first["preferred_capture_path"]["command"]
        .as_str()
        .expect("CLI command should be present")
        .contains("volicord inbox resolve"));
    assert!(first["form"]["choices"][0].get("machine_action").is_none());
    assert!(first["form"]["choices"][0]
        .get("resolution_outcome")
        .is_none());

    let removed_open = run_with_home_env_in_dir(
        runtime_home.path(),
        ["inbox", "open", user_action_request_id.as_str()],
        &[],
        &repo_root,
    )?;
    assert_eq!(removed_open.status.code(), Some(2));
    assert!(stderr(&removed_open).contains("unknown inbox command: open"));

    let record_note = "Recorded from inbox CLI";
    let record = run_with_home_env_in_dir(
        runtime_home.path(),
        [
            "inbox",
            "resolve",
            user_action_request_id.as_str(),
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
    assert!(text.contains("User action resolved"));
    assert!(!text.contains("project_user_channel"));
    assert!(!text.contains(user_action_request_id.as_str()));
    assert!(!text.contains("operation_category"));

    let store =
        CoreProjectStore::open(runtime_home.path(), &ProjectId::new("project_user_channel"))?;
    let committed_state_version = store.project_state()?.state_version;
    let retry_args = [
        "inbox",
        "resolve",
        user_action_request_id.as_str(),
        "--choice",
        "accept",
        "--note",
        record_note,
        "--json",
    ];
    let exact_retry = run_with_home_env_in_dir(runtime_home.path(), retry_args, &[], &repo_root)?;
    assert_success(&exact_retry);
    let exact_retry_again =
        run_with_home_env_in_dir(runtime_home.path(), retry_args, &[], &repo_root)?;
    assert_success(&exact_retry_again);
    assert_eq!(stdout(&exact_retry), stdout(&exact_retry_again));
    assert_eq!(
        store.project_state()?.state_version,
        committed_state_version
    );
    let metrics = read_workflow_metric_aggregates(runtime_home.path(), "project_user_channel")?;
    assert!(metrics.iter().any(|row| {
        row.metric_kind == "user_roundtrip" && row.value_total == 1 && row.sample_count == 1
    }));

    let state_db = runtime_home
        .path()
        .join("projects")
        .join("project_user_channel")
        .join("state.sqlite");
    let conn = rusqlite::Connection::open(state_db)?;
    let (original_basis_status, original_basis_json): (String, String) = conn.query_row(
        "SELECT basis_status, basis_json
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params!["project_user_channel", user_action_request_id.as_str()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut stale_basis: Value = serde_json::from_str(&original_basis_json)?;
    stale_basis["coordinates"]["compatibility_status"] = json!("stale");
    conn.execute(
        "UPDATE user_action_requests
            SET basis_status = 'stale',
                basis_json = ?3
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![
            "project_user_channel",
            user_action_request_id.as_str(),
            stale_basis.to_string()
        ],
    )?;
    let current_time = UtcTimestamp::parse(&store.current_timestamp()?)?;
    assert_eq!(
        store
            .user_action_record(&user_action_request_id, &current_time)?
            .expect("resolved user action should remain addressable")
            .status,
        volicord_types::UserActionStatus::Stale
    );
    let before_stale_replay = store.effect_counts()?;
    let before_stale_replay_floor = store.project_state()?.updated_at;
    let stale_exact_retry =
        run_with_home_env_in_dir(runtime_home.path(), retry_args, &[], &repo_root)?;
    assert_success(&stale_exact_retry);
    assert_eq!(stdout(&stale_exact_retry), stdout(&exact_retry));
    assert_eq!(store.effect_counts()?, before_stale_replay);
    assert_eq!(store.project_state()?.updated_at, before_stale_replay_floor);
    conn.execute(
        "UPDATE user_action_requests
            SET basis_status = ?3,
                basis_json = ?4
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        rusqlite::params![
            "project_user_channel",
            user_action_request_id.as_str(),
            original_basis_status,
            original_basis_json
        ],
    )?;

    let changed_retry = run_with_home_env_in_dir(
        runtime_home.path(),
        [
            "inbox",
            "resolve",
            user_action_request_id.as_str(),
            "--choice",
            "decline",
            "--note",
            record_note,
        ],
        &[],
        &repo_root,
    )?;
    assert_success(&changed_retry);
    assert!(
        stdout(&changed_retry).contains("idempotency_key was reused with a different request hash")
    );
    assert_eq!(
        store.project_state()?.state_version,
        committed_state_version
    );

    let persisted = store
        .user_action_record(
            &user_action_request_id,
            &volicord_types::UtcTimestamp::parse("2026-12-01T00:00:00Z")?,
        )?
        .expect("resolved user action should be stored");
    assert_eq!(persisted.status, volicord_types::UserActionStatus::Resolved);
    let persisted_resolution = persisted
        .resolution
        .expect("user-action resolution should be stored");
    assert_eq!(persisted_resolution.resolved_by_actor_source, "local_user");
    assert_eq!(
        persisted_resolution.resolved_verification_basis,
        "cli_direct_user_channel"
    );
    assert_eq!(
        persisted_resolution.resolved_assurance_level,
        "local_user_channel"
    );
    let resolution_json: Value = serde_json::from_str(&persisted_resolution.resolution_json)?;
    assert_eq!(resolution_json["note"], record_note);

    let empty_list = run_with_home_env_in_dir(runtime_home.path(), ["inbox"], &[], &repo_root)?;
    assert_success(&empty_list);
    let empty_list_text = stdout(&empty_list);
    assert!(empty_list_text.contains("Pending user actions: pending (0)"));
    assert!(empty_list_text.contains("No pending user actions."));
    assert!(!empty_list_text.contains("not_selected"));

    let empty_list_json =
        run_with_home_env_in_dir(runtime_home.path(), ["inbox", "--json"], &[], &repo_root)?;
    assert_success(&empty_list_json);
    let empty_list_value = json_stdout(&empty_list_json)?;
    assert_eq!(empty_list_value["summary_card"]["user_action"], "none");
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
            confidence: "confirmed".to_owned(),
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
    assert!(dry_text.contains("Close readiness blockers that would remain (total):"));
    assert!(dry_text.contains("Projected next actions (total):"));
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
            confidence: "confirmed".to_owned(),
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
    assert!(text.contains("Unrecorded Product Repository changes: none"));
    assert!(text.contains(
        "Volicord record effect for this command: recorded (does not describe product-file writes or Runtime Home write capability)"
    ));
    assert!(text.contains("Primary next action:"));
    assert!(text.contains("Close readiness blockers (total):"));
    assert!(text.contains("Top-level next actions (total):"));
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn prepared_command_capture(
    prefix: &str,
    argv: &[String],
) -> Result<(CoreFixture, String), Box<dyn Error>> {
    let input_sha256 = canonical_json_bare_sha256(&argv)?;
    prepared_capture(
        prefix,
        EvidenceCaptureSpec::VerifiedCommandExecution {
            command_sha256: input_sha256,
            command_label: "CLI command capture fixture".to_owned(),
            expected_exit_code: RequiredNullable::null(),
        },
        None,
        None,
    )
}

#[cfg(unix)]
fn bare_canonical_sha256(value: &Value) -> Result<String, Box<dyn Error>> {
    Ok(canonical_json_bare_sha256(value)?)
}

#[cfg(unix)]
fn install_active_guard(
    fixture: &CoreFixture,
    session_id: &str,
    installation_id: &str,
) -> Result<(), Box<dyn Error>> {
    let observed_at = "2026-07-13T00:00:00Z";
    let capability = evidence_guard_capability(fixture, installation_id, "shared");
    upsert_guard_installation(
        fixture.runtime_home_path(),
        GuardInstallationUpsert {
            guard_installation_id: installation_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: Some(fixture.project_id().to_owned()),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            host_capability_json: capability.to_string(),
            installation_status: "active".to_owned(),
            installed_at: Some(observed_at.to_owned()),
            last_checked_at: observed_at.to_owned(),
            first_seen_at: Some(observed_at.to_owned()),
            last_seen_at: Some(observed_at.to_owned()),
            last_seen_phase: Some("post_tool".to_owned()),
            observed_host_kind: Some(HOST_KIND_CODEX.to_owned()),
            observed_policy_hash: Some("policy-hash".to_owned()),
            observed_binary_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            metadata_json: "{}".to_owned(),
        },
    )?;
    insert_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionInsert {
            session_id: session_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: Some(installation_id.to_owned()),
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            started_at: observed_at.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

#[cfg(unix)]
fn evidence_guard_capability(
    fixture: &CoreFixture,
    installation_id: &str,
    connection_intent: &str,
) -> Value {
    let repo_root = fixture.product_repo_path();
    let phases = [
        ("session_start_hook", "session_start", "session-start"),
        ("pre_tool_hook", "pre_tool", "pre-tool"),
        ("post_tool_hook", "post_tool", "post-tool"),
        (
            "user_prompt_submit_hook",
            "prompt_capture",
            "prompt-capture",
        ),
        ("stop_hook", "stop", "stop"),
    ];
    let host_hook_commands = phases
        .iter()
        .map(|(phase, policy_key, command_name)| {
            json!({
                "host_kind": "codex",
                "phase": phase,
                "purpose": "detective_guard",
                "policy_key": policy_key,
                "command_shape": "shell_command_string",
                "command": format!(
                    "sh -c '{}'",
                    format!(
                        "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/.codex/hooks/volicord-dispatch.sh\" {command_name}"
                    )
                    .replace('\'', "'\\''")
                ),
                "args": null,
                "expected_wrapper_path": repo_root.join(".codex/hooks/volicord-dispatch.sh").display().to_string(),
                "expected_phase_wrapper_path": repo_root.join(format!(".codex/hooks/volicord-{command_name}.sh")).display().to_string(),
                "root_resolution_basis": "git_work_tree",
                "hook_command_path_basis": "git_root_runtime",
                "cwd_independent": true,
                "subdirectory_safe": true,
                "wrapper_resolution_status": "ok",
                "verification": {
                    "basis_verified_by": "test_fixture",
                    "host_contract_source": "test_fixture",
                },
            })
        })
        .collect::<Vec<_>>();
    let mut files = host_hook_commands
        .iter()
        .zip(phases.iter())
        .map(|(command, (_, _, command_name))| {
            let wrapper_args = evidence_guard_command_args(
                &repo_root,
                fixture.connection_id(),
                installation_id,
                command_name,
                Some("policy-hash"),
            );
            json!({
                "kind": "host_hook_wrapper",
                "path": command["expected_phase_wrapper_path"],
                "status": "unchanged",
                "content_hash": format!("wrapper-hash-{}", command["policy_key"].as_str().expect("policy key")),
                "ownership": "managed_script",
                "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                "executable_required": true,
                "managed_script_command": evidence_command_line("volicord", &wrapper_args),
                "host_kind": "codex",
                "phase": command["policy_key"],
                "purpose": "detective_guard",
                "connection_id": fixture.connection_id(),
                "guard_installation_id": installation_id,
                "policy_hash": "policy-hash",
                "host_output": "codex",
            })
        })
        .collect::<Vec<_>>();
    files.extend([
        json!({
            "kind": "host_hook_dispatch",
            "path": repo_root.join(".codex/hooks/volicord-dispatch.sh").display().to_string(),
            "status": "unchanged",
            "content_hash": "dispatch-hash",
            "ownership": "managed_script",
            "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
            "executable_required": true,
            "managed_script_role": "codex_dispatch",
            "host_kind": "codex",
            "phase": "dispatch",
        }),
        json!({
            "kind": "host_hook_config",
            "path": repo_root.join(".codex/hooks.json").display().to_string(),
            "status": "unchanged",
            "content_hash": "config-hash",
            "ownership": "managed_json",
        }),
        json!({
            "kind": "host_rule_instruction",
            "path": repo_root.join(".codex/rules/volicord.rules").display().to_string(),
            "status": "unchanged",
            "content_hash": "rule-hash",
            "ownership": "managed_block",
            "managed_marker_start": "# BEGIN VOLICORD MANAGED CODEX RULES",
            "managed_marker_end": "# END VOLICORD MANAGED CODEX RULES",
        }),
    ]);
    let root_phases = host_hook_commands
        .iter()
        .map(|command| {
            json!({
                "phase": command["phase"],
                "root_resolution_basis": command["root_resolution_basis"],
                "hook_command_path_basis": command["hook_command_path_basis"],
                "cwd_independent": command["cwd_independent"],
                "subdirectory_safe": command["subdirectory_safe"],
                "wrapper_resolution_status": command["wrapper_resolution_status"],
            })
        })
        .collect::<Vec<_>>();
    let safety_commands = host_hook_commands
        .iter()
        .map(|command| {
            json!({
                "phase": command["phase"],
                "hook_command_path_basis": command["hook_command_path_basis"],
                "cwd_independent": command["cwd_independent"],
                "subdirectory_safe": command["subdirectory_safe"],
                "wrapper_resolution_status": command["wrapper_resolution_status"],
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "volicord-host-hook-capability-v2",
        "policy_hash": "policy-hash",
        "selected_profile": "detective",
        "connection_intent": connection_intent,
        "final_output_authority_disclosure_implementation_available": true,
        "native_host_output_adapter": "codex",
        "native_host_output_adapter_config_verified": true,
        "bash_shell_mutation_coverage": true,
        "direct_file_write_matcher_coverage": true,
        "host_capabilities": {
            "stdio_mcp": true,
            "http_mcp": false,
            "session_start_hook": true,
            "pre_tool_hook": true,
            "post_tool_hook": true,
            "user_prompt_submit_hook": true,
            "stop_hook": true,
            "rule_file_support": true,
            "project_local_configuration": true,
        },
        "required_hook_phases": [
            "session_start_hook",
            "pre_tool_hook",
            "post_tool_hook",
            "user_prompt_submit_hook",
            "stop_hook"
        ],
        "missing_required_hooks": [],
        "prompt_capture": true,
        "files": files,
        "host_hook_commands": host_hook_commands,
        "hook_root_resolution": {
            "basis": "git_work_tree",
            "all_cwd_independent": true,
            "all_subdirectory_safe": true,
            "overall_status": "ok",
            "phases": root_phases,
        },
        "hook_path_safety": {
            "overall_status": "ok",
            "all_cwd_independent": true,
            "all_subdirectory_safe": true,
            "commands": safety_commands,
        },
        "commands": {
            "session_start": {"command": "volicord", "args": evidence_guard_command_args(&repo_root, fixture.connection_id(), installation_id, "session-start", None)},
            "pre_tool": {"command": "volicord", "args": evidence_guard_command_args(&repo_root, fixture.connection_id(), installation_id, "pre-tool", None)},
            "post_tool": {"command": "volicord", "args": evidence_guard_command_args(&repo_root, fixture.connection_id(), installation_id, "post-tool", None)},
            "prompt_capture": {"command": "volicord", "args": evidence_guard_command_args(&repo_root, fixture.connection_id(), installation_id, "prompt-capture", None)},
            "stop": {"command": "volicord", "args": evidence_guard_command_args(&repo_root, fixture.connection_id(), installation_id, "stop", None)},
        },
    })
}

#[cfg(unix)]
fn evidence_guard_command_args(
    repo_root: &Path,
    connection_id: &str,
    installation_id: &str,
    command_name: &str,
    policy_hash: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "_hook".to_owned(),
        command_name.to_owned(),
        "--repo".to_owned(),
        repo_root.display().to_string(),
        "--connection".to_owned(),
        connection_id.to_owned(),
        "--guard-installation".to_owned(),
        installation_id.to_owned(),
        "--host".to_owned(),
        "codex".to_owned(),
        "--integration-profile".to_owned(),
        "detective".to_owned(),
    ];
    if let Some(policy_hash) = policy_hash {
        args.extend(["--policy-hash".to_owned(), policy_hash.to_owned()]);
    }
    args.extend(["--host-output".to_owned(), "codex".to_owned()]);
    args
}

#[cfg(unix)]
fn evidence_command_line(command: &str, args: &[String]) -> String {
    std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(evidence_shell_word)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(unix)]
fn evidence_shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(unix)]
fn overwrite_guard_capability(
    fixture: &CoreFixture,
    installation_id: &str,
    capability: &Value,
) -> Result<(), Box<dyn Error>> {
    let registry = rusqlite::Connection::open(volicord_store::sqlite::registry_db_path(
        fixture.runtime_home_path(),
    ))?;
    let updated = registry.execute(
        "UPDATE guard_installations
            SET host_capability_json = ?1
          WHERE guard_installation_id = ?2",
        rusqlite::params![capability.to_string(), installation_id],
    )?;
    assert_eq!(updated, 1, "fixture capability row should exist");
    Ok(())
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn insert_tool_guard_event(
    fixture: &CoreFixture,
    event_id: &str,
    event_kind: &str,
    session_id: &str,
    installation_id: &str,
    raw_event: Value,
    tool_input_sha256: &str,
    tool_result_sha256: Option<&str>,
    occurred_at: &str,
) -> Result<(), Box<dyn Error>> {
    let tool_result_size_bytes = ["tool_response", "tool_result", "result", "output"]
        .iter()
        .find_map(|field| raw_event.get(*field))
        .map(canonical_json_bytes)
        .transpose()?
        .map(|bytes| bytes.len() as u64);
    insert_guard_event(
        fixture.runtime_home_path(),
        fixture.project_id(),
        GuardEventInsert {
            guard_event_id: event_id.to_owned(),
            session_id: Some(session_id.to_owned()),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: Some(installation_id.to_owned()),
            event_kind: event_kind.to_owned(),
            decision: "allow".to_owned(),
            subject_json: json!({
                "raw_event_sha256": format!("sha256:{}", bare_canonical_sha256(&raw_event)?),
                "tool_input_sha256": tool_input_sha256,
                "tool_result_sha256": tool_result_sha256,
                "tool_result_size_bytes": tool_result_size_bytes,
                "raw_event": raw_event,
            })
            .to_string(),
            result_json: "{}".to_owned(),
            occurred_at: occurred_at.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

#[cfg(unix)]
fn capture_intent_timestamp(
    fixture: &CoreFixture,
    intent_id: &str,
    field: &str,
) -> Result<String, Box<dyn Error>> {
    let intent = fixture
        .store()?
        .evidence_capture_intent_record(intent_id)?
        .ok_or("capture intent should exist")?;
    match field {
        "created_at" => Ok(intent.created_at),
        "expires_at" => Ok(intent.expires_at),
        _ => Err(format!("unsupported capture-intent timestamp field: {field}").into()),
    }
}

#[cfg(unix)]
fn set_capture_intent_clock(
    fixture: &CoreFixture,
    intent_id: &str,
    created_at: &str,
    expires_at: &str,
    project_clock_floor: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = fixture.conn()?;
    conn.execute(
        "UPDATE evidence_capture_intents
            SET created_at = ?3,
                expires_at = ?4
          WHERE project_id = ?1
            AND evidence_capture_intent_id = ?2",
        rusqlite::params![fixture.project_id(), intent_id, created_at, expires_at],
    )?;
    conn.execute(
        "UPDATE project_state
            SET updated_at = ?2
          WHERE project_id = ?1",
        rusqlite::params![fixture.project_id(), project_clock_floor],
    )?;
    Ok(())
}

#[cfg(unix)]
fn prepared_watch_capture(
    prefix: &str,
    degraded: bool,
) -> Result<(CoreFixture, String, String), Box<dyn Error>> {
    prepared_watch_capture_with_degradation(prefix, degraded, degraded)
}

#[cfg(unix)]
fn prepared_watch_capture_with_degradation(
    prefix: &str,
    baseline_degraded: bool,
    current_degraded: bool,
) -> Result<(CoreFixture, String, String), Box<dyn Error>> {
    let fixture = CoreFixture::new(prefix)?;
    initialize_fixture_git(&fixture)?;
    let session_id = "session_evidence_watch";
    let baseline_id = "watch_baseline_evidence";
    let observation_id = "watch_observation_evidence";
    let registered_at =
        DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::AutoSi, true);
    insert_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        AgentSessionInsert {
            session_id: session_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            host_kind: HOST_KIND_CODEX.to_owned(),
            guard_mode: "detective".to_owned(),
            started_at: registered_at.clone(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let watched_path = fixture.product_repo_path().join("watch.txt");
    fs::write(&watched_path, "before")?;
    let mut baseline_options = WatchSnapshotOptions {
        watch_paths: vec!["watch.txt".into()],
        ..WatchSnapshotOptions::default()
    };
    if baseline_degraded {
        baseline_options.max_file_size_bytes = 1;
    }
    let baseline_snapshot = snapshot_product_repository(
        fixture.runtime_home_path(),
        fixture.product_repo_path(),
        baseline_options,
    )?;
    create_watch_baseline(
        fixture.runtime_home_path(),
        fixture.project_id(),
        WatchBaselineCreate {
            watch_baseline_id: baseline_id.to_owned(),
            session_id: session_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            guard_installation_id: None,
            status: SessionWatchStatus::Active,
            snapshot: baseline_snapshot.clone(),
            created_at: registered_at,
            metadata_json: "{}".to_owned(),
        },
    )?;
    fs::write(&watched_path, "after")?;
    let mut current_options = WatchSnapshotOptions {
        watch_paths: vec!["watch.txt".into()],
        ..WatchSnapshotOptions::default()
    };
    if current_degraded {
        current_options.max_file_size_bytes = 1;
    }
    let capture = EvidenceCaptureSpec::RegisteredConnectionObservation {
        source_selector: ConnectionObservationSourceSelector::SessionWatcher {},
        expected_complete: RequiredNullable::null(),
    };
    let (fixture, intent_id) = prepare_capture_on_fixture(fixture, capture, Some(session_id))?;
    let observed_at =
        DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::AutoSi, true);
    let current_snapshot = snapshot_product_repository(
        fixture.runtime_home_path(),
        fixture.product_repo_path(),
        current_options,
    )?;
    let diff = compare_watch_snapshots(&baseline_snapshot, &current_snapshot);
    let scan_metadata = json!({ "scan_summary": &current_snapshot.scan_summary });
    record_watch_observation(
        fixture.runtime_home_path(),
        fixture.project_id(),
        WatchObservationInsert {
            watch_observation_id: observation_id.to_owned(),
            watch_baseline_id: baseline_id.to_owned(),
            expected_write_id: None,
            snapshot: current_snapshot,
            diff,
            observed_at,
            metadata_json: scan_metadata.to_string(),
        },
    )?;
    Ok((fixture, intent_id, observation_id.to_owned()))
}

#[cfg(unix)]
fn prepared_capture(
    prefix: &str,
    capture: EvidenceCaptureSpec,
    session_id: Option<&str>,
    guard_installation_id: Option<&str>,
) -> Result<(CoreFixture, String), Box<dyn Error>> {
    let fixture = CoreFixture::new(prefix)?;
    initialize_fixture_git(&fixture)?;
    match (session_id, guard_installation_id) {
        (Some(session_id), Some(installation_id)) => {
            install_active_guard(&fixture, session_id, installation_id)?;
        }
        (None, None) | (Some(_), None) => {}
        (None, Some(_)) => return Err("guard installation requires a fixture session".into()),
    }
    prepare_capture_on_fixture(fixture, capture, session_id)
}

#[cfg(unix)]
fn initialize_fixture_git(fixture: &CoreFixture) -> Result<(), Box<dyn Error>> {
    let git_dir = fixture.product_repo_path().join(".git");
    fs::create_dir_all(git_dir.join("refs/heads"))?;
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;
    Ok(())
}

#[cfg(unix)]
fn prepare_capture_on_fixture(
    fixture: CoreFixture,
    capture: EvidenceCaptureSpec,
    session_id: Option<&str>,
) -> Result<(CoreFixture, String), Box<dyn Error>> {
    let snapshot = capture_git_workspace_snapshot(&fixture.product_repo_path())?
        .ok_or("fixture should expose Git workspace context")?;
    let workspace = GitWorkspaceContext {
        git_common_dir: snapshot.layout.common_dir.display().to_string(),
        worktree_id: snapshot.worktree_id,
        branch_ref: snapshot.branch_ref,
        head_sha: snapshot.head_sha,
        workspace_fingerprint: snapshot.workspace_fingerprint,
    };
    let invocation = || {
        let invocation = InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::AgentWorkflow,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        )
        .with_git_workspace_context(workspace.clone());
        match session_id {
            Some(session_id) => invocation.with_session_id(session_id),
            None => invocation,
        }
    };
    let service = CoreService::new(fixture.runtime_home_path());
    let intake = service.intake(
        fixture.intake_request(
            "req_cli_evidence_intake",
            "idem_cli_evidence_intake",
            false,
            Some(0),
        ),
        invocation(),
    )?;
    let task_id = intake
        .resolved_task_id
        .as_ref()
        .ok_or("intake should resolve a Task")?
        .as_str()
        .to_owned();
    let intake_version = intake.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("intake should expose state version")?;
    let scope = service.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_cli_evidence_scope",
            idempotency_key: "idem_cli_evidence_scope",
            dry_run: false,
            expected_state_version: Some(intake_version),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Prepare command evidence capture.",
        }),
        invocation(),
    )?;
    let scope_version = scope.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("scope should expose state version")?;
    let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope should expose Change Unit")?;
    let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .ok_or("scope should expose criterion")?;
    let prepared = service.prepare_evidence_capture(
        PrepareEvidenceCaptureRequest {
            envelope: fixture.envelope(
                "req_cli_evidence_prepare",
                Some("idem_cli_evidence_prepare"),
                false,
                Some(scope_version),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            change_unit_id: volicord_types::ChangeUnitId::new(change_unit_id),
            baseline_ref: BaselineRef::new(DEFAULT_BASELINE_REF),
            target: EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id: volicord_types::AcceptanceCriterionId::new(criterion_id),
            },
            capture,
        },
        invocation(),
    )?;
    let intent_id = prepared.response_value["capture_intent_ref"]["record_id"]
        .as_str()
        .ok_or("prepare should expose intent")?
        .to_owned();
    Ok((fixture, intent_id))
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

#[cfg(unix)]
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

#[cfg(unix)]
fn assert_connection_text_omits_diagnostic_dump_fields(text: &str) {
    for forbidden in [
        "Result:",
        "Why:",
        "Does not prove:",
        "runtime_home_state:",
        "runtime_home:",
        "connection_state:",
        "project_registration_state:",
        "connected_repositories:",
        "mcp_config_state:",
        "mcp_config:",
        "selected_profile:",
        "observation_summary:",
        "observation_capabilities:",
        "host_hooks_active:",
        "session_watcher_active:",
        "actor_identity_provable:",
        "os_enforced:",
        "detective_installation_state:",
        "detective_configuration_state:",
        "host_hook_observation_state:",
        "detective_effective_state:",
        "detective_files_state:",
        "agents_block_state:",
        "volicord_policy_file_state:",
        "rule_instruction_config_state:",
        "hook_config_state:",
        "hook_path_safety:",
        "required_hook_phases_state:",
        "required_hook_phases_missing:",
        "host_hook_observed:",
        "detective_hook_observed:",
        "last_host_hook_event:",
        "prompt_capture_state:",
        "host_reload_required:",
        "detective_blockers:",
        "host_verification:",
        "mcp_handshake:",
        "next_action:",
    ] {
        assert!(
            !text.contains(forbidden),
            "connection text should not expose `{forbidden}`:\n{text}"
        );
    }
}

#[cfg(unix)]
fn assert_complete_host_feature_support(value: &Value, host_kind: HostKind) {
    let expected = match host_kind {
        HostKind::Codex => json!({
            "native_user_action": "implemented_unverified",
            "local_web_user_channel": "implemented_unverified",
            "verified_tool_producer": "implemented_unverified",
            "registered_connection_observation": "implemented_unverified",
            "record_final_output": "implemented_unverified",
            "detective_final_output": "implemented_unverified"
        }),
        HostKind::ClaudeCode => json!({
            "native_user_action": "implemented_unverified",
            "local_web_user_channel": "implemented_unverified",
            "verified_tool_producer": "implemented_unverified",
            "registered_connection_observation": "implemented_unverified",
            "record_final_output": "implemented_unverified",
            "detective_final_output": "implemented_unverified"
        }),
        HostKind::Generic => json!({
            "native_user_action": "unsupported_by_host",
            "local_web_user_channel": "unsupported_by_host",
            "verified_tool_producer": "unsupported_by_host",
            "registered_connection_observation": "unsupported_by_host",
            "record_final_output": "unsupported_by_host",
            "detective_final_output": "unsupported_by_host"
        }),
    };
    assert_eq!(value["states"]["host_feature_support"], expected, "{value}");
    assert_eq!(
        value["states"]["host_feature_support"]
            .as_object()
            .map(serde_json::Map::len),
        Some(6),
        "host support must use the exact six-key contract: {value}"
    );
    assert!(
        value["host_hook"].get("host_feature_support").is_none(),
        "host_hook must not duplicate the owner states.host_feature_support projection: {value}"
    );
    assert!(
        value["host_hook"]
            .get("final_output_authority_disclosure")
            .is_none(),
        "host_hook must not duplicate the owner states.final_output_authority_disclosure projection: {value}"
    );
}

#[cfg(unix)]
fn assert_complete_codex_connection_json(value: &Value, expected_profile: &str) {
    assert_eq!(value["status"], VERIFIED_STATUS_COMPLETE, "{value}");
    assert_eq!(
        value["connection"]["verification_status"], VERIFIED_STATUS_COMPLETE,
        "{value}"
    );
    let verification = if value["verification"].is_object() {
        &value["verification"]
    } else {
        &value["connection"]["verification_report"]
    };
    assert_eq!(verification["status"], VERIFIED_STATUS_COMPLETE);
    assert_eq!(value["states"]["connection"], VERIFIED_STATUS_COMPLETE);
    assert_eq!(value["states"]["mcp_config"], "match");
    assert_eq!(verification["host"]["managed_config"], "match");
    assert_eq!(verification["host"]["host_executable"], "available");
    assert_eq!(verification["host"]["host_gate"], "ready");
    assert_eq!(verification["host"]["host_configuration"], "discovered");
    assert_eq!(verification["host"]["mcp_handshake_allowed"], true);
    assert_eq!(verification["project_trust"]["status"], "trusted");
    assert_eq!(verification["cli_mcp_preflight"]["status"], "passed");
    assert_eq!(verification["cli_mcp_handshake"]["status"], "passed");
    assert_eq!(verification["host_runtime"]["status"], "observed");
    assert_eq!(
        verification["host_runtime"]["managed_host_startup"],
        "observed"
    );
    assert_eq!(
        verification["host_runtime"]["managed_host_tools_list"],
        "observed"
    );
    assert_eq!(
        verification["host_runtime"]["managed_host_tool_call"],
        "observed"
    );
    assert_eq!(verification["managed_host_startup"], "observed");
    assert_eq!(verification["managed_host_tools_list"], "observed");
    assert_eq!(verification["managed_host_tool_call"], "observed");
    assert_eq!(verification["active_tool_exposure"], "confirmed");
    assert_eq!(
        verification["host_runtime"]["managed_host_storage"]["storage_read"],
        "passed"
    );
    assert_eq!(
        verification["host_runtime"]["managed_host_storage"]["storage_write"],
        "passed"
    );
    assert_eq!(
        verification["host_runtime"]["managed_host_storage"]["effective_tool_mode"],
        "workflow"
    );
    assert_eq!(value["primary_next_action"], Value::Null);
    assert_eq!(value["summary_card"]["next"], "none");
    assert_complete_host_feature_support(value, HostKind::Codex);
    if expected_profile == "record" {
        assert_record_profile_detective_checks_are_skipped(value);
    } else {
        assert_eq!(expected_profile, "not_configured");
        assert_eq!(value["states"]["selected_profile"], expected_profile);
        assert_eq!(
            value["states"]["control_surface"]["selected_profile"],
            expected_profile
        );
        assert!(value["states"]["final_output_authority_disclosure"].is_null());
    }
    assert_complete_codex_json_omits_known_regressions(value);

    let checks = value["checks"]
        .as_array()
        .expect("checks should be an array");
    for id in [
        "managed_host_startup",
        "managed_host_tools_list",
        "managed_host_tool_call",
    ] {
        assert!(
            checks.iter().any(|check| check["id"] == id
                && check["status"] == "passed"
                && check["details"]["value"] == "observed"),
            "complete output should include passed {id} check: {value}"
        );
    }
    assert!(checks
        .iter()
        .any(|check| check["id"] == "active_tool_exposure"
            && check["status"] == "passed"
            && check["details"]["value"] == "confirmed"));
    assert!(checks
        .iter()
        .any(|check| check["id"] == "managed_host_storage_read"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));
    assert!(checks
        .iter()
        .any(|check| check["id"] == "managed_host_storage_write"
            && check["status"] == "passed"
            && check["details"]["value"] == "passed"));
    assert!(checks
        .iter()
        .any(|check| check["id"] == "managed_host_effective_tools"
            && check["status"] == "passed"
            && check["details"]["value"] == "workflow"));
    assert!(!checks
        .iter()
        .any(|check| check["id"] == "active_tool_exposure"
            && check["details"]["value"] == "unconfirmed"));
}

#[cfg(unix)]
fn assert_record_profile_detective_checks_are_skipped(value: &Value) {
    assert_eq!(value["states"]["selected_profile"], "record");
    assert!(
        matches!(
            value["states"]["guard_files"].as_str(),
            Some("disabled" | "not_configured")
        ),
        "record profile guard files should be disabled or not configured: {value}"
    );
    assert!(
        matches!(
            value["states"]["hook_config"].as_str(),
            Some("disabled" | "not_configured")
        ),
        "record profile hook config should be disabled or not configured: {value}"
    );
    assert!(
        matches!(
            value["states"]["required_hook_phases"].as_str(),
            Some("configured" | "disabled" | "not_configured")
        ),
        "record profile hook phases should not require detective action: {value}"
    );
    assert!(
        matches!(
            value["states"]["hook_path_safety"].as_str(),
            Some("not_applicable" | "not_checked")
        ),
        "record profile hook path safety should be skipped or not applicable: {value}"
    );
    assert!(
        matches!(
            value["host_hook"]["hook_path_safety"].as_str(),
            Some("not_applicable" | "not_checked")
        ),
        "record profile host hook path safety should be skipped or not applicable: {value}"
    );

    let checks = value["checks"]
        .as_array()
        .expect("checks should be an array");
    let guard_files_check = checks
        .iter()
        .find(|check| check["id"] == "guard_files_installed")
        .expect("guard files check should be present");
    assert_eq!(guard_files_check["status"], "skipped");
    assert_eq!(
        guard_files_check["summary"],
        "detective host-hook files are not applicable for the record profile"
    );
    let prompt_capture_check = checks
        .iter()
        .find(|check| check["id"] == "prompt_capture_available")
        .expect("prompt capture check should be present");
    assert_eq!(prompt_capture_check["status"], "skipped");
    assert!(checks.iter().all(|check| {
        check["summary"]
            .as_str()
            .is_none_or(|summary| summary != "detective host-hook files are stale")
    }));
}

#[cfg(unix)]
fn assert_complete_codex_json_omits_known_regressions(value: &Value) {
    let text = value.to_string();
    for forbidden in [
        "host_trust_required",
        "host_mcp_command_path_unconfirmed",
        "mcp_config_changed",
        "detective host-hook files are stale",
        "active_tool_exposure_unconfirmed",
        "\"unconfirmed\"",
    ] {
        assert!(
            !text.contains(forbidden),
            "complete Codex output should not contain `{forbidden}`: {value}"
        );
    }
}

fn assert_non_connection_text_omits_diagnostic_dump_fields(text: &str) {
    for forbidden in [
        "Result:",
        "Why:",
        "runtime_home_state:",
        "installation_profile_state:",
        "mcp_config_state:",
        "observation_summary:",
        "observation_capabilities:",
        "prompt_capture_state:",
        "next_action:",
    ] {
        assert!(
            !text.contains(forbidden),
            "non-connection text should not expose `{forbidden}`:\n{text}"
        );
    }
}

fn assert_text_renders_volicord_commands_as_standalone_lines(text: &str, commands: &[&str]) {
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    for command in commands {
        assert!(
            lines.iter().any(|line| line == command),
            "expected standalone command `{command}` in:\n{text}"
        );
    }
    for line in lines
        .iter()
        .copied()
        .filter(|line| contains_volicord_shell_command(line))
    {
        assert!(
            line.starts_with("volicord "),
            "volicord command should be standalone, got `{line}` in:\n{text}"
        );
        assert!(
            !matches!(line.chars().last(), Some('.' | ',' | ';' | ':')),
            "volicord command should not have trailing punctuation, got `{line}` in:\n{text}"
        );
    }
}

#[cfg(unix)]
fn assert_order(text: &str, before: &str, after: &str) {
    let before_index = text
        .find(before)
        .unwrap_or_else(|| panic!("expected `{before}` in:\n{text}"));
    let after_index = text
        .find(after)
        .unwrap_or_else(|| panic!("expected `{after}` in:\n{text}"));
    assert!(
        before_index < after_index,
        "expected `{before}` before `{after}` in:\n{text}"
    );
}

fn contains_volicord_shell_command(line: &str) -> bool {
    let Some(start) = line.find("volicord ") else {
        return false;
    };
    let rest = &line[start + "volicord ".len()..];
    let command = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '-');
    matches!(
        command,
        "init"
            | "doctor"
            | "connection"
            | "status"
            | "inbox"
            | "changes"
            | "project"
            | "export"
            | "mcp"
            | "serve"
            | "--help"
            | "--version"
    )
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

#[cfg(unix)]
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

#[cfg(unix)]
fn write_codex_project_trust(
    codex_home: &Path,
    repo_root: &Path,
    trust_level: &str,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(codex_home)?;
    fs::write(
        codex_home.join("config.toml"),
        format!(
            "[projects.\"{}\"]\ntrust_level = \"{}\"\n",
            repo_root.display(),
            trust_level
        ),
    )?;
    Ok(())
}

#[cfg(unix)]
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

#[cfg(unix)]
fn insert_test_watch_baseline(
    runtime_home: &Path,
    project: &volicord_store::agent_connections::ConnectionProjectRecord,
    suffix: &str,
    metadata_json: &str,
) -> Result<(), Box<dyn Error>> {
    let session_id = format!("session_{suffix}");
    let created_at = "2026-07-01T00:00:00Z".to_owned();
    insert_agent_session(
        runtime_home,
        &project.project_id,
        AgentSessionInsert {
            session_id: session_id.clone(),
            connection_internal_id: project.connection_internal_id.clone(),
            guard_installation_id: None,
            host_kind: "codex".to_owned(),
            guard_mode: "record".to_owned(),
            started_at: created_at.clone(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let snapshot = snapshot_product_repository(
        runtime_home,
        &project.project.repo_root,
        WatchSnapshotOptions::default(),
    )?;
    create_watch_baseline(
        runtime_home,
        &project.project_id,
        WatchBaselineCreate {
            watch_baseline_id: format!("watch_base_{suffix}"),
            session_id,
            connection_internal_id: project.connection_internal_id.clone(),
            guard_installation_id: None,
            status: SessionWatchStatus::Active,
            snapshot,
            created_at,
            metadata_json: metadata_json.to_owned(),
        },
    )?;
    Ok(())
}

#[cfg(unix)]
fn insert_managed_codex_tool_call_baseline(
    runtime_home: &Path,
    project: &volicord_store::agent_connections::ConnectionProjectRecord,
    suffix: &str,
) -> Result<(), Box<dyn Error>> {
    insert_test_watch_baseline(
        runtime_home,
        project,
        suffix,
        &json!({
            "lifecycle_events": [
                {
                    "connection_id": project.connection_internal_id,
                    "project_id": project.project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_startup",
                    "timestamp": "2026-07-01T00:03:00Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": project.connection_internal_id,
                    "project_id": project.project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tools_list",
                    "timestamp": "2026-07-01T00:03:01Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                },
                {
                    "connection_id": project.connection_internal_id,
                    "project_id": project.project_id,
                    "host_kind": "codex",
                    "launch_origin": "managed_host",
                    "lifecycle_event": "managed_host_tool_call",
                    "timestamp": "2026-07-01T00:03:02Z",
                    "storage_capability": "read_write",
                    "effective_tool_mode": "workflow"
                }
            ]
        })
        .to_string(),
    )
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

#[cfg(unix)]
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
fn unique_guard_event_id_for_connection(
    runtime_home: &TempRuntimeHome,
    project_id: &str,
    connection_id: &str,
    event_kind: &str,
) -> Result<String, Box<dyn Error>> {
    let connection = rusqlite::Connection::open_with_flags(
        runtime_home.project_state_db_path(project_id),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let mut statement = connection.prepare(
        "SELECT guard_event_id
           FROM guard_events
          WHERE project_id = ?1
            AND connection_internal_id = ?2
            AND event_kind = ?3
          ORDER BY guard_event_id",
    )?;
    let event_ids = statement
        .query_map(
            rusqlite::params![project_id, connection_id, event_kind],
            |row| row.get::<_, String>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    match event_ids.as_slice() {
        [event_id] => Ok(event_id.clone()),
        _ => Err(format!(
            "expected exactly one {event_kind} GuardEvent for project {project_id} and connection \
             {connection_id}, found {}",
            event_ids.len()
        )
        .into()),
    }
}

#[cfg(unix)]
fn canonical_volicord_command() -> String {
    path_text(
        &fs::canonicalize(volicord_bin())
            .expect("the test Volicord binary should have a canonical absolute path"),
    )
}

#[cfg(unix)]
fn replace_connection_managed_fingerprint(
    runtime_home: &Path,
    connection_id: &str,
    managed_fingerprint: &str,
) -> Result<(), Box<dyn Error>> {
    let existing = agent_connection_record(runtime_home, connection_id)?
        .ok_or_else(|| format!("missing Agent Connection {connection_id}"))?;
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
            managed_fingerprint: managed_fingerprint.to_owned(),
            last_verification_status: existing.last_verification_status,
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    Ok(())
}

#[cfg(unix)]
fn generated_script_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(unix)]
fn assert_generated_wrapper_binding(wrapper: &str, runtime_home: &Path, args_prefix: &str) {
    let runtime_home_assignment = format!(
        "VOLICORD_HOME={}",
        generated_script_word(path_text(runtime_home).as_str())
    );
    let command_prefix = format!(
        "exec {} {args_prefix}",
        generated_script_word(canonical_volicord_command().as_str())
    );
    assert!(
        wrapper.lines().any(|line| line == runtime_home_assignment),
        "generated wrapper did not bind the init-selected Runtime Home:\n{wrapper}"
    );
    assert!(wrapper.lines().any(|line| line == "export VOLICORD_HOME"));
    assert!(wrapper.lines().any(|line| {
        line == format!("{MANAGED_PROCESS_BINDING_ENV}={MANAGED_PROCESS_BINDING_V1}")
    }));
    assert!(wrapper
        .lines()
        .any(|line| line == format!("export {MANAGED_PROCESS_BINDING_ENV}")));
    assert!(
        wrapper
            .lines()
            .any(|line| line.starts_with(&command_prefix)),
        "generated wrapper did not invoke the installation profile command:\n{wrapper}"
    );
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
        assert_eq!(command["command"], canonical_volicord_command());
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
        requested_control_level: RequestedControlLevel::Auto,
        resume_policy: ResumePolicy::CreateNew,
        acceptance_policy: volicord_types::RequiredNullable::null(),
        lineage: volicord_types::RequiredNullable::null(),
        initial_scope: InitialScope {
            boundary: "Exercise the local User Channel.".to_owned(),
            non_goals: vec!["Changing unrelated CLI behavior.".to_owned()],
            acceptance_criteria: vec![AcceptanceCriterionInput {
                statement: "The pending user action can be resolved locally.".to_owned(),
                evidence_requirement: EvidenceRequirement::Required,
            }],
        },
        initial_context_refs: Vec::new(),
        initial_source_refs: Vec::new(),
    }
}

fn request_user_action_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: &str,
) -> volicord_types::RequestUserActionRequest {
    volicord_types::RequestUserActionRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: RequiredNullable::null(),
        action: UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
            judgment_kind: JudgmentKind::ProductDecision,
            presentation: JudgmentPresentation::Short,
            question: "Should the focused CLI user-channel choice be accepted?".to_owned(),
            options: Some(vec![
                UserActionOptionInput {
                    option_id: UserActionOptionId::new("accept"),
                    label: "Accept focused choice".to_owned(),
                    description: "Record the focused user-owned choice.".to_owned(),
                    consequence: "Only this user action is resolved.".to_owned(),
                    is_default: true,
                },
                UserActionOptionInput {
                    option_id: UserActionOptionId::new("decline"),
                    label: "Decline focused choice".to_owned(),
                    description: "Decline the focused user-owned choice.".to_owned(),
                    consequence: "The user action resolves without acceptance.".to_owned(),
                    is_default: false,
                },
            ])
            .into(),
            context: UserActionContext {
                summary: "The CLI needs a pending user action to resolve.".to_owned(),
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
                produced_at_state_version: expected_state_version.into(),
            }],
            sensitive_action_scope: RequiredNullable::null(),
        })),
        required_for: vec![UserActionRequiredFor::Informational],
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
