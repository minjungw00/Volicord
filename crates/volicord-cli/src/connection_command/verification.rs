use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    str::FromStr,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use volicord_mcp::ManagedMcpInvocationPurpose;
use volicord_mcp_protocol::ProtocolRegistry;
use volicord_store::{
    agent_connections::{AgentConnectionRecord, ConnectionProjectRecord},
    diagnostic_findings::{
        diagnostic_finding, diagnostic_root_cause_ids, insert_diagnostic_finding,
    },
    guards::{guard_observation_summary, list_guard_installations},
    operational_sessions::{
        connection_integration_revision, current_managed_runtime_sessions,
        latest_managed_runtime_session, mcp_runtime_session_for_process, McpRuntimeSessionRecord,
    },
};
#[cfg(test)]
use volicord_types::ConnectionStatus;
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, ConnectionAction, ConnectionActionKind,
    ConnectionCheck, ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus,
    ConnectionVerificationReport, DiagnosticCode, DiagnosticDomain, DiagnosticFactSource,
    DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId, DiagnosticSeverity, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject, GuardManagedArtifact, IntegrationRevision, UtcTimestamp,
    LIST_PROJECTS_TOOL_NAME, MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
};

use crate::guard_integration::audit::{
    guard_file_findings_for_installation, guard_manifest_binding_valid_for_installation,
    GuardArtifactIssue, GuardAuditFacts, GuardManifestIssue,
};
use crate::host_integration::{
    codex::{self, CodexAdapter},
    verification::{HostExecutableStatus, ManagedConfigStatus, ProjectTrustStatus, Verification},
    HostAdapter, HostKind, HostPlan, HostScope,
};
use crate::operational_diagnostics::{
    connection_operational_finding_id, guard_artifact_kind, persist_connection_operational_finding,
    GuardDiagnostic, OperationalDiagnostic, OperationalDiagnosticFacts, RevisionDiagnostic,
    TrustDiagnostic,
};

use super::{
    codex_environment,
    mcp_process::{
        materialize_connection_invocation, run_connection_preflight, McpPersistedDiagnostic,
        McpProcessDiagnosticContext, McpProcessFailure, McpVerification,
    },
    parse_host_kind, ConnectionCommandError, ConnectionProcess,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum StepStatus {
    Passed,
    Failed,
    Pending,
}

impl StepStatus {
    pub(in crate::connection_command) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Pending => "pending",
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::connection_command) struct VerificationStep {
    pub(in crate::connection_command) status: StepStatus,
    pub(in crate::connection_command) code: String,
    pub(in crate::connection_command) details: String,
    pub(in crate::connection_command) preflight_diagnostics: Option<McpPreflightDiagnostics>,
    pub(in crate::connection_command) process_id: Option<u32>,
    pub(in crate::connection_command) failure: Option<McpProcessFailure>,
    pub(in crate::connection_command) diagnostic: Option<McpPersistedDiagnostic>,
}

impl VerificationStep {
    pub(in crate::connection_command) fn passed_with_code(
        code: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            status: StepStatus::Passed,
            code: code.into(),
            details: details.into(),
            preflight_diagnostics: None,
            process_id: None,
            failure: None,
            diagnostic: None,
        }
    }

    pub(in crate::connection_command) fn failed_with_code(
        code: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            status: StepStatus::Failed,
            code: code.into(),
            details: details.into(),
            preflight_diagnostics: None,
            process_id: None,
            failure: None,
            diagnostic: None,
        }
    }

    pub(in crate::connection_command) fn pending(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Pending,
            code: "pending".to_owned(),
            details: details.into(),
            preflight_diagnostics: None,
            process_id: None,
            failure: None,
            diagnostic: None,
        }
    }

    pub(in crate::connection_command) fn with_preflight_diagnostics(
        mut self,
        diagnostics: Option<McpPreflightDiagnostics>,
    ) -> Self {
        self.preflight_diagnostics = diagnostics;
        self
    }

    pub(in crate::connection_command) fn with_process_failure(
        mut self,
        process_id: Option<u32>,
        failure: McpProcessFailure,
    ) -> Self {
        self.process_id = process_id;
        self.failure = Some(failure);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::connection_command) struct McpPreflightDiagnostics {
    pub(in crate::connection_command) storage_read: String,
    pub(in crate::connection_command) storage_write: String,
    pub(in crate::connection_command) effective_tool_mode: String,
}

impl McpPreflightDiagnostics {
    pub(in crate::connection_command) fn from_preflight_report(
        report: &BTreeMap<String, String>,
    ) -> Option<Self> {
        Some(Self {
            storage_read: report.get("project_state_read")?.to_owned(),
            storage_write: report.get("project_state_write")?.to_owned(),
            effective_tool_mode: report.get("effective_tool_mode")?.to_owned(),
        })
    }

    pub(in crate::connection_command) fn to_json(&self) -> Value {
        json!({
            "storage_read": &self.storage_read,
            "storage_write": &self.storage_write,
            "effective_tool_mode": &self.effective_tool_mode,
        })
    }
}

#[derive(Debug, Clone)]
pub(in crate::connection_command) struct VerificationReport {
    pub(in crate::connection_command) report: ConnectionVerificationReport,
}

pub(in crate::connection_command) fn verify_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    repo_root: &Path,
    project_id: Option<&str>,
    process: &mut impl ConnectionProcess,
) -> Result<VerificationReport, ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let host = verify_host_plan(host_kind, host_plan, process)?;
    let preflight_launch = materialize_connection_invocation(
        &host_plan.entry,
        runtime_home,
        repo_root,
        ManagedMcpInvocationPurpose::cli_preflight_check(
            &connection.connection_internal_id,
            project_id,
        )
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let mut preflight = run_connection_preflight(
        process,
        &preflight_launch,
        &connection.connection_internal_id,
        &connection.mode,
    );
    let mut handshake = if preflight.status == StepStatus::Passed {
        let handshake_launch = materialize_connection_invocation(
            &host_plan.entry,
            runtime_home,
            repo_root,
            ManagedMcpInvocationPurpose::CliStdioHandshake,
        )
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        McpVerification::from_exchange(
            process.verify_mcp_stdio(&handshake_launch, &connection.mode),
        )
    } else {
        McpVerification::not_run()
    };
    persist_process_diagnostics(runtime_home, connection, &mut preflight, &mut handshake)?;
    let report =
        canonical_verification_report(runtime_home, connection, &host, &preflight, &handshake)?;
    Ok(VerificationReport { report })
}

fn persist_process_diagnostics(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    preflight: &mut VerificationStep,
    handshake: &mut McpVerification,
) -> Result<(), ConnectionCommandError> {
    if let Some(failure) = preflight.failure.as_ref() {
        preflight.diagnostic = Some(persist_process_finding(
            runtime_home,
            connection,
            preflight.process_id,
            failure,
        )?);
    }
    let Some(exchange) = handshake.exchange.as_mut() else {
        return Ok(());
    };
    if !exchange.conformance.is_empty() || !exchange.host_compatibility.is_empty() {
        for probe in &mut exchange.conformance {
            if let Some(failure) = probe.failure.as_ref() {
                probe.diagnostic = Some(persist_process_finding(
                    runtime_home,
                    connection,
                    probe.progress.process_id,
                    failure,
                )?);
            }
        }
        for probe in &mut exchange.host_compatibility {
            if let Some(failure) = probe.failure.as_ref() {
                probe.diagnostic = Some(persist_process_finding(
                    runtime_home,
                    connection,
                    probe.progress.process_id,
                    failure,
                )?);
            }
        }
        exchange.diagnostic = exchange
            .conformance
            .iter()
            .find_map(|probe| probe.diagnostic.clone())
            .or_else(|| {
                exchange
                    .host_compatibility
                    .iter()
                    .find_map(|probe| probe.diagnostic.clone())
            });
    } else if let Some(failure) = exchange.failure.as_ref() {
        exchange.diagnostic = Some(persist_process_finding(
            runtime_home,
            connection,
            exchange.progress.process_id,
            failure,
        )?);
    }
    Ok(())
}

fn persist_process_finding(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    process_id: Option<u32>,
    failure: &McpProcessFailure,
) -> Result<McpPersistedDiagnostic, ConnectionCommandError> {
    let runtime = process_id
        .map(|process_id| {
            mcp_runtime_session_for_process(
                runtime_home,
                &connection.connection_internal_id,
                process_id,
            )
        })
        .transpose()?
        .flatten();
    let observed_at = current_timestamp();
    let finding_id = runtime.as_ref().map_or_else(
        || {
            format!(
                "finding.process.{}",
                observed_at.to_canonical_string().to_ascii_lowercase()
            )
        },
        |runtime| format!("finding.{}.supervisor", runtime.runtime_session_id),
    );
    let revision = connection_integration_revision(connection)?;
    let finding = failure
        .to_diagnostic_finding(McpProcessDiagnosticContext {
            finding_id,
            observed_at,
            connection_id: connection.connection_internal_id.clone(),
            integration_revision: revision,
            runtime_session_id: runtime
                .as_ref()
                .map(|runtime| runtime.runtime_session_id.clone()),
            requested_revision: runtime
                .as_ref()
                .and_then(|runtime| runtime.requested_protocol_version.clone()),
            selected_revision: runtime
                .as_ref()
                .and_then(|runtime| runtime.selected_protocol_version.clone()),
            negotiated_revision: runtime
                .as_ref()
                .and_then(|runtime| runtime.negotiated_protocol_version.clone()),
            production_supported_revisions: ProtocolRegistry::production()
                .oldest_to_newest()
                .map(|profile| profile.revision().as_str().to_owned())
                .collect(),
            attempted_client_name: runtime
                .as_ref()
                .and_then(|runtime| runtime.attempted_client_name.clone()),
            attempted_client_version: runtime
                .as_ref()
                .and_then(|runtime| runtime.attempted_client_version.clone()),
        })
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    insert_diagnostic_finding(runtime_home, &finding)?;
    Ok(McpPersistedDiagnostic {
        finding_id: finding.id().to_string(),
        code: finding.code().to_string(),
    })
}

pub(in crate::connection_command) fn effective_connection_report(
    connection: &AgentConnectionRecord,
) -> Result<ConnectionVerificationReport, ConnectionCommandError> {
    connection
        .effective_verification_report(current_timestamp())
        .map_err(ConnectionCommandError::from)
}

pub(in crate::connection_command) fn connection_metadata_failure_report(
    current: &ConnectionVerificationReport,
) -> Result<ConnectionVerificationReport, ConnectionCommandError> {
    let mut checks = current
        .checks()
        .iter()
        .filter(|check| check.id() != ConnectionCheckKind::ManagedConfig)
        .cloned()
        .collect::<Vec<_>>();
    checks.push(canonical_check(
        ConnectionCheckKind::ManagedConfig,
        ConnectionCheckStatus::Failed,
        "connection_metadata_invalid",
        "Agent Connection metadata is invalid, so managed Codex configuration cannot be inspected",
        None,
        None,
    )?);
    let actions = actions_for_checks(&checks)?;
    ConnectionVerificationReport::try_new(current.checked_at().clone(), checks, actions)
        .map_err(ConnectionCommandError::from)
}

fn canonical_verification_report(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host: &Verification,
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<ConnectionVerificationReport, ConnectionCommandError> {
    let current_revision = connection_integration_revision(connection)?;
    let current_sessions =
        current_managed_runtime_sessions(runtime_home, &connection.connection_internal_id)?;
    persist_peer_path_mismatch_findings(runtime_home, connection, host, &current_sessions)?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let host_findings = persist_host_boundary_findings(
        runtime_home,
        connection,
        host,
        &current_sessions,
        latest_session.as_ref(),
        &current_revision,
    )?;
    let mut checks = vec![
        with_direct_causes(managed_config_check(host)?, host_findings.managed_config)?,
        host_executable_check(host)?,
        with_direct_causes(
            mcp_server_check(preflight, handshake)?,
            mcp_server_finding_ids(preflight, handshake)?,
        )?,
        with_direct_causes(project_trust_check(host)?, host_findings.project_trust)?,
    ];
    checks.extend(host_session_checks(
        host,
        current_revision.as_str(),
        &current_sessions,
        latest_session.as_ref(),
    )?);
    let projects = volicord_store::agent_connections::list_connection_projects_for_diagnostics(
        runtime_home,
        &connection.connection_internal_id,
    )?;
    checks.extend(guard_checks_for_connection(
        runtime_home,
        connection,
        &projects,
        true,
    )?);
    checks = finalize_check_graph(runtime_home, checks)?;
    let actions = actions_for_checks(&checks)?;
    ConnectionVerificationReport::try_new(current_timestamp(), checks, actions)
        .map_err(ConnectionCommandError::from)
}

#[derive(Default)]
struct HostBoundaryFindings {
    managed_config: Vec<DiagnosticFindingId>,
    project_trust: Vec<DiagnosticFindingId>,
}

fn persist_host_boundary_findings(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host: &Verification,
    current_sessions: &[McpRuntimeSessionRecord],
    latest_session: Option<&McpRuntimeSessionRecord>,
    current_revision: &IntegrationRevision,
) -> Result<HostBoundaryFindings, ConnectionCommandError> {
    let mut findings = HostBoundaryFindings::default();
    if let Some(diagnostic) = host.managed_config_diagnostic {
        findings
            .managed_config
            .push(persist_connection_operational_finding(
                runtime_home,
                connection,
                OperationalDiagnostic::ManagedConfig(diagnostic),
                &OperationalDiagnosticFacts {
                    observed_state: Some(host.managed_config.as_str()),
                    ..OperationalDiagnosticFacts::default()
                },
                current_timestamp(),
            )?);
    }
    if let Some(diagnostic) = host
        .project_trust
        .as_ref()
        .and_then(|trust| TrustDiagnostic::from_status(trust.status))
    {
        findings
            .project_trust
            .push(persist_connection_operational_finding(
                runtime_home,
                connection,
                OperationalDiagnostic::Trust(diagnostic),
                &OperationalDiagnosticFacts {
                    observed_state: host
                        .project_trust
                        .as_ref()
                        .map(|trust| trust.status.as_str()),
                    ..OperationalDiagnosticFacts::default()
                },
                current_timestamp(),
            )?);
    }
    if current_sessions.is_empty() {
        if let Some(latest) = latest_session
            .filter(|latest| latest.connection_integration_revision != current_revision.as_str())
        {
            persist_connection_operational_finding(
                runtime_home,
                connection,
                OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale),
                &OperationalDiagnosticFacts {
                    expected_revision: Some(current_revision.as_str().to_owned()),
                    observed_revision: Some(latest.connection_integration_revision.clone()),
                    ..OperationalDiagnosticFacts::default()
                },
                current_timestamp(),
            )?;
        }
    }
    Ok(findings)
}

fn managed_config_check(host: &Verification) -> Result<ConnectionCheck, ConnectionCommandError> {
    let (status, summary) = match host.managed_config {
        ManagedConfigStatus::Match => (
            ConnectionCheckStatus::Passed,
            "Managed Codex configuration matches the canonical entry",
        ),
        ManagedConfigStatus::Missing => (
            ConnectionCheckStatus::Failed,
            "Required managed Codex configuration is missing",
        ),
        ManagedConfigStatus::Unmanaged => (
            ConnectionCheckStatus::Failed,
            "The managed Codex server name has an ownership conflict",
        ),
        ManagedConfigStatus::Changed => (
            ConnectionCheckStatus::Failed,
            "Managed Codex configuration differs from the canonical entry",
        ),
        ManagedConfigStatus::Malformed => (
            ConnectionCheckStatus::Failed,
            "Managed Codex configuration is malformed",
        ),
        ManagedConfigStatus::Unavailable | ManagedConfigStatus::Unknown => (
            ConnectionCheckStatus::Failed,
            "Managed Codex configuration could not be inspected",
        ),
    };
    let code = match host.managed_config {
        ManagedConfigStatus::Match => "managed_config_matches",
        ManagedConfigStatus::Missing => "managed_config_missing",
        ManagedConfigStatus::Unmanaged => "managed_config_ownership_conflict",
        ManagedConfigStatus::Changed => "managed_config_mismatch",
        ManagedConfigStatus::Malformed => "managed_config_malformed",
        ManagedConfigStatus::Unavailable | ManagedConfigStatus::Unknown => {
            "managed_config_unavailable"
        }
    };
    canonical_check(
        ConnectionCheckKind::ManagedConfig,
        status,
        code,
        summary,
        Some(json!({
            "target": host.config_target,
            "diagnostic_code": host.managed_config_diagnostic.map(|diagnostic| diagnostic.code()).unwrap_or(code),
            "observed_state": host.managed_config.as_str(),
            "diagnostic": host.managed_config_details,
        })),
        None,
    )
}

fn host_executable_check(host: &Verification) -> Result<ConnectionCheck, ConnectionCommandError> {
    let (status, summary) = match host.host_executable {
        HostExecutableStatus::Available => (
            ConnectionCheckStatus::Passed,
            "Codex executable discovery and version probe succeeded",
        ),
        HostExecutableStatus::Unavailable => (
            ConnectionCheckStatus::Failed,
            "Codex executable discovery or version probe failed",
        ),
        HostExecutableStatus::NotChecked => (
            ConnectionCheckStatus::Pending,
            "Codex executable has not been probed",
        ),
    };
    canonical_check(
        ConnectionCheckKind::HostExecutable,
        status,
        &host.host_executable_code,
        summary,
        Some(json!({
            "path": host.executable_path,
            "version": host.host_version,
            "diagnostic": host.host_executable_details,
        })),
        None,
    )
}

pub(in crate::connection_command) fn mcp_server_check(
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let step = &handshake.step;
    let (status, code, summary) = if preflight.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            preflight.code.as_str(),
            "Volicord CLI MCP preflight failed",
        )
    } else if step.status == StepStatus::Passed {
        (
            ConnectionCheckStatus::Passed,
            step.code.as_str(),
            "Volicord MCP server self-test passed",
        )
    } else if step.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            step.code.as_str(),
            "Volicord MCP server self-test failed",
        )
    } else {
        (
            ConnectionCheckStatus::Failed,
            "mcp_server_self_test_not_run",
            "Volicord MCP server self-test did not run",
        )
    };
    let progress = handshake
        .exchange
        .as_ref()
        .map(|exchange| &exchange.progress);
    let exchange = handshake.exchange.as_ref();
    let mut self_test = json!({
        "status": step.status.as_str(),
        "code": step.code,
        "diagnostic": step.details,
        "safe_read_only_tool": LIST_PROJECTS_TOOL_NAME,
    });
    if exchange.is_some_and(|exchange| {
        !exchange.conformance.is_empty() || !exchange.host_compatibility.is_empty()
    }) {
        let exchange = exchange.expect("matrix exchange was checked");
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                (
                    "production_supported_revisions".to_owned(),
                    json!(exchange
                        .conformance
                        .iter()
                        .map(|probe| probe.revision.as_str())
                        .collect::<Vec<_>>()),
                ),
                (
                    "conformance".to_owned(),
                    Value::Array(
                        exchange
                            .conformance
                            .iter()
                            .map(|probe| {
                                probe_result_json(
                                    &probe.progress,
                                    probe.failure.as_ref(),
                                    probe.diagnostic.as_ref(),
                                    [("revision", json!(probe.revision))],
                                )
                            })
                            .collect(),
                    ),
                ),
                (
                    "host_compatibility_profiles".to_owned(),
                    json!(exchange
                        .host_compatibility
                        .iter()
                        .map(|probe| probe.profile.as_str())
                        .collect::<Vec<_>>()),
                ),
                (
                    "host_compatibility".to_owned(),
                    Value::Array(
                        exchange
                            .host_compatibility
                            .iter()
                            .map(|probe| {
                                probe_result_json(
                                    &probe.progress,
                                    probe.failure.as_ref(),
                                    probe.diagnostic.as_ref(),
                                    [
                                        ("profile", json!(probe.profile.as_str())),
                                        ("fixture", json!(probe.fixture_id)),
                                    ],
                                )
                            })
                            .collect(),
                    ),
                ),
            ]);
        if let Some(tools) = exchange
            .conformance
            .iter()
            .find_map(|probe| probe.progress.tools_list.as_ref())
        {
            self_test
                .as_object_mut()
                .expect("self-test details are an object")
                .insert("tools_list".to_owned(), json!(tools));
        }
    } else {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                (
                    "initialize".to_owned(),
                    json!(progress.is_some_and(|progress| progress.initialize_completed)),
                ),
                (
                    "tools_list_observed".to_owned(),
                    json!(progress.is_some_and(|progress| progress.tools_list.is_some())),
                ),
                (
                    "required_tools_validated".to_owned(),
                    json!(progress.is_some_and(|progress| progress.required_tools_validated)),
                ),
                (
                    "safe_read_only_tool_completed".to_owned(),
                    json!(progress.is_some_and(|progress| progress.safe_tool_call_completed)),
                ),
                (
                    "shutdown_completed".to_owned(),
                    json!(progress.is_some_and(|progress| progress.shutdown_completed)),
                ),
            ]);
    }
    if let Some(tools) = progress.and_then(|progress| progress.tools_list.as_ref()) {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .insert("tools_list".to_owned(), json!(tools));
    }
    if let Some(failure) = handshake
        .exchange
        .as_ref()
        .and_then(|exchange| exchange.failure.as_ref())
    {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                (
                    "diagnostic_code".to_owned(),
                    json!(failure.diagnostic_code()),
                ),
                ("failure_stage".to_owned(), json!(failure.stage().as_str())),
            ]);
    }
    if let Some(diagnostic) = handshake
        .exchange
        .as_ref()
        .and_then(|exchange| exchange.diagnostic.as_ref())
    {
        self_test
            .as_object_mut()
            .expect("self-test details are an object")
            .extend([
                ("finding_id".to_owned(), json!(diagnostic.finding_id)),
                ("diagnostic_code".to_owned(), json!(diagnostic.code)),
            ]);
    }
    canonical_check(
        ConnectionCheckKind::McpServer,
        status,
        code,
        summary,
        Some(json!({
            "preflight": {
                "status": preflight.status.as_str(),
                "code": preflight.code,
                "diagnostic": preflight.details,
                "storage": preflight.preflight_diagnostics.as_ref().map(McpPreflightDiagnostics::to_json),
                "finding_id": preflight.diagnostic.as_ref().map(|diagnostic| diagnostic.finding_id.as_str()),
                "diagnostic_code": preflight.diagnostic.as_ref().map(|diagnostic| diagnostic.code.as_str()),
                "failure_stage": preflight.failure.as_ref().map(|failure| failure.stage().as_str()),
            },
            "self_test": self_test,
        })),
        None,
    )
}

fn probe_result_json<const N: usize>(
    progress: &crate::connection_command::McpExchangeProgress,
    failure: Option<&McpProcessFailure>,
    diagnostic: Option<&McpPersistedDiagnostic>,
    identity: [(&str, Value); N],
) -> Value {
    let mut result = json!({
        "status": if failure.is_none() { "passed" } else { "failed" },
        "requested_revision": progress.requested_revision,
        "negotiated_revision": progress.negotiated_revision,
        "initialize": progress.initialize_completed,
        "initialized_notification": progress.initialized_notification_completed,
        "pinned_schema_validated": progress.pinned_schema_validated,
        "tools_list_observed": progress.tools_list.is_some(),
        "tools_returned": progress.tools_list.as_ref().map(Vec::len),
        "required_tools_validated": progress.required_tools_validated,
        "safe_read_only_tool": LIST_PROJECTS_TOOL_NAME,
        "safe_read_only_tool_completed": progress.safe_tool_call_completed,
        "shutdown_completed": progress.shutdown_completed,
    });
    let object = result.as_object_mut().expect("probe result is an object");
    for (field, value) in identity {
        object.insert(field.to_owned(), value);
    }
    if let Some(failure) = failure {
        object.insert(
            "diagnostic_code".to_owned(),
            json!(failure.diagnostic_code()),
        );
        object.insert("failure_stage".to_owned(), json!(failure.stage().as_str()));
    }
    if let Some(diagnostic) = diagnostic {
        object.insert("finding_id".to_owned(), json!(diagnostic.finding_id));
        object.insert("diagnostic_code".to_owned(), json!(diagnostic.code));
    }
    result
}

fn project_trust_check(host: &Verification) -> Result<ConnectionCheck, ConnectionCommandError> {
    let Some(trust) = host.project_trust.as_ref() else {
        return canonical_check(
            ConnectionCheckKind::ProjectTrust,
            ConnectionCheckStatus::NotApplicable,
            "project_trust_not_applicable",
            "No separate project trust action applies to this connection scope",
            Some(json!({"applicable": false})),
            None,
        );
    };
    let (status, summary) = match trust.status {
        ProjectTrustStatus::Trusted => (
            ConnectionCheckStatus::Passed,
            "Codex project trust is satisfied",
        ),
        ProjectTrustStatus::Malformed => (
            ConnectionCheckStatus::Failed,
            "Codex project trust configuration is malformed or contradictory",
        ),
        ProjectTrustStatus::Untrusted
        | ProjectTrustStatus::Missing
        | ProjectTrustStatus::Unknown
        | ProjectTrustStatus::Unreadable => (
            ConnectionCheckStatus::Pending,
            "Codex project trust or reload action is required",
        ),
    };
    canonical_check(
        ConnectionCheckKind::ProjectTrust,
        status,
        &trust.code,
        summary,
        Some(json!({
            "config_path": trust.config_path,
            "repo_root": trust.repo_root,
            "observed_state": trust.status.as_str(),
            "diagnostic": trust.details,
        })),
        None,
    )
}

fn observed_host_version(session: &McpRuntimeSessionRecord) -> Option<&str> {
    session.observed_host_executable_version.as_deref()
}

#[derive(Serialize)]
struct ActualMcpPeerClientInfo<'a> {
    name: Option<&'a str>,
    version: &'a str,
}

#[derive(Serialize)]
struct PathExecutableProbe<'a> {
    path: Option<&'a str>,
    version: &'a str,
}

#[derive(Serialize)]
struct PeerPathMismatchFacts<'a> {
    summary: &'static str,
    runtime_session_id: &'a str,
    actual_mcp_peer_client_info: ActualMcpPeerClientInfo<'a>,
    path_executable_probe: PathExecutableProbe<'a>,
}

impl DiagnosticFactSource for PeerPathMismatchFacts<'_> {}

fn persist_peer_path_mismatch_findings(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host: &Verification,
    sessions: &[McpRuntimeSessionRecord],
) -> Result<(), ConnectionCommandError> {
    let (Some(path_version), path) = (
        host.host_version.as_deref(),
        host.executable_path.as_deref(),
    ) else {
        return Ok(());
    };
    for session in sessions.iter().filter(|session| {
        session.session_source == volicord_types::McpRuntimeSessionSource::ManagedHost
    }) {
        let Some(peer_version) = session.attempted_client_version.as_deref() else {
            continue;
        };
        if peer_version == path_version {
            continue;
        }
        let finding_id = DiagnosticFindingId::parse(format!(
            "finding.{}.peer_path_version_mismatch",
            session.runtime_session_id
        ))
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        if diagnostic_finding(runtime_home, &finding_id)?.is_some() {
            continue;
        }
        let facts = DiagnosticFacts::project(&PeerPathMismatchFacts {
            summary: "the actual MCP peer client version differed from the PATH executable probe",
            runtime_session_id: &session.runtime_session_id,
            actual_mcp_peer_client_info: ActualMcpPeerClientInfo {
                name: session.attempted_client_name.as_deref(),
                version: peer_version,
            },
            path_executable_probe: PathExecutableProbe {
                path,
                version: path_version,
            },
        })
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        let finding = DiagnosticFinding::try_new(
            finding_id,
            DiagnosticCode::parse("host.codex.peer_version_differs_from_path_probe")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticDomain::parse("host")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticStage::parse("host_observation")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticSeverity::Warning,
            DiagnosticSource::parse("cli_host_verification")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticSubject::try_new("runtime_session", &session.runtime_session_id)
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            facts,
            current_timestamp(),
        )
        .and_then(|finding| {
            finding
                .with_connection_id(AgentConnectionId::new(
                    connection.connection_internal_id.clone(),
                ))?
                .with_runtime_session_id(AgentRuntimeSessionId::new(
                    session.runtime_session_id.clone(),
                ))
        })
        .map(|finding| {
            finding.with_integration_revision(
                IntegrationRevision::parse(session.connection_integration_revision.clone())
                    .expect("persisted runtime session has a validated integration revision"),
            )
        })
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        insert_diagnostic_finding(runtime_home, &finding)?;
    }
    Ok(())
}

fn host_session_checks(
    host: &Verification,
    current_revision: &str,
    current: &[McpRuntimeSessionRecord],
    latest: Option<&McpRuntimeSessionRecord>,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let current = current
        .iter()
        .filter(|session| {
            session.session_source == volicord_types::McpRuntimeSessionSource::ManagedHost
                && session.connection_integration_revision == current_revision
        })
        .collect::<Vec<_>>();
    let latest = latest.filter(|session| {
        session.session_source == volicord_types::McpRuntimeSessionSource::ManagedHost
    });
    let version_fresh = |session: &McpRuntimeSessionRecord| {
        host.host_version.as_deref().is_none()
            || observed_host_version(session).is_none()
            || host.host_version.as_deref() == observed_host_version(session)
    };
    let details = |observed: Option<&McpRuntimeSessionRecord>| {
        json!({
            "current_integration_revision": current_revision,
            "observed_integration_revision": observed.map(|session| session.connection_integration_revision.as_str()),
            "path_executable_probe": {
                "path": host.executable_path,
                "version": host.host_version,
            },
            "observed_host_executable_version": observed.and_then(observed_host_version),
            "runtime_session_id": observed.map(|session| session.runtime_session_id.as_str()),
            "actual_mcp_peer_client_info": {
                "name": observed.and_then(|session| session.attempted_client_name.as_deref()),
                "version": observed.and_then(|session| session.attempted_client_version.as_deref()),
            },
            "requested_protocol_version": observed.and_then(|session| session.requested_protocol_version.as_deref()),
            "selected_protocol_version": observed.and_then(|session| session.selected_protocol_version.as_deref()),
            "negotiated_protocol_version": observed.and_then(|session| session.negotiated_protocol_version.as_deref()),
            "last_observed_at": observed.map(|session| session.last_observed_at.as_str()),
            "terminal_finding_id": observed.and_then(|session| session.terminal_finding_id.as_deref()),
        })
    };
    let diagnostic = current.first().copied();
    let started = current
        .iter()
        .copied()
        .find(|session| version_fresh(session));
    let initialized = current.iter().copied().find(|session| {
        version_fresh(session)
            && session.initialize_completed_at.is_some()
            && session.initialized_notification_at.is_some()
    });
    let tools_present = current
        .iter()
        .copied()
        .find(|session| version_fresh(session) && session.required_tools_present == Some(true));
    let round_trip = current.iter().copied().find(|session| {
        version_fresh(session) && session.designated_safe_tool_observed_at.is_some()
    });

    let (session_status, session_code, session_summary, session_observed_at, session_detail) =
        match (initialized, diagnostic) {
            (Some(session), _) => (
                ConnectionCheckStatus::Passed,
                "host_session_initialized",
                "A current managed-host session completed MCP initialize",
                session.initialized_notification_at.as_deref(),
                Some(session),
            ),
            (None, None) if latest.is_some() => (
                ConnectionCheckStatus::Pending,
                "host_session_revision_stale",
                "Managed host has not loaded the current connection revision",
                latest.map(|session| session.last_observed_at.as_str()),
                latest,
            ),
            (None, None) => (
                ConnectionCheckStatus::Pending,
                "host_session_not_observed",
                "Managed host connection use has not been observed",
                None,
                None,
            ),
            (None, Some(session)) if !version_fresh(session) => (
                ConnectionCheckStatus::Pending,
                "host_version_observation_stale",
                "Codex version changed after the newest managed-host observation",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
            (None, Some(session)) if session.terminal_finding_id.is_some() => (
                ConnectionCheckStatus::Failed,
                "host_session_initialize_failed",
                "Newest current managed-host session failed before MCP initialize completed",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
            (None, Some(session)) => (
                ConnectionCheckStatus::Pending,
                "host_session_initialize_pending",
                "Newest current managed-host session has not completed MCP initialize",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
        };
    let process_startup = canonical_check(
        ConnectionCheckKind::ProcessStartup,
        if started.is_some() {
            ConnectionCheckStatus::Passed
        } else {
            ConnectionCheckStatus::Pending
        },
        if started.is_some() {
            "process_startup_observed"
        } else {
            "process_startup_not_observed"
        },
        if started.is_some() {
            "A current managed host started the configured Volicord MCP process"
        } else {
            "Managed host process startup has not been observed"
        },
        Some(details(started.or(latest))),
        started.map(|session| session.process_started_at.as_str()),
    )?;

    let host_session = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::HostSession,
            session_status,
            session_code,
            session_summary,
            Some(details(session_detail)),
            session_observed_at,
        )?,
        terminal_cause_ids(session_detail)?,
    )?;

    let (tools_status, tools_code, tools_summary, tools_observed_at, tools_detail) =
        match (tools_present, diagnostic) {
            (Some(session), _) => (
                ConnectionCheckStatus::Passed,
                "required_tools_present",
                "A current managed host exposed every required tool",
                session.tools_list_observed_at.as_deref(),
                Some(session),
            ),
            (None, None) => (
                ConnectionCheckStatus::Pending,
                "required_tools_not_observed",
                "Current managed host has not reported tools/list",
                None,
                latest,
            ),
            (None, Some(session)) if !version_fresh(session) => (
                ConnectionCheckStatus::Pending,
                "required_tools_observation_stale",
                "Newest required-tool observation predates the current Codex version",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
            (None, Some(session)) if session.required_tools_present == Some(false) => (
                ConnectionCheckStatus::Failed,
                "required_tools_missing",
                "Newest current managed host is missing one or more required tools",
                session.tools_list_observed_at.as_deref(),
                Some(session),
            ),
            (None, Some(session))
                if session.initialize_completed_at.is_some()
                    && session.terminal_finding_id.is_some() =>
            {
                (
                    ConnectionCheckStatus::Failed,
                    "required_tools_invalid",
                    "Newest current managed-host tool discovery ended in a protocol failure",
                    Some(session.last_observed_at.as_str()),
                    Some(session),
                )
            }
            (None, Some(session)) => (
                ConnectionCheckStatus::Pending,
                "required_tools_not_observed",
                "Newest current managed host has not reported tools/list",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
        };
    let required_tools = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::RequiredTools,
            tools_status,
            tools_code,
            tools_summary,
            Some(details(tools_detail)),
            tools_observed_at,
        )?,
        terminal_cause_ids(tools_detail)?,
    )?;

    let (round_trip_status, round_trip_code, round_trip_summary, round_trip_observed_at, round_detail) =
        match (round_trip, diagnostic) {
            (Some(session), _) => (
                ConnectionCheckStatus::Passed,
                "tool_round_trip_passed",
                "A current managed host completed the designated read-only Volicord tool call",
                session.designated_safe_tool_observed_at.as_deref(),
                Some(session),
            ),
            (None, None) => (
                ConnectionCheckStatus::Pending,
                "tool_round_trip_not_observed",
                "Current managed host has not completed the designated read-only Volicord tool call",
                None,
                latest,
            ),
            (None, Some(session)) if !version_fresh(session) => (
                ConnectionCheckStatus::Pending,
                "tool_round_trip_observation_stale",
                "Newest designated read-only tool-call observation predates the current Codex version",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
            (None, Some(session))
                if session.required_tools_present == Some(true)
                    && session.terminal_finding_id.is_some() =>
            {
                (
                    ConnectionCheckStatus::Failed,
                    "tool_round_trip_failed",
                    "Newest current managed-host session reported a protocol or contract failure",
                    Some(session.last_observed_at.as_str()),
                    Some(session),
                )
            }
            (None, Some(session)) => (
                ConnectionCheckStatus::Pending,
                "tool_round_trip_not_observed",
                "Newest current managed host has not completed the designated read-only Volicord tool call",
                Some(session.last_observed_at.as_str()),
                Some(session),
            ),
        };
    let tool_round_trip = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::ToolRoundTrip,
            round_trip_status,
            round_trip_code,
            round_trip_summary,
            Some(details(round_detail)),
            round_trip_observed_at,
        )?,
        terminal_cause_ids(round_detail)?,
    )?;
    block_failed_dependencies(vec![
        process_startup,
        host_session,
        required_tools,
        tool_round_trip,
    ])
}

fn terminal_cause_ids(
    session: Option<&McpRuntimeSessionRecord>,
) -> Result<Vec<DiagnosticFindingId>, ConnectionCommandError> {
    session
        .and_then(|session| session.terminal_finding_id.as_deref())
        .map(|finding_id| {
            DiagnosticFindingId::parse(finding_id.to_owned())
                .map(|finding_id| vec![finding_id])
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn guard_checks_for_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    persist_findings: bool,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let mut installations = Vec::new();
    for project in projects {
        installations.extend(list_guard_installations(
            runtime_home,
            &connection.connection_internal_id,
            Some(&project.project_id),
        )?);
    }
    if installations.is_empty() {
        installations =
            list_guard_installations(runtime_home, &connection.connection_internal_id, None)?;
    }

    let mut audit = GuardAuditFacts::default();
    let mut all_required_phases_observed = !installations.is_empty();
    let mut prompt_capture_observed = !installations.is_empty();
    let mut required_phases = Vec::new();
    let mut observed_phases = Vec::new();
    let mut incompatible_event_ids = Vec::new();
    let mut last_current_observation_at = None;
    let mut installation_ids = Vec::new();
    let mut observation_revision_mismatch = false;

    for installation in &installations {
        installation_ids.push(installation.guard_installation_id.clone());
        audit.merge(guard_file_findings_for_installation(
            runtime_home,
            installation,
            connection,
            projects,
        ));
        let binding_is_current =
            guard_manifest_binding_valid_for_installation(installation, connection, projects);
        observation_revision_mismatch |= !binding_is_current;
        let observation =
            guard_observation_summary(runtime_home, &installation.project_id, installation)?;
        required_phases.extend(observation.required_phases.iter().cloned());
        observed_phases.extend(observation.observed_phases.iter().cloned());
        incompatible_event_ids.extend(observation.incompatible_event_ids.iter().cloned());
        let observation_is_current =
            binding_is_current && observation.all_required_phases_observed();
        all_required_phases_observed &= observation_is_current;
        prompt_capture_observed &= observation_is_current && observation.prompt_capture_observed();
        last_current_observation_at = latest_timestamp(
            last_current_observation_at,
            observation.last_observed_at.as_deref(),
        )?;
    }

    audit.sort_dedup();
    installation_ids.sort();
    installation_ids.dedup();
    required_phases.sort();
    required_phases.dedup();
    observed_phases.sort();
    observed_phases.dedup();
    incompatible_event_ids.sort();
    incompatible_event_ids.dedup();

    let missing_required_phases = required_phases
        .iter()
        .filter(|phase| !observed_phases.contains(phase))
        .cloned()
        .collect::<Vec<_>>();
    let configured_phase_gaps = audit
        .missing_required_phases
        .iter()
        .map(|phase| phase.as_str().to_owned())
        .collect::<Vec<_>>();
    let files_status = if !installations.is_empty()
        && audit.generated_config_verified()
        && configured_phase_gaps.is_empty()
    {
        ConnectionCheckStatus::Passed
    } else {
        ConnectionCheckStatus::Failed
    };
    let observation_status = if !incompatible_event_ids.is_empty() {
        ConnectionCheckStatus::Failed
    } else if all_required_phases_observed {
        ConnectionCheckStatus::Passed
    } else {
        ConnectionCheckStatus::Pending
    };
    let hook_execution_status = if observed_phases.is_empty() && incompatible_event_ids.is_empty() {
        ConnectionCheckStatus::Pending
    } else {
        ConnectionCheckStatus::Passed
    };

    let artifact_issues = audit
        .findings
        .iter()
        .map(|finding| {
            json!({
                "artifact": guard_managed_artifact_name(finding.artifact),
                "path": finding.path.display().to_string(),
                "issue": guard_artifact_issue_name(finding.issue),
                "details": finding.details,
            })
        })
        .collect::<Vec<_>>();
    let manifest_issues = audit
        .manifest_issues
        .iter()
        .map(|issue| guard_manifest_issue_name(*issue))
        .collect::<Vec<_>>();
    let affected_paths = audit
        .affected_paths()
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let observed_at = last_current_observation_at
        .as_ref()
        .map(UtcTimestamp::to_canonical_string);

    let guard_findings = persist_guard_boundary_findings(
        runtime_home,
        connection,
        &audit,
        files_status == ConnectionCheckStatus::Failed,
        &missing_required_phases,
        !incompatible_event_ids.is_empty(),
        prompt_capture_observed,
        observation_revision_mismatch,
        current_timestamp(),
        persist_findings,
    )?;

    block_failed_dependencies(vec![
        with_direct_causes(
            canonical_check(
                ConnectionCheckKind::GuardFiles,
                files_status,
                "guard_files_failed",
                if files_status == ConnectionCheckStatus::Passed {
                    "Guard managed files match the current typed manifest expectations"
                } else {
                    "Guard managed files do not match the current typed manifest expectations"
                },
                Some(json!({
                    "installation_ids": installation_ids,
                    "affected_paths": affected_paths,
                    "artifact_issues": artifact_issues,
                    "manifest_issues": manifest_issues,
                    "missing_required_phases": configured_phase_gaps,
                })),
                None,
            )?,
            guard_findings.files,
        )?,
        canonical_check(
            ConnectionCheckKind::GuardHookExecution,
            hook_execution_status,
            if hook_execution_status == ConnectionCheckStatus::Passed {
                "guard_hook_execution_observed"
            } else {
                "guard_hook_execution_pending"
            },
            if hook_execution_status == ConnectionCheckStatus::Passed {
                "A current managed Guard hook executed"
            } else {
                "Current managed Guard hook execution has not been observed"
            },
            Some(json!({
                "observed_phases": observed_phases,
                "last_current_observation_at": observed_at,
            })),
            observed_at.as_deref(),
        )?,
        with_direct_causes(
            canonical_check(
                ConnectionCheckKind::GuardObservation,
                observation_status,
                match observation_status {
                    ConnectionCheckStatus::Passed => "guard_observation_passed",
                    ConnectionCheckStatus::Pending => "guard_observation_pending",
                    ConnectionCheckStatus::Failed => "guard_observation_failed",
                    ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                        unreachable!("raw Guard observation uses passed, pending, or failed")
                    }
                },
                match observation_status {
                    ConnectionCheckStatus::Passed => {
                        "Every current required Guard hook phase was observed"
                    }
                    ConnectionCheckStatus::Pending => {
                        "One or more current required Guard hook phases have not been observed"
                    }
                    ConnectionCheckStatus::Failed => {
                        "A current Guard event reported an incompatible hook contract"
                    }
                    ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                        unreachable!("raw Guard observation uses passed, pending, or failed")
                    }
                },
                Some(json!({
                    "required_phases": required_phases,
                    "observed_phases": observed_phases,
                    "missing_required_phases": missing_required_phases,
                    "incompatible_event_ids": incompatible_event_ids,
                    "prompt_capture": {
                        "host_supported": audit.prompt_capture_host_supported,
                        "configured": audit.prompt_capture_configured,
                        "observed": prompt_capture_observed,
                    },
                    "last_current_observation_at": observed_at,
                })),
                observed_at.as_deref(),
            )?,
            guard_findings.observation,
        )?,
    ])
}

#[derive(Default)]
struct GuardBoundaryFindings {
    files: Vec<DiagnosticFindingId>,
    observation: Vec<DiagnosticFindingId>,
}

#[allow(clippy::too_many_arguments)]
fn persist_guard_boundary_findings(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    audit: &GuardAuditFacts,
    guard_files_failed: bool,
    missing_required_phases: &[String],
    incompatible_observation: bool,
    prompt_capture_observed: bool,
    observation_revision_mismatch: bool,
    observed_at: UtcTimestamp,
    persist_findings: bool,
) -> Result<GuardBoundaryFindings, ConnectionCommandError> {
    let mut findings = GuardBoundaryFindings::default();
    for finding in &audit.findings {
        findings.files.push(guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::from_artifact_issue(
                finding.artifact,
                finding.issue,
            )),
            &OperationalDiagnosticFacts {
                artifact_kind: Some(guard_artifact_kind(finding.artifact)),
                ..OperationalDiagnosticFacts::default()
            },
            observed_at.clone(),
            persist_findings,
        )?);
    }
    for issue in &audit.manifest_issues {
        findings.files.push(guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::from_manifest_issue(*issue)),
            &OperationalDiagnosticFacts::default(),
            observed_at.clone(),
            persist_findings,
        )?);
    }
    for status in &audit.hook_path_safety_statuses {
        if let Some(diagnostic) = GuardDiagnostic::from_hook_wrapper_status(*status) {
            findings.files.push(guard_boundary_finding_id(
                runtime_home,
                connection,
                OperationalDiagnostic::Guard(diagnostic),
                &OperationalDiagnosticFacts {
                    observed_state: Some(status.as_str()),
                    ..OperationalDiagnosticFacts::default()
                },
                observed_at.clone(),
                persist_findings,
            )?);
        }
    }
    if guard_files_failed && findings.files.is_empty() {
        findings.files.push(guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::ManifestMismatch),
            &OperationalDiagnosticFacts::default(),
            observed_at.clone(),
            persist_findings,
        )?);
    }
    for phase in &audit.missing_required_phases {
        guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
            &OperationalDiagnosticFacts {
                guard_phase: Some(phase.as_str().to_owned()),
                ..OperationalDiagnosticFacts::default()
            },
            observed_at.clone(),
            persist_findings,
        )?;
    }
    for phase in missing_required_phases {
        guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
            &OperationalDiagnosticFacts {
                guard_phase: Some(phase.clone()),
                ..OperationalDiagnosticFacts::default()
            },
            observed_at.clone(),
            persist_findings,
        )?;
    }
    if incompatible_observation {
        findings.observation.push(guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::IncompatibleObservation),
            &OperationalDiagnosticFacts::default(),
            observed_at.clone(),
            persist_findings,
        )?);
    }
    if audit.prompt_capture_configured && !audit.prompt_capture_host_supported {
        guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnsupported),
            &OperationalDiagnosticFacts::default(),
            observed_at.clone(),
            persist_findings,
        )?;
    } else if audit.prompt_capture_configured && !prompt_capture_observed {
        guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnobserved),
            &OperationalDiagnosticFacts::default(),
            observed_at.clone(),
            persist_findings,
        )?;
    }
    if observation_revision_mismatch {
        let revision = connection_integration_revision(connection)?;
        guard_boundary_finding_id(
            runtime_home,
            connection,
            OperationalDiagnostic::Revision(RevisionDiagnostic::ObservationMismatch),
            &OperationalDiagnosticFacts {
                expected_revision: Some(revision.as_str().to_owned()),
                ..OperationalDiagnosticFacts::default()
            },
            observed_at,
            persist_findings,
        )?;
    }
    Ok(findings)
}

fn guard_boundary_finding_id(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    diagnostic: OperationalDiagnostic,
    facts: &OperationalDiagnosticFacts,
    observed_at: UtcTimestamp,
    persist: bool,
) -> Result<DiagnosticFindingId, ConnectionCommandError> {
    if persist {
        persist_connection_operational_finding(
            runtime_home,
            connection,
            diagnostic,
            facts,
            observed_at,
        )
    } else {
        connection_operational_finding_id(connection, diagnostic)
    }
}

fn guard_artifact_issue_name(issue: GuardArtifactIssue) -> &'static str {
    match issue {
        GuardArtifactIssue::Missing => "missing",
        GuardArtifactIssue::Malformed => "malformed",
        GuardArtifactIssue::ContentMismatch => "content_mismatch",
        GuardArtifactIssue::OwnershipMismatch => "ownership_mismatch",
        GuardArtifactIssue::PermissionMismatch => "permission_mismatch",
        GuardArtifactIssue::HookContractMismatch => "hook_contract_mismatch",
    }
}

fn guard_managed_artifact_name(artifact: GuardManagedArtifact) -> String {
    match artifact {
        GuardManagedArtifact::HostHookWrapper(phase) => {
            format!("host_hook_wrapper:{}", phase.as_str())
        }
        artifact => artifact.kind().as_str().to_owned(),
    }
}

fn guard_manifest_issue_name(issue: GuardManifestIssue) -> &'static str {
    match issue {
        GuardManifestIssue::Malformed => "malformed",
        GuardManifestIssue::OwnershipMismatch => "ownership_mismatch",
    }
}

fn latest_timestamp(
    current: Option<UtcTimestamp>,
    candidate: Option<&str>,
) -> Result<Option<UtcTimestamp>, ConnectionCommandError> {
    let Some(candidate) = candidate else {
        return Ok(current);
    };
    let candidate = UtcTimestamp::parse(candidate).map_err(|_| {
        ConnectionCommandError::runtime(
            "stored guard_events.occurred_at is not a canonical RFC 3339 UTC instant",
        )
    })?;
    Ok(Some(current.map_or(candidate.clone(), |current| {
        current.max(candidate)
    })))
}

fn mcp_server_finding_ids(
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<Vec<DiagnosticFindingId>, ConnectionCommandError> {
    let mut ids = BTreeMap::<String, DiagnosticFindingId>::new();
    let mut insert = |value: &str| -> Result<(), ConnectionCommandError> {
        let id = DiagnosticFindingId::parse(value.to_owned())
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        ids.insert(value.to_owned(), id);
        Ok(())
    };
    if preflight.status == StepStatus::Failed {
        if let Some(diagnostic) = preflight.diagnostic.as_ref() {
            insert(&diagnostic.finding_id)?;
        }
    }
    if handshake.step.status == StepStatus::Failed {
        if let Some(exchange) = handshake.exchange.as_ref() {
            if let Some(diagnostic) = exchange.diagnostic.as_ref() {
                insert(&diagnostic.finding_id)?;
            }
            for probe in &exchange.conformance {
                if let Some(diagnostic) = probe.diagnostic.as_ref() {
                    insert(&diagnostic.finding_id)?;
                }
            }
            for probe in &exchange.host_compatibility {
                if let Some(diagnostic) = probe.diagnostic.as_ref() {
                    insert(&diagnostic.finding_id)?;
                }
            }
        }
    }
    Ok(ids.into_values().collect())
}

fn with_direct_causes(
    check: ConnectionCheck,
    cause_finding_ids: Vec<DiagnosticFindingId>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    if check.status() == ConnectionCheckStatus::Failed && !cause_finding_ids.is_empty() {
        check
            .with_cause_finding_ids(cause_finding_ids)
            .map_err(ConnectionCommandError::from)
    } else {
        Ok(check)
    }
}

fn finalize_check_graph(
    runtime_home: &Path,
    checks: Vec<ConnectionCheck>,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let mut rooted = Vec::with_capacity(checks.len());
    for check in checks {
        if matches!(
            check.status(),
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
        ) && !check.cause_finding_ids().is_empty()
        {
            let mut roots = BTreeSet::new();
            for finding_id in check.cause_finding_ids() {
                if diagnostic_finding(runtime_home, finding_id)?.is_some() {
                    roots.extend(diagnostic_root_cause_ids(
                        runtime_home,
                        std::slice::from_ref(finding_id),
                        MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
                    )?);
                } else {
                    roots.insert(finding_id.clone());
                }
            }
            rooted.push(check.with_cause_finding_ids(roots.into_iter().collect())?);
        } else {
            rooted.push(check);
        }
    }

    block_failed_dependencies(rooted)
}

fn block_failed_dependencies(
    mut checks: Vec<ConnectionCheck>,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    for _ in 0..checks.len() {
        let states = checks
            .iter()
            .map(|check| {
                (
                    check.id(),
                    (check.status(), check.cause_finding_ids().to_vec()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut changed = false;
        for check in &mut checks {
            if matches!(
                check.status(),
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable
            ) {
                continue;
            }
            let causes = check
                .depends_on()
                .iter()
                .filter_map(|dependency| states.get(dependency))
                .filter(|(status, _)| {
                    matches!(
                        status,
                        ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
                    )
                })
                .flat_map(|(_, causes)| causes.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if causes.is_empty() {
                continue;
            }
            *check = check.clone().blocked_by(causes)?;
            changed = true;
        }
        if !changed {
            return Ok(checks);
        }
    }
    Err(ConnectionCommandError::runtime(
        "connection check dependency traversal exceeded the check bound",
    ))
}

fn actions_for_checks(
    checks: &[ConnectionCheck],
) -> Result<Vec<ConnectionAction>, ConnectionCommandError> {
    let mut actions = BTreeMap::<ConnectionActionKind, &str>::new();
    for check in checks {
        match (check.id(), check.status()) {
            (ConnectionCheckKind::ManagedConfig, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::RepairManagedConfig,
                    "Repair or recreate the Volicord-managed Codex MCP entry",
                );
            }
            (ConnectionCheckKind::HostExecutable, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::InstallOrRepairCodex,
                    "Install or repair Codex so `codex --version` succeeds on PATH",
                );
            }
            (ConnectionCheckKind::McpServer, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::RepairMcpServer,
                    "Repair the Volicord MCP configuration or storage error and verify again",
                );
            }
            (ConnectionCheckKind::ProjectTrust, ConnectionCheckStatus::Pending) => {
                actions.insert(
                    ConnectionActionKind::HostTrustRequired,
                    "Trust the project in Codex, then restart or reload Codex",
                );
            }
            (
                ConnectionCheckKind::HostSession
                | ConnectionCheckKind::RequiredTools
                | ConnectionCheckKind::ToolRoundTrip
                | ConnectionCheckKind::GuardObservation,
                ConnectionCheckStatus::Pending,
            ) => {
                actions.insert(
                    ConnectionActionKind::ObserveCodex,
                    "Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool so actual Codex connection and Guard activity can be observed",
                );
            }
            (
                ConnectionCheckKind::HostSession
                | ConnectionCheckKind::RequiredTools
                | ConnectionCheckKind::ToolRoundTrip
                | ConnectionCheckKind::GuardObservation,
                ConnectionCheckStatus::Failed,
            ) => {
                actions.insert(
                    ConnectionActionKind::InspectCodexProtocol,
                    "Inspect the recorded Codex protocol failure, repair the incompatible configuration or behavior, then verify again",
                );
            }
            (ConnectionCheckKind::GuardFiles, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::RepairGuard,
                    "Repair the Volicord Guard integration and verify the connection again",
                );
            }
            _ => {}
        }
    }
    actions
        .into_iter()
        .map(|(id, instruction)| {
            ConnectionAction::try_new(id, instruction).map_err(ConnectionCommandError::from)
        })
        .collect()
}

fn canonical_check(
    id: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    details: Option<Value>,
    observed_at: Option<&str>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let details = details
        .map(compact_json_value)
        .map(|value| {
            let Value::Object(object) = value else {
                return Err(ConnectionCommandError::runtime(
                    "connection check details must be a JSON object",
                ));
            };
            ConnectionCheckDetails::try_new(object).map_err(ConnectionCommandError::from)
        })
        .transpose()?;
    let observed_at = observed_at
        .map(|value| {
            UtcTimestamp::from_str(value).map_err(|_| {
                ConnectionCommandError::runtime(format!(
                    "connection check observation time is invalid: {value}"
                ))
            })
        })
        .transpose()?;
    ConnectionCheck::try_new(
        id,
        status,
        Vec::new(),
        (status != ConnectionCheckStatus::Passed).then(|| code.to_owned()),
        summary,
        details,
        observed_at,
    )
    .map_err(ConnectionCommandError::from)
}

fn compact_json_value(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .filter_map(|(key, value)| {
                    (value != Value::Null).then(|| (key, compact_json_value(value)))
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(compact_json_value).collect()),
        other => other,
    }
}

fn verify_host_plan(
    host_kind: HostKind,
    host_plan: &HostPlan,
    process: &impl ConnectionProcess,
) -> Result<Verification, ConnectionCommandError> {
    match host_kind {
        HostKind::Codex => CodexAdapter::new(codex_environment(process))
            .verify(host_plan)
            .map_err(ConnectionCommandError::from),
    }
}

pub(in crate::connection_command) fn current_status_host_diagnostic(
    _runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: Option<&HostPlan>,
    projects: &[ConnectionProjectRecord],
    process: &impl ConnectionProcess,
) -> Result<Option<Verification>, ConnectionCommandError> {
    let Some(host_plan) = host_plan else {
        return Ok(None);
    };
    let evaluation = codex::managed_identity_evaluation_for_plan(host_plan)?;
    let mut host = Verification::unobserved(&connection.config_target);
    host.managed_config = evaluation.status;
    host.managed_config_diagnostic = evaluation.diagnostic;
    host.managed_config_details = evaluation.details;
    if host_plan.host_scope == HostScope::Project {
        if let Some(project) = projects.first() {
            host.project_trust = Some(codex::project_trust_diagnostic(
                &codex_environment(process),
                &project.project.repo_root,
            ));
        }
    }
    Ok(Some(host))
}

pub(in crate::connection_command) fn current_status_report(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: Option<&HostPlan>,
    projects: &[ConnectionProjectRecord],
    process: &impl ConnectionProcess,
) -> Result<(Option<Verification>, ConnectionVerificationReport), ConnectionCommandError> {
    let current_host =
        current_status_host_diagnostic(runtime_home, connection, host_plan, projects, process)?;
    let persisted = connection.verification_report()?;
    let Some(mut host) = current_host else {
        return Ok((
            None,
            persisted.unwrap_or(effective_connection_report(connection)?),
        ));
    };
    let stored_executable = persisted
        .as_ref()
        .and_then(|report| {
            report
                .checks()
                .iter()
                .find(|check| check.id() == ConnectionCheckKind::HostExecutable)
        })
        .cloned();
    if let Some(check) = stored_executable.as_ref() {
        host.host_executable = match check.status() {
            ConnectionCheckStatus::Passed => HostExecutableStatus::Available,
            ConnectionCheckStatus::Failed => HostExecutableStatus::Unavailable,
            ConnectionCheckStatus::Pending
            | ConnectionCheckStatus::Blocked
            | ConnectionCheckStatus::NotApplicable => HostExecutableStatus::NotChecked,
        };
        host.host_executable_code = check
            .code()
            .unwrap_or("host_executable_not_checked")
            .to_owned();
        if let Some(details) = check.details().map(ConnectionCheckDetails::as_object) {
            host.executable_path = details
                .get("path")
                .and_then(Value::as_str)
                .map(str::to_owned);
            host.host_version = details
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned);
            host.host_executable_details = details
                .get("diagnostic")
                .and_then(Value::as_str)
                .unwrap_or(check.summary())
                .to_owned();
        }
    }
    let stored_mcp = persisted
        .as_ref()
        .and_then(|report| {
            report
                .checks()
                .iter()
                .find(|check| check.id() == ConnectionCheckKind::McpServer)
        })
        .cloned()
        .unwrap_or(canonical_check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Pending,
            "mcp_server_not_verified",
            "Volicord MCP server has not been actively verified",
            None,
            None,
        )?);
    let stored_mcp = if stored_mcp.status() == ConnectionCheckStatus::Blocked {
        canonical_check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Pending,
            "mcp_server_reverification_required",
            "Volicord MCP server requires active verification after its blocker is resolved",
            stored_mcp
                .details()
                .map(ConnectionCheckDetails::as_object)
                .cloned()
                .map(Value::Object),
            None,
        )?
    } else {
        stored_mcp
    };
    let current_revision = connection_integration_revision(connection)?;
    let current_sessions =
        current_managed_runtime_sessions(runtime_home, &connection.connection_internal_id)?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let managed_config_causes = host
        .managed_config_diagnostic
        .map(|diagnostic| {
            connection_operational_finding_id(
                connection,
                OperationalDiagnostic::ManagedConfig(diagnostic),
            )
        })
        .transpose()?
        .into_iter()
        .collect();
    let mut checks = vec![
        with_direct_causes(managed_config_check(&host)?, managed_config_causes)?,
        stored_mcp,
        project_trust_check(&host)?,
    ];
    checks.push(stored_executable.unwrap_or(canonical_check(
        ConnectionCheckKind::HostExecutable,
        ConnectionCheckStatus::Pending,
        "host_executable_not_verified",
        "Codex executable has not been actively verified",
        None,
        None,
    )?));
    checks.extend(host_session_checks(
        &host,
        current_revision.as_str(),
        &current_sessions,
        latest_session.as_ref(),
    )?);
    checks.extend(guard_checks_for_connection(
        runtime_home,
        connection,
        projects,
        false,
    )?);
    checks = finalize_check_graph(runtime_home, checks)?;
    let actions = actions_for_checks(&checks)?;
    let report = ConnectionVerificationReport::try_new(current_timestamp(), checks, actions)?;
    Ok((Some(host), report))
}

fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
}

#[cfg(test)]
mod tests {
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
            designated_safe_tool_observed_at: Some("2026-07-18T00:00:04Z".to_owned()),
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
    fn initialize_response_without_initialized_notification_remains_pending() {
        let host = host("future");
        let mut session = managed_session("future", true);
        session.negotiated_protocol_version = None;
        session.initialized_notification_at = None;
        session.tools_list_observed_at = None;
        session.required_tools_present = None;
        session.designated_safe_tool_observed_at = None;

        let checks = host_session_checks(
            &host,
            "revision_current",
            std::slice::from_ref(&session),
            Some(&session),
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
        newer.designated_safe_tool_observed_at = None;
        newer.last_observed_at = "2026-07-18T00:01:00Z".to_owned();

        let sessions = vec![newer.clone(), completed.clone()];
        let checks = host_session_checks(&host, "revision_current", &sessions, Some(&newer))
            .expect("concurrent session checks");
        assert!(checks
            .iter()
            .all(|check| check.status() == ConnectionCheckStatus::Passed));

        newer.terminal_finding_id = Some("finding.later_crash".to_owned());
        let sessions = vec![newer.clone(), completed];
        let checks = host_session_checks(&host, "revision_current", &sessions, Some(&newer))
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
            host_session_checks(&host, "revision_current", &[], Some(&old)).expect("stale checks");
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
        let checks = host_session_checks(&host("future"), "revision_current", &[], None)
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
        session.designated_safe_tool_observed_at = None;
        session.terminal_finding_id = Some("finding.protocol_contract_mismatch".to_owned());
        let checks = host_session_checks(
            &host,
            "revision_current",
            std::slice::from_ref(&session),
            Some(&session),
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
        session.designated_safe_tool_observed_at = None;
        session.terminal_finding_id = Some("finding.initialize_failed".to_owned());
        let checks = host_session_checks(
            &host,
            "revision_current",
            std::slice::from_ref(&session),
            Some(&session),
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
        session.designated_safe_tool_observed_at = None;
        session.terminal_finding_id = Some("finding.tools_list_failed".to_owned());
        let checks = host_session_checks(
            &host,
            "revision_current",
            std::slice::from_ref(&session),
            Some(&session),
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
            host_session_checks(&host, "revision_current", &[], None).expect("pending host checks"),
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
        let report =
            ConnectionVerificationReport::try_new(current_timestamp(), checks, first.clone())
                .expect("canonical report");
        assert_eq!(report.status(), ConnectionStatus::Failed);
        assert_eq!(report.actions(), first);
    }

    #[test]
    fn mcp_server_details_use_the_public_safe_tool_name_constant() {
        let check = mcp_server_check(
            &VerificationStep::passed_with_code("mcp_preflight_ready", "ready"),
            &McpVerification::from_exchange(
                crate::connection_command::McpExchangeOutcome::completed(
                    crate::connection_command::McpExchangeProgress::observed(
                        true,
                        Some(vec![LIST_PROJECTS_TOOL_NAME.to_owned()]),
                        true,
                        true,
                        true,
                    ),
                ),
            ),
        )
        .expect("MCP server check");
        let details = check.details().expect("MCP details").as_object();

        assert_eq!(
            details["self_test"]["safe_read_only_tool"],
            LIST_PROJECTS_TOOL_NAME
        );
    }

    fn projected_self_test(
        progress: crate::connection_command::McpExchangeProgress,
        failure: Option<McpProcessFailure>,
    ) -> Value {
        let exchange = match failure {
            Some(failure) => {
                crate::connection_command::McpExchangeOutcome::failed(progress, failure)
            }
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
            crate::connection_command::McpExchangeProgress::observed(
                true, None, false, false, false,
            ),
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
                Some(vec![LIST_PROJECTS_TOOL_NAME.to_owned()]),
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
            json!([LIST_PROJECTS_TOOL_NAME])
        );
        assert_eq!(safe_call_failed["required_tools_validated"], true);
        assert_eq!(safe_call_failed["safe_read_only_tool_completed"], false);

        let shutdown_failed = projected_self_test(
            crate::connection_command::McpExchangeProgress::observed(
                true,
                Some(vec![LIST_PROJECTS_TOOL_NAME.to_owned()]),
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
            json!([LIST_PROJECTS_TOOL_NAME])
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
}
