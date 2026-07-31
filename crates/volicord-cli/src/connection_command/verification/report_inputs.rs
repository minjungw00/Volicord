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
        &evaluation.metadata.evaluated_at,
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
    connection: &AgentConnectionRecord,
    host: &Verification,
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<ConnectionEvaluation, ConnectionCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let evaluated_at = current_timestamp();
    let current_revision = connection_integration_revision(connection)?;
    let current_sessions =
        current_managed_runtime_sessions(runtime_home, &connection.connection_internal_id)?;
    let session_evidence =
        McpSessionEvidenceSelection::select(&current_revision, &current_sessions)?;
    persist_peer_path_mismatch_findings(context, connection, host, &current_sessions)?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let host_findings = host_boundary_findings(
        connection,
        host,
        &current_sessions,
        latest_session.as_ref(),
        &current_revision,
        &evaluated_at,
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
    let guard = guard_checks_for_connection(
        runtime_home,
        connection,
        &projects,
        &current_revision,
        &evaluated_at,
        None,
    )
    .map_err(GuardCheckEvaluationUnavailable::into_source)?;
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
            evaluated_at,
            integration_revision: current_revision,
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum CurrentConnectionEvaluationUnavailableCause {
    RegistrationMetadataCorrupt,
    PersistedActiveVerificationEvidenceCorrupt,
    ManagedConfigurationUnreadableOrInvalid,
    ProjectMembershipUnavailable,
    ProjectStoreUnavailable,
    GuardStateUnavailable,
    IntegrationRevisionUnavailableOrInconsistent,
    RuntimeSessionStateUnavailable,
    DiagnosticStateUnavailable,
    EvaluationAssemblyUnavailable,
}

impl CurrentConnectionEvaluationUnavailableCause {
    pub(in crate::connection_command) const fn as_str(self) -> &'static str {
        match self {
            Self::RegistrationMetadataCorrupt => "registration_metadata_corrupt",
            Self::PersistedActiveVerificationEvidenceCorrupt => {
                "persisted_active_verification_evidence_corrupt"
            }
            Self::ManagedConfigurationUnreadableOrInvalid => {
                "managed_configuration_unreadable_or_invalid"
            }
            Self::ProjectMembershipUnavailable => "project_membership_unavailable",
            Self::ProjectStoreUnavailable => "project_store_unavailable",
            Self::GuardStateUnavailable => "guard_state_unavailable",
            Self::IntegrationRevisionUnavailableOrInconsistent => {
                "integration_revision_unavailable_or_inconsistent"
            }
            Self::RuntimeSessionStateUnavailable => "runtime_session_state_unavailable",
            Self::DiagnosticStateUnavailable => "diagnostic_state_unavailable",
            Self::EvaluationAssemblyUnavailable => "evaluation_assembly_unavailable",
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::connection_command) struct CurrentConnectionEvaluationUnavailable {
    cause: CurrentConnectionEvaluationUnavailableCause,
    source: ConnectionCommandError,
}

impl CurrentConnectionEvaluationUnavailable {
    fn new(
        cause: CurrentConnectionEvaluationUnavailableCause,
        source: impl Into<ConnectionCommandError>,
    ) -> Self {
        Self {
            cause,
            source: source.into(),
        }
    }

    pub(in crate::connection_command) const fn cause(
        &self,
    ) -> CurrentConnectionEvaluationUnavailableCause {
        self.cause
    }

    pub(in crate::connection_command) fn bounded_detail(&self) -> String {
        self.source.to_string().chars().take(1_024).collect()
    }
}

impl std::fmt::Display for CurrentConnectionEvaluationUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "CURRENT_CONNECTION_EVALUATION_UNAVAILABLE: {}: {}",
            self.cause.as_str(),
            self.source
        )
    }
}

impl std::error::Error for CurrentConnectionEvaluationUnavailable {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PersistedMcpPreflightStatus {
    Passed,
    Failed,
    Pending,
}

impl PersistedMcpPreflightStatus {
    const fn into_step_status(self) -> StepStatus {
        match self {
            Self::Passed => StepStatus::Passed,
            Self::Failed => StepStatus::Failed,
            Self::Pending => StepStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedMcpPreflightDetails {
    status: PersistedMcpPreflightStatus,
    code: String,
    diagnostic: String,
    evidence: Option<McpPreflightEvidence>,
    finding_id: Option<String>,
    diagnostic_code: Option<String>,
    #[serde(rename = "failure_stage")]
    _failure_stage: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedMcpServerDetails {
    preflight: PersistedMcpPreflightDetails,
    last_active_verification: Option<McpActiveVerificationEvidence>,
}

fn unavailable(
    cause: CurrentConnectionEvaluationUnavailableCause,
    source: impl Into<ConnectionCommandError>,
) -> CurrentConnectionEvaluationUnavailable {
    CurrentConnectionEvaluationUnavailable::new(cause, source)
}

fn persisted_evidence_corrupt(detail: impl Into<String>) -> CurrentConnectionEvaluationUnavailable {
    unavailable(
        CurrentConnectionEvaluationUnavailableCause::PersistedActiveVerificationEvidenceCorrupt,
        ConnectionCommandError::runtime(detail),
    )
}

fn validate_current_membership_coordinate(
    runtime_home: &Path,
    expected_connection: &AgentConnectionRecord,
    expected_membership: &ConnectionProjectRecord,
    expected_revision: &IntegrationRevision,
) -> Result<(), CurrentConnectionEvaluationUnavailable> {
    let connection = agent_connection_record_read_only(
        runtime_home,
        &expected_connection.connection_internal_id,
    )
    .map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::IntegrationRevisionUnavailableOrInconsistent,
            ConnectionCommandError::from(error),
        )
    })?
    .ok_or_else(|| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::IntegrationRevisionUnavailableOrInconsistent,
            ConnectionCommandError::runtime("selected Agent Connection is no longer registered"),
        )
    })?;
    let revision = connection_integration_revision(&connection).map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::IntegrationRevisionUnavailableOrInconsistent,
            ConnectionCommandError::from(error),
        )
    })?;
    if revision != *expected_revision || connection != *expected_connection {
        return Err(unavailable(
            CurrentConnectionEvaluationUnavailableCause::IntegrationRevisionUnavailableOrInconsistent,
            ConnectionCommandError::concurrent_modification(
                "selected Agent Connection changed during current evaluation",
            ),
        ));
    }
    let memberships = list_connection_projects_read_only(
        runtime_home,
        &expected_connection.connection_internal_id,
    )
    .map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::ProjectMembershipUnavailable,
            ConnectionCommandError::from(error),
        )
    })?;
    let current = memberships
        .iter()
        .find(|membership| membership.project_id == expected_membership.project_id)
        .ok_or_else(|| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::ProjectMembershipUnavailable,
                ConnectionCommandError::runtime(
                    "selected Product Repository membership is no longer registered",
                ),
            )
        })?;
    if current != expected_membership {
        return Err(unavailable(
            CurrentConnectionEvaluationUnavailableCause::ProjectMembershipUnavailable,
            ConnectionCommandError::concurrent_modification(
                "selected Product Repository membership changed during current evaluation",
            ),
        ));
    }
    Ok(())
}

fn apply_persisted_host_executable_evidence(
    host: &mut Verification,
    persisted: Option<&ConnectionVerificationReport>,
) -> Result<(), CurrentConnectionEvaluationUnavailable> {
    let Some(check) = persisted.and_then(|report| {
        report
            .checks()
            .iter()
            .find(|check| check.id() == ConnectionCheckKind::HostExecutable)
    }) else {
        host.host_executable_code = "host_executable_not_verified".to_owned();
        host.host_executable_details = "Codex executable has not been actively verified".to_owned();
        return Ok(());
    };
    let details = check
        .details()
        .map(ConnectionCheckDetails::as_object)
        .cloned()
        .map(Value::Object)
        .ok_or_else(|| {
            persisted_evidence_corrupt(
                "persisted host executable evidence is missing typed details",
            )
        })?;
    let evidence =
        serde_json::from_value::<HostExecutableProbeDetails>(details).map_err(|error| {
            persisted_evidence_corrupt(format!(
                "persisted host executable evidence is invalid: {error}"
            ))
        })?;
    host.host_executable = evidence.status();
    host.executable_path = evidence.probe().discovered_path.clone();
    host.host_version = evidence.probe().version.clone();
    host.host_executable_code = check
        .code()
        .unwrap_or(match evidence.status() {
            HostExecutableStatus::Available => "host_executable_available",
            HostExecutableStatus::Unavailable => "host_executable_unavailable",
            HostExecutableStatus::NotChecked => "host_executable_not_checked",
        })
        .to_owned();
    host.host_executable_details = evidence.diagnostic().to_owned();
    Ok(())
}

fn preflight_evidence_passed(evidence: &McpPreflightEvidence) -> bool {
    evidence.configuration() == McpEvidenceCheckStatus::Passed
        && evidence.registry_read() == McpEvidenceCheckStatus::Passed
        && evidence
            .project_reads()
            .iter()
            .all(|project| project.state_read() == McpEvidenceCheckStatus::Passed)
        && evidence.schema_validation() == McpEvidenceCheckStatus::Passed
        && evidence.protocol_profiles() == McpEvidenceCheckStatus::Passed
        && evidence.host_contracts() == McpEvidenceCheckStatus::Passed
}

fn active_evidence_passed(evidence: &McpActiveVerificationEvidence) -> bool {
    evidence.registry_write() == McpEvidenceCheckStatus::Passed
        && evidence
            .project_writes()
            .iter()
            .all(|project| project.state_write() == McpEvidenceCheckStatus::Passed)
        && evidence
            .protocol_conformance()
            .iter()
            .all(|probe| probe.probe().status() == McpEvidenceCheckStatus::Passed)
        && evidence
            .host_compatibility()
            .iter()
            .all(|probe| probe.probe().status() == McpEvidenceCheckStatus::Passed)
}

fn active_evidence_diagnostic_code(evidence: &McpActiveVerificationEvidence) -> Option<&str> {
    evidence
        .protocol_conformance()
        .iter()
        .map(McpRevisionConformance::probe)
        .chain(
            evidence
                .host_compatibility()
                .iter()
                .map(McpHostCompatibilityEvidence::probe),
        )
        .find(|probe| probe.status() == McpEvidenceCheckStatus::Failed)
        .and_then(McpProbeEvidence::diagnostic_code)
}

fn persisted_mcp_finding_ids(
    details: &PersistedMcpServerDetails,
) -> Result<Vec<DiagnosticFindingId>, CurrentConnectionEvaluationUnavailable> {
    let values = details
        .preflight
        .finding_id
        .iter()
        .map(String::as_str)
        .chain(
            details
                .last_active_verification
                .iter()
                .flat_map(McpActiveVerificationEvidence::protocol_conformance)
                .map(McpRevisionConformance::probe)
                .filter_map(McpProbeEvidence::finding_id),
        )
        .chain(
            details
                .last_active_verification
                .iter()
                .flat_map(McpActiveVerificationEvidence::host_compatibility)
                .map(McpHostCompatibilityEvidence::probe)
                .filter_map(McpProbeEvidence::finding_id),
        );
    let mut ids = values
        .map(|value| {
            DiagnosticFindingId::parse(value.to_owned()).map_err(|error| {
                persisted_evidence_corrupt(format!(
                    "persisted MCP evidence has an invalid finding ID: {error}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn current_mcp_server_check(
    persisted: Option<&ConnectionVerificationReport>,
) -> Result<ConnectionCheck, CurrentConnectionEvaluationUnavailable> {
    let Some(check) = persisted.and_then(|report| {
        report
            .checks()
            .iter()
            .find(|check| check.id() == ConnectionCheckKind::McpServer)
    }) else {
        return canonical_check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Pending,
            "mcp_server_not_verified",
            "Volicord MCP server has not been actively verified",
            None,
            None,
        )
        .map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                error,
            )
        });
    };
    let value = check
        .details()
        .map(ConnectionCheckDetails::as_object)
        .cloned()
        .map(Value::Object)
        .ok_or_else(|| {
            persisted_evidence_corrupt("persisted MCP verification evidence is missing details")
        })?;
    let details = serde_json::from_value::<PersistedMcpServerDetails>(value).map_err(|error| {
        persisted_evidence_corrupt(format!(
            "persisted MCP verification evidence is invalid: {error}"
        ))
    })?;
    if details.preflight.status == PersistedMcpPreflightStatus::Passed
        && details
            .preflight
            .evidence
            .as_ref()
            .is_none_or(|evidence| !preflight_evidence_passed(evidence))
    {
        return Err(persisted_evidence_corrupt(
            "persisted MCP preflight passed without complete passing evidence",
        ));
    }
    let preflight = VerificationStep {
        status: details.preflight.status.into_step_status(),
        code: details.preflight.code.clone(),
        details: details.preflight.diagnostic.clone(),
        preflight_evidence: details.preflight.evidence.clone(),
        process_id: None,
        failure: None,
        diagnostic: details
            .preflight
            .finding_id
            .as_ref()
            .zip(details.preflight.diagnostic_code.as_ref())
            .map(|(finding_id, code)| McpPersistedDiagnostic {
                finding_id: finding_id.clone(),
                code: code.clone(),
            }),
    };
    let active = details.last_active_verification.clone();
    let step = match active.as_ref() {
        Some(evidence) if active_evidence_passed(evidence) => VerificationStep::passed_with_code(
            "mcp_server_ready",
            "Persisted active MCP verification evidence passed",
        ),
        Some(evidence) => VerificationStep::failed_with_code(
            active_evidence_diagnostic_code(evidence)
                .unwrap_or("mcp_server_active_verification_failed"),
            "Persisted active MCP verification evidence contains a failed observation",
        ),
        None => VerificationStep::pending("Active MCP verification has not run"),
    };
    let handshake = McpVerification {
        step,
        exchange: None,
        active_evidence: active,
    };
    let causes = persisted_mcp_finding_ids(&details)?;
    let check = mcp_server_check(&preflight, &handshake).map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
            error,
        )
    })?;
    with_direct_causes(check, causes).map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
            error,
        )
    })
}

#[derive(Debug, Clone)]
struct CurrentConnectionSharedInputs {
    current_revision: IntegrationRevision,
    persisted: Option<ConnectionVerificationReport>,
    current_sessions: Vec<McpRuntimeSessionRecord>,
    session_evidence: McpSessionEvidenceSelection,
    latest_session: Option<McpRuntimeSessionRecord>,
}

/// Request-scoped inputs for one Connection's current membership evaluations.
///
/// The context intentionally has no process-global lifetime. Connection-level
/// Persisted active evidence and runtime-session inputs are read at most once
/// for every membership evaluated by one command invocation, while membership
/// coordinates, project state, and Guard state remain membership local.
pub(in crate::connection_command) struct CurrentConnectionEvaluationContext<'a, P>
where
    P: ConnectionProcess,
{
    runtime_home: &'a Path,
    connection: &'a AgentConnectionRecord,
    evaluated_at: UtcTimestamp,
    process: &'a P,
    shared: Option<Result<CurrentConnectionSharedInputs, CurrentConnectionEvaluationUnavailable>>,
}

impl<'a, P> CurrentConnectionEvaluationContext<'a, P>
where
    P: ConnectionProcess,
{
    pub(in crate::connection_command) fn new(
        runtime_home: &'a Path,
        connection: &'a AgentConnectionRecord,
        evaluated_at: UtcTimestamp,
        process: &'a P,
    ) -> Self {
        Self {
            runtime_home,
            connection,
            evaluated_at,
            process,
            shared: None,
        }
    }

    pub(in crate::connection_command) fn evaluate(
        &mut self,
        selected_membership: &ConnectionProjectRecord,
    ) -> Result<VerificationReport, CurrentConnectionEvaluationUnavailable> {
        parse_metadata(
            &self.connection.metadata_json,
            Some(selected_membership.project_id.as_str()),
        )
        .map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::RegistrationMetadataCorrupt,
                error,
            )
        })?;
        let runtime_home = self.runtime_home;
        let connection = self.connection;
        let evaluated_at = self.evaluated_at.clone();
        let process = self.process;
        let shared = self.shared_inputs()?;
        current_status_report_with_inputs(
            runtime_home,
            connection,
            selected_membership,
            evaluated_at,
            process,
            shared,
        )
    }

    fn shared_inputs(
        &mut self,
    ) -> Result<&CurrentConnectionSharedInputs, CurrentConnectionEvaluationUnavailable> {
        if self.shared.is_none() {
            self.shared = Some(current_connection_shared_inputs(
                self.runtime_home,
                self.connection,
            ));
        }
        self.shared
            .as_ref()
            .expect("current Connection shared inputs were initialized")
            .as_ref()
            .map_err(Clone::clone)
    }
}

fn current_connection_shared_inputs(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
) -> Result<CurrentConnectionSharedInputs, CurrentConnectionEvaluationUnavailable> {
    let current_revision = connection_integration_revision(connection).map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::IntegrationRevisionUnavailableOrInconsistent,
            ConnectionCommandError::from(error),
        )
    })?;
    let persisted = connection.verification_report().map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::PersistedActiveVerificationEvidenceCorrupt,
            ConnectionCommandError::from(error),
        )
    })?;
    let current_sessions =
        current_managed_runtime_sessions(runtime_home, &connection.connection_internal_id)
            .map_err(|error| {
                unavailable(
                    CurrentConnectionEvaluationUnavailableCause::RuntimeSessionStateUnavailable,
                    ConnectionCommandError::from(error),
                )
            })?;
    let session_evidence =
        McpSessionEvidenceSelection::select(&current_revision, &current_sessions).map_err(
            |error| {
                unavailable(
                    CurrentConnectionEvaluationUnavailableCause::RuntimeSessionStateUnavailable,
                    ConnectionCommandError::from(error),
                )
            },
        )?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id).map_err(
            |error| {
                unavailable(
                    CurrentConnectionEvaluationUnavailableCause::RuntimeSessionStateUnavailable,
                    ConnectionCommandError::from(error),
                )
            },
        )?;
    Ok(CurrentConnectionSharedInputs {
        current_revision,
        persisted,
        current_sessions,
        session_evidence,
        latest_session,
    })
}

fn current_status_report_with_inputs(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    selected_membership: &ConnectionProjectRecord,
    evaluated_at: UtcTimestamp,
    process: &impl ConnectionProcess,
    shared: &CurrentConnectionSharedInputs,
) -> Result<VerificationReport, CurrentConnectionEvaluationUnavailable> {
    let current_revision = &shared.current_revision;
    validate_current_membership_coordinate(
        runtime_home,
        connection,
        selected_membership,
        current_revision,
    )?;
    CoreProjectStore::open_read_only(
        runtime_home,
        &ProjectId::new(selected_membership.project_id.clone()),
    )
    .map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::ProjectStoreUnavailable,
            ConnectionCommandError::from(error),
        )
    })?;

    let persisted = shared.persisted.as_ref();
    let host_plan =
        existing_host_plan(connection, runtime_home, process, Some(selected_membership)).map_err(
            |error| {
                unavailable(
            CurrentConnectionEvaluationUnavailableCause::ManagedConfigurationUnreadableOrInvalid,
            error,
        )
            },
        )?;
    let managed = codex::managed_identity_evaluation_for_plan(&host_plan).map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::ManagedConfigurationUnreadableOrInvalid,
            ConnectionCommandError::from(error),
        )
    })?;
    if managed.status == ManagedConfigStatus::Unavailable {
        return Err(unavailable(
            CurrentConnectionEvaluationUnavailableCause::ManagedConfigurationUnreadableOrInvalid,
            ConnectionCommandError::runtime(managed.details),
        ));
    }
    let mut host = Verification::unobserved(&connection.config_target);
    host.managed_config = managed.status;
    host.managed_config_diagnostic = managed.diagnostic;
    host.managed_config_details = managed.details;
    if host_plan.host_scope == HostScope::Project {
        host.project_trust = Some(codex::project_trust_diagnostic(
            &codex_environment(process),
            &selected_membership.project.repo_root,
        ));
    }
    apply_persisted_host_executable_evidence(&mut host, persisted)?;
    let mcp_check = current_mcp_server_check(persisted)?;

    let host_findings = host_boundary_findings(
        connection,
        &host,
        &shared.current_sessions,
        shared.latest_session.as_ref(),
        current_revision,
        &evaluated_at,
    )
    .map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::RuntimeSessionStateUnavailable,
            error,
        )
    })?;
    let mut checks = vec![
        with_direct_causes(
            managed_config_check(&host).map_err(|error| {
                unavailable(
                    CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                    error,
                )
            })?,
            host_findings.managed_config.clone(),
        )
        .map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                error,
            )
        })?,
        mcp_check,
        with_direct_causes(
            project_trust_check(&host).map_err(|error| {
                unavailable(
                    CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                    error,
                )
            })?,
            host_findings.project_trust.clone(),
        )
        .map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                error,
            )
        })?,
        host_executable_check(&host).map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                error,
            )
        })?,
    ];
    checks.extend(
        host_session_checks(
            &host,
            current_revision,
            &shared.session_evidence,
            shared.latest_session.as_ref(),
            &host_findings.tool_round_trip,
        )
        .map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
                error,
            )
        })?,
    );
    let guard = guard_checks_for_connection(
        runtime_home,
        connection,
        std::slice::from_ref(selected_membership),
        current_revision,
        &evaluated_at,
        Some(selected_membership),
    )
    .map_err(|error| {
        let cause = match error.cause {
            GuardCheckEvaluationUnavailableCause::ProjectStore => {
                CurrentConnectionEvaluationUnavailableCause::ProjectStoreUnavailable
            }
            GuardCheckEvaluationUnavailableCause::GuardState => {
                CurrentConnectionEvaluationUnavailableCause::GuardStateUnavailable
            }
        };
        unavailable(cause, error.into_source())
    })?;
    checks.extend(guard.checks);
    let mut inline_findings = host_findings.current;
    inline_findings.extend(guard.inline_findings);
    let evaluation = ConnectionEvaluation::try_new(
        checks,
        inline_findings,
        guard.persisted_finding_seed_ids,
        ConnectionEvaluationEvidence::CurrentStatus {
            managed_config: host.managed_config,
            host_executable: host.host_executable,
        },
        ConnectionEvaluationMetadata {
            kind: ConnectionEvaluationKind::Status,
            evaluated_at,
            integration_revision: current_revision.clone(),
        },
    )
    .map_err(|error| {
        unavailable(
            CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable,
            error,
        )
    })?;
    let report =
        assemble_connection_evaluation(runtime_home, connection, evaluation).map_err(|error| {
            unavailable(
                CurrentConnectionEvaluationUnavailableCause::DiagnosticStateUnavailable,
                error,
            )
        })?;
    validate_current_membership_coordinate(
        runtime_home,
        connection,
        selected_membership,
        current_revision,
    )?;
    Ok(report)
}

pub(in crate::connection_command) fn current_status_report(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    selected_membership: &ConnectionProjectRecord,
    evaluated_at: UtcTimestamp,
    process: &impl ConnectionProcess,
) -> Result<VerificationReport, CurrentConnectionEvaluationUnavailable> {
    CurrentConnectionEvaluationContext::new(runtime_home, connection, evaluated_at, process)
        .evaluate(selected_membership)
}
