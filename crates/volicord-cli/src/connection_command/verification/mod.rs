//! Connection verification coordination and shared report types.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    str::FromStr,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use volicord_mcp::ManagedMcpInvocationPurpose;
use volicord_mcp_protocol::ProtocolRegistry;
use volicord_store::{
    agent_connections::{
        agent_connection_record_read_only, list_connection_projects_read_only,
        AgentConnectionRecord, ConnectionProjectRecord,
    },
    core_pipeline::CoreProjectStore,
    diagnostic_findings::{diagnostic_occurrences_for_runtime_session, insert_occurrence_finding},
    guards::{guard_observation_summary, list_guard_installations},
    integration_verification::{
        current_guard_integration_verification_workflow, guard_probe_observations,
        latest_completed_guard_integration_verification_for_connection,
        latest_completed_guard_integration_verification_for_membership,
        latest_guard_integration_verification_for_connection,
        latest_guard_integration_verification_for_membership,
        GuardIntegrationVerificationRunRecord,
    },
    operational_sessions::{
        connection_integration_revision, current_managed_runtime_sessions,
        latest_managed_runtime_session, mcp_runtime_session_for_process, McpRuntimeSessionRecord,
        McpSessionEvidenceSelection, McpSessionMilestones,
    },
    sqlite::{project_state_database_write_capability, registry_database_write_capability},
    RuntimeHomeMutationContext, StoreError,
};
#[cfg(test)]
use volicord_types::connection_verification::ConnectionStatus;
use volicord_types::connection_verification::{
    derive_integration_activation_state, ActivationStep, ActivationStepId, ConnectionCheck,
    ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus,
    ConnectionVerificationReport, HookActivationEvidence, HookActivationState,
    IntegrationActivationPlan, IntegrationActivationState,
};
use volicord_types::diagnostics::{
    CurrentDiagnosticFinding, DiagnosticCode, DiagnosticDomain, DiagnosticFactSource,
    DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId, DiagnosticSeverity, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject, MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
};
use volicord_types::guard_manifest::GuardManagedArtifact;
use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId, ProjectId};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::integration_verification::{
    GuardIntegrationVerificationStatus, GuardProbeObservationStage,
    GuardVerificationRecoverability, GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    IntegrationVerificationWorkflowState,
};
use volicord_types::mcp_verification_evidence::{
    McpActiveVerificationEvidence, McpEvidenceCheckStatus, McpHostCompatibilityEvidence,
    McpPreflightEvidence, McpProbeEvidence, McpProjectWriteEvidence, McpRevisionConformance,
    McpSideEffectKind,
};
use volicord_types::values::UtcTimestamp;

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
    codex_environment, existing_host_plan,
    mcp_process::{
        materialize_connection_invocation, run_connection_preflight, McpPersistedDiagnostic,
        McpProcessDiagnosticContext, McpProcessFailure, McpVerification,
    },
    parse_host_kind, parse_metadata, ConnectionCommandError, ConnectionProcess,
};

mod dependency_graph;
mod evidence;
mod finding_projection;
mod guard_checks;
mod host_checks;
mod mcp_checks;
mod report_inputs;

use dependency_graph::*;
pub(in crate::connection_command) use dependency_graph::{
    activation_plan_for_checks, activation_plan_for_checks_with_hook_state,
};
use evidence::*;
use finding_projection::*;
use guard_checks::*;
use host_checks::*;
pub(in crate::connection_command) use mcp_checks::mcp_server_check;
use mcp_checks::mcp_server_finding_ids;
use report_inputs::{assemble_connection_evaluation, canonical_verification_evaluation};
pub(in crate::connection_command) use report_inputs::{
    current_status_report, effective_connection_report, report_with_hook_review_required,
    CurrentConnectionEvaluationContext, CurrentConnectionEvaluationUnavailable,
    CurrentConnectionEvaluationUnavailableCause,
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
    pub(in crate::connection_command) preflight_evidence: Option<McpPreflightEvidence>,
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
            preflight_evidence: None,
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
            preflight_evidence: None,
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
            preflight_evidence: None,
            process_id: None,
            failure: None,
            diagnostic: None,
        }
    }

    pub(in crate::connection_command) fn with_preflight_evidence(
        mut self,
        evidence: McpPreflightEvidence,
    ) -> Self {
        self.preflight_evidence = Some(evidence);
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

    pub(in crate::connection_command) fn with_persisted_diagnostic(
        mut self,
        diagnostic: McpPersistedDiagnostic,
    ) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }
}

#[derive(Debug, Clone)]
pub(in crate::connection_command) struct VerificationReport {
    pub(in crate::connection_command) report: ConnectionVerificationReport,
    pub(in crate::connection_command) findings: Vec<DiagnosticFinding>,
    pub(in crate::connection_command) integration_revision: IntegrationRevision,
}

pub(in crate::connection_command) fn verify_connection(
    context: &RuntimeHomeMutationContext<'_>,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    repo_root: &Path,
    project_id: Option<&str>,
    process: &mut impl ConnectionProcess,
) -> Result<VerificationReport, ConnectionCommandError> {
    let runtime_home = context.runtime_home().as_path();
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
    let preflight = run_connection_preflight(
        process,
        &preflight_launch,
        &connection.connection_internal_id,
        &connection.mode,
    );
    let writeability = (preflight.status == StepStatus::Passed)
        .then(|| verify_selected_store_writeability(context, connection, project_id));
    let handshake = if let Some(writeability) = &writeability {
        if let Some(details) = writeability.failure.as_deref() {
            McpVerification::writeability_failed(details)
        } else {
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
        }
    } else {
        McpVerification::not_run()
    };
    let (preflight, mut handshake) =
        persist_process_diagnostics(context, connection, preflight, handshake)?;
    if let Some(writeability) = &writeability {
        let evidence = active_verification_evidence(writeability, &handshake, current_timestamp());
        handshake = handshake.with_active_evidence(evidence);
    }
    let evaluation =
        canonical_verification_evaluation(context, connection, &host, &preflight, &handshake)?;
    let scope = volicord_types::diagnostics::DiagnosticScope::try_new(
        volicord_types::diagnostics::DiagnosticScopeKind::Connection,
        &connection.connection_internal_id,
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    reconcile_current_findings_for_scope(
        context,
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

#[derive(Debug, Clone)]
pub(in crate::connection_command) struct McpStoreWriteabilityEvidence {
    pub(in crate::connection_command) registry_write: McpEvidenceCheckStatus,
    pub(in crate::connection_command) project_writes: Vec<McpProjectWriteEvidence>,
    pub(in crate::connection_command) failure: Option<String>,
}

fn verify_selected_store_writeability(
    context: &RuntimeHomeMutationContext<'_>,
    connection: &AgentConnectionRecord,
    selected_project_id: Option<&str>,
) -> McpStoreWriteabilityEvidence {
    let runtime_home = context.runtime_home().as_path();
    let (registry_write, mut failure) = match registry_database_write_capability(context) {
        Ok(true) => (McpEvidenceCheckStatus::Passed, None),
        Ok(false) => (
            McpEvidenceCheckStatus::Failed,
            Some("Registry writeability probe reported read-only storage".to_owned()),
        ),
        Err(error) => (
            McpEvidenceCheckStatus::Failed,
            Some(format!("Registry writeability probe failed: {error}")),
        ),
    };
    let projects = match list_connection_projects_read_only(
        runtime_home,
        &connection.connection_internal_id,
    ) {
        Ok(projects) => projects,
        Err(error) => {
            failure.get_or_insert_with(|| {
                format!("failed to read Connection projects for writeability probe: {error}")
            });
            return McpStoreWriteabilityEvidence {
                registry_write,
                project_writes: Vec::new(),
                failure,
            };
        }
    };
    let mut project_writes = Vec::new();
    for project in projects
        .into_iter()
        .filter(|project| selected_project_id.is_none_or(|selected| project.project_id == selected))
    {
        let state_write = match project_state_database_write_capability(context, &project.project) {
            Ok(true) => McpEvidenceCheckStatus::Passed,
            Ok(false) => {
                failure.get_or_insert_with(|| {
                    format!(
                        "project {} writeability probe reported read-only storage",
                        project.project_id
                    )
                });
                McpEvidenceCheckStatus::Failed
            }
            Err(error) => {
                failure.get_or_insert_with(|| {
                    format!(
                        "project {} writeability probe failed: {error}",
                        project.project_id
                    )
                });
                McpEvidenceCheckStatus::Failed
            }
        };
        project_writes.push(McpProjectWriteEvidence::new(
            project.project_id,
            state_write,
        ));
    }
    McpStoreWriteabilityEvidence {
        registry_write,
        project_writes,
        failure,
    }
}

pub(in crate::connection_command) fn active_verification_evidence(
    writeability: &McpStoreWriteabilityEvidence,
    handshake: &McpVerification,
    observed_at: UtcTimestamp,
) -> McpActiveVerificationEvidence {
    let exchange = handshake.exchange.as_ref();
    let mut protocol_conformance = exchange
        .into_iter()
        .flat_map(|exchange| &exchange.conformance)
        .map(|probe| {
            McpRevisionConformance::new(
                &probe.revision,
                active_probe_evidence(
                    &probe.progress,
                    probe.failure.as_ref(),
                    probe.diagnostic.as_ref(),
                ),
            )
        })
        .collect::<Vec<_>>();
    if protocol_conformance.is_empty() {
        if let Some(exchange) = exchange {
            protocol_conformance.push(McpRevisionConformance::new(
                ProtocolRegistry::production()
                    .preferred_server_profile()
                    .revision()
                    .as_str(),
                active_probe_evidence(
                    &exchange.progress,
                    exchange.failure.as_ref(),
                    exchange.diagnostic.as_ref(),
                ),
            ));
        }
    }
    let host_compatibility = exchange
        .into_iter()
        .flat_map(|exchange| &exchange.host_compatibility)
        .map(|probe| {
            McpHostCompatibilityEvidence::new(
                probe.profile.as_str(),
                &probe.fixture_id,
                active_probe_evidence(
                    &probe.progress,
                    probe.failure.as_ref(),
                    probe.diagnostic.as_ref(),
                ),
            )
        })
        .collect::<Vec<_>>();
    let mut side_effects = vec![McpSideEffectKind::RollbackOnlyRegistryWriteProbe];
    if !writeability.project_writes.is_empty() {
        side_effects.push(McpSideEffectKind::RollbackOnlyProjectWriteProbe);
    }
    if !protocol_conformance.is_empty() {
        side_effects.push(McpSideEffectKind::DisposableProtocolConformance);
    }
    if !host_compatibility.is_empty() {
        side_effects.push(McpSideEffectKind::DisposableHostCompatibility);
    }
    McpActiveVerificationEvidence::new(
        writeability.registry_write,
        writeability.project_writes.clone(),
        protocol_conformance,
        host_compatibility,
        observed_at,
        side_effects,
    )
}

fn active_probe_evidence(
    progress: &crate::connection_command::McpExchangeProgress,
    failure: Option<&McpProcessFailure>,
    diagnostic: Option<&McpPersistedDiagnostic>,
) -> McpProbeEvidence {
    McpProbeEvidence::new(
        if failure.is_none() {
            McpEvidenceCheckStatus::Passed
        } else {
            McpEvidenceCheckStatus::Failed
        },
        progress.requested_revision.clone(),
        progress.negotiated_revision.clone(),
        progress.initialize_completed,
        progress.initialized_notification_completed,
        progress.pinned_schema_validated,
        progress.tools_list.is_some(),
        progress.tools_list.as_ref().map(Vec::len),
        progress.required_tools_validated,
        super::managed_host_round_trip_tool().wire_name(),
        progress.safe_tool_call_completed,
        progress.shutdown_completed,
        failure
            .map(|failure| failure.diagnostic_code().to_owned())
            .or_else(|| diagnostic.map(|diagnostic| diagnostic.code.clone())),
        failure.map(|failure| failure.stage().as_str().to_owned()),
        diagnostic.map(|diagnostic| diagnostic.finding_id.clone()),
    )
}

pub(in crate::connection_command) fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
}

#[cfg(test)]
mod tests;
