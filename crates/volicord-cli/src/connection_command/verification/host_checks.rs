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
        Some(typed_details(
            &HostExecutableProbeDetails::from_verification(host),
        )?),
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
    current_revision: &IntegrationRevision,
    selection: &McpSessionEvidenceSelection,
    latest: Option<&McpRuntimeSessionRecord>,
    tool_round_trip_finding_ids: &[DiagnosticFindingId],
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let attempt = selection.latest_attempt.as_ref();
    let complete_proof = selection.latest_complete_proof.as_ref();
    let stale_observed_at = latest
        .filter(|session| {
            session.session_source
                == volicord_types::integration_revision::McpRuntimeSessionSource::ManagedHost
                && session.connection_integration_revision != current_revision.as_str()
        })
        .map(|session| session.last_observed_at.as_str());
    let attempt_details = || {
        typed_details(&ManagedSessionAttemptDetails::new(
            current_revision,
            attempt,
            host,
        ))
    };
    let proof_details = || {
        complete_proof
            .map(|proof| {
                typed_details(&ManagedCapabilityProofDetails::new(
                    current_revision,
                    proof,
                    host,
                ))
            })
            .transpose()
    };

    let process_started_at =
        attempt.map(|session| session.process_started_at.to_canonical_string());
    let process_startup = canonical_check(
        ConnectionCheckKind::HostReload,
        if attempt.is_some() {
            ConnectionCheckStatus::Passed
        } else {
            ConnectionCheckStatus::Pending
        },
        if attempt.is_some() {
            "host_reload_observed"
        } else {
            "host_reload_required"
        },
        if attempt.is_some() {
            "Codex loaded the current managed connection revision"
        } else {
            "Codex has not loaded the current managed connection revision"
        },
        Some(attempt_details()?),
        process_started_at.as_deref(),
    )?;

    let (session_status, session_code, session_summary, session_observed_at) = match attempt {
        Some(session) if session.terminally_failed() => (
            ConnectionCheckStatus::Failed,
            "host_session_current_attempt_failed",
            "The latest current managed-host attempt terminated with a protocol failure",
            Some(session.last_observed_at.to_canonical_string()),
        ),
        Some(session) if session.initialized_notification_at.is_some() => (
            ConnectionCheckStatus::Passed,
            "host_session_initialized",
            "The latest current managed-host attempt completed MCP initialize",
            session
                .initialized_notification_at
                .as_ref()
                .map(UtcTimestamp::to_canonical_string),
        ),
        Some(session) => (
            ConnectionCheckStatus::Pending,
            "host_session_initialize_pending",
            "The latest current managed-host attempt has not completed MCP initialize",
            Some(session.last_observed_at.to_canonical_string()),
        ),
        None if stale_observed_at.is_some() => (
            ConnectionCheckStatus::Pending,
            "host_session_revision_stale",
            "Managed host has not loaded the current connection revision",
            stale_observed_at.map(str::to_owned),
        ),
        None => (
            ConnectionCheckStatus::Pending,
            "host_session_not_observed",
            "Managed host connection use has not been observed",
            None,
        ),
    };
    let host_session = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::ManagedSessionHealth,
            session_status,
            session_code,
            session_summary,
            Some(attempt_details()?),
            session_observed_at.as_deref(),
        )?,
        milestone_terminal_cause_ids(attempt),
    )?;

    let designation_mismatch = attempt.is_some_and(|session| {
        session.verification_tool_observed_at.is_some()
            && session.verification_tool_name.as_deref() != Some(expected_verification_tool_name())
    });
    let (round_trip_status, round_trip_code, round_trip_summary, round_trip_observed_at) =
        match complete_proof {
        Some(proof) => (
            ConnectionCheckStatus::Passed,
            "tool_round_trip_passed",
            "One current-revision managed-host session completed the full same-session capability proof",
            proof
                .milestones()
                .verification_tool_observed_at
                .as_ref()
                .map(UtcTimestamp::to_canonical_string),
        ),
        None if attempt.is_some_and(|session| session.required_tools_present == Some(false)) => (
            ConnectionCheckStatus::Failed,
            "managed_capability_required_tools_missing",
            "The latest current managed-host attempt is missing required Volicord tools",
            attempt
                .and_then(|session| session.tools_list_observed_at.as_ref())
                .map(UtcTimestamp::to_canonical_string),
        ),
        None if designation_mismatch => (
            ConnectionCheckStatus::Failed,
            "tool_round_trip_designation_mismatch",
            "The observed verification tool does not match the canonical role owner",
            attempt
                .and_then(|session| session.verification_tool_observed_at.as_ref())
                .map(UtcTimestamp::to_canonical_string),
        ),
        None if attempt.is_some_and(McpSessionMilestones::terminally_failed) => (
            ConnectionCheckStatus::Failed,
            "tool_round_trip_failed",
            "The latest current managed-host attempt failed without completing the canonical verification tool call",
            attempt.map(|session| session.last_observed_at.to_canonical_string()),
        ),
        None => (
            ConnectionCheckStatus::Pending,
            "tool_round_trip_not_observed",
            "No current-revision managed-host session has completed the full same-session capability proof",
            attempt.map(|session| session.last_observed_at.to_canonical_string()),
        ),
    };
    let mut round_trip_causes = milestone_terminal_cause_ids(attempt);
    if designation_mismatch {
        round_trip_causes.extend_from_slice(tool_round_trip_finding_ids);
    }
    round_trip_causes.sort();
    round_trip_causes.dedup();
    let tool_round_trip = with_direct_causes(
        canonical_check(
            ConnectionCheckKind::ManagedCapabilityProof,
            round_trip_status,
            round_trip_code,
            round_trip_summary,
            Some(proof_details()?.unwrap_or(attempt_details()?)),
            round_trip_observed_at.as_deref(),
        )?,
        round_trip_causes,
    )?;
    block_failed_dependencies(vec![process_startup, host_session, tool_round_trip])
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
