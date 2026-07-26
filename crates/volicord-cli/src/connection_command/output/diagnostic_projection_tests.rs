use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};
use volicord_store::diagnostic_findings::{
    insert_occurrence_finding, stored_diagnostic_findings_by_ids,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::connection_verification::{
    derive_integration_activation_state, ActivationStep, ActivationStepId, ConnectionCheck,
    ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus,
    ConnectionVerificationReport, HookActivationState, IntegrationActivationPlan,
};
use volicord_types::diagnostics::{
    DiagnosticAction, DiagnosticCode, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts,
    DiagnosticFinding, DiagnosticFindingData, DiagnosticFindingId, DiagnosticSeverity,
    DiagnosticSource, DiagnosticStage, DiagnosticSubject, OccurrenceDiagnosticFinding,
    MAX_DIAGNOSTIC_FACT_BYTES, MAX_DIAGNOSTIC_FACT_STRING_BYTES,
};
use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::values::UtcTimestamp;

use super::{render_command_report, CommandConnection, CommandOperation, ConnectionCommandReport};
use crate::connection_command::{args::HumanOutputDetail, OutputFormat};

#[derive(Debug, Serialize)]
struct ProjectionFacts {
    summary: &'static str,
    observation_state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_revision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_revision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negotiated_revision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    production_supported_revisions: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attempted_client_name: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attempted_client_version: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounded_stderr_excerpt: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_values: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_token: Option<&'static str>,
}

impl DiagnosticFactSource for ProjectionFacts {}

impl ProjectionFacts {
    fn actual_expected(
        summary: &'static str,
        actual: &'static str,
        expected: &'static str,
    ) -> Self {
        Self {
            summary,
            observation_state: "observed",
            actual: Some(actual),
            expected: Some(expected),
            requested_revision: None,
            selected_revision: None,
            negotiated_revision: None,
            production_supported_revisions: None,
            attempted_client_name: None,
            attempted_client_version: None,
            timeout_ms: None,
            process_exit_code: None,
            bounded_stderr_excerpt: None,
            observed_values: None,
            api_token: None,
        }
    }
}

fn timestamp() -> UtcTimestamp {
    UtcTimestamp::parse("2026-07-22T01:02:03Z").unwrap()
}

fn integration_revision() -> IntegrationRevision {
    IntegrationRevision::parse(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .unwrap()
}

fn finding(
    id: &str,
    code: &str,
    domain: &str,
    facts: ProjectionFacts,
    action_code: &str,
    action_summary: &str,
) -> DiagnosticFinding {
    DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(id).unwrap(),
        DiagnosticCode::parse(code).unwrap(),
        DiagnosticDomain::parse(domain).unwrap(),
        DiagnosticStage::parse("verification").unwrap(),
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("projection_test").unwrap(),
        DiagnosticSubject::try_new("connection", "connection_projection").unwrap(),
        DiagnosticFacts::project(&facts).unwrap(),
        timestamp(),
    )
    .and_then(|finding| {
        finding.with_actions(vec![DiagnosticAction::try_new(
            DiagnosticCode::parse(action_code)?,
            action_summary,
        )?])
    })
    .and_then(|finding| finding.with_connection_id(AgentConnectionId::new("connection_projection")))
    .map(|finding| finding.with_integration_revision(integration_revision()))
    .unwrap()
}

fn matrix_finding(
    id: &str,
    code: &str,
    domain: &str,
    facts: ProjectionFacts,
    severity: DiagnosticSeverity,
    action_code: Option<&str>,
) -> DiagnosticFinding {
    let finding = DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(id).unwrap(),
        DiagnosticCode::parse(code).unwrap(),
        DiagnosticDomain::parse(domain).unwrap(),
        DiagnosticStage::parse("verification").unwrap(),
        severity,
        DiagnosticSource::parse("failure_matrix").unwrap(),
        DiagnosticSubject::try_new("scenario", id).unwrap(),
        DiagnosticFacts::project(&facts).unwrap(),
        timestamp(),
    )
    .unwrap();
    match action_code {
        Some(action_code) => finding
            .with_actions(vec![DiagnosticAction::try_new(
                DiagnosticCode::parse(action_code).unwrap(),
                "Apply the typed remediation for this root cause",
            )
            .unwrap()])
            .unwrap(),
        None => finding,
    }
}

fn matrix_occurrence(finding: &DiagnosticFinding) -> OccurrenceDiagnosticFinding {
    let data = DiagnosticFindingData::try_new(
        finding.code().clone(),
        finding.domain().clone(),
        finding.stage().clone(),
        finding.severity(),
        finding.source().clone(),
        finding.subject().clone(),
        finding.facts().clone(),
        finding.observed_at().clone(),
    )
    .and_then(|data| data.with_causes(finding.causes().to_vec()))
    .and_then(|data| data.with_actions(finding.actions().to_vec()))
    .expect("matrix occurrence data");
    OccurrenceDiagnosticFinding::try_new(data, None).expect("matrix occurrence")
}

#[derive(Clone, Copy)]
struct DiagnosticMatrixScenario {
    name: &'static str,
    code: &'static str,
    domain: &'static str,
    finding_action: Option<&'static str>,
    primary_check: ConnectionCheckKind,
    primary_status: ConnectionCheckStatus,
    blocked_checks: &'static [ConnectionCheckKind],
    connection_action: Option<ActivationStepId>,
    report_action: Option<&'static str>,
    is_root: bool,
}

fn typed_action_id_for_diagnostic(code: &str) -> &'static str {
    if code == "action.host.reload_after_configuration_change" {
        "reload_codex"
    } else if code.starts_with("action.host.") {
        "read_connection_status"
    } else if code.starts_with("action.managed_config.")
        || code.starts_with("action.storage.")
        || code.starts_with("action.store.")
        || code.starts_with("action.runtime_home.")
    {
        "repair_managed_configuration"
    } else if matches!(
        code,
        "action.guard.trigger_phase" | "action.guard.retry_verification"
    ) {
        "request_integration_verification"
    } else if code.starts_with("action.guard.") {
        "repair_hook_contract"
    } else if code.starts_with("action.process.")
        || code.starts_with("action.mcp.")
        || code.starts_with("action.protocol.")
    {
        "read_connection_status"
    } else {
        "repair_managed_configuration"
    }
}

impl DiagnosticMatrixScenario {
    #[allow(clippy::too_many_arguments)]
    const fn failure(
        name: &'static str,
        code: &'static str,
        domain: &'static str,
        finding_action: Option<&'static str>,
        primary_check: ConnectionCheckKind,
        blocked_checks: &'static [ConnectionCheckKind],
        connection_action: Option<ActivationStepId>,
        report_action: &'static str,
    ) -> Self {
        Self {
            name,
            code,
            domain,
            finding_action,
            primary_check,
            primary_status: ConnectionCheckStatus::Failed,
            blocked_checks,
            connection_action,
            report_action: Some(report_action),
            is_root: true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    const fn observation(
        name: &'static str,
        code: &'static str,
        domain: &'static str,
        finding_action: Option<&'static str>,
        primary_check: ConnectionCheckKind,
        primary_status: ConnectionCheckStatus,
        connection_action: Option<ActivationStepId>,
        report_action: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            code,
            domain,
            finding_action,
            primary_check,
            primary_status,
            blocked_checks: &[],
            connection_action,
            report_action,
            is_root: false,
        }
    }
}

fn check(
    kind: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    causes: &[&str],
    details: Option<Value>,
) -> ConnectionCheck {
    let details = details
        .map(|value| ConnectionCheckDetails::try_new(value.as_object().unwrap().clone()).unwrap());
    ConnectionCheck::try_new(
        kind,
        status,
        causes
            .iter()
            .map(|id| DiagnosticFindingId::parse(*id).unwrap())
            .collect(),
        (status != ConnectionCheckStatus::Passed).then(|| code.to_owned()),
        summary,
        details,
        Some(timestamp()),
    )
    .unwrap()
}

fn report(
    checks: Vec<ConnectionCheck>,
    findings: Vec<DiagnosticFinding>,
    required_steps: Vec<ActivationStep>,
) -> ConnectionCommandReport {
    let state = derive_integration_activation_state(&checks, HookActivationState::Unknown);
    let activation_plan =
        IntegrationActivationPlan::try_new(state, required_steps, Vec::new()).unwrap();
    let verification =
        ConnectionVerificationReport::try_new(timestamp(), checks, activation_plan).unwrap();
    ConnectionCommandReport::from_verification(
        CommandOperation::Verify,
        None,
        Path::new("/runtime"),
        CommandConnection::new(
            "connection_projection",
            "codex",
            "user",
            "workflow",
            Path::new("/workspace/product"),
            "/home/user/.codex/config.toml",
        ),
        &verification,
    )
    .with_diagnostic_findings(findings, Some(integration_revision()))
}

fn projections(report: &ConnectionCommandReport) -> (String, String, Value) {
    let concise = render_command_report(OutputFormat::Human(HumanOutputDetail::Concise), report)
        .unwrap()
        .output;
    let verbose = render_command_report(OutputFormat::Human(HumanOutputDetail::Verbose), report)
        .unwrap()
        .output;
    let json = render_command_report(OutputFormat::Json, report)
        .unwrap()
        .output;
    (concise, verbose, serde_json::from_str(&json).unwrap())
}

fn assert_same_roots(concise: &str, verbose: &str, json: &Value, expected: &[&str]) {
    assert_eq!(json["root_cause_ids"], json!(expected));
    for id in expected {
        assert!(concise.contains(&format!("Finding: {id}")), "{concise}");
        assert!(verbose.contains(&format!("[root] {id}")), "{verbose}");
    }
}

fn role_check(kind: ConnectionCheckKind, role: &str, runtime_session_id: &str) -> ConnectionCheck {
    check(
        kind,
        ConnectionCheckStatus::Passed,
        "passed",
        "Role-bearing session evidence passed",
        &[],
        Some(json!({
            "evidence_role": role,
            "runtime_session_id": runtime_session_id,
        })),
    )
}

#[test]
fn report_context_deduplicates_same_session_roles_with_human_json_parity() {
    let runtime_session_id = "runtime_session_same_proof";
    let report = report(
        vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
                "passed",
                "Managed configuration passed",
                &[],
                None,
            ),
            role_check(
                ConnectionCheckKind::ProcessStartup,
                "latest_managed_attempt",
                runtime_session_id,
            ),
            role_check(
                ConnectionCheckKind::HostSession,
                "latest_managed_attempt",
                runtime_session_id,
            ),
            role_check(
                ConnectionCheckKind::RequiredTools,
                "latest_managed_capability_proof",
                runtime_session_id,
            ),
            role_check(
                ConnectionCheckKind::ToolRoundTrip,
                "latest_managed_capability_proof",
                runtime_session_id,
            ),
        ],
        Vec::new(),
        Vec::new(),
    );
    let (_, verbose, json) = projections(&report);
    assert_eq!(
        json["connection"]["runtime_session_ids"],
        json!([runtime_session_id])
    );
    assert_eq!(
        json["connection"]["runtime_sessions"],
        json!([{
            "id": runtime_session_id,
            "roles": ["latest_managed_attempt", "latest_managed_capability_proof"],
        }])
    );
    assert!(verbose.contains(&format!(
        "Runtime sessions: {runtime_session_id} (latest_managed_attempt, latest_managed_capability_proof)"
    )));
}

#[test]
fn report_context_contains_both_distinct_role_bearing_sessions() {
    let report = report(
        vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
                "passed",
                "Managed configuration passed",
                &[],
                None,
            ),
            role_check(
                ConnectionCheckKind::ProcessStartup,
                "latest_managed_attempt",
                "runtime_session_attempt",
            ),
            role_check(
                ConnectionCheckKind::HostSession,
                "latest_managed_attempt",
                "runtime_session_attempt",
            ),
            role_check(
                ConnectionCheckKind::RequiredTools,
                "latest_managed_capability_proof",
                "runtime_session_proof",
            ),
            role_check(
                ConnectionCheckKind::ToolRoundTrip,
                "latest_managed_capability_proof",
                "runtime_session_proof",
            ),
        ],
        Vec::new(),
        Vec::new(),
    );
    let (_, verbose, json) = projections(&report);
    assert_eq!(
        json["connection"]["runtime_sessions"],
        json!([
            {"id": "runtime_session_attempt", "roles": ["latest_managed_attempt"]},
            {"id": "runtime_session_proof", "roles": ["latest_managed_capability_proof"]},
        ])
    );
    assert!(verbose.contains("runtime_session_attempt (latest_managed_attempt)"));
    assert!(verbose.contains("runtime_session_proof (latest_managed_capability_proof)"));
}

#[test]
fn guard_failure_keeps_typed_attempt_context_and_renderer_parity() {
    let report = report(
        vec![
            check(
                ConnectionCheckKind::AmbientHookCoverage,
                ConnectionCheckStatus::Passed,
                "ambient_hook_coverage_passed",
                "A current managed Guard hook executed",
                &[],
                Some(json!({
                    "current_hook_definition_executed": true,
                    "configured_phases_observed": true,
                })),
            ),
            check(
                ConnectionCheckKind::CorrelatedGuardVerification,
                ConnectionCheckStatus::Failed,
                "correlated_guard_verification_failed",
                "The latest correlated Guard verification requires repair",
                &[],
                Some(json!({
                    "recoverability": "recoverable",
                    "latest_attempt": {
                        "evidence_role": "guard_verification_attempt",
                        "verification_id": "guard_verification_current",
                        "runtime_session_id": "runtime_session_guard",
                        "host_session_id": "host_session_guard",
                        "host_turn_id": "host_turn_guard",
                        "attempt_state": "repair_required",
                        "expected_agent_tool_id": "volicord.guard_probe",
                        "expected_host_callable_identity": "mcp__volicord__guard_probe",
                        "observed_host_callable_identity": "mcp__other__guard_probe",
                        "acquisition_stage": "callable_identity_mismatch",
                        "repair_reason": "callable_identity_mismatch",
                        "retry_policy": "new_turn_required",
                        "recovery_action": "request_integration_verification",
                    },
                    "latest_completed_proof": {
                        "evidence_role": "guard_verification_proof",
                        "verification_id": "guard_verification_older",
                        "runtime_session_id": "runtime_session_guard",
                    }
                })),
            ),
        ],
        Vec::new(),
        Vec::new(),
    );

    let (concise, verbose, json) = projections(&report);
    assert_eq!(json["status"], "action_required");
    assert_eq!(
        json["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .find(|check| check["id"] == "correlated_guard_verification")
            .expect("correlated check")["status"],
        "failed"
    );
    assert!(concise.contains("Hook installation and ambient execution: passed"));
    assert!(concise.contains("Correlated Guard verification: failed"));
    assert!(concise.contains("Reason: callable_identity_mismatch"));
    for expected in [
        "Verification ID: guard_verification_current",
        "Runtime session: runtime_session_guard",
        "Host session: host_session_guard",
        "Host turn: host_turn_guard",
        "Attempt state: repair_required",
        "Acquisition stage: callable_identity_mismatch",
        "Expected host callable: mcp__volicord__guard_probe",
        "Observed host callable: mcp__other__guard_probe",
        "Retry policy: new_turn_required",
    ] {
        assert!(verbose.contains(expected), "{verbose}");
    }
    assert_eq!(
        json["connection"]["runtime_sessions"],
        json!([{
            "id": "runtime_session_guard",
            "roles": ["guard_verification_attempt", "guard_verification_proof"],
        }])
    );
    assert_eq!(
        json["connection"]["verification_ids"],
        json!(["guard_verification_current", "guard_verification_older"])
    );
}

#[test]
fn protocol_mismatch_projection_is_exact_and_actionable() {
    let id = "finding.protocol_mismatch";
    let mut facts = ProjectionFacts::actual_expected(
        "the requested MCP protocol revision is unsupported",
        "2024-11-05",
        "one production-supported revision",
    );
    facts.requested_revision = Some("2024-11-05");
    facts.selected_revision = Some("2025-11-25");
    facts.negotiated_revision = None;
    facts.production_supported_revisions = Some(vec!["2025-06-18", "2025-11-25"]);
    facts.attempted_client_name = Some("codex");
    facts.attempted_client_version = Some("0.42.0");
    let root = finding(
        id,
        "mcp.protocol.unsupported_revision",
        "mcp",
        facts,
        "action.mcp.use_supported_protocol_revision",
        "Configure the client to request a production-supported protocol revision",
    )
    .with_runtime_session_id(AgentRuntimeSessionId::new("runtime_session_projection"))
    .unwrap();
    let report = report(
        vec![
            check(
                ConnectionCheckKind::HostSession,
                ConnectionCheckStatus::Failed,
                "host_session_protocol_mismatch",
                "MCP initialize selected no supported protocol",
                &[id],
                Some(json!({
                    "evidence_role": "latest_managed_attempt",
                    "runtime_session_id": "runtime_session_projection",
                    "managed_peer": {
                        "client_info": {"name": "codex", "version": "0.42.0"},
                        "requested_protocol_revision": "2024-11-05",
                        "selected_protocol_revision": "2025-11-25",
                    },
                    "host_executable_probe": {
                        "discovered_path": "/opt/codex",
                        "version": "0.42.0",
                    },
                    "terminal_finding_id": id,
                })),
            ),
            check(
                ConnectionCheckKind::RequiredTools,
                ConnectionCheckStatus::Failed,
                "required_tools_not_proven",
                "tools/list did not produce a complete same-session proof",
                &[id],
                None,
            ),
            check(
                ConnectionCheckKind::ToolRoundTrip,
                ConnectionCheckStatus::Blocked,
                "blocked_by_failed_prerequisite",
                "read-only tool validation was blocked by initialize",
                &[id],
                None,
            ),
        ],
        vec![root],
        Vec::new(),
    );
    let (concise, verbose, json) = projections(&report);
    assert_same_roots(&concise, &verbose, &json, &[id]);
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["operation"], "verify");
    assert_eq!(json["status"], "failed");
    assert_eq!(
        json["findings"][0]["code"],
        "mcp.protocol.unsupported_revision"
    );
    assert_eq!(
        json["activation_plan"]["required_steps"],
        json!([{
            "id": "read_connection_status",
            "initiator": "user",
            "executor": "volicord",
            "execution_channel": "cli",
            "prerequisites": [],
            "completes_checks": [],
            "root_finding_ids": [id],
            "instruction": "Configure the client to request a production-supported protocol revision",
            "diagnostic_only": false,
            "agent_sequence": [],
        }])
    );
    assert!(concise.contains("Actual MCP client: codex 0.42.0"));
    assert!(concise.contains("Requested protocol: 2024-11-05"));
    assert!(concise.contains("Supported protocols: 2025-06-18, 2025-11-25"));
    assert!(concise.contains("Blocked checks: tool_round_trip"));
    assert!(concise
        .contains("Configure the client to request a production-supported protocol revision"));
    assert!(!concise.contains("inspect the failure"));
    for expected in [
        "Requested protocol: 2024-11-05",
        "Selected protocol: 2025-11-25",
        "Actual MCP peer: codex",
        "PATH executable: /opt/codex",
        "Runtime sessions: runtime_session_projection",
        "Integration revision: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Blocked by: required_tools",
        "Report limits",
    ] {
        assert!(
            verbose.contains(expected),
            "missing {expected:?}\n{verbose}"
        );
    }
}

#[test]
fn failure_scenarios_have_exact_lossless_root_projections() {
    let scenarios = [
        (
            "process_timeout",
            "finding.process_timeout",
            "process.supervision.timeout",
            "process",
            ProjectionFacts {
                timeout_ms: Some(5_000),
                bounded_stderr_excerpt: Some("startup remained incomplete"),
                ..ProjectionFacts::actual_expected(
                    "the managed MCP process exceeded its startup deadline",
                    "timeout",
                    "initialize before deadline",
                )
            },
            "action.process.resolve_timeout",
        ),
        (
            "readonly_database",
            "finding.readonly_database",
            "storage.sqlite.readonly_database",
            "storage",
            ProjectionFacts::actual_expected(
                "the Runtime Home database rejected a required write",
                "read_only",
                "writable",
            ),
            "action.storage.restore_writable_runtime_home",
        ),
        (
            "managed_config_drift",
            "finding.managed_config_drift",
            "host.codex.managed_config_drift",
            "host",
            ProjectionFacts::actual_expected(
                "the managed Codex entry differs from the canonical plan",
                "changed",
                "canonical",
            ),
            "action.managed_config.repair",
        ),
        (
            "guard_integrity",
            "finding.guard_integrity",
            "guard.integrity.managed_file_mismatch",
            "guard",
            ProjectionFacts::actual_expected(
                "a Guard managed file failed its integrity check",
                "digest_mismatch",
                "expected_digest",
            ),
            "action.guard.repair",
        ),
    ];

    for (name, id, code, domain, facts, action_code) in scenarios {
        let root = finding(
            id,
            code,
            domain,
            facts,
            action_code,
            "Apply the typed remediation for this root cause",
        );
        let report = report(
            vec![check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Failed,
                "typed_failure",
                "Typed verification failure",
                &[id],
                None,
            )],
            vec![root],
            Vec::new(),
        );
        let (concise, verbose, json) = projections(&report);
        assert_same_roots(&concise, &verbose, &json, &[id]);
        assert_eq!(json["findings"][0]["code"], code, "{name}");
        assert_eq!(
            json["activation_plan"]["required_steps"][0]["id"],
            typed_action_id_for_diagnostic(action_code),
            "{name}"
        );
        assert_eq!(
            json["findings"][0]["facts"]["data"]["observation_state"],
            "observed"
        );
        assert!(concise.contains(code), "{name}: {concise}");
        assert!(verbose.contains("Bounded typed facts"), "{name}: {verbose}");
    }
}

#[test]
fn diagnostic_failure_matrix_persists_bounded_roots_and_agrees_across_projections() {
    const CONFIG_BLOCKED: &[ConnectionCheckKind] = &[
        ConnectionCheckKind::McpServer,
        ConnectionCheckKind::ProcessStartup,
        ConnectionCheckKind::HostSession,
    ];
    const PROCESS_BLOCKED: &[ConnectionCheckKind] = &[ConnectionCheckKind::HostSession];
    const HOST_SESSION_BLOCKED: &[ConnectionCheckKind] = &[];
    const TOOLS_BLOCKED: &[ConnectionCheckKind] = &[ConnectionCheckKind::ToolRoundTrip];
    const GUARD_BLOCKED: &[ConnectionCheckKind] =
        &[ConnectionCheckKind::CorrelatedGuardVerification];

    let scenarios = [
        DiagnosticMatrixScenario::failure(
            "runtime_home_missing",
            "runtime_home.path.missing",
            "runtime_home",
            Some("action.runtime_home.correct_path"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.runtime_home.correct_path",
        ),
        DiagnosticMatrixScenario::failure(
            "runtime_home_relative",
            "runtime_home.path.empty_or_relative",
            "runtime_home",
            Some("action.runtime_home.correct_path"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.runtime_home.correct_path",
        ),
        DiagnosticMatrixScenario::failure(
            "runtime_home_permission",
            "runtime_home.permission.denied",
            "runtime_home",
            Some("action.runtime_home.repair_permissions"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.runtime_home.repair_permissions",
        ),
        DiagnosticMatrixScenario::failure(
            "registry_missing",
            "runtime_home.registry.missing",
            "runtime_home",
            Some("action.runtime_home.initialize_registry"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.runtime_home.initialize_registry",
        ),
        DiagnosticMatrixScenario::failure(
            "sqlite_readonly",
            "store.sqlite.readonly",
            "storage",
            Some("action.store.repair_write_access"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.store.repair_write_access",
        ),
        DiagnosticMatrixScenario::failure(
            "sqlite_busy",
            "store.sqlite.busy",
            "storage",
            Some("action.store.free_locked_database"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.store.free_locked_database",
        ),
        DiagnosticMatrixScenario::failure(
            "sqlite_schema_mismatch",
            "store.schema.mismatch",
            "storage",
            Some("action.store.repair_schema"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.store.repair_schema",
        ),
        DiagnosticMatrixScenario::failure(
            "sqlite_corruption",
            "store.integrity.corruption_failure",
            "storage",
            Some("action.store.restore_database"),
            ConnectionCheckKind::McpServer,
            &[],
            None,
            "action.store.restore_database",
        ),
        DiagnosticMatrixScenario::failure(
            "managed_config_missing",
            "managed_config.entry.missing",
            "configuration",
            Some("action.managed_config.repair"),
            ConnectionCheckKind::ManagedConfig,
            CONFIG_BLOCKED,
            None,
            "action.managed_config.repair",
        ),
        DiagnosticMatrixScenario::failure(
            "managed_config_disabled",
            "managed_config.entry.disabled",
            "configuration",
            Some("action.managed_config.repair"),
            ConnectionCheckKind::ManagedConfig,
            CONFIG_BLOCKED,
            None,
            "action.managed_config.repair",
        ),
        DiagnosticMatrixScenario::failure(
            "managed_config_malformed",
            "managed_config.toml.parse_failed",
            "configuration",
            Some("action.managed_config.repair"),
            ConnectionCheckKind::ManagedConfig,
            CONFIG_BLOCKED,
            None,
            "action.managed_config.repair",
        ),
        DiagnosticMatrixScenario::failure(
            "managed_config_drifted",
            "managed_config.command.drift",
            "configuration",
            Some("action.managed_config.repair"),
            ConnectionCheckKind::ManagedConfig,
            CONFIG_BLOCKED,
            None,
            "action.managed_config.repair",
        ),
        DiagnosticMatrixScenario::failure(
            "host_executable_missing",
            "installation.executable.missing",
            "installation",
            Some("action.installation.reinstall_current_build"),
            ConnectionCheckKind::HostExecutable,
            &[],
            None,
            "action.installation.reinstall_current_build",
        ),
        DiagnosticMatrixScenario::observation(
            "host_peer_path_version_mismatch",
            "host.codex.peer_version_differs_from_path_probe",
            "host",
            None,
            ConnectionCheckKind::HostExecutable,
            ConnectionCheckStatus::Passed,
            None,
            None,
        ),
        DiagnosticMatrixScenario::failure(
            "process_spawn_failure",
            "process.spawn.failed",
            "process",
            Some("action.process.repair_launch"),
            ConnectionCheckKind::ProcessStartup,
            PROCESS_BLOCKED,
            None,
            "action.process.repair_launch",
        ),
        DiagnosticMatrixScenario::failure(
            "process_timeout",
            "process.initialize.timeout",
            "process",
            Some("action.process.resolve_timeout"),
            ConnectionCheckKind::ProcessStartup,
            PROCESS_BLOCKED,
            None,
            "action.process.resolve_timeout",
        ),
        DiagnosticMatrixScenario::failure(
            "process_early_exit",
            "process.child.exited",
            "process",
            Some("action.process.repair_child_exit"),
            ConnectionCheckKind::ProcessStartup,
            PROCESS_BLOCKED,
            None,
            "action.process.repair_child_exit",
        ),
        DiagnosticMatrixScenario::failure(
            "process_pipe_failure",
            "process.pipe.read_failed",
            "process",
            Some("action.process.repair_stdio"),
            ConnectionCheckKind::ProcessStartup,
            PROCESS_BLOCKED,
            None,
            "action.process.repair_stdio",
        ),
        DiagnosticMatrixScenario::failure(
            "process_cleanup_failure",
            "process.cleanup.failed",
            "process",
            Some("action.process.repair_cleanup"),
            ConnectionCheckKind::ProcessStartup,
            PROCESS_BLOCKED,
            None,
            "action.process.repair_cleanup",
        ),
        DiagnosticMatrixScenario::failure(
            "json_rpc_parse_failure",
            "mcp.json_rpc.parse_error",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::HostSession,
            HOST_SESSION_BLOCKED,
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "json_rpc_invalid_request",
            "mcp.json_rpc.invalid_request",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::HostSession,
            HOST_SESSION_BLOCKED,
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "lifecycle_ordering_failure",
            "mcp.lifecycle.operation_before_ready",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::HostSession,
            HOST_SESSION_BLOCKED,
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "protocol_mismatch",
            "mcp.protocol.unsupported_version",
            "mcp",
            Some("action.mcp.use_supported_protocol_revision"),
            ConnectionCheckKind::HostSession,
            HOST_SESSION_BLOCKED,
            None,
            "action.mcp.use_supported_protocol_revision",
        ),
        DiagnosticMatrixScenario::failure(
            "counter_offer_rejected",
            "mcp.protocol.counter_offer_rejected",
            "mcp",
            Some("action.mcp.use_supported_protocol_revision"),
            ConnectionCheckKind::HostSession,
            HOST_SESSION_BLOCKED,
            None,
            "action.mcp.use_supported_protocol_revision",
        ),
        DiagnosticMatrixScenario::failure(
            "capability_failure",
            "mcp.protocol.capability_shape_invalid",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::HostSession,
            HOST_SESSION_BLOCKED,
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "tools_list_failure",
            "mcp.tools.protocol_error",
            "mcp",
            Some("action.mcp.restore_required_tools"),
            ConnectionCheckKind::RequiredTools,
            TOOLS_BLOCKED,
            None,
            "action.mcp.restore_required_tools",
        ),
        DiagnosticMatrixScenario::failure(
            "required_tool_missing",
            "mcp.tools.required_missing",
            "mcp",
            Some("action.mcp.restore_required_tools"),
            ConnectionCheckKind::RequiredTools,
            TOOLS_BLOCKED,
            None,
            "action.mcp.restore_required_tools",
        ),
        DiagnosticMatrixScenario::failure(
            "invalid_codex_metadata",
            "host.codex.metadata_malformed",
            "host",
            Some("action.host.repair_session_correlation"),
            ConnectionCheckKind::ToolRoundTrip,
            &[],
            None,
            "action.host.repair_session_correlation",
        ),
        DiagnosticMatrixScenario::failure(
            "session_correlation_failure",
            "mcp.tool_call.session_correlation_invalid",
            "mcp",
            Some("action.mcp.repair_read_only_tool"),
            ConnectionCheckKind::ToolRoundTrip,
            &[],
            None,
            "action.mcp.repair_read_only_tool",
        ),
        DiagnosticMatrixScenario::failure(
            "tool_validation_failure",
            "mcp.tool_call.invalid_arguments",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::ToolRoundTrip,
            &[],
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "tool_execution_failure",
            "mcp.tool_call.adapter_execution_failed",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::ToolRoundTrip,
            &[],
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "response_budget_failure",
            "mcp.tool_call.response_budget_failed",
            "mcp",
            Some("action.mcp.repair_protocol_exchange"),
            ConnectionCheckKind::ToolRoundTrip,
            &[],
            None,
            "action.mcp.repair_protocol_exchange",
        ),
        DiagnosticMatrixScenario::failure(
            "guard_file_integrity_failure",
            "guard.managed_file.integrity_failed",
            "guard",
            Some("action.guard.repair"),
            ConnectionCheckKind::AmbientHookCoverage,
            GUARD_BLOCKED,
            None,
            "action.guard.repair",
        ),
        DiagnosticMatrixScenario::observation(
            "guard_phase_unobserved",
            "guard.phase.required_not_observed",
            "guard",
            Some("action.guard.trigger_phase"),
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Pending,
            Some(ActivationStepId::RequestIntegrationVerification),
            Some("action.host.observe_activity"),
        ),
        DiagnosticMatrixScenario::observation(
            "stale_integration_revision",
            "revision.integration.stale",
            "revision",
            Some("action.host.reload_after_configuration_change"),
            ConnectionCheckKind::HostSession,
            ConnectionCheckStatus::Pending,
            Some(ActivationStepId::ReloadCodex),
            Some("action.host.reload_after_configuration_change"),
        ),
        DiagnosticMatrixScenario::failure(
            "unexpected_internal_failure",
            "internal.unexpected_failure",
            "internal",
            None,
            ConnectionCheckKind::McpServer,
            &[],
            Some(ActivationStepId::RepairManagedConfiguration),
            "action.mcp.repair_server",
        ),
    ];

    let fixture = CoreFixture::new("diagnostic-failure-matrix").unwrap();
    for scenario in scenarios {
        let mut id = format!("finding.matrix.{}", scenario.name);
        let mut facts = ProjectionFacts::actual_expected(
            "the typed diagnostic matrix scenario was observed",
            scenario.name,
            "successful observation",
        );
        facts.observed_values = Some(vec!["bounded-value"; 40]);
        facts.api_token = Some("matrix-secret-must-not-escape");
        let root = matrix_finding(
            &id,
            scenario.code,
            scenario.domain,
            facts,
            if scenario.is_root {
                DiagnosticSeverity::Error
            } else {
                DiagnosticSeverity::Warning
            },
            scenario.finding_action,
        );
        let occurrence = matrix_occurrence(&root);
        let root = occurrence.to_diagnostic_finding();
        id = root.id().to_string();
        insert_occurrence_finding(
            &fixture.mutation_context().expect("mutation context"),
            &occurrence,
        )
        .unwrap();
        assert_eq!(
            stored_diagnostic_findings_by_ids(
                fixture.runtime_home_path(),
                std::slice::from_ref(root.id()),
            )
            .unwrap()
            .into_iter()
            .map(|finding| finding.to_diagnostic_finding())
            .collect::<Vec<_>>(),
            vec![root.clone()],
            "{} did not round-trip through Registry persistence",
            scenario.name
        );
        let serialized_facts = serde_json::to_string(root.facts()).unwrap();
        assert!(serialized_facts.len() <= MAX_DIAGNOSTIC_FACT_BYTES);
        assert!(!serialized_facts.contains("matrix-secret-must-not-escape"));
        assert_eq!(root.facts().data()["api_token"], "[redacted]");
        assert!(root.facts().data()["observed_values"]
            .as_array()
            .is_some_and(|values| values.len() == 32));
        assert!(root.facts().data().values().all(|value| value
            .as_str()
            .is_none_or(|text| text.len() <= MAX_DIAGNOSTIC_FACT_STRING_BYTES)));

        let causes = scenario
            .is_root
            .then_some(&id[..])
            .into_iter()
            .collect::<Vec<_>>();
        let mut checks = vec![check(
            scenario.primary_check,
            scenario.primary_status,
            "typed_matrix_failure",
            "Typed diagnostic matrix observation",
            &causes,
            None,
        )];
        checks.extend(scenario.blocked_checks.iter().map(|kind| {
            check(
                *kind,
                ConnectionCheckStatus::Blocked,
                "blocked_by_failed_prerequisite",
                "Blocked by the typed matrix root cause",
                &[&id],
                None,
            )
        }));
        let connection_actions = scenario
            .connection_action
            .map(|kind| {
                ActivationStep::try_new(kind, Vec::new(), "Apply the typed report action").unwrap()
            })
            .into_iter()
            .collect();
        let report = report(checks, vec![root], connection_actions);
        let (concise, verbose, json) = projections(&report);
        let expected_roots = scenario
            .is_root
            .then_some(id.as_str())
            .into_iter()
            .collect::<Vec<_>>();
        assert_same_roots(&concise, &verbose, &json, &expected_roots);
        if scenario.is_root {
            assert!(
                concise.contains(scenario.code),
                "{}: {concise}",
                scenario.name
            );
            assert!(
                verbose.contains(scenario.code),
                "{}: {verbose}",
                scenario.name
            );
        } else {
            assert!(
                !concise.contains("Finding:"),
                "{}: {concise}",
                scenario.name
            );
            assert!(!verbose.contains("[root]"), "{}: {verbose}", scenario.name);
        }
        assert_eq!(
            json["findings"][0]["code"], scenario.code,
            "{}",
            scenario.name
        );

        let mut actual_blocked = json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|check| check["status"] == "blocked")
            .filter_map(|check| check["id"].as_str())
            .collect::<Vec<_>>();
        actual_blocked.sort_unstable();
        let mut expected_blocked = scenario
            .blocked_checks
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>();
        expected_blocked.sort_unstable();
        assert_eq!(actual_blocked, expected_blocked, "{}", scenario.name);
        for blocked in json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|check| check["status"] == "blocked")
        {
            assert_eq!(blocked["cause_finding_ids"], json!([id]));
        }

        match scenario.report_action {
            Some(expected) => {
                assert_eq!(
                    json["activation_plan"]["required_steps"]
                        .as_array()
                        .map(Vec::len),
                    Some(1),
                    "{}",
                    scenario.name
                );
                let expected_id = scenario
                    .connection_action
                    .map(ActivationStepId::as_str)
                    .unwrap_or_else(|| typed_action_id_for_diagnostic(expected));
                assert_eq!(
                    json["activation_plan"]["required_steps"][0]["id"], expected_id,
                    "{}",
                    scenario.name
                );
            }
            None => assert_eq!(
                json["activation_plan"]["required_steps"],
                json!([]),
                "{}",
                scenario.name
            ),
        }
    }
}

#[test]
fn multiple_roots_are_deduplicated_without_renderer_inference() {
    let config_id = "finding.independent_config";
    let guard_id = "finding.independent_guard";
    let report = report(
        vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Failed,
                "managed_config_drift",
                "Managed configuration drifted",
                &[config_id],
                None,
            ),
            check(
                ConnectionCheckKind::GuardFiles,
                ConnectionCheckStatus::Failed,
                "guard_integrity_failed",
                "Guard integrity failed",
                &[guard_id],
                None,
            ),
        ],
        vec![
            finding(
                config_id,
                "host.codex.managed_config_drift",
                "host",
                ProjectionFacts::actual_expected("managed config drift", "changed", "canonical"),
                "action.managed_config.repair",
                "Repair the managed host configuration",
            ),
            finding(
                guard_id,
                "guard.integrity.managed_file_mismatch",
                "guard",
                ProjectionFacts::actual_expected(
                    "Guard managed file mismatch",
                    "mismatch",
                    "match",
                ),
                "action.guard.repair",
                "Repair the current Guard installation",
            ),
        ],
        Vec::new(),
    );
    let (concise, verbose, json) = projections(&report);
    assert_same_roots(&concise, &verbose, &json, &[config_id, guard_id]);
    assert_eq!(
        json["activation_plan"]["required_steps"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(concise.matches("Finding: finding.").count(), 2);
    assert_eq!(verbose.matches("  [root] finding.").count(), 2);
}

#[test]
fn pending_and_complete_reports_are_exact() {
    let pending = report(
        vec![check(
            ConnectionCheckKind::HostSession,
            ConnectionCheckStatus::Pending,
            "host_session_not_observed",
            "Managed host connection use has not been observed",
            &[],
            Some(json!({
                "evidence_role": "latest_managed_attempt",
                "current_integration_revision": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "host_executable_probe": {
                    "discovered_path": "/opt/codex",
                    "version": "0.42.0",
                },
            })),
        )],
        Vec::new(),
        vec![ActivationStep::try_new(
            ActivationStepId::RequestIntegrationVerification,
            Vec::new(),
            "Restart or reload Codex and use the connection",
        )
        .unwrap()],
    );
    let (concise, verbose, json) = projections(&pending);
    assert_eq!(json["root_cause_ids"], json!([]));
    assert_eq!(json["status"], "action_required");
    assert_eq!(json["checks"][0]["status"], "pending");
    assert!(concise.contains("Checks: 0 ready, 0 blocked, 1 waiting, 0 failed"));
    assert!(concise.contains("Restart or reload Codex and use the connection"));
    assert!(
        verbose.contains("Actual MCP peer: none observed") || !verbose.contains("Actual MCP peer:")
    );

    let complete = report(
        vec![check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Passed,
            "managed_config_ready",
            "Managed configuration is ready",
            &[],
            Some(json!({"observed_values": []})),
        )],
        Vec::new(),
        Vec::new(),
    );
    let (concise, verbose, json) = projections(&complete);
    assert_eq!(
        concise,
        "Verification completed: 1 ready.\n\nOperation: active verification\nEvidence class: active_verification\nSide effects: rollback-only Store writeability probes, disposable protocol conformance, diagnostic reconciliation, verification-report persistence\nDoes not prove: managed-host operation, future launch availability, Product Repository correctness outside checked contracts\n\nRepository: /workspace/product\nMode: workflow\nActivation: host_reload_required\nHook activation: unknown\nChecks: 1 ready, 0 blocked, 0 waiting, 0 failed\n"
    );
    assert_eq!(json["status"], "complete");
    assert_eq!(json["root_cause_ids"], json!([]));
    assert_eq!(json["checks"][0]["details"]["observed_values"], json!([]));
    assert!(verbose.contains("Status: complete"));
}
