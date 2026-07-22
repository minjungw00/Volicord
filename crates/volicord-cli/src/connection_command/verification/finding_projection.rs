//! Projection and persistence of verification observations as diagnostic findings.

use super::*;

pub(super) fn diagnostic_occurrence_for_runtime_code(
    runtime_home: &Path,
    runtime_session_id: &str,
    code: &str,
) -> Result<Option<volicord_types::OccurrenceDiagnosticFinding>, ConnectionCommandError> {
    Ok(
        diagnostic_occurrences_for_runtime_session(runtime_home, runtime_session_id)?
            .into_iter()
            .find(|finding| finding.data().code().as_str() == code),
    )
}

pub(super) fn persist_process_diagnostics(
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

pub(super) fn persist_process_finding(
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
    let revision = connection_integration_revision(connection)?;
    let finding = failure
        .to_diagnostic_data(McpProcessDiagnosticContext {
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
    let finding = volicord_types::OccurrenceDiagnosticFinding::try_new(
        finding,
        runtime
            .as_ref()
            .map(|runtime| AgentRuntimeSessionId::new(runtime.runtime_session_id.clone())),
    )
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    volicord_store::diagnostic_findings::insert_occurrence_finding(runtime_home, &finding)?;
    Ok(McpPersistedDiagnostic {
        finding_id: finding.id().to_string(),
        code: finding.data().code().to_string(),
    })
}

#[derive(Default)]
pub(super) struct HostBoundaryFindings {
    pub(super) managed_config: Vec<DiagnosticFindingId>,
    pub(super) project_trust: Vec<DiagnosticFindingId>,
    pub(super) tool_round_trip: Vec<DiagnosticFindingId>,
    pub(super) current: Vec<volicord_types::CurrentDiagnosticFinding>,
}

pub(super) fn host_boundary_findings(
    connection: &AgentConnectionRecord,
    host: &Verification,
    current_sessions: &[McpRuntimeSessionRecord],
    latest_session: Option<&McpRuntimeSessionRecord>,
    current_revision: &IntegrationRevision,
) -> Result<HostBoundaryFindings, ConnectionCommandError> {
    let mut findings = HostBoundaryFindings::default();
    let observed_at = current_timestamp();
    if let Some(diagnostic) = host.managed_config_diagnostic {
        let subject = ManagedConfigurationTarget::for_connection(
            &connection.connection_internal_id,
            Path::new(&connection.config_target),
        )
        .map_err(ConnectionCommandError::runtime)?;
        let finding = current_connection_finding(
            connection,
            OperationalDiagnostic::ManagedConfig(diagnostic),
            &subject,
            &ManagedConfigurationFacts::from_status(host.managed_config),
            OperationalCheckState::Failed,
            observed_at.clone(),
        )?;
        findings.managed_config.push(finding.id().clone());
        findings.current.push(finding);
    }
    if let Some(trust) = host.project_trust.as_ref() {
        if let Some(diagnostic) = TrustDiagnostic::from_status(trust.status) {
            let subject = TrustSubject::for_repository(
                &connection.connection_internal_id,
                Path::new(&trust.repo_root),
            )
            .map_err(ConnectionCommandError::runtime)?;
            let check_state = if trust.status == ProjectTrustStatus::Malformed {
                OperationalCheckState::Failed
            } else {
                OperationalCheckState::Pending
            };
            let finding = current_connection_finding(
                connection,
                OperationalDiagnostic::Trust(diagnostic),
                &subject,
                &TrustFacts::from_status(trust.status),
                check_state,
                observed_at.clone(),
            )?;
            findings.project_trust.push(finding.id().clone());
            findings.current.push(finding);
        }
    }
    if current_sessions.is_empty() {
        if let Some(latest) = latest_session
            .filter(|latest| latest.connection_integration_revision != current_revision.as_str())
        {
            let subject = IntegrationRevisionSubject::for_runtime_session(
                &connection.connection_internal_id,
                &latest.runtime_session_id,
            )
            .map_err(ConnectionCommandError::runtime)?;
            let observed_revision =
                IntegrationRevision::parse(latest.connection_integration_revision.clone())
                    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
            findings.current.push(current_connection_finding(
                connection,
                OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale),
                &subject,
                &IntegrationRevisionFacts::new(current_revision, Some(&observed_revision)),
                OperationalCheckState::Pending,
                observed_at.clone(),
            )?);
        }
    }
    let expected_tool_name = super::super::managed_host_round_trip_tool().wire_name();
    for session in current_sessions.iter().filter(|session| {
        session.verification_tool_observed_at.is_some()
            && session
                .verification_tool_name
                .as_deref()
                .is_some_and(|observed| observed != expected_tool_name)
    }) {
        let subject = VerificationToolSubject::for_runtime_session(
            &connection.connection_internal_id,
            &session.runtime_session_id,
        )
        .map_err(ConnectionCommandError::runtime)?;
        let finding = current_connection_finding(
            connection,
            OperationalDiagnostic::ToolVerification(
                ToolVerificationDiagnostic::DesignationMismatch,
            ),
            &subject,
            &VerificationToolFacts::new(
                expected_tool_name,
                session
                    .verification_tool_name
                    .as_deref()
                    .expect("filtered verification-tool observation has a name"),
            ),
            OperationalCheckState::Failed,
            observed_at.clone(),
        )?;
        findings.tool_round_trip.push(finding.id().clone());
        findings.current.push(finding);
    }
    findings
        .current
        .sort_by(|left, right| left.id().cmp(right.id()));
    findings
        .current
        .dedup_by(|left, right| left.id() == right.id());
    Ok(findings)
}

pub(super) fn observed_host_version(session: &McpRuntimeSessionRecord) -> Option<&str> {
    session.observed_host_executable_version.as_deref()
}

#[derive(Serialize)]
pub(super) struct ActualMcpPeerClientInfo<'a> {
    name: Option<&'a str>,
    version: &'a str,
}

#[derive(Serialize)]
pub(super) struct PathExecutableProbe<'a> {
    path: Option<&'a str>,
    version: &'a str,
}

#[derive(Serialize)]
pub(super) struct PeerPathMismatchFacts<'a> {
    summary: &'static str,
    runtime_session_id: &'a str,
    actual_mcp_peer_client_info: ActualMcpPeerClientInfo<'a>,
    path_executable_probe: PathExecutableProbe<'a>,
}

impl DiagnosticFactSource for PeerPathMismatchFacts<'_> {}

pub(super) fn persist_peer_path_mismatch_findings(
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
        if diagnostic_occurrence_for_runtime_code(
            runtime_home,
            &session.runtime_session_id,
            "host.codex.peer_version_differs_from_path_probe",
        )?
        .is_some()
        {
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
        let data = volicord_types::DiagnosticFindingData::try_new(
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
        .and_then(|data| {
            data.with_connection_id(AgentConnectionId::new(
                connection.connection_internal_id.clone(),
            ))
        })
        .map(|data| {
            data.with_integration_revision(
                IntegrationRevision::parse(session.connection_integration_revision.clone())
                    .expect("persisted runtime session has a validated integration revision"),
            )
        })
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        let finding = volicord_types::OccurrenceDiagnosticFinding::try_new(
            data,
            Some(AgentRuntimeSessionId::new(
                session.runtime_session_id.clone(),
            )),
        )
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        insert_occurrence_finding(runtime_home, &finding)?;
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct GuardBoundaryFindings {
    pub(super) files: Vec<DiagnosticFindingId>,
    pub(super) observation: Vec<DiagnosticFindingId>,
    pub(super) current: Vec<volicord_types::CurrentDiagnosticFinding>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn guard_boundary_findings(
    connection: &AgentConnectionRecord,
    audit: &GuardAuditFacts,
    installation_ids: &[String],
    guard_files_failed: bool,
    missing_required_phases: &[String],
    incompatible_event_ids: &[String],
    prompt_capture_observed: bool,
    observation_revision_mismatch_installation_ids: &[String],
    observed_at: UtcTimestamp,
) -> Result<GuardBoundaryFindings, ConnectionCommandError> {
    let mut findings = GuardBoundaryFindings::default();
    let mut current = Vec::new();
    for finding in &audit.findings {
        let diagnostic = GuardDiagnostic::from_artifact_issue(finding.artifact, finding.issue);
        let subject = GuardManagedArtifactSubject::for_connection(
            &connection.connection_internal_id,
            finding.artifact,
            &finding.path,
        )
        .map_err(ConnectionCommandError::runtime)?;
        let projected = current_connection_finding(
            connection,
            OperationalDiagnostic::Guard(diagnostic),
            &subject,
            &GuardArtifactFacts::new(guard_artifact_kind(finding.artifact)),
            OperationalCheckState::Failed,
            observed_at.clone(),
        )?;
        findings.files.push(projected.id().clone());
        current.push(projected);
    }
    for issue in &audit.manifest_issues {
        for installation_id in installation_ids {
            let subject = GuardInstallationSubject::for_connection(
                &connection.connection_internal_id,
                installation_id,
            )
            .map_err(ConnectionCommandError::runtime)?;
            let projected = current_connection_finding(
                connection,
                OperationalDiagnostic::Guard(GuardDiagnostic::from_manifest_issue(*issue)),
                &subject,
                &GuardInstallationFacts::default(),
                OperationalCheckState::Failed,
                observed_at.clone(),
            )?;
            findings.files.push(projected.id().clone());
            current.push(projected);
        }
    }
    for status in &audit.hook_path_safety_statuses {
        if let Some(diagnostic) = GuardDiagnostic::from_hook_wrapper_status(*status) {
            for installation_id in installation_ids {
                let subject = GuardInstallationSubject::for_connection(
                    &connection.connection_internal_id,
                    installation_id,
                )
                .map_err(ConnectionCommandError::runtime)?;
                let projected = current_connection_finding(
                    connection,
                    OperationalDiagnostic::Guard(diagnostic),
                    &subject,
                    &GuardInstallationFacts::from_hook_wrapper_status(*status),
                    OperationalCheckState::Failed,
                    observed_at.clone(),
                )?;
                findings.files.push(projected.id().clone());
                current.push(projected);
            }
        }
    }
    if guard_files_failed && findings.files.is_empty() {
        let subject =
            GuardInstallationSubject::inventory_for_connection(&connection.connection_internal_id)
                .map_err(ConnectionCommandError::runtime)?;
        let projected = current_connection_finding(
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::ManifestMismatch),
            &subject,
            &GuardInstallationFacts::default(),
            OperationalCheckState::Failed,
            observed_at.clone(),
        )?;
        findings.files.push(projected.id().clone());
        current.push(projected);
    }
    let missing_phases = audit
        .missing_required_phases
        .iter()
        .map(|phase| phase.as_str())
        .chain(missing_required_phases.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    for phase in missing_phases {
        let phase = volicord_types::GuardHookPhase::from_str(phase)
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        let subject = GuardPhaseSubject::for_connection(&connection.connection_internal_id, phase)
            .map_err(ConnectionCommandError::runtime)?;
        current.push(current_connection_finding(
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
            &subject,
            &GuardPhaseFacts::new(phase),
            OperationalCheckState::Pending,
            observed_at.clone(),
        )?);
    }
    for event_id in incompatible_event_ids {
        let subject =
            GuardEventSubject::for_connection(&connection.connection_internal_id, event_id)
                .map_err(ConnectionCommandError::runtime)?;
        let projected = current_connection_finding(
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::IncompatibleObservation),
            &subject,
            &GuardEventFacts::default(),
            OperationalCheckState::Failed,
            observed_at.clone(),
        )?;
        findings.observation.push(projected.id().clone());
        current.push(projected);
    }
    if audit.prompt_capture_configured && !audit.prompt_capture_host_supported {
        let phase = volicord_types::GuardHookPhase::PromptCapture;
        let subject = GuardPhaseSubject::for_connection(&connection.connection_internal_id, phase)
            .map_err(ConnectionCommandError::runtime)?;
        current.push(current_connection_finding(
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnsupported),
            &subject,
            &GuardPhaseFacts::new(phase),
            OperationalCheckState::Pending,
            observed_at.clone(),
        )?);
    } else if audit.prompt_capture_configured && !prompt_capture_observed {
        let phase = volicord_types::GuardHookPhase::PromptCapture;
        let subject = GuardPhaseSubject::for_connection(&connection.connection_internal_id, phase)
            .map_err(ConnectionCommandError::runtime)?;
        current.push(current_connection_finding(
            connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::PromptCaptureUnobserved),
            &subject,
            &GuardPhaseFacts::new(phase),
            OperationalCheckState::Pending,
            observed_at.clone(),
        )?);
    }
    for installation_id in observation_revision_mismatch_installation_ids {
        let revision = connection_integration_revision(connection)?;
        let subject = IntegrationRevisionSubject::for_guard_installation(
            &connection.connection_internal_id,
            installation_id,
        )
        .map_err(ConnectionCommandError::runtime)?;
        let projected = current_connection_finding(
            connection,
            OperationalDiagnostic::Revision(RevisionDiagnostic::ObservationMismatch),
            &subject,
            &IntegrationRevisionFacts::new(&revision, None),
            OperationalCheckState::Failed,
            observed_at.clone(),
        )?;
        findings.files.push(projected.id().clone());
        current.push(projected);
    }
    findings.files.sort();
    findings.files.dedup();
    findings.observation.sort();
    findings.observation.dedup();
    let current = current
        .into_iter()
        .map(|finding| (finding.id().clone(), finding))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect::<Vec<_>>();
    findings.current = current;
    Ok(findings)
}

pub(super) fn guard_artifact_issue_name(issue: GuardArtifactIssue) -> &'static str {
    match issue {
        GuardArtifactIssue::Missing => "missing",
        GuardArtifactIssue::Malformed => "malformed",
        GuardArtifactIssue::ContentMismatch => "content_mismatch",
        GuardArtifactIssue::OwnershipMismatch => "ownership_mismatch",
        GuardArtifactIssue::PermissionMismatch => "permission_mismatch",
        GuardArtifactIssue::HookContractMismatch => "hook_contract_mismatch",
    }
}

pub(super) fn guard_managed_artifact_name(artifact: GuardManagedArtifact) -> String {
    match artifact {
        GuardManagedArtifact::HostHookWrapper(phase) => {
            format!("host_hook_wrapper:{}", phase.as_str())
        }
        artifact => artifact.kind().as_str().to_owned(),
    }
}

pub(super) fn guard_manifest_issue_name(issue: GuardManifestIssue) -> &'static str {
    match issue {
        GuardManifestIssue::Malformed => "malformed",
        GuardManifestIssue::OwnershipMismatch => "ownership_mismatch",
    }
}

pub(super) fn latest_timestamp(
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
