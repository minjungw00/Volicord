//! Active verification and current-status report input assembly.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConnectionEvaluationKind {
    Status,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConnectionEvaluationEvidence {
    CurrentStatus {
        managed_config: ManagedConfigStatus,
        host_executable: HostExecutableStatus,
    },
    ActiveVerification {
        managed_config: ManagedConfigStatus,
        host_executable: HostExecutableStatus,
        preflight: StepStatus,
        mcp_server: StepStatus,
    },
}

impl ConnectionEvaluationEvidence {
    const fn kind(&self) -> ConnectionEvaluationKind {
        match self {
            Self::CurrentStatus { .. } => ConnectionEvaluationKind::Status,
            Self::ActiveVerification { .. } => ConnectionEvaluationKind::Verify,
        }
    }

    fn validate(&self) -> Result<(), ConnectionCommandError> {
        let labels = match self {
            Self::CurrentStatus {
                managed_config,
                host_executable,
            } => vec![managed_config.as_str(), host_executable.as_str()],
            Self::ActiveVerification {
                managed_config,
                host_executable,
                preflight,
                mcp_server,
            } => vec![
                managed_config.as_str(),
                host_executable.as_str(),
                preflight.as_str(),
                mcp_server.as_str(),
            ],
        };
        if labels.iter().any(|label| label.is_empty()) {
            return Err(ConnectionCommandError::runtime(
                "connection evaluation evidence contains an empty typed state",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConnectionEvaluationMetadata {
    pub(super) kind: ConnectionEvaluationKind,
    pub(super) evaluated_at: UtcTimestamp,
    pub(super) integration_revision: IntegrationRevision,
}

/// Complete current-domain result before selected-Connection report assembly.
#[derive(Debug)]
pub(super) struct ConnectionEvaluation {
    pub(super) checks: Vec<ConnectionCheck>,
    pub(super) inline_findings: Vec<CurrentDiagnosticFinding>,
    pub(super) persisted_finding_seed_ids: Vec<DiagnosticFindingId>,
    pub(super) evidence: ConnectionEvaluationEvidence,
    pub(super) activation_plan: Option<IntegrationActivationPlan>,
    pub(super) metadata: ConnectionEvaluationMetadata,
}

impl ConnectionEvaluation {
    fn try_new(
        checks: Vec<ConnectionCheck>,
        mut inline_findings: Vec<CurrentDiagnosticFinding>,
        persisted_finding_seed_ids: Vec<DiagnosticFindingId>,
        evidence: ConnectionEvaluationEvidence,
        metadata: ConnectionEvaluationMetadata,
    ) -> Result<Self, ConnectionCommandError> {
        if evidence.kind() != metadata.kind {
            return Err(ConnectionCommandError::runtime(
                "connection evaluation evidence does not match its metadata",
            ));
        }
        evidence.validate()?;
        inline_findings.sort_by(|left, right| left.id().cmp(right.id()));
        inline_findings.dedup_by(|left, right| left.id() == right.id());
        let inline_ids = inline_findings
            .iter()
            .map(|finding| finding.id().clone())
            .collect::<BTreeSet<_>>();
        let mut persisted_finding_seed_ids = persisted_finding_seed_ids
            .into_iter()
            .chain(
                checks
                    .iter()
                    .flat_map(|check| check.cause_finding_ids().iter().cloned())
                    .chain(inline_findings.iter().flat_map(|finding| {
                        finding
                            .snapshot()
                            .causes()
                            .iter()
                            .map(|cause| cause.finding_id().clone())
                    })),
            )
            .filter(|finding_id| !inline_ids.contains(finding_id))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        persisted_finding_seed_ids.sort();
        Ok(Self {
            checks,
            inline_findings,
            persisted_finding_seed_ids,
            evidence,
            activation_plan: None,
            metadata,
        })
    }

    pub(super) fn finding_overlay(&self) -> DiagnosticFindingOverlay {
        let mut overlay = DiagnosticFindingOverlay::default();
        overlay.extend_inline_current(&self.inline_findings);
        let mut persisted = DiagnosticFindingOverlay::default();
        persisted.extend_persisted_seeds(self.persisted_finding_seed_ids.iter().cloned());
        overlay.merge(persisted);
        debug_assert_eq!(overlay.inline_findings().len(), self.inline_findings.len());
        debug_assert_eq!(
            overlay.persisted_finding_seed_ids().len(),
            self.persisted_finding_seed_ids.len()
        );
        overlay
    }
}

pub(super) fn assemble_connection_evaluation(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    mut evaluation: ConnectionEvaluation,
) -> Result<VerificationReport, ConnectionCommandError> {
    if evaluation.evidence.kind() != evaluation.metadata.kind {
        return Err(ConnectionCommandError::runtime(
            "connection evaluation evidence changed before report assembly",
        ));
    }
    evaluation.evidence.validate()?;
    let overlay = evaluation.finding_overlay();
    let (findings, integration_revision) = current_report_findings_with_overlay(
        runtime_home,
        connection,
        &evaluation.checks,
        &overlay,
    )?;
    evaluation.checks = finalize_check_graph(evaluation.checks, &findings)?;
    evaluation.activation_plan = Some(activation_plan_for_checks(&evaluation.checks)?);
    let report = ConnectionVerificationReport::try_new(
        evaluation.metadata.evaluated_at,
        evaluation.checks,
        evaluation.activation_plan.ok_or_else(|| {
            ConnectionCommandError::runtime("connection evaluation is missing its activation plan")
        })?,
    )
    .map_err(ConnectionCommandError::from)?;
    if integration_revision != evaluation.metadata.integration_revision {
        return Err(ConnectionCommandError::runtime(
            "connection integration revision changed during evaluation assembly",
        ));
    }
    Ok(VerificationReport {
        report,
        findings,
        integration_revision,
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
    let activation_plan = activation_plan_for_checks(&checks)?;
    ConnectionVerificationReport::try_new(current.checked_at().clone(), checks, activation_plan)
        .map_err(ConnectionCommandError::from)
}

pub(in crate::connection_command) fn report_with_hook_review_required(
    current: &ConnectionVerificationReport,
) -> Result<ConnectionVerificationReport, ConnectionCommandError> {
    let reset = [
        (
            ConnectionCheckKind::HostReload,
            "host_reload_required_after_hook_change",
            "Restart or reload Codex so it reads the current project hook definition",
        ),
        (
            ConnectionCheckKind::ManagedSessionHealth,
            "managed_session_required_after_hook_change",
            "A new managed Codex conversation is required after the hook definition changed",
        ),
        (
            ConnectionCheckKind::ManagedCapabilityProof,
            "managed_capability_proof_required_after_hook_change",
            "The new managed Codex conversation has not completed current capability proof",
        ),
        (
            ConnectionCheckKind::AmbientHookCoverage,
            "ambient_hook_coverage_required_after_hook_change",
            "The changed project hook definition does not yet have ambient phase coverage",
        ),
        (
            ConnectionCheckKind::CorrelatedGuardVerification,
            "correlated_guard_verification_required_after_hook_change",
            "The first-party correlated Guard verification has not completed for the changed hook definition",
        ),
    ]
    .into_iter()
    .map(|(kind, code, summary)| (kind, (code, summary)))
    .collect::<BTreeMap<_, _>>();
    let mut checks = Vec::with_capacity(current.checks().len());
    for check in current.checks() {
        if check.id() == ConnectionCheckKind::HookSourceActivation {
            let mut details = check
                .details()
                .map(ConnectionCheckDetails::as_object)
                .cloned()
                .unwrap_or_default();
            details.insert(
                "activation_state".to_owned(),
                Value::String(
                    HookActivationState::ReviewRequiredBySetup
                        .as_str()
                        .to_owned(),
                ),
            );
            checks.push(ConnectionCheck::try_new(
                ConnectionCheckKind::HookSourceActivation,
                ConnectionCheckStatus::Pending,
                Vec::new(),
                Some("hook_source_review_required_by_setup".to_owned()),
                "Current setup changed the project hook definition; host-owned review is required",
                Some(ConnectionCheckDetails::try_new(details)?),
                None,
            )?);
        } else if let Some((code, summary)) = reset.get(&check.id()) {
            checks.push(ConnectionCheck::try_new(
                check.id(),
                ConnectionCheckStatus::Pending,
                Vec::new(),
                Some((*code).to_owned()),
                *summary,
                check.details().cloned(),
                None,
            )?);
        } else {
            checks.push(check.clone());
        }
    }
    let checks = block_failed_dependencies(checks)?;
    let activation_plan = activation_plan_for_checks_with_hook_state(
        &checks,
        HookActivationState::ReviewRequiredBySetup,
    )?;
    ConnectionVerificationReport::try_new_with_hook_activation(
        current.checked_at().clone(),
        checks,
        HookActivationState::ReviewRequiredBySetup,
        activation_plan,
    )
    .map_err(ConnectionCommandError::from)
}

pub(super) fn canonical_verification_evaluation(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host: &Verification,
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<ConnectionEvaluation, ConnectionCommandError> {
    let current_revision = connection_integration_revision(connection)?;
    let current_sessions =
        current_managed_runtime_sessions(runtime_home, &connection.connection_internal_id)?;
    let session_evidence =
        McpSessionEvidenceSelection::select(&current_revision, &current_sessions)?;
    persist_peer_path_mismatch_findings(
        context,
        runtime_home,
        connection,
        host,
        &current_sessions,
    )?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let host_findings = host_boundary_findings(
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
        &current_revision,
        &session_evidence,
        latest_session.as_ref(),
        &host_findings.tool_round_trip,
    )?);
    let projects = volicord_store::agent_connections::list_connection_projects_for_diagnostics(
        runtime_home,
        &connection.connection_internal_id,
    )?;
    let guard = guard_checks_for_connection(runtime_home, connection, &projects)?;
    checks.extend(guard.checks);
    let mut inline_findings = host_findings.current;
    inline_findings.extend(guard.inline_findings);
    ConnectionEvaluation::try_new(
        checks,
        inline_findings,
        guard.persisted_finding_seed_ids,
        ConnectionEvaluationEvidence::ActiveVerification {
            managed_config: host.managed_config,
            host_executable: host.host_executable,
            preflight: preflight.status,
            mcp_server: handshake.step.status,
        },
        ConnectionEvaluationMetadata {
            kind: ConnectionEvaluationKind::Verify,
            evaluated_at: current_timestamp(),
            integration_revision: current_revision,
        },
    )
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
) -> Result<VerificationReport, ConnectionCommandError> {
    let current_host =
        current_status_host_diagnostic(runtime_home, connection, host_plan, projects, process)?;
    let persisted = connection.verification_report()?;
    let Some(mut host) = current_host else {
        let report = persisted.unwrap_or(effective_connection_report(connection)?);
        return assemble_connection_evaluation(
            runtime_home,
            connection,
            ConnectionEvaluation::try_new(
                report.checks().to_vec(),
                Vec::new(),
                Vec::new(),
                ConnectionEvaluationEvidence::CurrentStatus {
                    managed_config: ManagedConfigStatus::Unknown,
                    host_executable: HostExecutableStatus::NotChecked,
                },
                ConnectionEvaluationMetadata {
                    kind: ConnectionEvaluationKind::Status,
                    evaluated_at: current_timestamp(),
                    integration_revision: connection_integration_revision(connection)?,
                },
            )?,
        );
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
                .get("probe")
                .and_then(Value::as_object)
                .and_then(|probe| probe.get("discovered_path"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            host.host_version = details
                .get("probe")
                .and_then(Value::as_object)
                .and_then(|probe| probe.get("version"))
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
    let session_evidence =
        McpSessionEvidenceSelection::select(&current_revision, &current_sessions)?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let host_findings = host_boundary_findings(
        connection,
        &host,
        &current_sessions,
        latest_session.as_ref(),
        &current_revision,
    )?;
    let mut checks = vec![
        with_direct_causes(
            managed_config_check(&host)?,
            host_findings.managed_config.clone(),
        )?,
        stored_mcp,
        with_direct_causes(
            project_trust_check(&host)?,
            host_findings.project_trust.clone(),
        )?,
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
        &current_revision,
        &session_evidence,
        latest_session.as_ref(),
        &host_findings.tool_round_trip,
    )?);
    let guard = guard_checks_for_connection(runtime_home, connection, projects)?;
    checks.extend(guard.checks);
    let mut inline_findings = host_findings.current;
    inline_findings.extend(guard.inline_findings);
    assemble_connection_evaluation(
        runtime_home,
        connection,
        ConnectionEvaluation::try_new(
            checks,
            inline_findings,
            guard.persisted_finding_seed_ids,
            ConnectionEvaluationEvidence::CurrentStatus {
                managed_config: host.managed_config,
                host_executable: host.host_executable,
            },
            ConnectionEvaluationMetadata {
                kind: ConnectionEvaluationKind::Status,
                evaluated_at: current_timestamp(),
                integration_revision: current_revision,
            },
        )?,
    )
}
