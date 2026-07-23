use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    str::FromStr,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use volicord_types::{
    diagnostic_root_cause_ids, ConnectionAction, ConnectionActionKind, ConnectionActivationState,
    ConnectionCheck, ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus,
    ConnectionStatus, ConnectionVerificationReport, DiagnosticAction, DiagnosticCode,
    DiagnosticConnectionContext, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts,
    DiagnosticFinding, DiagnosticFindingId, DiagnosticOperation, DiagnosticReport,
    DiagnosticReportAction, DiagnosticRuntimeSessionContext, DiagnosticSeverity, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject, HookActivationState, IntegrationProfile,
    IntegrationRevision, RuntimeSessionEvidenceRole, UtcTimestamp,
    MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
};

use super::{
    cooperative_assurance_limits, human::render_command_report_concise, path_text,
    verbose::render_command_report_verbose, ConnectionCommandError, OutputFormat,
    PlannedConnectionChange, PlannedConnectionChangeKind,
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
    #[cfg(test)]
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
        applied: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub(in crate::connection_command) struct ConnectionCommandReport {
    pub(super) operation: CommandOperation,
    pub(super) dry_run: bool,
    pub(super) status: ConnectionStatus,
    pub(super) activation_state: ConnectionActivationState,
    pub(super) hook_activation_state: HookActivationState,
    pub(super) runtime_home: String,
    pub(super) connection: CommandConnection,
    pub(super) checks: Vec<ConnectionCheck>,
    pub(super) actions: Vec<ConnectionAction>,
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
        setup_result: Option<bool>,
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
            actions: verification.actions().to_vec(),
            generated_at: verification.checked_at().clone(),
            findings: Vec::new(),
            integration_revision: None,
            result: setup_result.map(|applied| ConnectionCommandResult::Setup { applied }),
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
        plan_actions: &[ConnectionAction],
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
                    ConnectionCheckKind::GuardHookExecution,
                    ConnectionCheckStatus::Pending,
                    "guard_hook_execution_pending",
                    "A current managed Guard hook has not executed",
                    None,
                )?,
                command_check(
                    ConnectionCheckKind::GuardVerification,
                    ConnectionCheckStatus::Pending,
                    "guard_verification_pending",
                    "In-chat MCP and Guard integration verification has not completed",
                    None,
                )?,
            ]);
        }

        let mut actions = plan_actions.to_vec();
        if let Some(current) = current {
            actions.extend(current.actions().iter().cloned());
        }
        let hook_definition_changes = planned_changes
            .iter()
            .any(|change| change.kind() == PlannedConnectionChangeKind::HookDefinition);
        if current.is_none() || hook_definition_changes {
            for action in [
                ConnectionAction::try_new(
                    ConnectionActionKind::ReloadHost,
                    "After setup is applied, restart or reload Codex in this repository",
                )?,
                ConnectionAction::try_new(
                    ConnectionActionKind::ReviewHooks,
                    "Review the current project hook definition in the Codex hook UI or with `/hooks`",
                )?,
            ] {
                if !actions.iter().any(|current| current.id() == action.id()) {
                    actions.push(action);
                }
            }
        }
        if current.is_none() {
            for action in [
                ConnectionAction::try_new(
                    ConnectionActionKind::RunMcpVerification,
                    "Start a new conversation and request `Run the Volicord integration verification.`",
                )?,
                ConnectionAction::try_new(
                    ConnectionActionKind::RunGuardProbe,
                    "Call the returned Guard probe and then read the integration-verification result",
                )?,
            ] {
                if !actions.iter().any(|current| current.id() == action.id()) {
                    actions.push(action);
                }
            }
        }
        Self::from_components(
            operation,
            true,
            runtime_home,
            connection,
            checks,
            actions,
            Some(ConnectionCommandResult::Setup { applied: false }),
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
        let actions = if changed {
            vec![ConnectionAction::try_new(
                ConnectionActionKind::ReloadHost,
                format!(
                    "Restart or reload Codex, then use the current Volicord integration so new runtime and Guard observations bind revision {current_integration_revision}"
                ),
            )?]
        } else {
            Vec::new()
        };
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
        Self::from_components(
            CommandOperation::Mode,
            false,
            runtime_home,
            connection,
            checks,
            actions,
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
            Vec::new(),
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
        let actions = Vec::new();
        Self::from_components(
            CommandOperation::Remove,
            true,
            runtime_home,
            connection,
            checks,
            actions,
            None,
            Some(planned_changes),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::connection_command) fn setup_failure(
        operation: CommandOperation,
        runtime_home: &Path,
        connection: CommandConnection,
        summary: &str,
        details: Value,
        actions: Vec<ConnectionAction>,
    ) -> Result<Self, ConnectionCommandError> {
        let finding_id = DiagnosticFindingId::parse("finding.setup.partial_application")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        let diagnostic_action = if let Some(action) = actions.first() {
            DiagnosticAction::try_new(
                DiagnosticCode::parse(connection_action_code(action.id()))
                    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
                action.instruction(),
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
            "setup_partial_application",
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
            actions,
            Some(ConnectionCommandResult::Setup { applied: false }),
            None,
        )?;
        let finding = DiagnosticFinding::try_new(
            finding_id,
            DiagnosticCode::parse("setup.partial_application")
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
                expected: "complete setup application",
                actual: "partial setup application",
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
        actions: Vec<ConnectionAction>,
        result: Option<ConnectionCommandResult>,
        planned_changes: Option<Vec<PlannedConnectionChange>>,
    ) -> Result<Self, ConnectionCommandError> {
        let canonical =
            ConnectionVerificationReport::try_new(current_timestamp(), checks, actions)?;
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
            actions: canonical.actions().to_vec(),
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
            projected_actions(self)?,
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
            let role = match details.get("evidence_role").and_then(Value::as_str) {
                Some("latest_attempt") => Some(RuntimeSessionEvidenceRole::LatestAttempt),
                Some("latest_complete_proof") => {
                    Some(RuntimeSessionEvidenceRole::LatestCompleteProof)
                }
                Some(value) => {
                    return Err(ConnectionCommandError::runtime(format!(
                        "connection check contains unknown runtime-session evidence role: {value}"
                    )))
                }
                None => None,
            };
            let runtime_session_id = details.get("runtime_session_id").and_then(Value::as_str);
            match (role, runtime_session_id) {
                (Some(role), Some(runtime_session_id)) => {
                    by_id
                        .entry(runtime_session_id.to_owned())
                        .or_default()
                        .insert(role);
                }
                (None, None) | (Some(_), None) => {}
                (None, Some(_)) => {}
            }
        }
        by_id
            .into_iter()
            .map(|(id, roles)| {
                DiagnosticRuntimeSessionContext::try_new(
                    volicord_types::AgentRuntimeSessionId::new(id),
                    roles.into_iter().collect(),
                )
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
            })
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

#[derive(Serialize)]
struct SetupFailureDiagnosticFacts<'a> {
    summary: &'a str,
    observation_state: &'static str,
    expected: &'static str,
    actual: &'static str,
}

impl DiagnosticFactSource for SetupFailureDiagnosticFacts<'_> {}

pub(super) fn projected_actions(
    report: &ConnectionCommandReport,
) -> Result<Vec<DiagnosticReportAction>, ConnectionCommandError> {
    let roots = projected_root_cause_ids(report)?;
    let root_set = roots.iter().collect::<BTreeSet<_>>();
    let mut actions = report
        .actions
        .iter()
        .cloned()
        .map(|action| (action.id(), action))
        .collect::<BTreeMap<_, _>>();
    for finding in &report.findings {
        if !root_set.contains(finding.id()) {
            continue;
        }
        for finding_action in finding.actions() {
            let Some(kind) = diagnostic_action_kind(finding_action.code().as_str()) else {
                continue;
            };
            if actions.contains_key(&kind) {
                continue;
            }
            let action = ConnectionAction::try_new(kind, finding_action.summary())?
                .with_root_finding_ids(vec![finding.id().clone()])?;
            actions.insert(kind, action);
        }
    }
    actions
        .values()
        .map(|action| {
            let action_roots = if action.root_finding_ids().is_empty() {
                roots.clone()
            } else {
                action.root_finding_ids().to_vec()
            };
            DiagnosticReportAction::from_connection_action(action, action_roots)
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        })
        .collect()
}

fn diagnostic_action_kind(code: &str) -> Option<ConnectionActionKind> {
    if code == "action.host.reload_after_configuration_change" {
        Some(ConnectionActionKind::ReloadHost)
    } else if code.starts_with("action.host.") {
        Some(ConnectionActionKind::InspectRuntimeSession)
    } else if code.starts_with("action.managed_config.")
        || code.starts_with("action.storage.")
        || code.starts_with("action.store.")
        || code.starts_with("action.runtime_home.")
    {
        Some(ConnectionActionKind::RepairManagedConfiguration)
    } else if code == "action.guard.trigger_phase" {
        Some(ConnectionActionKind::RunGuardProbe)
    } else if code.starts_with("action.guard.") {
        Some(ConnectionActionKind::InspectHookContract)
    } else if code.starts_with("action.process.")
        || code.starts_with("action.mcp.")
        || code.starts_with("action.protocol.")
    {
        Some(ConnectionActionKind::InspectRuntimeSession)
    } else if code.starts_with("action.internal.")
        || code.starts_with("action.connection.reinstall")
        || code.starts_with("action.installation.")
    {
        Some(ConnectionActionKind::ReinstallCurrentBuild)
    } else {
        None
    }
}

pub(super) fn projected_root_cause_ids(
    report: &ConnectionCommandReport,
) -> Result<Vec<volicord_types::DiagnosticFindingId>, ConnectionCommandError> {
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
) -> Result<Vec<volicord_types::DiagnosticFindingId>, ConnectionCommandError> {
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

const fn connection_action_code(kind: ConnectionActionKind) -> &'static str {
    match kind {
        ConnectionActionKind::ReloadHost => "action.host.reload_after_configuration_change",
        ConnectionActionKind::ReviewHooks => "action.host.review_hooks",
        ConnectionActionKind::RunMcpVerification => "action.mcp.run_verification",
        ConnectionActionKind::RunGuardProbe => "action.guard.run_probe",
        ConnectionActionKind::InspectHookContract => "action.guard.inspect_hook_contract",
        ConnectionActionKind::RepairManagedConfiguration => "action.managed_config.repair",
        ConnectionActionKind::InspectRuntimeSession => "action.mcp.inspect_runtime_session",
        ConnectionActionKind::ReinstallCurrentBuild => "action.connection.reinstall_current_build",
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
    let output = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&report.diagnostic_report()?)
            .map(|output| format!("{output}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        OutputFormat::Human(HumanOutputDetail::Concise) => render_command_report_concise(report)?,
        OutputFormat::Human(HumanOutputDetail::Verbose) => render_command_report_verbose(report)?,
    };
    Ok(RenderedCommandReport {
        output,
        status: report.status(),
    })
}

fn command_status(report: &ConnectionVerificationReport) -> ConnectionStatus {
    if report.status() == ConnectionStatus::Complete && !report.actions().is_empty() {
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
        ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            vec![ConnectionCheck::try_new(
                ConnectionCheckKind::ManagedConfig,
                status,
                Vec::new(),
                (status != ConnectionCheckStatus::Passed)
                    .then(|| "managed_config_failed".to_owned()),
                "Managed configuration check",
                None,
                None,
            )
            .unwrap()],
            Vec::new(),
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
            "actions",
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
    fn every_operation_uses_the_same_exact_top_level_shape() {
        for operation in [
            CommandOperation::Init,
            CommandOperation::Add,
            CommandOperation::Status,
            CommandOperation::Verify,
        ] {
            let report = ConnectionCommandReport::from_verification(
                operation,
                matches!(operation, CommandOperation::Init | CommandOperation::Add).then_some(true),
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
            assert_eq!(value["actions"], json!([]));
            if matches!(operation, CommandOperation::Init | CommandOperation::Add) {
                assert_eq!(
                    value["operation_details"]["result"],
                    json!({"kind": "setup", "applied": true})
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
    fn dry_run_and_mode_status_come_from_typed_checks_and_actions() {
        let action_only_verification = ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            verification(ConnectionCheckStatus::Passed)
                .checks()
                .to_vec(),
            vec![
                ConnectionAction::try_new(ConnectionActionKind::ReloadHost, "Reload Codex")
                    .unwrap(),
            ],
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
            &[],
        )
        .unwrap();
        let changed = diagnostic_value(&changed);
        assert_top_level_keys(&changed);
        assert_eq!(changed["operation"], "add");
        assert_eq!(changed["status"], "action_required");
        assert_eq!(
            changed["operation_details"]["result"],
            json!({"kind": "setup", "applied": false})
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
        assert_eq!(mode["actions"][0]["id"], "reload_host");
        assert_eq!(mode["actions"][0]["owner"], "user");
        assert_eq!(mode["actions"][0]["channel"], "codex_ui");
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
        assert_eq!(removal["actions"], json!([]));
        assert!(removal["operation_details"].get("result").is_none());
    }

    #[test]
    fn setup_dry_run_preserves_canonical_host_actions_and_rejects_duplicate_kinds() {
        let host_action = ConnectionAction::try_new(
            ConnectionActionKind::RunMcpVerification,
            "Run connection verification",
        )
        .unwrap();
        let report = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            Vec::new(),
            std::slice::from_ref(&host_action),
        )
        .unwrap();
        let action = report
            .actions
            .iter()
            .find(|action| action.id() == ConnectionActionKind::RunMcpVerification)
            .expect("host-supplied action");
        assert_eq!(action.instruction(), "Run connection verification");
        assert_eq!(
            serde_json::to_value(action).unwrap(),
            json!({
                "id": "run_mcp_verification",
                "owner": "agent",
                "channel": "mcp_tool",
                "prerequisites": ["hook_source_activation"],
                "completes_checks": ["managed_capability_proof", "managed_session_health"],
                "root_finding_ids": [],
                "instruction": "Run connection verification",
            })
        );

        let error = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            Vec::new(),
            &[host_action.clone(), host_action],
        )
        .expect_err("duplicate action kinds must fail");
        assert!(error.to_string().contains("duplicate action"));
    }

    #[test]
    fn json_and_verbose_human_render_the_same_typed_status_and_actions() {
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
        assert!(text.output.contains("[fail] Managed Codex configuration"));
    }
}
