use volicord_store::diagnostic_findings::insert_occurrence_finding;
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{DiagnosticCause, DiagnosticFindingData, OccurrenceDiagnosticFinding};

use super::*;
use crate::host_integration::verification::ProjectTrustDiagnostic;

const CURRENT_REVISION: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const OLD_REVISION: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";

#[cfg(unix)]
#[test]
fn active_verification_writeability_probe_is_bounded_and_detects_read_only_project_store() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = CoreFixture::new("connection-active-writeability-probe").expect("fixture");
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )
    .expect("connection lookup")
    .expect("connection");
    let writable = verify_selected_store_writeability(
        &fixture.mutation_context().expect("mutation context"),
        fixture.runtime_home_path(),
        &connection,
        Some(fixture.project_id()),
    );
    assert!(writable.failure.is_none());
    assert_eq!(writable.registry_write, McpEvidenceCheckStatus::Passed);
    assert_eq!(
        writable.project_writes[0].state_write(),
        McpEvidenceCheckStatus::Passed
    );
    let project = volicord_store::bootstrap::project_record_read_only(
        fixture.runtime_home_path(),
        fixture.project_id(),
    )
    .expect("project lookup")
    .expect("project");
    std::fs::set_permissions(
        &project.state_db_path,
        std::fs::Permissions::from_mode(0o444),
    )
    .expect("read-only project database");
    let read_only = verify_selected_store_writeability(
        &fixture.mutation_context().expect("mutation context"),
        fixture.runtime_home_path(),
        &connection,
        Some(fixture.project_id()),
    );
    assert!(read_only
        .failure
        .as_deref()
        .is_some_and(|error| error.contains("writeability probe reported read-only storage")));
    assert_eq!(
        read_only.project_writes[0].state_write(),
        McpEvidenceCheckStatus::Failed
    );

    let registry = rusqlite::Connection::open_with_flags(
        volicord_store::sqlite::registry_db_path(fixture.runtime_home_path()),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read-only Registry");
    let probe_tables: u64 = registry
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = '__volicord_write_probe_do_not_persist'",
            [],
            |row| row.get(0),
        )
        .expect("probe table count");
    assert_eq!(probe_tables, 0);
}

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

#[test]
fn unknown_project_trust_remains_a_typed_pending_observation() {
    let mut verification = host("999.123-preview+custom");
    verification.project_trust = Some(ProjectTrustDiagnostic {
        status: ProjectTrustStatus::Unknown,
        code: "project_trust_config_unavailable".to_owned(),
        config_path: String::new(),
        repo_root: "/repo".to_owned(),
        details: "Codex user configuration could not be located".to_owned(),
    });

    let check = project_trust_check(&verification).expect("project trust check");
    assert_eq!(check.status(), ConnectionCheckStatus::Pending);
    assert_eq!(check.code(), Some("project_trust_config_unavailable"));
    let details = check
        .details()
        .expect("typed project trust details")
        .as_object();
    assert_eq!(details.get("observed_state"), Some(&json!("unknown")));
    assert_eq!(details.get("repo_root"), Some(&json!("/repo")));
}

fn managed_session(version: &str, required_tools_present: bool) -> McpRuntimeSessionRecord {
    McpRuntimeSessionRecord {
        runtime_session_id: "mcp_runtime_fixture".to_owned(),
        connection_internal_id: "connection_fixture".to_owned(),
        session_source: volicord_types::McpRuntimeSessionSource::ManagedHost,
        connection_integration_revision: CURRENT_REVISION.to_owned(),
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
        returned_tool_identities: Some(vec![
            crate::connection_command::managed_host_round_trip_tool()
                .wire_name()
                .to_owned(),
        ]),
        required_tools_present: Some(required_tools_present),
        required_tools_validated_at: required_tools_present
            .then(|| "2026-07-18T00:00:03Z".to_owned()),
        verification_tool_name: required_tools_present.then(|| {
            crate::connection_command::managed_host_round_trip_tool()
                .wire_name()
                .to_owned()
        }),
        verification_tool_observed_at: required_tools_present
            .then(|| "2026-07-18T00:00:04Z".to_owned()),
        last_observed_at: "2026-07-18T00:00:04Z".to_owned(),
        terminal_finding_id: None,
        graceful_close_at: None,
    }
}

fn test_host_session_checks(
    host: &Verification,
    _current_revision: &str,
    sessions: &[McpRuntimeSessionRecord],
    latest: Option<&McpRuntimeSessionRecord>,
    tool_round_trip_finding_ids: &[DiagnosticFindingId],
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let revision = IntegrationRevision::parse(CURRENT_REVISION).expect("current revision");
    let selection = McpSessionEvidenceSelection::select(&revision, sessions)?;
    host_session_checks(
        host,
        &revision,
        &selection,
        latest,
        tool_round_trip_finding_ids,
    )
}

fn check_for(checks: &[ConnectionCheck], id: ConnectionCheckKind) -> &ConnectionCheck {
    let id = match id {
        ConnectionCheckKind::ProcessStartup => ConnectionCheckKind::HostReload,
        ConnectionCheckKind::HostSession => ConnectionCheckKind::ManagedSessionHealth,
        ConnectionCheckKind::RequiredTools | ConnectionCheckKind::ToolRoundTrip => {
            ConnectionCheckKind::ManagedCapabilityProof
        }
        ConnectionCheckKind::GuardFiles | ConnectionCheckKind::GuardObservation => {
            ConnectionCheckKind::AmbientHookCoverage
        }
        id => id,
    };
    checks
        .iter()
        .find(|check| check.id() == id)
        .expect("expected connection check")
}

#[test]
fn changed_hook_definition_resets_activation_to_the_host_owned_workflow() {
    let passed = |id, details| {
        canonical_check(
            id,
            ConnectionCheckStatus::Passed,
            "passed",
            "passed",
            details,
            Some("2026-07-18T00:00:00Z"),
        )
        .unwrap()
    };
    let current = ConnectionVerificationReport::try_new(
        current_timestamp(),
        vec![
            passed(ConnectionCheckKind::ManagedConfig, None),
            passed(ConnectionCheckKind::HostReload, None),
            passed(
                ConnectionCheckKind::HookSourceActivation,
                Some(json!({"activation_state": "effective_by_observation"})),
            ),
            passed(ConnectionCheckKind::ManagedSessionHealth, None),
            passed(ConnectionCheckKind::ManagedCapabilityProof, None),
            passed(ConnectionCheckKind::AmbientHookCoverage, None),
            passed(ConnectionCheckKind::CorrelatedGuardVerification, None),
        ],
        IntegrationActivationPlan::empty(IntegrationActivationState::Complete),
    )
    .unwrap();
    assert_eq!(
        current.hook_activation_state(),
        HookActivationState::EffectiveByObservation
    );
    assert!(!current
        .activation_plan()
        .required_steps()
        .iter()
        .any(|action| action.id() == ActivationStepId::ReviewProjectHooks));

    let changed = report_with_hook_review_required(&current).unwrap();
    assert_eq!(
        changed.hook_activation_state(),
        HookActivationState::ReviewRequiredBySetup
    );
    assert_eq!(
        changed.activation_state(),
        volicord_types::IntegrationActivationState::HostReloadRequired
    );
    assert_eq!(
        changed
            .activation_plan()
            .required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![
            ActivationStepId::ReloadCodex,
            ActivationStepId::ReviewProjectHooks,
            ActivationStepId::RequestIntegrationVerification,
            ActivationStepId::ReadConnectionStatus,
        ]
    );
}

#[test]
fn arbitrary_future_version_can_complete_managed_host_checks() {
    let host = host("999.123-preview+custom");
    let session = managed_session("999.123-preview+custom", true);

    let session_checks = test_host_session_checks(
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
        activation_plan_for_checks(&checks).expect("activation plan"),
    )
    .expect("canonical report");
    assert_eq!(report.status(), ConnectionStatus::Complete);
}

#[test]
fn peer_path_version_mismatch_does_not_invalidate_managed_evidence() {
    let host = host("1000.0-new-host");
    let session = managed_session("999.123-preview+custom", true);

    let checks = test_host_session_checks(
        &host,
        "revision_current",
        std::slice::from_ref(&session),
        Some(&session),
        &[],
    )
    .expect("valid checks");

    assert!(checks
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Passed));
    let details = check_for(&checks, ConnectionCheckKind::HostSession)
        .details()
        .expect("managed-session details")
        .as_object();
    assert_eq!(
        details["managed_peer"]["client_info"]["version"],
        "999.123-preview+custom"
    );
    assert_eq!(
        details["host_executable_probe"]["version"],
        "1000.0-new-host"
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

    let checks = test_host_session_checks(
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
        details["verification_tool"]["observed_tool_identity"],
        volicord_types::AgentToolId::STATUS.wire_name()
    );
    assert_eq!(
        details["verification_tool"]["expected_tool_identity"],
        expected_verification_tool_name()
    );
}

#[test]
fn initialize_response_without_initialized_notification_remains_pending() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.negotiated_protocol_version = None;
    session.initialized_notification_at = None;
    session.tools_list_observed_at = None;
    session.returned_tool_identities = None;
    session.required_tools_present = None;
    session.required_tools_validated_at = None;
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;

    let checks = test_host_session_checks(
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
fn latest_attempt_health_is_not_hidden_by_an_older_complete_proof() {
    let host = host("future");
    let completed = managed_session("future", true);
    let mut newer = managed_session("future", true);
    newer.runtime_session_id = "mcp_runtime_newer".to_owned();
    newer.selected_protocol_version = None;
    newer.initialize_completed_at = None;
    newer.initialized_notification_at = None;
    newer.negotiated_protocol_version = None;
    newer.tools_list_observed_at = None;
    newer.returned_tool_identities = None;
    newer.required_tools_present = None;
    newer.required_tools_validated_at = None;
    newer.verification_tool_name = None;
    newer.verification_tool_observed_at = None;
    newer.last_observed_at = "2026-07-18T00:01:00Z".to_owned();

    let sessions = vec![newer.clone(), completed.clone()];
    let checks = test_host_session_checks(&host, "revision_current", &sessions, Some(&newer), &[])
        .expect("concurrent session checks");
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).status(),
        ConnectionCheckStatus::Pending
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::RequiredTools).status(),
        ConnectionCheckStatus::Passed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ToolRoundTrip).status(),
        ConnectionCheckStatus::Passed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession)
            .details()
            .unwrap()
            .as_object()["runtime_session_id"],
        "mcp_runtime_newer"
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::RequiredTools)
            .details()
            .unwrap()
            .as_object()["runtime_session_id"],
        "mcp_runtime_fixture"
    );

    newer.terminal_finding_id = Some("finding.later_crash".to_owned());
    let sessions = vec![newer.clone(), completed];
    let checks = test_host_session_checks(&host, "revision_current", &sessions, Some(&newer), &[])
        .expect("terminal diagnostic checks");
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::HostSession).status(),
        ConnectionCheckStatus::Failed
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::RequiredTools).status(),
        ConnectionCheckStatus::Blocked
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ToolRoundTrip).status(),
        ConnectionCheckStatus::Blocked
    );
}

#[test]
fn old_revision_and_cli_preflight_observations_remain_action_required() {
    let host = host("future");
    let mut old = managed_session("future", true);
    old.connection_integration_revision = OLD_REVISION.to_owned();
    let stale = test_host_session_checks(&host, "revision_current", &[], Some(&old), &[])
        .expect("stale checks");
    assert!(stale
        .iter()
        .all(|check| check.status() == ConnectionCheckStatus::Pending));
    assert_eq!(
        check_for(&stale, ConnectionCheckKind::HostSession).code(),
        Some("host_session_revision_stale")
    );

    old.session_source = volicord_types::McpRuntimeSessionSource::CliPreflight;
    let cli = test_host_session_checks(
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
    let checks = test_host_session_checks(&host("future"), "revision_current", &[], None, &[])
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
            ConnectionCheckKind::HostReload,
            ConnectionCheckStatus::Pending,
            "process_startup_not_observed",
            "Process startup has not been observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::ManagedSessionHealth,
            ConnectionCheckStatus::Pending,
            "host_session_not_observed",
            "Initialize has not been observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::ManagedCapabilityProof,
            ConnectionCheckStatus::Pending,
            "required_tools_not_observed",
            "Tools have not been observed",
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
    ] {
        let check = check_for(&checks, id);
        assert_eq!(check.status(), ConnectionCheckStatus::Blocked);
        assert_eq!(check.cause_finding_ids(), std::slice::from_ref(&root));
    }
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ManagedCapabilityProof).status(),
        ConnectionCheckStatus::Blocked
    );
    assert_eq!(
        activation_plan_for_checks(&checks)
            .unwrap()
            .required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![ActivationStepId::RepairManagedConfiguration]
    );
}

#[test]
fn guard_file_failure_blocks_hook_execution_and_phase_observation() {
    let root = DiagnosticFindingId::parse("finding.guard_file_failure").unwrap();
    let checks = block_failed_dependencies(vec![
        canonical_check(
            ConnectionCheckKind::HookSourceActivation,
            ConnectionCheckStatus::Failed,
            "hook_source_contract_failed",
            "Hook source contract failed",
            Some(json!({"activation_state": "disabled"})),
            None,
        )
        .unwrap()
        .with_cause_finding_ids(vec![root.clone()])
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::AmbientHookCoverage,
            ConnectionCheckStatus::Pending,
            "ambient_hook_coverage_pending",
            "Ambient Guard hook coverage is not observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::CorrelatedGuardVerification,
            ConnectionCheckStatus::Pending,
            "guard_verification_pending",
            "Guard correlation is not verified",
            None,
            None,
        )
        .unwrap(),
    ])
    .expect("blocked Guard graph");

    assert_eq!(
        check_for(&checks, ConnectionCheckKind::AmbientHookCoverage).status(),
        ConnectionCheckStatus::Blocked
    );
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::CorrelatedGuardVerification,).status(),
        ConnectionCheckStatus::Blocked
    );
    assert_eq!(
        activation_plan_for_checks(&checks)
            .unwrap()
            .required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![ActivationStepId::RepairHookContract]
    );
}

#[test]
fn actual_current_protocol_incompatibility_fails_only_demonstrated_checks() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;
    session.terminal_finding_id = Some("finding.protocol_contract_mismatch".to_owned());
    let checks = test_host_session_checks(
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
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ManagedCapabilityProof).status(),
        ConnectionCheckStatus::Blocked,
    );
    assert_eq!(
        activation_plan_for_checks(&checks)
            .expect("protocol action")
            .required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![ActivationStepId::ReadConnectionStatus]
    );
}

#[test]
fn initialize_failure_blocks_tools_list_and_tool_round_trip() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.selected_protocol_version = None;
    session.initialize_completed_at = None;
    session.initialized_notification_at = None;
    session.negotiated_protocol_version = None;
    session.tools_list_observed_at = None;
    session.returned_tool_identities = None;
    session.required_tools_present = None;
    session.required_tools_validated_at = None;
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;
    session.terminal_finding_id = Some("finding.initialize_failed".to_owned());
    let checks = test_host_session_checks(
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
    let required = check_for(&checks, ConnectionCheckKind::RequiredTools);
    assert_eq!(required.status(), ConnectionCheckStatus::Blocked);
    assert_eq!(
        required.cause_finding_ids(),
        &[DiagnosticFindingId::parse("finding.initialize_failed").unwrap()]
    );
    let round_trip = check_for(&checks, ConnectionCheckKind::ToolRoundTrip);
    assert_eq!(round_trip.status(), ConnectionCheckStatus::Blocked);
    assert_eq!(
        round_trip.cause_finding_ids(),
        &[DiagnosticFindingId::parse("finding.initialize_failed").unwrap()]
    );
}

#[test]
fn tool_discovery_failure_blocks_the_tool_call() {
    let host = host("future");
    let mut session = managed_session("future", true);
    session.tools_list_observed_at = None;
    session.returned_tool_identities = None;
    session.required_tools_present = None;
    session.required_tools_validated_at = None;
    session.verification_tool_name = None;
    session.verification_tool_observed_at = None;
    session.terminal_finding_id = Some("finding.tools_list_failed".to_owned());
    let checks = test_host_session_checks(
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
    assert_eq!(
        check_for(&checks, ConnectionCheckKind::ManagedCapabilityProof).status(),
        ConnectionCheckStatus::Blocked,
    );
}

#[test]
fn successful_cli_active_verification_without_host_observation_is_action_required() {
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
        test_host_session_checks(&host, "revision_current", &[], None, &[])
            .expect("pending host checks"),
    );
    let report = ConnectionVerificationReport::try_new(
        current_timestamp(),
        checks.clone(),
        activation_plan_for_checks(&checks).expect("activation plan"),
    )
    .expect("canonical report");

    assert_eq!(report.status(), ConnectionStatus::ActionRequired);
    assert_eq!(
        report
            .activation_plan()
            .required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![
            ActivationStepId::ReloadCodex,
            ActivationStepId::ReviewProjectHooks,
            ActivationStepId::RequestIntegrationVerification,
            ActivationStepId::ReadConnectionStatus,
        ]
    );
}

#[test]
fn current_status_activation_plan_projects_only_the_remaining_suffix() {
    let checks = vec![
        canonical_check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Passed,
            "managed_config_ready",
            "Managed configuration is ready",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::HostReload,
            ConnectionCheckStatus::Passed,
            "host_reload_observed",
            "Codex loaded the current integration",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::HookSourceActivation,
            ConnectionCheckStatus::Passed,
            "hook_source_effective",
            "Current hooks were observed",
            Some(json!({"activation_state": "effective_by_observation"})),
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::ManagedSessionHealth,
            ConnectionCheckStatus::Pending,
            "managed_session_not_observed",
            "Managed session has not been observed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::ManagedCapabilityProof,
            ConnectionCheckStatus::Pending,
            "managed_capability_not_observed",
            "Managed capability proof has not been observed",
            None,
            None,
        )
        .unwrap(),
    ];
    let plan = activation_plan_for_checks(&checks).unwrap();

    assert_eq!(
        plan.state(),
        IntegrationActivationState::McpObservationRequired
    );
    assert_eq!(
        plan.required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![ActivationStepId::RequestIntegrationVerification]
    );
    assert_eq!(
        plan.optional_diagnostics()[0].id(),
        ActivationStepId::RunOptionalActiveDiagnostics
    );
}

#[test]
fn correlated_repair_required_projects_the_typed_repair_step_without_a_guard_probe() {
    let checks = vec![
        canonical_check(
            ConnectionCheckKind::AmbientHookCoverage,
            ConnectionCheckStatus::Passed,
            "ambient_hook_coverage_passed",
            "Current hook coverage passed",
            None,
            None,
        )
        .unwrap(),
        canonical_check(
            ConnectionCheckKind::CorrelatedGuardVerification,
            ConnectionCheckStatus::Failed,
            "correlated_guard_verification_failed",
            "The correlated attempt requires repair",
            Some(json!({
                "recoverability": "recoverable",
                "latest_attempt": {
                    "attempt_state": "repair_required",
                    "recovery_action": "repair_hook_contract"
                }
            })),
            None,
        )
        .unwrap(),
    ];
    let plan = activation_plan_for_checks(&checks).unwrap();

    assert_eq!(
        plan.required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![ActivationStepId::RepairHookContract]
    );
    assert!(plan.required_steps().iter().all(|step| step
        .agent_sequence()
        .iter()
        .all(|nested| nested.tool() != AgentToolId::GUARD_PROBE)));
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
fn aggregation_and_activation_plans_are_deterministic() {
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
    let first = activation_plan_for_checks(&checks).expect("activation plan");
    let second = activation_plan_for_checks(&checks).expect("repeat actions");
    assert_eq!(first, second);
    assert_eq!(
        first
            .required_steps()
            .iter()
            .map(ActivationStep::id)
            .collect::<Vec<_>>(),
        vec![ActivationStepId::RepairManagedConfiguration]
    );
    let report = ConnectionVerificationReport::try_new(current_timestamp(), checks, first.clone())
        .expect("canonical report");
    assert_eq!(report.status(), ConnectionStatus::Failed);
    assert_eq!(report.activation_plan(), &first);
}

#[test]
fn mcp_server_details_use_the_canonical_verification_role() {
    let handshake =
        McpVerification::from_exchange(crate::connection_command::McpExchangeOutcome::completed(
            crate::connection_command::McpExchangeProgress::observed(
                true,
                Some(vec![AgentToolId::LIST_PROJECTS.wire_name().to_owned()]),
                true,
                true,
                true,
            ),
        ));
    let active = active_verification_evidence(
        &McpStoreWriteabilityEvidence {
            registry_write: McpEvidenceCheckStatus::Passed,
            project_writes: Vec::new(),
            failure: None,
        },
        &handshake,
        UtcTimestamp::parse("2026-07-25T01:02:03Z").expect("timestamp"),
    );
    let check = mcp_server_check(
        &VerificationStep::passed_with_code("mcp_preflight_ready", "ready"),
        &handshake.with_active_evidence(active),
    )
    .expect("MCP server check");
    let details = check.details().expect("MCP details").as_object();

    assert_eq!(
        details["last_active_verification"]["protocol_conformance"][0]["safe_read_only_tool"],
        crate::connection_command::managed_host_round_trip_tool().wire_name()
    );
    assert_eq!(
        details["last_active_verification"]["source"],
        "connection_verify"
    );
    assert_eq!(
        details["last_active_verification"]["observed_at"],
        "2026-07-25T01:02:03Z"
    );
}

fn projected_active_probe(
    progress: crate::connection_command::McpExchangeProgress,
    failure: Option<McpProcessFailure>,
) -> Value {
    let exchange = match failure {
        Some(failure) => crate::connection_command::McpExchangeOutcome::failed(progress, failure),
        None => crate::connection_command::McpExchangeOutcome::completed(progress),
    };
    let handshake = McpVerification::from_exchange(exchange);
    let active = active_verification_evidence(
        &McpStoreWriteabilityEvidence {
            registry_write: McpEvidenceCheckStatus::Passed,
            project_writes: Vec::new(),
            failure: None,
        },
        &handshake,
        UtcTimestamp::parse("2026-07-25T01:02:03Z").expect("timestamp"),
    );
    let check = mcp_server_check(
        &VerificationStep::passed_with_code("mcp_preflight_ready", "ready"),
        &handshake.with_active_evidence(active),
    )
    .expect("MCP server check");
    check.details().expect("MCP details").as_object()["last_active_verification"]
        ["protocol_conformance"][0]
        .clone()
}

#[test]
fn active_evidence_projects_explicit_exchange_progress_for_every_terminal_stage() {
    let not_started = projected_active_probe(
        crate::connection_command::McpExchangeProgress::not_started(),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::Startup,
            "startup failed",
        )),
    );
    assert_eq!(not_started["initialize"], false);
    assert_eq!(not_started["tools_list_observed"], false);
    assert_eq!(not_started["tools_returned"], Value::Null);

    let tools_list_failed = projected_active_probe(
        crate::connection_command::McpExchangeProgress::observed(true, None, false, false, false),
        Some(McpProcessFailure::protocol(
            crate::connection_command::McpStage::ToolsList,
            "tools/list failed",
        )),
    );
    assert_eq!(tools_list_failed["initialize"], true);
    assert_eq!(tools_list_failed["tools_list_observed"], false);
    assert_eq!(tools_list_failed["tools_returned"], Value::Null);

    let observed_tools = vec!["fixture.alpha".to_owned(), "fixture.beta".to_owned()];
    let required_tools_failed = projected_active_probe(
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
    assert_eq!(
        required_tools_failed["tools_returned"],
        observed_tools.len()
    );
    assert_eq!(required_tools_failed["tools_list_observed"], true);
    assert_eq!(required_tools_failed["required_tools_validated"], false);

    let safe_call_failed = projected_active_probe(
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
    assert_eq!(safe_call_failed["tools_returned"], 1);
    assert_eq!(safe_call_failed["required_tools_validated"], true);
    assert_eq!(safe_call_failed["safe_read_only_tool_completed"], false);

    let shutdown_failed = projected_active_probe(
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
    assert_eq!(shutdown_failed["tools_returned"], 1);
    assert_eq!(shutdown_failed["required_tools_validated"], true);
    assert_eq!(shutdown_failed["safe_read_only_tool_completed"], true);
    assert_eq!(shutdown_failed["shutdown_completed"], false);
    assert_eq!(shutdown_failed["failure_stage"], "shutdown");

    let completed = projected_active_probe(
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
    assert_eq!(completed["tools_returned"], 0);
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
    insert_occurrence_finding(
        &fixture.mutation_context().expect("mutation context"),
        &root,
    )
    .expect("persist root cause");
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
    insert_occurrence_finding(
        &fixture.mutation_context().expect("mutation context"),
        &unrelated_history,
    )
    .expect("persist unrelated occurrence");
    let first_id = first.id().clone();
    let second_id = second.id().clone();
    let root_id = root.id();
    let unrelated_history_id = unrelated_history.id();
    let scope = first.key().scope().clone();
    reconcile_current_findings_for_scope(
        &fixture.mutation_context().expect("mutation context"),
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
        &fixture.mutation_context().expect("mutation context"),
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
        activation_plan_for_checks(&checks).expect("activation plan"),
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
        &fixture.mutation_context().expect("mutation context"),
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

#[test]
fn inline_current_finding_resolves_before_store_without_missing_record_substitution() {
    let fixture = CoreFixture::new("inline-current-diagnostic-overlay").expect("fixture");
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )
    .expect("connection lookup")
    .expect("connection");
    let subject = GuardEventSubject::for_connection(
        fixture.connection_id(),
        "guard_event_inline_incompatible",
    )
    .expect("subject");
    let inline = current_connection_finding(
        &connection,
        OperationalDiagnostic::Guard(GuardDiagnostic::IncompatibleObservation),
        &subject,
        &GuardEventFacts::default(),
        OperationalCheckState::Failed,
        UtcTimestamp::parse("2026-07-23T01:02:03Z").expect("time"),
    )
    .expect("inline finding");
    let check = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Failed,
            "guard_observation_failed",
            "A current Guard event reported an incompatible hook contract",
            None,
            None,
        )
        .expect("check"),
        vec![inline.id().clone()],
    )
    .expect("check cause");
    let mut overlay = DiagnosticFindingOverlay::default();
    overlay.insert_inline_current(&inline);

    let (findings, _) = current_report_findings_with_overlay(
        fixture.runtime_home_path(),
        &connection,
        &[check],
        &overlay,
    )
    .expect("overlay projection");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].id(), inline.id());
    assert_eq!(
        findings[0].code().as_str(),
        "guard.observation.incompatible"
    );
    assert_ne!(
        findings[0].code().as_str(),
        "diagnostics.finding_record_missing"
    );
}

#[test]
fn mixed_inline_and_persisted_causes_form_one_bounded_graph() {
    let fixture = CoreFixture::new("mixed-current-diagnostic-overlay").expect("fixture");
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )
    .expect("connection lookup")
    .expect("connection");
    let observed_at = UtcTimestamp::parse("2026-07-23T02:03:04Z").expect("time");
    let persisted_root = OccurrenceDiagnosticFinding::try_new(
        DiagnosticFindingData::try_new(
            DiagnosticCode::parse("guard.contract.persisted_root").expect("code"),
            DiagnosticDomain::parse("guard").expect("domain"),
            DiagnosticStage::parse("guard_observation").expect("stage"),
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("verification_test").expect("source"),
            DiagnosticSubject::try_new("guard_event", "persisted-root").expect("subject"),
            DiagnosticFacts::empty(),
            observed_at.clone(),
        )
        .expect("data")
        .with_connection_id(AgentConnectionId::new(fixture.connection_id()))
        .expect("connection")
        .with_integration_revision(connection_integration_revision(&connection).expect("revision")),
        None,
    )
    .expect("persisted root");
    insert_occurrence_finding(
        &fixture.mutation_context().expect("mutation context"),
        &persisted_root,
    )
    .expect("persist root");

    let subject = GuardEventSubject::for_connection(
        fixture.connection_id(),
        "guard_event_inline_with_persisted_cause",
    )
    .expect("subject");
    let inline = current_connection_finding(
        &connection,
        OperationalDiagnostic::Guard(GuardDiagnostic::IncompatibleObservation),
        &subject,
        &GuardEventFacts::default(),
        OperationalCheckState::Failed,
        observed_at,
    )
    .expect("inline finding");
    let inline = volicord_types::CurrentDiagnosticFinding::try_new(
        inline.key().clone(),
        inline
            .snapshot()
            .clone()
            .with_causes(vec![DiagnosticCause::new(persisted_root.id())])
            .expect("cause"),
    )
    .expect("inline with cause");
    let check = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Failed,
            "guard_observation_failed",
            "A current Guard event reported an incompatible hook contract",
            None,
            None,
        )
        .expect("check"),
        vec![inline.id().clone()],
    )
    .expect("check cause");
    let mut overlay = DiagnosticFindingOverlay::default();
    overlay.insert_inline_current(&inline);
    overlay.insert_persisted_seed(persisted_root.id());
    let (findings, _) = current_report_findings_with_overlay(
        fixture.runtime_home_path(),
        &connection,
        &[check],
        &overlay,
    )
    .expect("mixed graph");

    assert_eq!(findings.len(), 2);
    assert_eq!(
        volicord_types::diagnostic_root_cause_ids(
            &findings,
            std::slice::from_ref(inline.id()),
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
        )
        .expect("roots"),
        vec![persisted_root.id()]
    );
}

#[test]
fn explicitly_persisted_reference_deleted_from_store_gets_missing_record_finding() {
    let fixture = CoreFixture::new("deleted-persisted-diagnostic-overlay").expect("fixture");
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )
    .expect("connection lookup")
    .expect("connection");
    let persisted = OccurrenceDiagnosticFinding::try_new(
        DiagnosticFindingData::try_new(
            DiagnosticCode::parse("guard.contract.deleted_cause").expect("code"),
            DiagnosticDomain::parse("guard").expect("domain"),
            DiagnosticStage::parse("guard_observation").expect("stage"),
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("verification_test").expect("source"),
            DiagnosticSubject::try_new("guard_event", "deleted-cause").expect("subject"),
            DiagnosticFacts::empty(),
            UtcTimestamp::parse("2026-07-23T03:04:05Z").expect("time"),
        )
        .expect("data"),
        None,
    )
    .expect("finding");
    insert_occurrence_finding(
        &fixture.mutation_context().expect("mutation context"),
        &persisted,
    )
    .expect("persist finding");
    let registry_path = volicord_store::sqlite::registry_db_path(fixture.runtime_home_path());
    let registry = rusqlite::Connection::open(registry_path).expect("registry");
    registry
        .execute(
            "DELETE FROM diagnostic_findings WHERE finding_id = ?1",
            [persisted.id().as_str()],
        )
        .expect("delete persisted cause");
    drop(registry);

    let check = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Failed,
            "guard_observation_failed",
            "A persisted Guard cause was deleted",
            None,
            None,
        )
        .expect("check"),
        vec![persisted.id()],
    )
    .expect("check cause");
    let mut overlay = DiagnosticFindingOverlay::default();
    overlay.insert_persisted_seed(persisted.id());
    let (findings, _) = current_report_findings_with_overlay(
        fixture.runtime_home_path(),
        &connection,
        &[check],
        &overlay,
    )
    .expect("missing projection");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].id(), &persisted.id());
    assert_eq!(
        findings[0].code().as_str(),
        "diagnostics.finding_record_missing"
    );
}
