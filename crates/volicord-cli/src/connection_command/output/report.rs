use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    str::FromStr,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use volicord_platform_fs::{
    DirectoryEntryDurability, DirectoryTreeRemovalEffect, DirectoryTreeRemovalPhase,
    DirectoryTreeTargetState, RuntimeHomeMutationBusy, RuntimeHomeMutationLeaseMode,
};
use volicord_types::connection_verification::{
    derive_integration_activation_state, ActivationStep, ActivationStepId, ConnectionCheck,
    ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus, ConnectionStatus,
    ConnectionVerificationReport, HookActivationState, IntegrationActivationPlan,
    IntegrationActivationState,
};
#[cfg(test)]
use volicord_types::connection_verification::{
    ActivationActor, ActivationExecutionChannel, AgentSequenceStep,
};
use volicord_types::diagnostics::{
    diagnostic_root_cause_ids, DiagnosticAction, DiagnosticCode, DiagnosticConnectionContext,
    DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts, DiagnosticFinding,
    DiagnosticFindingId, DiagnosticOperation, DiagnosticReport, DiagnosticRuntimeSessionContext,
    DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject,
    RuntimeSessionEvidenceRole, MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
};
use volicord_types::integration_revision::IntegrationRevision;
#[cfg(test)]
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::{IntegrationProfile, UtcTimestamp};

use super::{
    cooperative_assurance_limits, human::render_command_report_concise, path_text,
    semantics::active_verification_snapshot, verbose::render_command_report_verbose,
    ConnectionCommandError, OutputFormat, PlannedConnectionChange, PlannedConnectionChangeKind,
};
use crate::connection_command::args::HumanOutputDetail;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::connection_command) enum CommandOperation {
    Init,
    Add,
    Status,
    Verify,
    Mode,
    Remove,
}

impl CommandOperation {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Add => "add",
            Self::Status => "status",
            Self::Verify => "verify",
            Self::Mode => "mode",
            Self::Remove => "remove",
        }
    }

    fn diagnostic_operation(self) -> DiagnosticOperation {
        match self {
            Self::Init => DiagnosticOperation::Init,
            Self::Add => DiagnosticOperation::Add,
            Self::Status => DiagnosticOperation::Status,
            Self::Verify => DiagnosticOperation::Verify,
            Self::Mode => DiagnosticOperation::Mode,
            Self::Remove => DiagnosticOperation::Remove,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(in crate::connection_command) struct CommandConnection {
    pub(super) id: String,
    pub(super) host: String,
    pub(super) scope: String,
    pub(super) profile: String,
    pub(super) mode: String,
    pub(super) repository: String,
    pub(super) config_target: String,
}

impl CommandConnection {
    pub(in crate::connection_command) fn new(
        id: impl Into<String>,
        host: impl Into<String>,
        scope: impl Into<String>,
        mode: impl Into<String>,
        repository: &Path,
        config_target: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            host: host.into(),
            scope: scope.into(),
            profile: IntegrationProfile::Record.as_str().to_owned(),
            mode: mode.into(),
            repository: path_text(repository),
            config_target: config_target.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ConnectionCommandResult {
    Setup {
        disposition: SetupDisposition,
        setup_lease: SetupLeaseStatus,
        runtime_home_publication: RuntimeHomePublicationStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        runtime_home_rollback: Option<RuntimeHomeRollbackResult>,
    },
    ModeTransition {
        changed: bool,
        previous_mode: String,
        current_mode: String,
        previous_integration_revision: String,
        current_integration_revision: String,
        rebound_guard_installation_ids: Vec<String>,
    },
    Removal {
        membership_removed: bool,
        connection_removed: bool,
        remaining_project_count: usize,
    },
}

/// Setup-lease state retained by a completed, planned, or rolled-back report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::connection_command) enum SetupLeaseStatus {
    Acquired,
}

/// Stable typed Runtime Home rollback result carried by failed setup output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(in crate::connection_command) enum RuntimeHomeRollbackResult {
    Removed {
        durability: DirectoryEntryDurability,
        #[serde(skip_serializing_if = "Option::is_none")]
        failure_phase: Option<DirectoryTreeRemovalPhase>,
    },
    RemovalIncomplete {
        effect: DirectoryTreeRemovalEffect,
        phase: DirectoryTreeRemovalPhase,
        final_path: DirectoryTreeTargetState,
    },
    Preserved {
        reason: String,
    },
    OwnershipLost {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::connection_command) enum RuntimeHomePublicationStatus {
    NotPublished,
    ExistingReady,
    PublishedByThisInvocation,
    OwnedPublicationRolledBack,
    OwnedPublicationRemovalIncomplete,
    OwnedPublicationPreserved,
    OwnershipLostDuringRollback,
}

impl RuntimeHomePublicationStatus {
    pub(in crate::connection_command) const fn as_str(self) -> &'static str {
        match self {
            Self::NotPublished => "not_published",
            Self::ExistingReady => "existing_ready",
            Self::PublishedByThisInvocation => "published_by_this_invocation",
            Self::OwnedPublicationRolledBack => "owned_publication_rolled_back",
            Self::OwnedPublicationRemovalIncomplete => "owned_publication_removal_incomplete",
            Self::OwnedPublicationPreserved => "owned_publication_preserved",
            Self::OwnershipLostDuringRollback => "ownership_lost_during_rollback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::connection_command) enum SetupDisposition {
    Planned,
    Committed,
    RolledBack,
    Preserved,
    PartiallyRolledBack,
}

impl SetupDisposition {
    pub(in crate::connection_command) const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Committed => "committed",
            Self::RolledBack => "rolled_back",
            Self::Preserved => "preserved",
            Self::PartiallyRolledBack => "partially_rolled_back",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum SetupFailureDiagnostic {
    TransactionFailed,
    ConcurrentModification,
    PartialRollback,
}

impl SetupFailureDiagnostic {
    pub(in crate::connection_command) const ALL: [Self; 3] = [
        Self::TransactionFailed,
        Self::ConcurrentModification,
        Self::PartialRollback,
    ];

    const fn finding_id(self) -> &'static str {
        match self {
            Self::TransactionFailed => "finding.setup.transaction_failed",
            Self::ConcurrentModification => "finding.setup.concurrent_modification",
            Self::PartialRollback => "finding.setup.partial_rollback",
        }
    }

    pub(in crate::connection_command) const fn code(self) -> &'static str {
        match self {
            Self::TransactionFailed => "setup.transaction_failed",
            Self::ConcurrentModification => "setup.concurrent_modification",
            Self::PartialRollback => "setup.partial_rollback",
        }
    }

    const fn check_reason(self) -> &'static str {
        match self {
            Self::TransactionFailed => "setup_transaction_failed",
            Self::ConcurrentModification => "setup_concurrent_modification",
            Self::PartialRollback => "setup_partial_rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::connection_command) struct ConnectionCommandReport {
    pub(super) operation: CommandOperation,
    pub(super) dry_run: bool,
    pub(super) status: ConnectionStatus,
    pub(super) activation_state: IntegrationActivationState,
    pub(super) hook_activation_state: HookActivationState,
    pub(super) runtime_home: String,
    pub(super) connection: CommandConnection,
    pub(super) checks: Vec<ConnectionCheck>,
    pub(super) activation_plan: IntegrationActivationPlan,
    pub(super) generated_at: UtcTimestamp,
    pub(super) findings: Vec<DiagnosticFinding>,
    pub(super) integration_revision: Option<IntegrationRevision>,
    pub(super) result: Option<ConnectionCommandResult>,
    pub(super) planned_changes: Option<Vec<PlannedConnectionChange>>,
    pub(super) limits: Vec<String>,
}

impl ConnectionCommandReport {
    pub(in crate::connection_command) fn from_verification(
        operation: CommandOperation,
        setup_result: Option<SetupDisposition>,
        runtime_home: &Path,
        connection: CommandConnection,
        verification: &ConnectionVerificationReport,
    ) -> Self {
        Self::from_verification_with_publication(
            operation,
            setup_result,
            setup_result.map(|_| RuntimeHomePublicationStatus::ExistingReady),
            runtime_home,
            connection,
            verification,
        )
    }

    pub(in crate::connection_command) fn from_verification_with_publication(
        operation: CommandOperation,
        setup_result: Option<SetupDisposition>,
        runtime_home_publication: Option<RuntimeHomePublicationStatus>,
        runtime_home: &Path,
        connection: CommandConnection,
        verification: &ConnectionVerificationReport,
    ) -> Self {
        Self {
            operation,
            dry_run: false,
            status: command_status(verification),
            activation_state: verification.activation_state(),
            hook_activation_state: verification.hook_activation_state(),
            runtime_home: path_text(runtime_home),
            connection,
            checks: verification.checks().to_vec(),
            activation_plan: verification.activation_plan().clone(),
            generated_at: verification.checked_at().clone(),
            findings: Vec::new(),
            integration_revision: None,
            result: setup_result.map(|disposition| ConnectionCommandResult::Setup {
                disposition,
                setup_lease: SetupLeaseStatus::Acquired,
                runtime_home_publication: runtime_home_publication
                    .unwrap_or(RuntimeHomePublicationStatus::ExistingReady),
                runtime_home_rollback: None,
            }),
            planned_changes: None,
            limits: cooperative_assurance_limits(),
        }
    }

    pub(in crate::connection_command) fn setup_dry_run(
        operation: CommandOperation,
        runtime_home: &Path,
        connection: CommandConnection,
        current: Option<&ConnectionVerificationReport>,
        planned_changes: Vec<PlannedConnectionChange>,
    ) -> Result<Self, ConnectionCommandError> {
        let has_changes = !planned_changes.is_empty();
        let mut checks = current
            .map(|report| {
                report
                    .checks()
                    .iter()
                    .filter(|check| {
                        !matches!(
                            check.id(),
                            ConnectionCheckKind::ManagedConfig
                                | ConnectionCheckKind::HookSourceActivation
                                | ConnectionCheckKind::SetupPlan
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        checks.extend([
            command_check(
                ConnectionCheckKind::ManagedConfig,
                if planned_changes.iter().any(|change| {
                    change.kind() == PlannedConnectionChangeKind::ManagedHostConfiguration
                }) {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "managed_config_change_planned",
                "Managed Codex configuration plan was inspected",
                None,
            )?,
            command_check(
                ConnectionCheckKind::HookSourceActivation,
                ConnectionCheckStatus::Pending,
                if planned_changes
                    .iter()
                    .any(|change| change.kind() == PlannedConnectionChangeKind::HookDefinition)
                {
                    "hook_source_review_required_by_setup"
                } else {
                    "hook_source_activation_unknown"
                },
                "Project-local hook-source activation remains host owned",
                Some(serde_json::json!({
                    "activation_state": if planned_changes
                        .iter()
                        .any(|change| change.kind() == PlannedConnectionChangeKind::HookDefinition)
                    {
                        "review_required_by_setup"
                    } else {
                        "unknown"
                    }
                })),
            )?,
            command_check(
                ConnectionCheckKind::SetupPlan,
                if has_changes {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "setup_changes_planned",
                if has_changes {
                    "Setup changes are ready to apply"
                } else {
                    "Setup already matches the requested configuration"
                },
                None,
            )?,
        ]);
        if current.is_none() {
            checks.extend([
                command_check(
                    ConnectionCheckKind::HostReload,
                    ConnectionCheckStatus::Pending,
                    "host_reload_required",
                    "Codex has not loaded the planned managed configuration",
                    None,
                )?,
                command_check(
                    ConnectionCheckKind::ManagedSessionHealth,
                    ConnectionCheckStatus::Pending,
                    "managed_session_not_observed",
                    "A current managed Codex session has not been observed",
                    None,
                )?,
                command_check(
                    ConnectionCheckKind::ManagedCapabilityProof,
                    ConnectionCheckStatus::Pending,
                    "managed_capability_proof_not_observed",
                    "A current managed capability proof has not been observed",
                    None,
                )?,
                command_check(
                    ConnectionCheckKind::AmbientHookCoverage,
                    ConnectionCheckStatus::Pending,
                    "ambient_hook_coverage_pending",
                    "Current Guard hook installation and ambient phase coverage are incomplete",
                    None,
                )?,
                command_check(
                    ConnectionCheckKind::CorrelatedGuardVerification,
                    ConnectionCheckStatus::Pending,
                    "correlated_guard_verification_pending",
                    "In-chat MCP and Guard integration verification has not completed",
                    None,
                )?,
            ]);
        }

        let hook_definition_changes = planned_changes
            .iter()
            .any(|change| change.kind() == PlannedConnectionChangeKind::HookDefinition);
        let hook_state = if current.is_none() || hook_definition_changes {
            HookActivationState::ReviewRequiredBySetup
        } else {
            current
                .map(ConnectionVerificationReport::hook_activation_state)
                .unwrap_or(HookActivationState::Unknown)
        };
        let activation_plan =
            crate::connection_command::verification::activation_plan_for_checks_with_hook_state(
                &checks, hook_state,
            )?;
        Self::from_components(
            operation,
            true,
            runtime_home,
            connection,
            checks,
            activation_plan,
            Some(ConnectionCommandResult::Setup {
                disposition: SetupDisposition::Planned,
                setup_lease: SetupLeaseStatus::Acquired,
                runtime_home_publication: RuntimeHomePublicationStatus::NotPublished,
                runtime_home_rollback: None,
            }),
            Some(planned_changes),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::connection_command) fn mode_transition(
        runtime_home: &Path,
        connection: CommandConnection,
        changed: bool,
        previous_mode: String,
        current_mode: String,
        previous_integration_revision: String,
        current_integration_revision: String,
        rebound_guard_installation_ids: Vec<String>,
    ) -> Result<Self, ConnectionCommandError> {
        let checks = vec![command_check(
            ConnectionCheckKind::ModeTransition,
            ConnectionCheckStatus::Passed,
            "mode_transition_applied",
            if changed {
                "Connection mode transition was applied"
            } else {
                "Connection mode already matched the requested mode"
            },
            None,
        )?];
        let activation_state =
            derive_integration_activation_state(&checks, HookActivationState::Unknown);
        let required_steps = if changed {
            vec![ActivationStep::try_new(
                ActivationStepId::ReloadCodex,
                Vec::new(),
                format!(
                    "Restart or reload Codex, then use the current Volicord integration so new runtime and Guard observations bind revision {current_integration_revision}"
                ),
            )?]
        } else {
            Vec::new()
        };
        let activation_plan =
            IntegrationActivationPlan::try_new(activation_state, required_steps, Vec::new())?;
        Self::from_components(
            CommandOperation::Mode,
            false,
            runtime_home,
            connection,
            checks,
            activation_plan,
            Some(ConnectionCommandResult::ModeTransition {
                changed,
                previous_mode,
                current_mode,
                previous_integration_revision,
                current_integration_revision,
                rebound_guard_installation_ids,
            }),
            None,
        )
    }

    pub(in crate::connection_command) fn removal(
        runtime_home: &Path,
        connection: CommandConnection,
        membership_removed: bool,
        connection_removed: bool,
        remaining_project_count: usize,
    ) -> Result<Self, ConnectionCommandError> {
        Self::from_components(
            CommandOperation::Remove,
            false,
            runtime_home,
            connection,
            vec![command_check(
                ConnectionCheckKind::ConnectionRemoval,
                ConnectionCheckStatus::Passed,
                "connection_removal_applied",
                "Selected Connection membership removal was applied",
                None,
            )?],
            IntegrationActivationPlan::empty(IntegrationActivationState::Configured),
            Some(ConnectionCommandResult::Removal {
                membership_removed,
                connection_removed,
                remaining_project_count,
            }),
            None,
        )
    }

    pub(in crate::connection_command) fn removal_dry_run(
        runtime_home: &Path,
        connection: CommandConnection,
        planned_changes: Vec<PlannedConnectionChange>,
    ) -> Result<Self, ConnectionCommandError> {
        let has_changes = !planned_changes.is_empty();
        let checks = vec![command_check(
            ConnectionCheckKind::ConnectionRemoval,
            if has_changes {
                ConnectionCheckStatus::Pending
            } else {
                ConnectionCheckStatus::Passed
            },
            "connection_removal_planned",
            if has_changes {
                "Selected Connection membership removal is ready to apply"
            } else {
                "No Connection removal is required"
            },
            None,
        )?];
        Self::from_components(
            CommandOperation::Remove,
            true,
            runtime_home,
            connection,
            checks,
            IntegrationActivationPlan::empty(IntegrationActivationState::Configured),
            None,
            Some(planned_changes),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::connection_command) fn setup_failure(
        operation: CommandOperation,
        runtime_home: &Path,
        connection: CommandConnection,
        disposition: SetupDisposition,
        runtime_home_publication: RuntimeHomePublicationStatus,
        runtime_home_rollback: Option<RuntimeHomeRollbackResult>,
        diagnostic: SetupFailureDiagnostic,
        summary: &str,
        details: Value,
        activation_plan: IntegrationActivationPlan,
    ) -> Result<Self, ConnectionCommandError> {
        let finding_id = DiagnosticFindingId::parse(diagnostic.finding_id())
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        let diagnostic_action = if let Some(step) = activation_plan.required_steps().first() {
            DiagnosticAction::try_new(
                DiagnosticCode::parse(activation_step_code(step.id()))
                    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
                step.instruction(),
            )
        } else {
            DiagnosticAction::try_new(
                DiagnosticCode::parse("action.connection.retry_setup")
                    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
                "Resolve the typed setup failure and rerun the setup operation",
            )
        }
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        let check = command_check(
            ConnectionCheckKind::SetupPlan,
            ConnectionCheckStatus::Failed,
            diagnostic.check_reason(),
            summary,
            Some(details),
        )?
        .with_cause_finding_ids(vec![finding_id.clone()])
        .map_err(ConnectionCommandError::from)?;
        let connection_id = connection.id.clone();
        let mut report = Self::from_components(
            operation,
            false,
            runtime_home,
            connection,
            vec![check],
            activation_plan,
            Some(ConnectionCommandResult::Setup {
                disposition,
                setup_lease: SetupLeaseStatus::Acquired,
                runtime_home_publication,
                runtime_home_rollback,
            }),
            None,
        )?;
        let finding = DiagnosticFinding::try_new(
            finding_id,
            DiagnosticCode::parse(diagnostic.code())
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticDomain::parse("setup")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticStage::parse("apply")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("administrative_cli")
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticSubject::try_new("connection", &connection_id)
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            DiagnosticFacts::project(&SetupFailureDiagnosticFacts {
                summary,
                observation_state: "failed",
                expected: "committed setup transaction",
                actual: disposition.as_str(),
            })
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            report.generated_at.clone(),
        )
        .and_then(|finding| finding.with_actions(vec![diagnostic_action]))
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        report.findings.push(finding);
        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    fn from_components(
        operation: CommandOperation,
        dry_run: bool,
        runtime_home: &Path,
        connection: CommandConnection,
        checks: Vec<ConnectionCheck>,
        activation_plan: IntegrationActivationPlan,
        result: Option<ConnectionCommandResult>,
        planned_changes: Option<Vec<PlannedConnectionChange>>,
    ) -> Result<Self, ConnectionCommandError> {
        let canonical =
            ConnectionVerificationReport::try_new(current_timestamp(), checks, activation_plan)?;
        let status = command_status(&canonical);
        Ok(Self {
            operation,
            dry_run,
            status,
            activation_state: canonical.activation_state(),
            hook_activation_state: canonical.hook_activation_state(),
            runtime_home: path_text(runtime_home),
            connection,
            checks: canonical.checks().to_vec(),
            activation_plan: canonical.activation_plan().clone(),
            generated_at: canonical.checked_at().clone(),
            findings: Vec::new(),
            integration_revision: None,
            result,
            planned_changes,
            limits: cooperative_assurance_limits(),
        })
    }

    pub(super) const fn status(&self) -> ConnectionStatus {
        self.status
    }

    pub(in crate::connection_command) fn with_diagnostic_findings(
        mut self,
        findings: Vec<DiagnosticFinding>,
        integration_revision: Option<IntegrationRevision>,
    ) -> Self {
        self.findings = findings;
        self.integration_revision = integration_revision;
        self
    }

    pub(super) fn diagnostic_report(&self) -> Result<DiagnosticReport, ConnectionCommandError> {
        let runtime_sessions = self.role_bearing_runtime_sessions()?;
        let verification_ids = self.relevant_verification_ids();
        let mut runtime_session_ids = self
            .findings
            .iter()
            .filter_map(|finding| finding.runtime_session_id().cloned())
            .chain(runtime_sessions.iter().map(|session| session.id().clone()))
            .collect::<Vec<_>>();
        runtime_session_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        runtime_session_ids.dedup();
        let context = DiagnosticConnectionContext::try_new(
            self.runtime_home.clone(),
            self.connection.id.clone(),
            self.connection.host.clone(),
            self.connection.scope.clone(),
            self.connection.profile.clone(),
            self.connection.mode.clone(),
            Some(self.connection.repository.clone()),
            Some(self.connection.config_target.clone()),
            self.integration_revision.clone(),
            verification_ids,
            runtime_session_ids,
            runtime_sessions,
        )
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        DiagnosticReport::try_new(
            self.operation.diagnostic_operation(),
            self.status,
            self.activation_state,
            self.hook_activation_state,
            self.generated_at.clone(),
            Some(context),
            self.checks.clone(),
            self.findings.clone(),
            projected_activation_plan(self)?,
            Some(self.operation_details()?),
            self.limits.clone(),
        )
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
    }

    pub(super) fn role_bearing_runtime_sessions(
        &self,
    ) -> Result<Vec<DiagnosticRuntimeSessionContext>, ConnectionCommandError> {
        let mut by_id = BTreeMap::<String, BTreeSet<RuntimeSessionEvidenceRole>>::new();
        for check in &self.checks {
            let Some(details) = check.details().map(ConnectionCheckDetails::as_object) else {
                continue;
            };
            collect_runtime_session_evidence(details, &mut by_id)?;
            for key in ["latest_attempt", "latest_completed_proof"] {
                if let Some(nested) = details.get(key).and_then(Value::as_object) {
                    collect_runtime_session_evidence(nested, &mut by_id)?;
                }
            }
        }
        by_id
            .into_iter()
            .map(|(id, roles)| {
                DiagnosticRuntimeSessionContext::try_new(
                    volicord_types::ids::AgentRuntimeSessionId::new(id),
                    roles.into_iter().collect(),
                )
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
            })
            .collect()
    }

    pub(super) fn relevant_verification_ids(
        &self,
    ) -> Vec<volicord_types::ids::GuardIntegrationVerificationId> {
        let mut ids = BTreeSet::new();
        for check in &self.checks {
            let Some(details) = check.details().map(ConnectionCheckDetails::as_object) else {
                continue;
            };
            for key in ["latest_attempt", "latest_completed_proof"] {
                if let Some(id) = details
                    .get(key)
                    .and_then(Value::as_object)
                    .and_then(|evidence| evidence.get("verification_id"))
                    .and_then(Value::as_str)
                {
                    ids.insert(id.to_owned());
                }
            }
        }
        ids.into_iter()
            .map(volicord_types::ids::GuardIntegrationVerificationId::new)
            .collect()
    }

    fn operation_details(&self) -> Result<serde_json::Map<String, Value>, ConnectionCommandError> {
        let mut details = serde_json::Map::new();
        details.insert("dry_run".to_owned(), Value::Bool(self.dry_run));
        if self.operation == CommandOperation::Verify {
            details.insert(
                "evidence_class".to_owned(),
                Value::String("active_verification".to_owned()),
            );
            details.insert(
                "side_effects".to_owned(),
                serde_json::json!([
                    "rollback_only_store_writeability_probes",
                    "disposable_protocol_conformance",
                    "diagnostic_reconciliation",
                    "verification_report_persistence"
                ]),
            );
            details.insert(
                "does_not_prove".to_owned(),
                serde_json::json!([
                    "managed_host_operation",
                    "future_launch_availability",
                    "product_repository_correctness_outside_checked_contracts"
                ]),
            );
        }
        if let Some(result) = self.result.as_ref() {
            details.insert(
                "result".to_owned(),
                serde_json::to_value(result)
                    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            );
        }
        if let Some(planned_changes) = self.planned_changes.as_ref() {
            details.insert(
                "planned_changes".to_owned(),
                serde_json::to_value(planned_changes)
                    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
            );
        }
        Ok(details)
    }
}

fn collect_runtime_session_evidence(
    details: &serde_json::Map<String, Value>,
    by_id: &mut BTreeMap<String, BTreeSet<RuntimeSessionEvidenceRole>>,
) -> Result<(), ConnectionCommandError> {
    let role = match details.get("evidence_role").and_then(Value::as_str) {
        Some("latest_managed_attempt") => Some(RuntimeSessionEvidenceRole::LatestManagedAttempt),
        Some("latest_managed_capability_proof") => {
            Some(RuntimeSessionEvidenceRole::LatestManagedCapabilityProof)
        }
        Some("guard_verification_attempt") => {
            Some(RuntimeSessionEvidenceRole::GuardVerificationAttempt)
        }
        Some("guard_verification_proof") => {
            Some(RuntimeSessionEvidenceRole::GuardVerificationProof)
        }
        Some(value) => {
            return Err(ConnectionCommandError::runtime(format!(
                "connection check contains unknown runtime-session evidence role: {value}"
            )))
        }
        None => None,
    };
    if let (Some(role), Some(runtime_session_id)) = (
        role,
        details.get("runtime_session_id").and_then(Value::as_str),
    ) {
        by_id
            .entry(runtime_session_id.to_owned())
            .or_default()
            .insert(role);
    }
    Ok(())
}

#[derive(Serialize)]
struct SetupFailureDiagnosticFacts<'a> {
    summary: &'a str,
    observation_state: &'static str,
    expected: &'static str,
    actual: &'static str,
}

impl DiagnosticFactSource for SetupFailureDiagnosticFacts<'_> {}

pub(super) fn projected_activation_plan(
    report: &ConnectionCommandReport,
) -> Result<IntegrationActivationPlan, ConnectionCommandError> {
    let roots = projected_root_cause_ids(report)?;
    let root_set = roots.iter().collect::<BTreeSet<_>>();
    let mut required_steps = report
        .activation_plan
        .required_steps()
        .iter()
        .cloned()
        .map(|step| (step.id(), step))
        .collect::<BTreeMap<_, _>>();
    for finding in &report.findings {
        if !root_set.contains(finding.id()) {
            continue;
        }
        for finding_action in finding.actions() {
            let Some(id) = diagnostic_activation_step_id(finding_action.code().as_str()) else {
                continue;
            };
            if required_steps.contains_key(&id) {
                continue;
            }
            let step = ActivationStep::try_new(id, Vec::new(), finding_action.summary())?
                .with_root_finding_ids(vec![finding.id().clone()])?;
            required_steps.insert(id, step);
        }
    }
    let required_steps = required_steps
        .into_values()
        .map(|step| {
            if step.root_finding_ids().is_empty() && !roots.is_empty() {
                step.with_root_finding_ids(roots.clone())
            } else {
                Ok(step)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    IntegrationActivationPlan::try_new(
        report.activation_plan.state(),
        required_steps,
        report.activation_plan.optional_diagnostics().to_vec(),
    )
    .map_err(ConnectionCommandError::from)
}

fn diagnostic_activation_step_id(code: &str) -> Option<ActivationStepId> {
    if code == "action.host.reload_after_configuration_change" {
        Some(ActivationStepId::ReloadCodex)
    } else if code.starts_with("action.host.") {
        Some(ActivationStepId::ReadConnectionStatus)
    } else if code.starts_with("action.managed_config.")
        || code.starts_with("action.storage.")
        || code.starts_with("action.store.")
        || code.starts_with("action.runtime_home.")
    {
        Some(ActivationStepId::RepairManagedConfiguration)
    } else if matches!(
        code,
        "action.guard.trigger_phase" | "action.guard.retry_verification"
    ) {
        Some(ActivationStepId::RequestIntegrationVerification)
    } else if code.starts_with("action.guard.") {
        Some(ActivationStepId::RepairHookContract)
    } else if code.starts_with("action.process.")
        || code.starts_with("action.mcp.")
        || code.starts_with("action.protocol.")
    {
        Some(ActivationStepId::ReadConnectionStatus)
    } else if code.starts_with("action.internal.")
        || code.starts_with("action.connection.reinstall")
        || code.starts_with("action.installation.")
    {
        Some(ActivationStepId::RepairManagedConfiguration)
    } else {
        None
    }
}

pub(super) fn projected_root_cause_ids(
    report: &ConnectionCommandReport,
) -> Result<Vec<volicord_types::diagnostics::DiagnosticFindingId>, ConnectionCommandError> {
    let selected = report
        .checks
        .iter()
        .filter(|check| {
            matches!(
                check.status(),
                ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
            )
        })
        .flat_map(|check| check.cause_finding_ids().iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    diagnostic_root_cause_ids(
        &report.findings,
        &selected.into_iter().collect::<Vec<_>>(),
        MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

pub(super) fn projected_check_root_cause_ids(
    report: &ConnectionCommandReport,
    check: &ConnectionCheck,
) -> Result<Vec<volicord_types::diagnostics::DiagnosticFindingId>, ConnectionCommandError> {
    if check.cause_finding_ids().is_empty() {
        return Ok(Vec::new());
    }
    diagnostic_root_cause_ids(
        &report.findings,
        check.cause_finding_ids(),
        MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

const fn activation_step_code(id: ActivationStepId) -> &'static str {
    match id {
        ActivationStepId::ReloadCodex => "action.host.reload_after_configuration_change",
        ActivationStepId::ReviewProjectHooks => "action.host.review_hooks",
        ActivationStepId::RequestIntegrationVerification => {
            "action.mcp.request_integration_verification"
        }
        ActivationStepId::ReadConnectionStatus => "action.connection.read_status",
        ActivationStepId::RunOptionalActiveDiagnostics => {
            "action.connection.run_optional_active_diagnostics"
        }
        ActivationStepId::RepairHookContract => "action.guard.repair_hook_contract",
        ActivationStepId::RepairManagedConfiguration => "action.managed_config.repair",
    }
}

pub(in crate::connection_command) struct RenderedCommandReport {
    pub(in crate::connection_command) output: String,
    pub(in crate::connection_command) status: ConnectionStatus,
}

pub(in crate::connection_command) fn render_command_report(
    format: OutputFormat,
    report: &ConnectionCommandReport,
) -> Result<RenderedCommandReport, ConnectionCommandError> {
    let active_verification = active_verification_snapshot(&report.checks)?;
    let output = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&report.diagnostic_report()?)
            .map(|output| format!("{output}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        OutputFormat::Human(HumanOutputDetail::Concise) => {
            render_command_report_concise(report, active_verification.as_ref())?
        }
        OutputFormat::Human(HumanOutputDetail::Verbose) => {
            render_command_report_verbose(report, active_verification.as_ref())?
        }
    };
    Ok(RenderedCommandReport {
        output,
        status: report.status(),
    })
}

/// Renders a canonical typed failure when setup cannot acquire its lease.
pub(in crate::connection_command) fn render_setup_lease_busy(
    format: OutputFormat,
    operation: CommandOperation,
    busy: &RuntimeHomeMutationBusy,
    dry_run: bool,
) -> Result<RenderedCommandReport, ConnectionCommandError> {
    if busy.requested_mode() != RuntimeHomeMutationLeaseMode::ExclusiveSetup {
        return Err(ConnectionCommandError::runtime(
            "setup busy rendering requires an exclusive Runtime Home mutation request",
        ));
    }
    let generated_at = current_timestamp();
    let finding_id = DiagnosticFindingId::parse("finding.setup.lease_busy")
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let action = DiagnosticAction::try_new(
        DiagnosticCode::parse("action.setup.wait_for_current_transaction")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        "Wait for the other setup invocation to finish, then rerun this setup operation",
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let elapsed_millis = u64::try_from(busy.elapsed().as_millis()).unwrap_or(u64::MAX);
    let details = serde_json::json!({
        "dry_run": dry_run,
        "setup_lease": {
            "outcome": "busy",
            "canonical_runtime_home": path_text(busy.target().as_path()),
            "requested_operation": operation.as_str(),
            "wait_policy": busy.wait_policy().as_str(),
            "elapsed_millis": elapsed_millis,
            "owner_observation": "another_setup_transaction",
        },
        "retry": "after_current_setup_finishes",
    });
    let check = command_check(
        ConnectionCheckKind::SetupPlan,
        ConnectionCheckStatus::Failed,
        "setup_lease_busy",
        "Another setup transaction currently owns the Runtime Home setup lease",
        Some(details.clone()),
    )?
    .with_cause_finding_ids(vec![finding_id.clone()])
    .map_err(ConnectionCommandError::from)?;
    let finding = DiagnosticFinding::try_new(
        finding_id,
        DiagnosticCode::parse("setup.lease_busy")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticDomain::parse("setup")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticStage::parse("lease_acquisition")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("administrative_cli")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticSubject::try_new("runtime_home", path_text(busy.target().as_path()))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticFacts::project(&SetupLeaseBusyDiagnosticFacts {
            outcome: "busy",
            requested_operation: operation.as_str(),
            wait_policy: busy.wait_policy().as_str(),
            elapsed_millis,
            owner_observation: "another_setup_transaction",
        })
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        generated_at.clone(),
    )
    .and_then(|finding| finding.with_actions(vec![action]))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let activation_plan = IntegrationActivationPlan::empty(IntegrationActivationState::Failed);
    let report = DiagnosticReport::try_new(
        operation.diagnostic_operation(),
        ConnectionStatus::Failed,
        IntegrationActivationState::Failed,
        HookActivationState::Unknown,
        generated_at,
        None,
        vec![check],
        vec![finding],
        activation_plan,
        Some(
            details
                .as_object()
                .cloned()
                .expect("setup lease busy details are an object"),
        ),
        cooperative_assurance_limits(),
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let output = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&report)
            .map(|output| format!("{output}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        OutputFormat::Human(HumanOutputDetail::Concise) => format!(
            "Setup is busy for Runtime Home {}.\nAnother setup transaction currently owns the setup lease. Wait for it to finish, then retry this operation.\n",
            busy.target().as_path().display()
        ),
        OutputFormat::Human(HumanOutputDetail::Verbose) => format!(
            "Setup lease\n  Outcome: busy\n  Runtime Home: {}\n  Requested operation: {}\n  Wait policy: {}\n  Elapsed: {} ms\n  Action: wait for the other setup invocation to finish, then retry; do not delete coordination files\n",
            busy.target().as_path().display(),
            operation.as_str(),
            busy.wait_policy().as_str(),
            elapsed_millis
        ),
    };
    Ok(RenderedCommandReport {
        output,
        status: ConnectionStatus::Failed,
    })
}

#[derive(Serialize)]
struct SetupLeaseBusyDiagnosticFacts {
    outcome: &'static str,
    requested_operation: &'static str,
    wait_policy: &'static str,
    elapsed_millis: u64,
    owner_observation: &'static str,
}

impl DiagnosticFactSource for SetupLeaseBusyDiagnosticFacts {}

fn command_status(report: &ConnectionVerificationReport) -> ConnectionStatus {
    if report.status() == ConnectionStatus::Complete
        && !report.activation_plan().required_steps().is_empty()
    {
        ConnectionStatus::ActionRequired
    } else {
        report.status()
    }
}

fn command_check(
    id: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    details: Option<Value>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let details = details
        .map(|details| {
            let Value::Object(details) = details else {
                return Err(ConnectionCommandError::runtime(
                    "command report check details must be an object",
                ));
            };
            ConnectionCheckDetails::try_new(details).map_err(ConnectionCommandError::from)
        })
        .transpose()?;
    ConnectionCheck::try_new(
        id,
        status,
        Vec::new(),
        (status != ConnectionCheckStatus::Passed).then(|| code.to_owned()),
        summary,
        details,
        None,
    )
    .map_err(ConnectionCommandError::from)
}

fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use crate::connection_command::planning::PlannedChangeOperation;

    use super::*;

    fn verification(status: ConnectionCheckStatus) -> ConnectionVerificationReport {
        let checks = vec![ConnectionCheck::try_new(
            ConnectionCheckKind::ManagedConfig,
            status,
            Vec::new(),
            (status != ConnectionCheckStatus::Passed).then(|| "managed_config_failed".to_owned()),
            "Managed configuration check",
            None,
            None,
        )
        .unwrap()];
        let activation_plan = IntegrationActivationPlan::empty(
            derive_integration_activation_state(&checks, HookActivationState::Unknown),
        );
        ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            checks,
            activation_plan,
        )
        .unwrap()
    }

    fn connection() -> CommandConnection {
        CommandConnection::new(
            "connection_1",
            "codex",
            "user",
            "workflow",
            Path::new("/workspace/product"),
            "/home/user/.codex/config.toml",
        )
    }

    fn diagnostic_value(report: &ConnectionCommandReport) -> Value {
        serde_json::to_value(report.diagnostic_report().unwrap()).unwrap()
    }

    fn assert_top_level_keys(value: &Value) {
        let expected = BTreeSet::from([
            "activation_plan",
            "activation_state",
            "checks",
            "connection",
            "findings",
            "generated_at",
            "hook_activation_state",
            "limits",
            "operation",
            "operation_details",
            "root_cause_ids",
            "schema_version",
            "status",
        ]);
        assert_eq!(
            value
                .as_object()
                .expect("command report object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected
        );
    }

    #[test]
    fn setup_lease_busy_renderers_are_typed_and_never_recommend_file_deletion() {
        use volicord_platform_fs::{
            RuntimeHomeMutationLease, RuntimeHomeMutationLeaseOutcome,
            RuntimeHomeMutationWaitPolicy,
        };

        let fixture = tempfile::tempdir().unwrap();
        let runtime_home = fixture.path().join("runtime-home");
        let RuntimeHomeMutationLeaseOutcome::Acquired(_lease) = RuntimeHomeMutationLease::acquire(
            &runtime_home,
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
            RuntimeHomeMutationWaitPolicy::Immediate,
        )
        .unwrap() else {
            panic!("first setup lease should be acquired");
        };
        let RuntimeHomeMutationLeaseOutcome::Busy(busy) = RuntimeHomeMutationLease::acquire(
            &runtime_home,
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
            RuntimeHomeMutationWaitPolicy::Immediate,
        )
        .unwrap() else {
            panic!("second setup lease should be busy");
        };

        for operation in [CommandOperation::Init, CommandOperation::Add] {
            let json = render_setup_lease_busy(OutputFormat::Json, operation, &busy, true).unwrap();
            assert_eq!(json.status, ConnectionStatus::Failed);
            let value: Value = serde_json::from_str(&json.output).unwrap();
            assert_eq!(value["operation"], operation.as_str());
            assert_eq!(value["operation_details"]["dry_run"], true);
            assert_eq!(value["operation_details"]["setup_lease"]["outcome"], "busy");
            assert_eq!(
                value["operation_details"]["setup_lease"]["requested_operation"],
                operation.as_str()
            );
            assert_eq!(value["checks"][0]["code"], "setup_lease_busy");
            assert_eq!(value["findings"][0]["code"], "setup.lease_busy");
            assert_eq!(
                value["findings"][0]["actions"][0]["code"],
                "action.setup.wait_for_current_transaction"
            );
        }

        for format in [
            OutputFormat::Human(HumanOutputDetail::Concise),
            OutputFormat::Human(HumanOutputDetail::Verbose),
        ] {
            let rendered =
                render_setup_lease_busy(format, CommandOperation::Init, &busy, false).unwrap();
            assert_eq!(rendered.status, ConnectionStatus::Failed);
            assert!(rendered.output.to_ascii_lowercase().contains("wait"));
            assert!(!rendered.output.contains(".lock"));
            assert!(!rendered.output.contains("remove"));
        }
    }

    #[test]
    fn every_operation_uses_the_same_exact_top_level_shape() {
        for operation in [
            CommandOperation::Init,
            CommandOperation::Add,
            CommandOperation::Status,
            CommandOperation::Verify,
        ] {
            let report = ConnectionCommandReport::from_verification(
                operation,
                matches!(operation, CommandOperation::Init | CommandOperation::Add)
                    .then_some(SetupDisposition::Committed),
                Path::new("/runtime"),
                connection(),
                &verification(ConnectionCheckStatus::Passed),
            );
            let value = diagnostic_value(&report);
            assert_top_level_keys(&value);
            assert_eq!(value["schema_version"], 2);
            assert_eq!(value["operation"], operation.as_str());
            if operation == CommandOperation::Verify {
                assert_eq!(
                    value["operation_details"]["evidence_class"],
                    "active_verification"
                );
                assert_eq!(
                    value["operation_details"]["side_effects"],
                    json!([
                        "rollback_only_store_writeability_probes",
                        "disposable_protocol_conformance",
                        "diagnostic_reconciliation",
                        "verification_report_persistence"
                    ])
                );
            }
            assert_eq!(value["status"], "complete");
            assert_eq!(value["checks"].as_array().map(Vec::len), Some(1));
            assert_eq!(value["activation_plan"]["required_steps"], json!([]));
            if matches!(operation, CommandOperation::Init | CommandOperation::Add) {
                assert_eq!(
                    value["operation_details"]["result"],
                    json!({
                        "kind": "setup",
                        "disposition": "committed",
                        "setup_lease": "acquired",
                        "runtime_home_publication": "existing_ready"
                    })
                );
            } else {
                assert!(value["operation_details"].get("result").is_none());
            }
        }

        let mode_report = ConnectionCommandReport::mode_transition(
            Path::new("/runtime"),
            connection(),
            false,
            "workflow".to_owned(),
            "workflow".to_owned(),
            "revision_1".to_owned(),
            "revision_1".to_owned(),
            Vec::new(),
        )
        .unwrap();
        let mode = diagnostic_value(&mode_report);
        assert_top_level_keys(&mode);
        assert_eq!(mode["operation"], "mode");
        assert_eq!(mode["status"], "complete");
        assert_eq!(
            mode["operation_details"]["result"],
            json!({
                "kind": "mode_transition",
                "changed": false,
                "previous_mode": "workflow",
                "current_mode": "workflow",
                "previous_integration_revision": "revision_1",
                "current_integration_revision": "revision_1",
                "rebound_guard_installation_ids": [],
            })
        );

        let removal_report =
            ConnectionCommandReport::removal(Path::new("/runtime"), connection(), true, false, 1)
                .unwrap();
        let removal = diagnostic_value(&removal_report);
        assert_top_level_keys(&removal);
        assert_eq!(removal["operation"], "remove");
        assert_eq!(removal["status"], "complete");
        assert_eq!(
            removal["operation_details"]["result"],
            json!({
                "kind": "removal",
                "membership_removed": true,
                "connection_removed": false,
                "remaining_project_count": 1,
            })
        );
    }

    #[test]
    fn setup_results_serialize_every_publication_ownership_state() {
        for (status, expected) in [
            (RuntimeHomePublicationStatus::NotPublished, "not_published"),
            (
                RuntimeHomePublicationStatus::ExistingReady,
                "existing_ready",
            ),
            (
                RuntimeHomePublicationStatus::PublishedByThisInvocation,
                "published_by_this_invocation",
            ),
            (
                RuntimeHomePublicationStatus::OwnedPublicationRolledBack,
                "owned_publication_rolled_back",
            ),
            (
                RuntimeHomePublicationStatus::OwnedPublicationRemovalIncomplete,
                "owned_publication_removal_incomplete",
            ),
            (
                RuntimeHomePublicationStatus::OwnedPublicationPreserved,
                "owned_publication_preserved",
            ),
            (
                RuntimeHomePublicationStatus::OwnershipLostDuringRollback,
                "ownership_lost_during_rollback",
            ),
        ] {
            let report = ConnectionCommandReport::from_verification_with_publication(
                CommandOperation::Init,
                Some(SetupDisposition::Committed),
                Some(status),
                Path::new("/runtime"),
                connection(),
                &verification(ConnectionCheckStatus::Passed),
            );
            let value = diagnostic_value(&report);
            assert_eq!(
                value["operation_details"]["result"]["runtime_home_publication"],
                expected
            );
            assert_eq!(status.as_str(), expected);
        }
    }

    #[test]
    fn setup_results_serialize_typed_runtime_home_rollback_effects() {
        for (status, rollback, expected) in [
            (
                RuntimeHomePublicationStatus::OwnedPublicationRolledBack,
                RuntimeHomeRollbackResult::Removed {
                    durability: DirectoryEntryDurability::ParentSynchronized,
                    failure_phase: None,
                },
                json!({
                    "outcome": "removed",
                    "durability": "parent_synchronized",
                }),
            ),
            (
                RuntimeHomePublicationStatus::OwnedPublicationRolledBack,
                RuntimeHomeRollbackResult::Removed {
                    durability: DirectoryEntryDurability::ParentSynchronizationFailed,
                    failure_phase: Some(DirectoryTreeRemovalPhase::ParentDirectorySynchronization),
                },
                json!({
                    "outcome": "removed",
                    "durability": "parent_synchronization_failed",
                    "failure_phase": "parent_directory_synchronization",
                }),
            ),
            (
                RuntimeHomePublicationStatus::OwnedPublicationRemovalIncomplete,
                RuntimeHomeRollbackResult::RemovalIncomplete {
                    effect: DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
                    phase: DirectoryTreeRemovalPhase::PostRemovalInspection,
                    final_path: DirectoryTreeTargetState::Unknown,
                },
                json!({
                    "outcome": "removal_incomplete",
                    "effect": "partially_removed_or_unknown",
                    "phase": "post_removal_inspection",
                    "final_path": "unknown",
                }),
            ),
            (
                RuntimeHomePublicationStatus::OwnedPublicationPreserved,
                RuntimeHomeRollbackResult::Preserved {
                    reason: "setup_policy".to_owned(),
                },
                json!({
                    "outcome": "preserved",
                    "reason": "setup_policy",
                }),
            ),
            (
                RuntimeHomePublicationStatus::OwnershipLostDuringRollback,
                RuntimeHomeRollbackResult::OwnershipLost {
                    reason: "final_path_missing".to_owned(),
                },
                json!({
                    "outcome": "ownership_lost",
                    "reason": "final_path_missing",
                }),
            ),
        ] {
            let report = ConnectionCommandReport::setup_failure(
                CommandOperation::Init,
                Path::new("/runtime"),
                connection(),
                SetupDisposition::PartiallyRolledBack,
                status,
                Some(rollback),
                SetupFailureDiagnostic::PartialRollback,
                "Setup rollback retained typed Runtime Home facts",
                json!({"retryable": true}),
                IntegrationActivationPlan::empty(IntegrationActivationState::Failed),
            )
            .expect("setup failure report");

            let value = diagnostic_value(&report);
            assert_eq!(
                value["operation_details"]["result"]["runtime_home_rollback"],
                expected
            );
        }
    }

    #[test]
    fn dry_run_and_mode_status_come_from_typed_checks_and_activation_plan() {
        let activation_plan = IntegrationActivationPlan::try_new(
            IntegrationActivationState::HostReloadRequired,
            vec![ActivationStep::try_new(
                ActivationStepId::ReloadCodex,
                Vec::new(),
                "Reload Codex",
            )
            .unwrap()],
            Vec::new(),
        )
        .unwrap();
        let action_only_verification = ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            verification(ConnectionCheckStatus::Passed)
                .checks()
                .to_vec(),
            activation_plan,
        )
        .unwrap();
        let action_only = ConnectionCommandReport::from_verification(
            CommandOperation::Verify,
            None,
            Path::new("/runtime"),
            connection(),
            &action_only_verification,
        );
        assert_eq!(action_only.status(), ConnectionStatus::ActionRequired);

        let changed = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            vec![PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ManagedHostConfiguration,
                PlannedChangeOperation::Update,
                "/home/user/.codex/config.toml",
            )],
        )
        .unwrap();
        let changed = diagnostic_value(&changed);
        assert_top_level_keys(&changed);
        assert_eq!(changed["operation"], "add");
        assert_eq!(changed["status"], "action_required");
        assert_eq!(
            changed["operation_details"]["result"],
            json!({
                "kind": "setup",
                "disposition": "planned",
                "setup_lease": "acquired",
                "runtime_home_publication": "not_published"
            })
        );

        let mode = ConnectionCommandReport::mode_transition(
            Path::new("/runtime"),
            connection(),
            true,
            "workflow".to_owned(),
            "read_only".to_owned(),
            "before".to_owned(),
            "after".to_owned(),
            vec!["guard_1".to_owned()],
        )
        .unwrap();
        let mode = diagnostic_value(&mode);
        assert_top_level_keys(&mode);
        assert_eq!(mode["status"], "action_required");
        assert_eq!(mode["checks"][0]["status"], "passed");
        assert_eq!(
            mode["activation_plan"]["required_steps"][0]["id"],
            "reload_codex"
        );
        assert_eq!(
            mode["activation_plan"]["required_steps"][0]["initiator"],
            "user"
        );
        assert_eq!(
            mode["activation_plan"]["required_steps"][0]["executor"],
            "host"
        );
        assert_eq!(
            mode["activation_plan"]["required_steps"][0]["execution_channel"],
            "codex_ui"
        );
        assert_eq!(mode["operation_details"]["result"]["changed"], true);

        let removal = ConnectionCommandReport::removal_dry_run(
            Path::new("/runtime"),
            connection(),
            vec![PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ConnectionMembership,
                PlannedChangeOperation::Remove,
                "connection_1:project_1",
            )],
        )
        .unwrap();
        let removal = diagnostic_value(&removal);
        assert_top_level_keys(&removal);
        assert_eq!(removal["operation"], "remove");
        assert_eq!(removal["status"], "action_required");
        assert_eq!(removal["checks"][0]["status"], "pending");
        assert_eq!(removal["activation_plan"]["required_steps"], json!([]));
        assert!(removal["operation_details"].get("result").is_none());
    }

    #[test]
    fn setup_dry_run_builds_the_shared_semantic_activation_plan() {
        let report = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            Vec::new(),
        )
        .unwrap();
        let step = report
            .activation_plan
            .required_steps()
            .iter()
            .find(|step| step.id() == ActivationStepId::RequestIntegrationVerification)
            .expect("shared verification step");
        assert_eq!(step.initiator(), ActivationActor::User);
        assert_eq!(step.executor(), ActivationActor::Agent);
        assert_eq!(
            step.execution_channel(),
            ActivationExecutionChannel::CodexChat
        );
        assert_eq!(
            step.agent_sequence()
                .iter()
                .map(AgentSequenceStep::tool)
                .collect::<Vec<_>>(),
            vec![
                AgentToolId::LIST_PROJECTS,
                AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
                AgentToolId::GUARD_PROBE,
                AgentToolId::GET_INTEGRATION_VERIFICATION,
            ]
        );
        assert_eq!(
            serde_json::to_value(step).unwrap()["id"],
            json!("request_integration_verification")
        );
        assert_eq!(
            report
                .activation_plan
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
    fn json_and_verbose_human_render_the_same_typed_status_and_activation_plan() {
        let report = ConnectionCommandReport::from_verification(
            CommandOperation::Verify,
            None,
            Path::new("/runtime"),
            connection(),
            &verification(ConnectionCheckStatus::Failed),
        );
        let json = render_command_report(OutputFormat::Json, &report).unwrap();
        let text = render_command_report(OutputFormat::Human(HumanOutputDetail::Verbose), &report)
            .unwrap();
        assert_eq!(json.status, ConnectionStatus::Failed);
        assert_eq!(text.status, ConnectionStatus::Failed);
        assert_eq!(
            serde_json::from_str::<Value>(&json.output).unwrap()["status"],
            "failed"
        );
        assert!(text.output.contains("  Activation: failed\n"));
        assert!(text.output.contains("  Hook activation: unknown\n"));
        assert!(text.output.contains("[failed] Managed Codex configuration"));
    }
}
