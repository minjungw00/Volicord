use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, ConnectionAction, ConnectionActionKind,
    ConnectionCheck, ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus,
    ConnectionVerificationReport, DiagnosticAction, DiagnosticCode, DiagnosticDomain,
    DiagnosticFactSource, DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId,
    DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject, IntegrationRevision,
    UtcTimestamp,
};

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
    actions: Vec<ConnectionAction>,
) -> ConnectionCommandReport {
    let verification = ConnectionVerificationReport::try_new(timestamp(), checks, actions).unwrap();
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
                    "actual_mcp_peer_client_info": {"name": "codex", "version": "0.42.0"},
                    "path_executable_probe": {"path": "/opt/codex", "version": "0.42.0"},
                    "requested_protocol_version": "2024-11-05",
                    "selected_protocol_version": "2025-11-25",
                    "negotiated_protocol_version": null,
                    "terminal_finding_id": id,
                })),
            ),
            check(
                ConnectionCheckKind::RequiredTools,
                ConnectionCheckStatus::Blocked,
                "blocked_by_failed_prerequisite",
                "tools/list was blocked by initialize",
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
        json["actions"],
        json!([{
            "code": "action.mcp.use_supported_protocol_revision",
            "summary": "Configure the client to request a production-supported protocol revision",
            "root_cause_ids": [id],
        }])
    );
    assert!(concise.contains("Actual MCP client: codex 0.42.0"));
    assert!(concise.contains("Requested protocol: 2024-11-05"));
    assert!(concise.contains("Supported protocols: 2025-06-18, 2025-11-25"));
    assert!(concise.contains("Blocked checks: required_tools, tool_round_trip"));
    assert!(concise.contains("action.mcp.use_supported_protocol_revision"));
    assert!(!concise.contains("inspect the failure"));
    for expected in [
        "Requested protocol: 2024-11-05",
        "Selected protocol: 2025-11-25",
        "Actual MCP peer: codex",
        "PATH executable: /opt/codex",
        "Runtime sessions: runtime_session_projection",
        "Integration revision: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Blocked by: host_session",
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
        assert_eq!(json["actions"][0]["code"], action_code, "{name}");
        assert_eq!(
            json["findings"][0]["facts"]["data"]["observation_state"],
            "observed"
        );
        assert!(concise.contains(code), "{name}: {concise}");
        assert!(verbose.contains("Bounded typed facts"), "{name}: {verbose}");
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
    assert_eq!(json["actions"].as_array().unwrap().len(), 2);
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
                "actual_mcp_peer_client_info": {"name": null, "version": null},
                "path_executable_probe": {"path": "/opt/codex", "version": "0.42.0"},
                "requested_protocol_version": null,
                "selected_protocol_version": null,
                "negotiated_protocol_version": null,
            })),
        )],
        Vec::new(),
        vec![ConnectionAction::try_new(
            ConnectionActionKind::ObserveCodex,
            "Restart or reload Codex and use the connection",
        )
        .unwrap()],
    );
    let (concise, verbose, json) = projections(&pending);
    assert_eq!(json["root_cause_ids"], json!([]));
    assert_eq!(json["status"], "action_required");
    assert_eq!(json["checks"][0]["status"], "pending");
    assert!(concise.contains("Checks: 0 ready, 0 blocked, 1 waiting, 0 failed"));
    assert!(concise.contains("action.host.observe_activity"));
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
        "Verification completed: 1 ready.\n\nRepository: /workspace/product\nMode: workflow\nChecks: 1 ready, 0 blocked, 0 waiting, 0 failed\n"
    );
    assert_eq!(json["status"], "complete");
    assert_eq!(json["root_cause_ids"], json!([]));
    assert_eq!(json["checks"][0]["details"]["observed_values"], json!([]));
    assert!(verbose.contains("Status: complete"));
}
