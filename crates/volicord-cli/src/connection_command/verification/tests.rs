use volicord_store::diagnostic_findings::insert_occurrence_finding;
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{DiagnosticCause, DiagnosticFindingData, OccurrenceDiagnosticFinding};

use super::*;

fn host(version: &str) -> Verification {
    Verification {
        config_target: "/tmp/codex/config.toml".to_owned(),
        managed_config: ManagedConfigStatus::Match,
        managed_config_diagnostic: None,
        managed_config_details: "matches".to_owned(),
        host_executable: HostExecutableStatus::Available,
        executable_path: Some("/opt/codex/bin/codex".to_owned()),
        host_version: Some(version.to_owned()),
        host_executable_code: "host_executable_available".to_owned(),
        host_executable_details: "version probe passed".to_owned(),
        project_trust: None,
    }
}

fn managed_session(version: &str, required_tools_present: bool) -> McpRuntimeSessionRecord {
    McpRuntimeSessionRecord {
        runtime_session_id: "mcp_runtime_fixture".to_owned(),
        connection_internal_id: "connection_fixture".to_owned(),
        session_source: volicord_types::McpRuntimeSessionSource::ManagedHost,
        connection_integration_revision: "revision_current".to_owned(),
        observed_host_executable_version: Some(version.to_owned()),
        attempted_client_name: Some("codex".to_owned()),
        attempted_client_version: Some(version.to_owned()),
        requested_protocol_version: Some("2025-11-25".to_owned()),
        selected_protocol_version: Some("2025-11-25".to_owned()),
        negotiated_protocol_version: Some("2025-11-25".to_owned()),
        process_id: 42,
        process_started_at: "2026-07-18T00:00:00Z".to_owned(),
        initialize_completed_at: Some("2026-07-18T00:00:01Z".to_owned()),
        initialized_notification_at: Some("2026-07-18T00:00:02Z".to_owned()),
        tools_list_observed_at: Some("2026-07-18T00:00:03Z".to_owned()),
        required_tools_present: Some(required_tools_present),
        verification_tool_name: Some(
            crate::connection_command::managed_host_round_trip_tool()
                .wire_name()
                .to_owned(),
        ),
        verification_tool_observed_at: Some("2026-07-18T00:00:04Z".to_owned()),
        last_observed_at: "2026-07-18T00:00:04Z".to_owned(),
        terminal_finding_id: None,
        graceful_close_at: None,
    }
}

fn check_for(checks: &[ConnectionCheck], id: ConnectionCheckKind) -> &ConnectionCheck {
    checks
        .iter()
        .find(|check| check.id() == id)
        .expect("expected connection check")
}

#[test]
fn arbitrary_future_version_can_complete_managed_host_checks() {
    let host = host("999.123-preview+custom");
    let session = managed_session("999.123-preview+custom", true);

    let session_checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("valid checks");

    assert!(session_checks
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Passed));
    let mut checks = vec![
        managed_config_check(&host).expect("managed config check"),
        host_executable_check(&host).expect("host executable check"),
        project_trust_check(&host).expect("project trust check"),
        canonical_check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Passed,
            "mcp_server_ready",
            "MCP server passed",
            None,
            None,
        )
        .expect("MCP check"),
    ];
    checks.extend(session_checks);
    for id in [
        ConnectionCheckKind::GuardFiles,
        ConnectionCheckKind::GuardObservation,
    ] {
        checks.push(
            canonical_check(
                id,
                ConnectionCheckStatus::Passed,
                &format!("{}_passed", id.as_str()),
                "Guard check passed",
                None,
                None,
            )
            .expect("Guard check"),
        );
    }
    let report = ConnectionVerificationReport::try_new(
        current_timestamp(),
        checks.clone(),
        actions_for_checks(&checks).expect("actions"),
    )
    .expect("canonical report");
    assert_eq!(report.status(), ConnectionStatus::Complete);
}

#[test]
fn host_version_change_requires_new_observation_without_rejection() {
    let host = host("1000.0-new-host");
    let session = managed_session("999.123-preview+custom", true);

    let checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("valid checks");

    assert!(checks
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Pending));
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).code(),
        Some("host_version_observation_stale")
    );
}

#[test]
fn mismatched_verification_tool_fails_with_expected_and_observed_names() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.verification_tool_name =
        Some(volicord_types::AgentToolId::STATUS.wire_name().to_owned());
    let cause = DiagnosticFindingId::parse(
        "finding.mcp_runtime_fixture.verification_tool_designation_mismatch",
    )
    .expect("finding id");

    let checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        std::slice::from_ref(&cause),
    )
    .expect("designation mismatch checks");
    let check = check_for(&checks, ConnectionCheckKind::ToolRoundTrip);
    assert_eq!(check.status(), ConnectionCheckStatus::Failed);
    assert_eq!(check.code(), Some("tool_round_trip_designation_mismatch"));
    assert_eq!(check.cause_finding_ids(), &[cause]);
    let details = check.details().expect("round-trip details").as_object();
    assert_eq!(
        details["expected_verification_tool_name"],
        crate::connection_command::managed_host_round_trip_tool().wire_name()
    );
    assert_eq!(
        details["observed_verification_tool_name"],
        volicord_types::AgentToolId::STATUS.wire_name()
    );
}

#[test]
fn initialize_response_without_initialized_notification_remains_pending() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.negotiated_protocol_version = None;
    session.initialized_notification_at = None;
    session.tools_list_observed_at = None;
    session.required_tools_present = None;
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;

    let checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("initialize-response-only checks");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ProcessStartup).status(),
        ConnectionCheckStatus::Passed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).status(),
        ConnectionCheckStatus::Pending
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).code(),
        Some("host_session_initialize_pending")
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::RequiredTools).status(),
        ConnectionCheckStatus::Pending
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ToolRoundTrip).status(),
        ConnectionCheckStatus::Pending
    );
}

#[test]
fn completed_current_session_wins_over_newer_incomplete_or_terminal_diagnostics() {
    let host = host("future");
    let completed = managed_session("future", true);
    let mut newer = managed_session("future", true);
    newer.runtime_session_id = "mcp_runtime_newer".to_owned();
    newer.initialize_completed_at = None;
    newer.initialized_notification_at = None;
    newer.tools_list_observed_at = None;
    newer.required_tools_present = None;
    newer.verification_tool_name = None;
    newer.verification_tool_observed_at = None;
    newer.last_observed_at = "2026-07-18T00:01:00Z".to_owned();

    let sessions = vec![newer.clone(), completed.clone()];
    let checks = host_session_checks(&host, "revision_current", &sessions, Some(&newer), &[])
        .expect("concurrent session checks");
    assert!(checks
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Passed));

    newer.terminal_finding_id = Some("finding.later_crash".to_owned());
    let sessions = vec![newer.clone(), completed];
    let checks = host_session_checks(&host, "revision_current", &sessions, Some(&newer), &[])
        .expect("terminal diagnostic checks");
    assert!(checks
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Passed));
}

#[test]
fn old_revision_and_cli_preflight_observations_remain_action_required() {
    let host = host("future");
    let mut old = managed_session("future", true);
    old.connection_integration_revision = "revision_old".to_owned();
    let stale =
        host_session_checks(&host, "revision_current", &[], Some(&old), &[]).expect("stale checks");
    assert!(stale
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Pending));
    assert_eq!(
        check_for(&stale, ConnectionCheckKind::HostSession).code(),
        Some("host_session_revision_stale")
    );

    old.session_source = volicord_types::McpRuntimeSessionSource::CliPreflight;
    let cli = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&old),
        Some(&old),
        &[],
    )
    .expect("CLI-preflight checks");
    assert!(cli
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Pending));
    assert_eq!(
        check_for(&cli, ConnectionCheckKind::HostSession).code(),
        Some("host_session_not_observed")
    );
}

#[test]
fn no_managed_host_activity_keeps_the_protocol_chain_pending() {
    let checks = host_session_checks(&host("future"), "revision_current", &[], None, &[])
        .expect("pending host checks");
    for id in [
        ConnectionCheckKind::ProcessStartup,
        ConnectionCheckKind::HostSession,
        ConnectionCheckKind::RequiredTools,
        ConnectionCheckKind::ToolRoundTrip,
    ] {
        let check = check_for(&checks, id);
        assert_eq!(check.status(), ConnectionCheckStatus::Pending);
        assert!(check.cause_finding_ids().is_empty());
    }
}

#[test]
fn managed_config_failure_blocks_process_and_protocol_checks() {
    let root = DiagnosticFindingId::parse("finding.managed_config_failure").unwrap();
    let checks = block_failed_dependencies(vec![
        canonical_check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Failed,
            "managed_config_malformed",
            "Managed configuration is malformed",
            None,
            None,
        )
        .unwrap()
        .with_cause_finding_ids(vec![root.clone()])
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Pending,
            "mcp_server_not_run",
            "MCP verification has not run",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::ProcessStartup,
            ConnectionCheckStatus::Pending,
            "process_startup_not_observed",
            "Process startup has not been observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::HostSession,
            ConnectionCheckStatus::Pending,
            "host_session_not_observed",
            "Initialize has not been observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::RequiredTools,
            ConnectionCheckStatus::Pending,
            "required_tools_not_observed",
            "Tools have not been observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::ToolRoundTrip,
            ConnectionCheckStatus::Pending,
            "tool_round_trip_not_observed",
            "Tool call has not been observed",
            None,
            None,
        )
        .unwrap(),
    ])
    .expect("blocked managed configuration graph");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ManagedConfig).status(),
        ConnectionCheckStatus::Failed
    );
    for id in [
        ConnectionCheckKind::McpServer,
        ConnectionCheckKind::ProcessStartup,
        ConnectionCheckKind::HostSession,
        ConnectionCheckKind::RequiredTools,
        ConnectionCheckKind::ToolRoundTrip,
    ] {
        let check = check_for(&checks, id);
        assert_eq!(check.status(), ConnectionCheckStatus::Blocked);
        assert_eq!(check.cause_finding_ids(), std::slice::from_ref(&root));
    }
    assert_eq!(
        actions_for_checks(&checks)
            .unwrap()
            .iter()
            .map(ConnectionAction::id)
            .collect::<Vec<_>>(),
        vec![ConnectionActionKind::RepairManagedConfig]
    );
}

#[test]
fn guard_file_failure_blocks_hook_execution_and_phase_observation() {
    let root = DiagnosticFindingId::parse("finding.guard_file_failure").unwrap();
    let checks = block_failed_dependencies(vec![
        canonical_check(
            ConnectionCheckKind::GuardFiles,
            ConnectionCheckStatus::Failed,
            "guard_files_failed",
            "Guard file integrity failed",
            None,
            None,
        )
        .unwrap()
        .with_cause_finding_ids(vec![root.clone()])
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::GuardHookExecution,
            ConnectionCheckStatus::Pending,
            "guard_hook_execution_pending",
            "Guard hook execution is not observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Pending,
            "guard_observation_pending",
            "Guard phases are not observed",
            None,
            None,
        )
        .unwrap(),
    ])
    .expect("blocked Guard graph");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::GuardHookExecution).status(),
        ConnectionCheckStatus::Blocked
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::GuardObservation).status(),
        ConnectionCheckStatus::Blocked
    );
    assert_eq!(
        actions_for_checks(&checks)
            .unwrap()
            .iter()
            .map(ConnectionAction::id)
            .collect::<Vec<_>>(),
        vec![ConnectionActionKind::RepairGuard]
    );
}

#[test]
fn actual_current_protocol_incompatibility_fails_only_demonstrated_checks() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;
    session.terminal_finding_id = Some("finding.protocol_contract_mismatch".to_owned());
    let checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("protocol checks");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).status(),
        ConnectionCheckStatus::Passed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::RequiredTools).status(),
        ConnectionCheckStatus::Passed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ToolRoundTrip).status(),
        ConnectionCheckStatus::Failed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ToolRoundTrip).code(),
        Some("tool_round_trip_failed")
    );
    assert_eq!(
        serde_json::to_value(actions_for_checks(&checks).expect("protocol action")).unwrap(),
        json!([{
            "id": "inspect_codex_protocol",
            "instruction": "Inspect the recorded Codex protocol failure, repair the incompatible configuration or behavior, then verify again",
        }])
    );
}

#[test]
fn initialize_failure_blocks_tools_list_and_tool_round_trip() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.initialize_completed_at = None;
    session.initialized_notification_at = None;
    session.tools_list_observed_at = None;
    session.required_tools_present = None;
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;
    session.terminal_finding_id = Some("finding.initialize_failed".to_owned());
    let checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("protocol checks");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).status(),
        ConnectionCheckStatus::Failed
    );
    for id in [
        ConnectionCheckKind::RequiredTools,
        ConnectionCheckKind::ToolRoundTrip,
    ] {
        let check = check_for(&checks, id);
        assert_eq!(check.status(), ConnectionCheckStatus::Blocked);
        assert_eq!(
            check.cause_finding_ids(),
            &[DiagnosticFindingId::parse("finding.initialize_failed").unwrap()]
        );
    }
}

#[test]
fn tool_discovery_failure_blocks_the_tool_call() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.tools_list_observed_at = None;
    session.required_tools_present = None;
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;
    session.terminal_finding_id = Some("finding.tools_list_failed".to_owned());
    let checks = host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("protocol checks");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).status(),
        ConnectionCheckStatus::Passed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::RequiredTools).status(),
        ConnectionCheckStatus::Failed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ToolRoundTrip).status(),
        ConnectionCheckStatus::Blocked
    );
}

#[test]
fn successful_cli_self_test_without_host_observation_is_action_required() {
    let host = host("unlisted-future-version");
    let mut checks = vec![
        managed_config_check(&host).expect("managed config check"),
        host_executable_check(&host).expect("host executable check"),
        project_trust_check(&host).expect("project trust check"),
        canonical_check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Passed,
            "mcp_server_ready",
            "MCP server passed",
            None,
            None,
        )
        .expect("MCP check"),
    ];
    checks.extend(
        host_session_checks(&host, "revision_current", &[], None, &[])
            .expect("pending host checks"),
    );
    let report = ConnectionVerificationReport::try_new(
        current_timestamp(),
        checks.clone(),
        actions_for_checks(&checks).expect("actions"),
    )
    .expect("canonical report");

    assert_eq!(report.status(), ConnectionStatus::ActionRequired);
    assert_eq!(
        report
            .actions()
            .iter()
            .map(ConnectionAction::id)
            .collect::<Vec<_>>(),
        vec![ConnectionActionKind::ObserveCodex]
    );
}

#[test]
fn managed_config_failures_keep_precise_codes() {
    let cases = [
        (ManagedConfigStatus::Missing, "managed_config_missing"),
        (
            ManagedConfigStatus::Unmanaged,
            "managed_config_ownership_conflict",
        ),
        (ManagedConfigStatus::Changed, "managed_config_mismatch"),
        (ManagedConfigStatus::Malformed, "managed_config_malformed"),
        (
            ManagedConfigStatus::Unavailable,
            "managed_config_unavailable",
        ),
    ];
    for (status, expected_code) in cases {
        let mut host = host("future");
        host.managed_config = status;
        let check = managed_config_check(&host).expect("managed config check");
        assert_eq!(check.status(), ConnectionCheckStatus::Failed);
        assert_eq!(check.code(), Some(expected_code));
    }
}

#[test]
fn unavailable_executable_is_a_failed_behavioral_check() {
    let mut host = host("future");
    host.host_executable = HostExecutableStatus::Unavailable;
    host.host_executable_code = "host_executable_probe_failed".to_owned();
    host.host_version = None;
    let check = host_executable_check(&host).expect("host executable check");
    assert_eq!(check.status(), ConnectionCheckStatus::Failed);
    assert_eq!(check.code(), Some("host_executable_probe_failed"));
}

#[test]
fn aggregation_and_actions_are_deterministic() {
    let checks = vec![
        canonical_check(
            ConnectionCheckKind::GuardFiles,
            ConnectionCheckStatus::Failed,
            "guard_files_failed",
            "Guard files failed",
            None,
            None,
        )
        .expect("Guard check"),
        canonical_check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Failed,
            "managed_config_malformed",
            "Config malformed",
            None,
            None,
        )
        .expect("config check"),
        canonical_check(
            ConnectionCheckKind::HostExecutable,
            ConnectionCheckStatus::Failed,
            "host_executable_failed",
            "Host executable failed",
            None,
            None,
        )
        .expect("executable check"),
    ];
    let first = actions_for_checks(&checks).expect("actions");
    let second = actions_for_checks(&checks).expect("repeat actions");
    assert_eq!(first, second);
    assert_eq!(
        first.iter().map(ConnectionAction::id).collect::<Vec<_>>(),
        vec![
            ConnectionActionKind::InstallOrRepairCodex,
            ConnectionActionKind::RepairGuard,
            ConnectionActionKind::RepairManagedConfig,
        ]
    );
    let report = ConnectionVerificationReport::try_new(current_timestamp(), checks, first.clone())
        .expect("canonical report");
    assert_eq!(report.status(), ConnectionStatus::Failed);
    assert_eq!(report.actions(), first);
}

#[test]
fn mcp_server_details_use_the_canonical_verification_role() {
    let check = mcp_server_check(
        &VerificationStep::passed_with_code("mcp_preflight_ready", "ready"),
        &McpVerification::from_exchange(crate::connection_command::McpExchangeOutcome::completed(
            crate::connection_command::McpExchangeProgress::observed(
                true,
                Some(vec![AgentToolId::LIST_PROJECTS.wire_name().to_owned()]),
                true,
                true,
                true,
            ),
        )),
    )
    .expect("MCP server check");
    let details = check.details().expect("MCP details").as_object();

    assert_eq!(
        details["self_test"]["safe_read_only_tool"],
        crate::connection_command::managed_host_round_trip_tool().wire_name()
    );
}

fn projected_self_test(
    progress: crate::connection_command::McpExchangeProgress,
    failure: Option<McpProcessFailure>,
) -> Value {
    let exchange = match failure {
        Some(failure) => crate::connection_command::McpExchangeOutcome::failed(progress, failure),
        None => crate::connection_command::McpExchangeOutcome::completed(progress),
    };
    let check = mcp_server_check(
        &VerificationStep::passed_with_code("mcp_preflight_ready", "ready"),
        &McpVerification::from_exchange(exchange),
    )
    .expect("MCP server check");
    check.details().expect("MCP details").as_object()["self_test"].clone()
}

#[test]
fn self_test_json_projects_explicit_exchange_progress_for_every_terminal_stage() {
    let not_started = projected_self_test(
        crate::connection_command::McpExchangeProgress::not_started(),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::Startup,
            "startup failed",
        )),
    );
    assert_eq!(not_started["initialize"], false);
    assert_eq!(not_started["tools_list_observed"], false);
    assert!(not_started.get("tools_list").is_none());

    let tools_list_failed = projected_self_test(
        crate::connection_command::McpExchangeProgress::observed(true, None, false, false, false),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::ToolsList,
            "tools/list failed",
        )),
    );
    assert_eq!(tools_list_failed["initialize"], true);
    assert_eq!(tools_list_failed["tools_list_observed"], false);
    assert!(tools_list_failed.get("tools_list").is_none());

    let observed_tools = vec!["fixture.alpha".to_owned(), "fixture.beta".to_owned()];
    let required_tools_failed = projected_self_test(
        crate::connection_command::McpExchangeProgress::observed(
            true,
            Some(observed_tools.clone()),
            false,
            false,
            false,
        ),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::ToolsList,
            "required tools failed",
        )),
    );
    assert_eq!(required_tools_failed["tools_list"], json!(observed_tools));
    assert_eq!(required_tools_failed["tools_list_observed"], true);
    assert_eq!(required_tools_failed["required_tools_validated"], false);

    let safe_call_failed = projected_self_test(
        crate::connection_command::McpExchangeProgress::observed(
            true,
            Some(vec![AgentToolId::LIST_PROJECTS.wire_name().to_owned()]),
            true,
            false,
            false,
        ),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::SafeToolCall,
            "designated read-only tool call failed",
        )),
    );
    assert_eq!(safe_call_failed["tools_list_observed"], true);
    assert_eq!(
        safe_call_failed["tools_list"],
        json!([AgentToolId::LIST_PROJECTS.wire_name()])
    );
    assert_eq!(safe_call_failed["required_tools_validated"], true);
    assert_eq!(safe_call_failed["safe_read_only_tool_completed"], false);

    let shutdown_failed = projected_self_test(
        crate::connection_command::McpExchangeProgress::observed(
            true,
            Some(vec![AgentToolId::LIST_PROJECTS.wire_name().to_owned()]),
            true,
            true,
            false,
        ),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::Shutdown,
            "shutdown failed",
        )),
    );
    assert_eq!(shutdown_failed["initialize"], true);
    assert_eq!(shutdown_failed["tools_list_observed"], true);
    assert_eq!(
        shutdown_failed["tools_list"],
        json!([AgentToolId::LIST_PROJECTS.wire_name()])
    );
    assert_eq!(shutdown_failed["required_tools_validated"], true);
    assert_eq!(shutdown_failed["safe_read_only_tool_completed"], true);
    assert_eq!(shutdown_failed["shutdown_completed"], false);
    assert_eq!(shutdown_failed["failure_stage"], "shutdown");

    let completed = projected_self_test(
        crate::connection_command::McpExchangeProgress::observed(
            true,
            Some(Vec::new()),
            true,
            true,
            true,
        ),
        None,
    );
    assert_eq!(completed["status"], "passed");
    assert_eq!(completed["initialize"], true);
    assert_eq!(completed["tools_list"], json!([]));
    assert_eq!(completed["tools_list_observed"], true);
    assert_eq!(completed["required_tools_validated"], true);
    assert_eq!(completed["safe_read_only_tool_completed"], true);
    assert_eq!(completed["shutdown_completed"], true);
    assert!(completed.get("failure").is_none());
}

#[test]
fn current_projection_selects_explicit_same_code_subjects_and_excludes_history() {
    let fixture = CoreFixture::new("current-diagnostic-projection").expect("fixture");
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )
    .expect("connection lookup")
    .expect("connection");
    let observed_at = UtcTimestamp::parse("2026-07-22T01:02:03Z").expect("time");
    let diagnostic = OperationalDiagnostic::Guard(GuardDiagnostic::ManagedFileMissing);
    let first_subject = GuardManagedArtifactSubject::for_connection(
        fixture.connection_id(),
        GuardManagedArtifact::VolicordPolicy,
        "/private/product/.volicord/guard-a.json",
    )
    .expect("first subject");
    let first = current_connection_finding(
        &connection,
        diagnostic,
        &first_subject,
        &GuardArtifactFacts::new("volicord_policy"),
        OperationalCheckState::Failed,
        observed_at.clone(),
    )
    .expect("first projection");
    let second_subject = GuardManagedArtifactSubject::for_connection(
        fixture.connection_id(),
        GuardManagedArtifact::HostHookConfig,
        "/private/product/.volicord/guard-b.json",
    )
    .expect("second subject");
    let second = current_connection_finding(
        &connection,
        diagnostic,
        &second_subject,
        &GuardArtifactFacts::new("host_hook_config"),
        OperationalCheckState::Failed,
        observed_at.clone(),
    )
    .expect("second projection");
    let root = OccurrenceDiagnosticFinding::try_new(
        DiagnosticFindingData::try_new(
            DiagnosticCode::parse("guard.managed_file.root_cause").expect("root code"),
            DiagnosticDomain::parse("guard").expect("root domain"),
            DiagnosticStage::parse("guard_files").expect("root stage"),
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("verification_test").expect("root source"),
            DiagnosticSubject::try_new("guard_owner", "root-cause").expect("root subject"),
            DiagnosticFacts::empty(),
            observed_at.clone(),
        )
        .expect("root data")
        .with_connection_id(AgentConnectionId::new(fixture.connection_id()))
        .expect("root connection")
        .with_integration_revision(
            connection_integration_revision(&connection).expect("connection revision"),
        ),
        None,
    )
    .expect("root occurrence");
    insert_occurrence_finding(fixture.runtime_home_path(), &root).expect("persist root cause");
    let second = volicord_types::CurrentDiagnosticFinding::try_new(
        second.key().clone(),
        second
            .snapshot()
            .clone()
            .with_causes(vec![DiagnosticCause::new(root.id())])
            .expect("root cause edge"),
    )
    .expect("second projection with cause");
    let unrelated_history = OccurrenceDiagnosticFinding::try_new(
        DiagnosticFindingData::try_new(
            DiagnosticCode::parse("guard.history.unrelated").expect("history code"),
            DiagnosticDomain::parse("guard").expect("history domain"),
            DiagnosticStage::parse("guard_files").expect("history stage"),
            DiagnosticSeverity::Warning,
            DiagnosticSource::parse("verification_test").expect("history source"),
            DiagnosticSubject::try_new("history", "unrelated-occurrence").expect("history subject"),
            DiagnosticFacts::empty(),
            observed_at.clone(),
        )
        .expect("history data"),
        None,
    )
    .expect("unrelated occurrence");
    insert_occurrence_finding(fixture.runtime_home_path(), &unrelated_history)
        .expect("persist unrelated occurrence");
    let first_id = first.id().clone();
    let second_id = second.id().clone();
    let root_id = root.id();
    let unrelated_history_id = unrelated_history.id();
    let scope = first.key().scope().clone();
    reconcile_current_findings_for_scope(
        fixture.runtime_home_path(),
        &scope,
        &[CurrentOperationalOwner::Guard],
        &[first, second],
        observed_at.clone(),
    )
    .expect("persist artifacts");

    let trust_subject =
        TrustSubject::for_repository(fixture.connection_id(), "/private/product/history-only")
            .expect("trust subject");
    let historical = current_connection_finding(
        &connection,
        OperationalDiagnostic::Trust(TrustDiagnostic::RepositoryNotTrusted),
        &trust_subject,
        &TrustFacts::from_status(ProjectTrustStatus::Untrusted),
        OperationalCheckState::Pending,
        observed_at.clone(),
    )
    .expect("historical projection");
    let historical_id = historical.id().clone();
    reconcile_current_findings_for_scope(
        fixture.runtime_home_path(),
        &scope,
        &[CurrentOperationalOwner::Trust],
        &[historical],
        observed_at,
    )
    .expect("persist unrelated history");

    assert_ne!(first_id, second_id);
    let checks = vec![with_direct_causes(
        canonical_check(
            ConnectionCheckKind::GuardFiles,
            ConnectionCheckStatus::Failed,
            "guard_files_failed",
            "Two managed Guard artifacts are missing",
            None,
            None,
        )
        .expect("check"),
        vec![second_id.clone(), first_id.clone()],
    )
    .expect("causes")];
    let report = ConnectionVerificationReport::try_new(
        UtcTimestamp::parse("2026-07-22T01:02:04Z").expect("time"),
        checks.clone(),
        actions_for_checks(&checks).expect("actions"),
    )
    .expect("report");
    let (findings, _) = current_report_findings(fixture.runtime_home_path(), &connection, &report)
        .expect("projection");
    let mut expected_ids = vec![first_id.clone(), root_id.clone(), second_id.clone()];
    expected_ids.sort();
    assert_eq!(
        findings
            .iter()
            .map(|finding| finding.id().clone())
            .collect::<Vec<_>>(),
        expected_ids
    );
    let first_finding = findings
        .iter()
        .find(|finding| finding.id() == &first_id)
        .expect("first selected current finding");
    let second_finding = findings
        .iter()
        .find(|finding| finding.id() == &second_id)
        .expect("second selected current finding");
    assert_eq!(first_finding.code(), second_finding.code());
    assert_ne!(first_finding.subject(), second_finding.subject());
    assert_ne!(first_finding.facts(), second_finding.facts());
    assert_eq!(second_finding.causes()[0].finding_id(), &root_id);
    assert!(!findings
        .iter()
        .any(|finding| finding.id() == &historical_id));
    assert!(!findings
        .iter()
        .any(|finding| finding.id() == &unrelated_history_id));

    reconcile_current_findings_for_scope(
        fixture.runtime_home_path(),
        &scope,
        &[CurrentOperationalOwner::Guard],
        &[],
        UtcTimestamp::parse("2026-07-22T01:03:04Z").expect("time"),
    )
    .expect("resolve artifacts");
    let (resolved_findings, _) =
        current_report_findings(fixture.runtime_home_path(), &connection, &report)
            .expect("resolved projection");
    assert!(resolved_findings.is_empty());
}
