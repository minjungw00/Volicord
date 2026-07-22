//! Host configuration, executable, trust, and managed-session checks.

use super::*;

pub(super) fn managed_config_check(
    host: &Verification,
) -> Result<ConnectionCheck, ConnectionCommandError> {
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

pub(super) fn host_executable_check(
    host: &Verification,
) -> Result<ConnectionCheck, ConnectionCommandError> {
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

pub(super) fn project_trust_check(
    host: &Verification,
) -> Result<ConnectionCheck, ConnectionCommandError> {
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

pub(super) fn host_session_checks(
    host: &Verification,
    current_revision: &str,
    current: &[McpRuntimeSessionRecord],
    latest: Option<&McpRuntimeSessionRecord>,
    tool_round_trip_finding_ids: &[DiagnosticFindingId],
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
    let expected_verification_tool_name = super::super::managed_host_round_trip_tool().wire_name();
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
            "expected_verification_tool_name": expected_verification_tool_name,
            "observed_verification_tool_name": observed.and_then(|session| session.verification_tool_name.as_deref()),
            "verification_tool_observed_at": observed.and_then(|session| session.verification_tool_observed_at.as_deref()),
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
        version_fresh(session)
            && session.verification_tool_name.as_deref() == Some(expected_verification_tool_name)
            && session.verification_tool_observed_at.is_some()
    });
    let designation_mismatch = current.iter().copied().find(|session| {
        version_fresh(session)
            && session.verification_tool_observed_at.is_some()
            && session
                .verification_tool_name
                .as_deref()
                .is_some_and(|observed| observed != expected_verification_tool_name)
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

    let (
        round_trip_status,
        round_trip_code,
        round_trip_summary,
        round_trip_observed_at,
        round_detail,
    ) = match (round_trip, designation_mismatch, diagnostic) {
        (Some(session), _, _) => (
            ConnectionCheckStatus::Passed,
            "tool_round_trip_passed",
            "A current managed host completed the canonical verification tool call",
            session.verification_tool_observed_at.as_deref(),
            Some(session),
        ),
        (None, Some(session), _) => (
            ConnectionCheckStatus::Failed,
            "tool_round_trip_designation_mismatch",
            "The observed verification tool does not match the canonical role owner",
            session.verification_tool_observed_at.as_deref(),
            Some(session),
        ),
        (None, None, None) => (
            ConnectionCheckStatus::Pending,
            "tool_round_trip_not_observed",
            "Current managed host has not completed the canonical verification tool call",
            None,
            latest,
        ),
        (None, None, Some(session)) if !version_fresh(session) => (
            ConnectionCheckStatus::Pending,
            "tool_round_trip_observation_stale",
            "Newest verification-tool observation predates the current Codex version",
            Some(session.last_observed_at.as_str()),
            Some(session),
        ),
        (None, None, Some(session))
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
        (None, None, Some(session)) => (
            ConnectionCheckStatus::Pending,
            "tool_round_trip_not_observed",
            "Newest current managed host has not completed the canonical verification tool call",
            Some(session.last_observed_at.as_str()),
            Some(session),
        ),
    };
    let mut round_trip_causes = terminal_cause_ids(round_detail)?;
    if designation_mismatch.is_some() {
        round_trip_causes.extend_from_slice(tool_round_trip_finding_ids);
    }
    round_trip_causes.sort();
    round_trip_causes.dedup();
    let tool_round_trip = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::ToolRoundTrip,
            round_trip_status,
            round_trip_code,
            round_trip_summary,
            Some(details(round_detail)),
            round_trip_observed_at,
        )?,
        round_trip_causes,
    )?;
    block_failed_dependencies(vec![
        process_startup,
        host_session,
        required_tools,
        tool_round_trip,
    ])
}

pub(super) fn verify_host_plan(
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
