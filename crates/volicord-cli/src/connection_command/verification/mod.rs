//! Connection verification coordination and shared report types.

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
    agent_connections::{
        list_connection_projects_read_only, AgentConnectionRecord, ConnectionProjectRecord,
    },
    diagnostic_findings::{diagnostic_occurrences_for_runtime_session, insert_occurrence_finding},
    guards::{guard_observation_summary, list_guard_installations},
    integration_verification::{
        current_guard_integration_verification_workflow, guard_probe_observations,
        latest_completed_guard_integration_verification_for_connection,
        latest_guard_integration_verification_for_connection,
        GuardIntegrationVerificationRunRecord,
    },
    operational_sessions::{
        connection_integration_revision, current_managed_runtime_sessions,
        latest_managed_runtime_session, mcp_runtime_session_for_process, McpRuntimeSessionRecord,
        McpSessionEvidenceSelection, McpSessionMilestones,
    },
    sqlite::{registry_db_path, sqlite_database_write_capability},
};
#[cfg(test)]
use volicord_types::ConnectionStatus;
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, AgentToolId, ConnectionAction, ConnectionActionKind,
    ConnectionCheck, ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus,
    ConnectionVerificationReport, CurrentDiagnosticFinding, DiagnosticCode, DiagnosticDomain,
    DiagnosticFactSource, DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId,
    DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject,
    GuardIntegrationVerificationStatus, GuardManagedArtifact, GuardProbeObservationStage,
    GuardVerificationRecoverability, GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    HookActivationEvidence, HookActivationState, IntegrationRevision,
    IntegrationVerificationWorkflowState, UtcTimestamp, MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
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
#[cfg(test)]
use crate::operational_diagnostics::current_report_findings;
use crate::operational_diagnostics::{
    current_connection_finding, current_report_findings_with_overlay, guard_artifact_kind,
    reconcile_current_findings_for_scope, CurrentOperationalOwner, DiagnosticFindingOverlay,
    GuardArtifactFacts, GuardDiagnostic, GuardEventFacts, GuardEventSubject,
    GuardInstallationFacts, GuardInstallationSubject, GuardManagedArtifactSubject, GuardPhaseFacts,
    GuardPhaseSubject, GuardProbeFacts, GuardVerificationAttemptSubject, IntegrationRevisionFacts,
    IntegrationRevisionSubject, ManagedConfigurationFacts, ManagedConfigurationTarget,
    OperationalCheckState, OperationalDiagnostic, RevisionDiagnostic, ToolVerificationDiagnostic,
    TrustDiagnostic, TrustFacts, TrustSubject, VerificationToolFacts, VerificationToolSubject,
};

use super::{
    codex_environment,
    mcp_process::{
        materialize_connection_invocation, run_connection_preflight, McpPersistedDiagnostic,
        McpProcessDiagnosticContext, McpProcessFailure, McpVerification,
    },
    parse_host_kind, ConnectionCommandError, ConnectionProcess,
};

mod dependency_graph;
mod evidence;
mod finding_projection;
mod guard_checks;
mod host_checks;
mod mcp_checks;
mod report_inputs;

use dependency_graph::*;
use evidence::*;
use finding_projection::*;
use guard_checks::*;
use host_checks::*;
pub(in crate::connection_command) use mcp_checks::mcp_server_check;
use mcp_checks::mcp_server_finding_ids;
use report_inputs::{assemble_connection_evaluation, canonical_verification_evaluation};
pub(in crate::connection_command) use report_inputs::{
    connection_metadata_failure_report, current_status_report, effective_connection_report,
    report_with_hook_review_required,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum StepStatus {
    Passed,
    Failed,
    Pending,
}

#[derive(Debug, Default)]
pub(super) struct ConnectionCheckEvaluation {
    pub(super) checks: Vec<ConnectionCheck>,
    pub(super) inline_findings: Vec<CurrentDiagnosticFinding>,
    pub(super) persisted_finding_seed_ids: Vec<DiagnosticFindingId>,
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
    pub(in crate::connection_command) fn from_preflight_report(report: &Value) -> Option<Self> {
        Some(Self {
            storage_read: report.get("project_state_read")?.as_str()?.to_owned(),
            storage_write: report
                .get("writeability")?
                .get("status")?
                .as_str()?
                .to_owned(),
            effective_tool_mode: report.get("effective_tool_mode")?.as_str()?.to_owned(),
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
    pub(in crate::connection_command) findings: Vec<DiagnosticFinding>,
    pub(in crate::connection_command) integration_revision: IntegrationRevision,
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
    if preflight.status == StepStatus::Passed {
        match verify_selected_store_writeability(runtime_home, connection, project_id) {
            Ok(()) => {
                if let Some(diagnostics) = preflight.preflight_diagnostics.as_mut() {
                    diagnostics.storage_write = "passed".to_owned();
                }
            }
            Err(details) => {
                preflight =
                    VerificationStep::failed_with_code("mcp_storage_writeability_failed", details)
                        .with_preflight_diagnostics(preflight.preflight_diagnostics);
            }
        }
    }
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
    let evaluation =
        canonical_verification_evaluation(runtime_home, connection, &host, &preflight, &handshake)?;
    let scope = volicord_types::DiagnosticScope::try_new(
        volicord_types::DiagnosticScopeKind::Connection,
        &connection.connection_internal_id,
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    reconcile_current_findings_for_scope(
        runtime_home,
        &scope,
        &[
            CurrentOperationalOwner::ManagedConfiguration,
            CurrentOperationalOwner::Trust,
            CurrentOperationalOwner::HostRevision,
            CurrentOperationalOwner::VerificationTool,
            CurrentOperationalOwner::Guard,
        ],
        &evaluation.inline_findings,
        evaluation.metadata.evaluated_at.clone(),
    )?;
    assemble_connection_evaluation(runtime_home, connection, evaluation)
}

fn verify_selected_store_writeability(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    selected_project_id: Option<&str>,
) -> Result<(), String> {
    match sqlite_database_write_capability(registry_db_path(runtime_home)) {
        Ok(true) => {}
        Ok(false) => {
            return Err("Registry writeability probe reported read-only storage".to_owned())
        }
        Err(error) => return Err(format!("Registry writeability probe failed: {error}")),
    }
    let projects =
        list_connection_projects_read_only(runtime_home, &connection.connection_internal_id)
            .map_err(|error| {
                format!("failed to read Connection projects for writeability probe: {error}")
            })?;
    for project in projects
        .into_iter()
        .filter(|project| selected_project_id.is_none_or(|selected| project.project_id == selected))
    {
        match sqlite_database_write_capability(&project.project.state_db_path) {
            Ok(true) => {}
            Ok(false) => {
                return Err(format!(
                    "project {} writeability probe reported read-only storage",
                    project.project_id
                ))
            }
            Err(error) => {
                return Err(format!(
                    "project {} writeability probe failed: {error}",
                    project.project_id
                ))
            }
        }
    }
    Ok(())
}

fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
}

#[cfg(test)]
mod tests;
