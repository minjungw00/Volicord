//! Active verification and current-status report input assembly.

use super::*;

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

pub(super) fn canonical_verification_report(
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
        &host_findings.tool_round_trip,
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
    let mut tool_round_trip_causes = Vec::new();
    for session in current_sessions.iter().filter(|session| {
        session.verification_tool_observed_at.is_some()
            && session.verification_tool_name.as_deref()
                != Some(super::super::managed_host_round_trip_tool().wire_name())
    }) {
        let subject = VerificationToolSubject::for_runtime_session(
            &connection.connection_internal_id,
            &session.runtime_session_id,
        )
        .map_err(ConnectionCommandError::runtime)?;
        let projected = current_connection_finding(
            connection,
            OperationalDiagnostic::ToolVerification(
                ToolVerificationDiagnostic::DesignationMismatch,
            ),
            &subject,
            &VerificationToolFacts::new(
                super::super::managed_host_round_trip_tool().wire_name(),
                session
                    .verification_tool_name
                    .as_deref()
                    .expect("filtered verification-tool observation has a name"),
            ),
            OperationalCheckState::Failed,
            current_timestamp(),
        )?;
        if persisted_diagnostic_finding(runtime_home, projected.id())?.is_some() {
            tool_round_trip_causes.push(projected.id().clone());
        }
    }
    let managed_config_causes = host
        .managed_config_diagnostic
        .map(|diagnostic| {
            let subject = ManagedConfigurationTarget::for_connection(
                &connection.connection_internal_id,
                Path::new(&connection.config_target),
            )
            .map_err(ConnectionCommandError::runtime)?;
            current_connection_finding(
                connection,
                OperationalDiagnostic::ManagedConfig(diagnostic),
                &subject,
                &ManagedConfigurationFacts::from_status(host.managed_config),
                OperationalCheckState::Failed,
                current_timestamp(),
            )
            .map(|finding| finding.id().clone())
        })
        .transpose()?
        .into_iter()
        .collect();
    let project_trust_causes = host
        .project_trust
        .as_ref()
        .and_then(|trust| {
            TrustDiagnostic::from_status(trust.status).map(|diagnostic| (trust, diagnostic))
        })
        .map(|(trust, diagnostic)| {
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
            current_connection_finding(
                connection,
                OperationalDiagnostic::Trust(diagnostic),
                &subject,
                &TrustFacts::from_status(trust.status),
                check_state,
                current_timestamp(),
            )
            .map(|finding| finding.id().clone())
        })
        .transpose()?
        .into_iter()
        .collect();
    let mut checks = vec![
        with_direct_causes(managed_config_check(&host)?, managed_config_causes)?,
        stored_mcp,
        with_direct_causes(project_trust_check(&host)?, project_trust_causes)?,
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
        &tool_round_trip_causes,
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
