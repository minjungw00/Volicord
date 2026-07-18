use std::{collections::BTreeMap, path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use volicord_store::{
    agent_connections::{AgentConnectionRecord, ConnectionProjectRecord},
    operational_sessions::{
        connection_integration_revision, latest_current_managed_runtime_session,
        latest_managed_runtime_session, McpRuntimeSessionRecord,
    },
};
use volicord_types::{
    ConnectionAction, ConnectionCheck, ConnectionCheckDetails, ConnectionCheckId,
    ConnectionCheckStatus, ConnectionStatus, ConnectionVerificationReport, UtcTimestamp,
};

use crate::host_integration::{
    codex::{self, CodexAdapter},
    verification::{HostExecutableStatus, ManagedConfigStatus, ProjectTrustStatus, Verification},
    HostAdapter, HostKind, HostPlan, HostScope, UserAction, UserActionKind,
};

use super::{
    codex_environment, guard_state_for_connection, mcp_process::run_connection_preflight,
    parse_host_kind, ConnectionCommandError, ConnectionProcess, GuardOperationalState, McpLaunch,
    McpVerification,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum AgentResultStatus {
    Complete,
    ActionRequired,
    Failed,
}

impl AgentResultStatus {
    pub(in crate::connection_command) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub(in crate::connection_command) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::connection_command) struct VerificationStep {
    pub(in crate::connection_command) status: StepStatus,
    pub(in crate::connection_command) code: String,
    pub(in crate::connection_command) details: String,
    pub(in crate::connection_command) preflight_diagnostics: Option<McpPreflightDiagnostics>,
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
        }
    }

    pub(in crate::connection_command) fn skipped(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Skipped,
            code: "pending".to_owned(),
            details: details.into(),
            preflight_diagnostics: None,
        }
    }

    pub(in crate::connection_command) fn with_preflight_diagnostics(
        mut self,
        diagnostics: Option<McpPreflightDiagnostics>,
    ) -> Self {
        self.preflight_diagnostics = diagnostics;
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
    pub(in crate::connection_command) fn from_preflight_report(
        report: &BTreeMap<String, String>,
    ) -> Option<Self> {
        Some(Self {
            storage_read: report.get("project_state_read")?.to_owned(),
            storage_write: report.get("project_state_write")?.to_owned(),
            effective_tool_mode: report.get("effective_tool_mode")?.to_owned(),
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
    pub(in crate::connection_command) host: Verification,
}

impl VerificationReport {
    pub(in crate::connection_command) fn status(&self) -> AgentResultStatus {
        agent_result_status(self.report.status())
    }
}

pub(in crate::connection_command) fn verify_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    launch: &McpLaunch,
    project_id: Option<&str>,
    process: &mut impl ConnectionProcess,
) -> Result<VerificationReport, ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let host = verify_host_plan(host_kind, host_plan, process)?;
    let preflight = run_connection_preflight(
        process,
        launch,
        runtime_home,
        &connection.connection_internal_id,
        project_id,
        &connection.mode,
    );
    let handshake = if preflight.status == StepStatus::Passed {
        match process.verify_mcp_stdio(
            launch,
            runtime_home,
            &connection.connection_internal_id,
            &connection.mode,
        ) {
            Ok(verification) => verification,
            Err(error) => McpVerification::failed(error),
        }
    } else {
        McpVerification {
            step: VerificationStep::skipped(
                "MCP server self-test did not run after failed preflight",
            ),
            tools: Vec::new(),
        }
    };
    let guard = guard_state_for_connection(
        runtime_home,
        connection,
        &volicord_store::agent_connections::list_connection_projects_for_diagnostics(
            runtime_home,
            &connection.connection_internal_id,
        )?,
    )?;
    let report = canonical_verification_report(
        runtime_home,
        connection,
        &host,
        &preflight,
        &handshake.step,
        &handshake.tools,
        &guard,
    )?;
    Ok(VerificationReport { report, host })
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
        .filter(|check| check.id().as_str() != "managed_config")
        .cloned()
        .collect::<Vec<_>>();
    checks.push(canonical_check(
        "managed_config",
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

fn canonical_verification_report(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host: &Verification,
    preflight: &VerificationStep,
    handshake: &VerificationStep,
    tools: &[String],
    guard: &GuardOperationalState,
) -> Result<ConnectionVerificationReport, ConnectionCommandError> {
    let current_revision = connection_integration_revision(connection)?;
    let current_session =
        latest_current_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let mut checks = vec![
        managed_config_check(host)?,
        host_executable_check(host)?,
        mcp_server_check(preflight, handshake, tools)?,
        project_trust_check(host)?,
    ];
    checks.extend(host_session_checks(
        host,
        current_revision.as_str(),
        current_session.as_ref(),
        latest_session.as_ref(),
    )?);
    checks.extend(guard_checks(guard)?);
    let actions = actions_for_checks(&checks)?;
    ConnectionVerificationReport::try_new(current_timestamp(), checks, actions)
        .map_err(ConnectionCommandError::from)
}

fn managed_config_check(host: &Verification) -> Result<ConnectionCheck, ConnectionCommandError> {
    let (status, code, summary) = match host.managed_config {
        ManagedConfigStatus::Match => (
            ConnectionCheckStatus::Passed,
            "managed_config_matches",
            "Managed Codex configuration matches the canonical entry",
        ),
        ManagedConfigStatus::Missing => (
            ConnectionCheckStatus::Failed,
            "managed_config_missing",
            "Required managed Codex configuration is missing",
        ),
        ManagedConfigStatus::Unmanaged => (
            ConnectionCheckStatus::Failed,
            "managed_config_ownership_conflict",
            "The managed Codex server name has an ownership conflict",
        ),
        ManagedConfigStatus::Changed => (
            ConnectionCheckStatus::Failed,
            "managed_config_mismatch",
            "Managed Codex configuration differs from the canonical entry",
        ),
        ManagedConfigStatus::Malformed => (
            ConnectionCheckStatus::Failed,
            "managed_config_malformed",
            "Managed Codex configuration is malformed",
        ),
        ManagedConfigStatus::Unavailable | ManagedConfigStatus::Unknown => (
            ConnectionCheckStatus::Failed,
            "managed_config_unavailable",
            "Managed Codex configuration could not be inspected",
        ),
    };
    canonical_check(
        "managed_config",
        status,
        code,
        summary,
        Some(json!({
            "target": host.config_target,
            "diagnostic_code": code,
            "observed_state": host.managed_config.as_str(),
            "diagnostic": host.managed_config_details,
        })),
        None,
    )
}

fn host_executable_check(host: &Verification) -> Result<ConnectionCheck, ConnectionCommandError> {
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
        "host_executable",
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

fn mcp_server_check(
    preflight: &VerificationStep,
    handshake: &VerificationStep,
    tools: &[String],
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let (status, code, summary) = if preflight.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            preflight.code.as_str(),
            "Volicord CLI MCP preflight failed",
        )
    } else if handshake.status == StepStatus::Passed {
        (
            ConnectionCheckStatus::Passed,
            handshake.code.as_str(),
            "Volicord MCP server self-test passed",
        )
    } else if handshake.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            handshake.code.as_str(),
            "Volicord MCP server self-test failed",
        )
    } else {
        (
            ConnectionCheckStatus::Failed,
            "mcp_server_self_test_not_run",
            "Volicord MCP server self-test did not run",
        )
    };
    canonical_check(
        "mcp_server",
        status,
        code,
        summary,
        Some(json!({
            "preflight": {
                "status": preflight.status.as_str(),
                "code": preflight.code,
                "diagnostic": preflight.details,
                "storage": preflight.preflight_diagnostics.as_ref().map(McpPreflightDiagnostics::to_json),
            },
            "self_test": {
                "status": handshake.status.as_str(),
                "code": handshake.code,
                "diagnostic": handshake.details,
                "initialize": handshake.status == StepStatus::Passed,
                "tools_list": tools,
                "safe_read_only_tool": "volicord.list_projects",
            }
        })),
        None,
    )
}

fn project_trust_check(host: &Verification) -> Result<ConnectionCheck, ConnectionCommandError> {
    let Some(trust) = host.project_trust.as_ref() else {
        return canonical_check(
            "project_trust",
            ConnectionCheckStatus::Passed,
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
        "project_trust",
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

fn host_session_checks(
    host: &Verification,
    current_revision: &str,
    current: Option<&McpRuntimeSessionRecord>,
    latest: Option<&McpRuntimeSessionRecord>,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let current = current.filter(|session| {
        session.session_source == volicord_types::McpRuntimeSessionSource::ManagedHost
    });
    let latest = latest.filter(|session| {
        session.session_source == volicord_types::McpRuntimeSessionSource::ManagedHost
    });
    let observed = current.or(latest);
    let observed_version = observed.and_then(|session| {
        session
            .observed_host_executable_version
            .as_ref()
            .or(session.client_version.as_ref())
    });
    let version_stale = current.is_some()
        && host.host_version.as_deref().is_some()
        && observed_version.is_some()
        && host.host_version.as_deref() != observed_version.map(String::as_str);
    let details = json!({
        "current_integration_revision": current_revision,
        "observed_integration_revision": observed.map(|session| session.connection_integration_revision.as_str()),
        "current_host_version": host.host_version,
        "observed_host_version": observed_version,
        "runtime_session_id": observed.map(|session| session.runtime_session_id.as_str()),
        "client_name": observed.and_then(|session| session.client_name.as_deref()),
        "client_version": observed.and_then(|session| session.client_version.as_deref()),
        "negotiated_protocol_version": observed.and_then(|session| session.negotiated_protocol_version.as_deref()),
        "last_observed_at": observed.map(|session| session.last_observed_at.as_str()),
        "terminal_failure_code": observed.and_then(|session| session.terminal_protocol_failure_code.as_deref()),
        "terminal_failure_details": observed.and_then(|session| session.terminal_protocol_failure_details.as_deref()),
    });

    let (session_status, session_code, session_summary, session_observed_at) = match current {
        None if latest.is_some() => (
            ConnectionCheckStatus::Pending,
            "host_session_revision_stale",
            "Managed host has not loaded the current connection revision",
            latest.map(|session| session.last_observed_at.as_str()),
        ),
        None => (
            ConnectionCheckStatus::Pending,
            "host_session_not_observed",
            "Managed host connection use has not been observed",
            None,
        ),
        Some(session) if version_stale => (
            ConnectionCheckStatus::Pending,
            "host_version_observation_stale",
            "Codex version changed after the latest managed-host observation",
            Some(session.last_observed_at.as_str()),
        ),
        Some(session) if session.initialize_completed_at.is_some() => (
            ConnectionCheckStatus::Passed,
            "host_session_initialized",
            "Current managed-host session completed MCP initialize",
            Some(session.last_observed_at.as_str()),
        ),
        Some(session) if session.terminal_protocol_failure_code.is_some() => (
            ConnectionCheckStatus::Failed,
            "host_session_initialize_failed",
            "Current managed-host session failed before MCP initialize completed",
            Some(session.last_observed_at.as_str()),
        ),
        Some(session) => (
            ConnectionCheckStatus::Pending,
            "host_session_initialize_pending",
            "Current managed-host session has not completed MCP initialize",
            Some(session.last_observed_at.as_str()),
        ),
    };
    let host_session = canonical_check(
        "host_session",
        session_status,
        session_code,
        session_summary,
        Some(details.clone()),
        session_observed_at,
    )?;

    let (tools_status, tools_code, tools_summary, tools_observed_at) = match current {
        None => (
            ConnectionCheckStatus::Pending,
            "required_tools_not_observed",
            "Current managed host has not reported tools/list",
            None,
        ),
        Some(session) if version_stale => (
            ConnectionCheckStatus::Pending,
            "required_tools_observation_stale",
            "Required-tool observation predates the current Codex version",
            Some(session.last_observed_at.as_str()),
        ),
        Some(session) if session.required_tools_present == Some(true) => (
            ConnectionCheckStatus::Passed,
            "required_tools_present",
            "Current managed host exposed every required tool",
            session.tools_list_observed_at.as_deref(),
        ),
        Some(session) if session.required_tools_present == Some(false) => (
            ConnectionCheckStatus::Failed,
            "required_tools_missing",
            "Current managed host is missing one or more required tools",
            session.tools_list_observed_at.as_deref(),
        ),
        Some(session)
            if session.initialize_completed_at.is_some()
                && session.terminal_protocol_failure_code.is_some() =>
        {
            (
                ConnectionCheckStatus::Failed,
                "required_tools_invalid",
                "Current managed-host tool discovery ended in a protocol failure",
                Some(session.last_observed_at.as_str()),
            )
        }
        Some(session) => (
            ConnectionCheckStatus::Pending,
            "required_tools_not_observed",
            "Current managed host has not reported tools/list",
            Some(session.last_observed_at.as_str()),
        ),
    };
    let required_tools = canonical_check(
        "required_tools",
        tools_status,
        tools_code,
        tools_summary,
        Some(details.clone()),
        tools_observed_at,
    )?;

    let (round_trip_status, round_trip_code, round_trip_summary, round_trip_observed_at) =
        match current {
            None => (
                ConnectionCheckStatus::Pending,
                "tool_round_trip_not_observed",
                "Current managed host has not completed the designated read-only Volicord tool call",
                None,
            ),
            Some(session) if version_stale => (
                ConnectionCheckStatus::Pending,
                "tool_round_trip_observation_stale",
                "Designated read-only tool-call observation predates the current Codex version",
                Some(session.last_observed_at.as_str()),
            ),
            Some(session) if session.last_safe_read_only_tool_call_at.is_some() => (
                ConnectionCheckStatus::Passed,
                "tool_round_trip_passed",
                "Current managed host completed the designated read-only Volicord tool call",
                session.last_safe_read_only_tool_call_at.as_deref(),
            ),
            Some(session)
                if session.required_tools_present == Some(true)
                    && session.terminal_protocol_failure_code.is_some() =>
            {
                (
                ConnectionCheckStatus::Failed,
                "tool_round_trip_failed",
                "Current managed-host session reported a protocol or contract failure",
                Some(session.last_observed_at.as_str()),
                )
            }
            Some(session) => (
                ConnectionCheckStatus::Pending,
                "tool_round_trip_not_observed",
                "Current managed host has not completed the designated read-only Volicord tool call",
                Some(session.last_observed_at.as_str()),
            ),
        };
    let tool_round_trip = canonical_check(
        "tool_round_trip",
        round_trip_status,
        round_trip_code,
        round_trip_summary,
        Some(details),
        round_trip_observed_at,
    )?;
    Ok(vec![host_session, required_tools, tool_round_trip])
}

fn guard_checks(
    guard: &GuardOperationalState,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let files_status = if guard.generated_config_verified
        && guard.missing_files.is_empty()
        && guard.stale_files.is_empty()
        && guard.broken_files.is_empty()
    {
        ConnectionCheckStatus::Passed
    } else if matches!(
        guard.installation_state.as_str(),
        "planned" | "reload_required"
    ) {
        ConnectionCheckStatus::Pending
    } else {
        ConnectionCheckStatus::Failed
    };
    let observation_status = match guard.hook_observed_state.as_str() {
        "observed" => ConnectionCheckStatus::Passed,
        "failed" => ConnectionCheckStatus::Failed,
        _ => ConnectionCheckStatus::Pending,
    };
    let mut affected_paths = guard.missing_files.clone();
    affected_paths.extend(guard.stale_files.iter().cloned());
    affected_paths.extend(guard.broken_files.iter().cloned());
    affected_paths.sort();
    affected_paths.dedup();
    let file_facts = json!({
        "manifest_audit_passed": files_status == ConnectionCheckStatus::Passed,
        "affected_paths": affected_paths,
        "required_hook_gaps": guard.missing_required_hooks,
    });
    let observation_facts = json!({
        "hook_activity_observed": guard.hook_observed_state == "observed",
        "prompt_capture_observed": matches!(
            guard.prompt_capture_state.as_str(),
            "active" | "observed"
        ),
    });
    Ok(vec![
        canonical_check(
            "guard_files",
            files_status,
            match files_status {
                ConnectionCheckStatus::Passed => "guard_files_passed",
                ConnectionCheckStatus::Pending => "guard_files_reload_pending",
                ConnectionCheckStatus::Failed => "guard_files_failed",
            },
            "Guard managed files were checked",
            Some(file_facts),
            guard.last_observed_at.as_deref(),
        )?,
        canonical_check(
            "guard_observation",
            observation_status,
            match observation_status {
                ConnectionCheckStatus::Passed => "guard_observation_passed",
                ConnectionCheckStatus::Pending => "guard_observation_pending",
                ConnectionCheckStatus::Failed => "guard_observation_failed",
            },
            "Current Guard hook phases were checked",
            Some(observation_facts),
            guard.last_observed_at.as_deref(),
        )?,
    ])
}

fn actions_for_checks(
    checks: &[ConnectionCheck],
) -> Result<Vec<ConnectionAction>, ConnectionCommandError> {
    let mut actions = BTreeMap::<&str, (&str, Option<String>)>::new();
    for check in checks {
        match (check.id().as_str(), check.status()) {
            ("managed_config", ConnectionCheckStatus::Failed) => {
                actions.insert(
                    "repair_managed_config",
                    (
                        "Repair or recreate the Volicord-managed Codex MCP entry",
                        None,
                    ),
                );
            }
            ("host_executable", ConnectionCheckStatus::Failed) => {
                actions.insert(
                    "install_or_repair_codex",
                    (
                        "Install or repair Codex so `codex --version` succeeds on PATH",
                        None,
                    ),
                );
            }
            ("mcp_server", ConnectionCheckStatus::Failed) => {
                actions.insert(
                    "repair_mcp_server",
                    (
                        "Repair the Volicord MCP configuration or storage error and verify again",
                        Some("volicord connection verify".to_owned()),
                    ),
                );
            }
            ("project_trust", ConnectionCheckStatus::Pending) => {
                actions.insert(
                    "host_trust_required",
                    (
                        "Trust the project in Codex, then restart or reload Codex",
                        None,
                    ),
                );
            }
            (
                "host_session" | "required_tools" | "tool_round_trip" | "guard_observation",
                ConnectionCheckStatus::Pending,
            ) => {
                actions.insert(
                    "observe_codex",
                    (
                        "Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool so actual Codex connection and Guard activity can be observed",
                        None,
                    ),
                );
            }
            (
                "host_session" | "required_tools" | "tool_round_trip" | "guard_observation",
                ConnectionCheckStatus::Failed,
            ) => {
                actions.insert(
                    "inspect_codex_protocol",
                    (
                        "Inspect the recorded Codex protocol failure, repair the incompatible configuration or behavior, then verify again",
                        Some("volicord connection verify".to_owned()),
                    ),
                );
            }
            ("guard_files", ConnectionCheckStatus::Failed) => {
                actions.insert(
                    "repair_guard",
                    (
                        "Repair the Volicord Guard integration and verify the connection again",
                        None,
                    ),
                );
            }
            _ => {}
        }
    }
    actions
        .into_iter()
        .map(|(id, (instruction, command))| {
            ConnectionAction::try_new(id, instruction, command)
                .map_err(ConnectionCommandError::from)
        })
        .collect()
}

fn canonical_check(
    id: &str,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    details: Option<Value>,
    observed_at: Option<&str>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let details = details
        .map(compact_json_value)
        .map(|value| {
            let Value::Object(object) = value else {
                return Err(ConnectionCommandError::runtime(
                    "connection check details must be a JSON object",
                ));
            };
            ConnectionCheckDetails::try_new(object).map_err(ConnectionCommandError::from)
        })
        .transpose()?;
    let observed_at = observed_at
        .map(|value| {
            UtcTimestamp::from_str(value).map_err(|_| {
                ConnectionCommandError::runtime(format!(
                    "connection check observation time is invalid: {value}"
                ))
            })
        })
        .transpose()?;
    ConnectionCheck::try_new(
        ConnectionCheckId::new(id),
        status,
        (status != ConnectionCheckStatus::Passed).then(|| code.to_owned()),
        summary,
        details,
        observed_at,
    )
    .map_err(ConnectionCommandError::from)
}

fn compact_json_value(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .filter_map(|(key, value)| {
                    (value != Value::Null).then(|| (key, compact_json_value(value)))
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(compact_json_value).collect()),
        other => other,
    }
}

fn verify_host_plan(
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
                .find(|check| check.id().as_str() == "host_executable")
        })
        .cloned();
    if let Some(check) = stored_executable.as_ref() {
        host.host_executable = match check.status() {
            ConnectionCheckStatus::Passed => HostExecutableStatus::Available,
            ConnectionCheckStatus::Failed => HostExecutableStatus::Unavailable,
            ConnectionCheckStatus::Pending => HostExecutableStatus::NotChecked,
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
                .find(|check| check.id().as_str() == "mcp_server")
        })
        .cloned()
        .unwrap_or(canonical_check(
            "mcp_server",
            ConnectionCheckStatus::Pending,
            "mcp_server_not_verified",
            "Volicord MCP server has not been actively verified",
            None,
            None,
        )?);
    let guard = guard_state_for_connection(runtime_home, connection, projects)?;
    let current_revision = connection_integration_revision(connection)?;
    let current_session =
        latest_current_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let latest_session =
        latest_managed_runtime_session(runtime_home, &connection.connection_internal_id)?;
    let mut checks = vec![
        managed_config_check(&host)?,
        stored_mcp,
        project_trust_check(&host)?,
    ];
    checks.push(stored_executable.unwrap_or(canonical_check(
        "host_executable",
        ConnectionCheckStatus::Pending,
        "host_executable_not_verified",
        "Codex executable has not been actively verified",
        None,
        None,
    )?));
    checks.extend(host_session_checks(
        &host,
        current_revision.as_str(),
        current_session.as_ref(),
        latest_session.as_ref(),
    )?);
    checks.extend(guard_checks(&guard)?);
    let actions = actions_for_checks(&checks)?;
    let report = ConnectionVerificationReport::try_new(current_timestamp(), checks, actions)?;
    Ok((Some(host), report))
}

pub(in crate::connection_command) fn connection_status_actions(
    _current_host: Option<&Verification>,
    report: &ConnectionVerificationReport,
) -> Vec<UserAction> {
    let mut actions = report
        .actions()
        .iter()
        .map(|action| UserAction::new(user_action_kind(action.id()), action.instruction()))
        .collect::<Vec<_>>();
    actions.sort_by(|left, right| left.message.cmp(&right.message));
    actions.dedup_by(|left, right| left.kind == right.kind && left.message == right.message);
    actions
}

fn user_action_kind(id: &str) -> UserActionKind {
    match id {
        "host_trust_required" => UserActionKind::HostTrustRequired,
        "repair_managed_config" => UserActionKind::RepairManagedConfig,
        "install_or_repair_codex" => UserActionKind::InstallOrRepairCodex,
        "repair_mcp_server" => UserActionKind::RepairMcpServer,
        "reload_host" => UserActionKind::ReloadHost,
        "use_volicord_tool" => UserActionKind::UseVolicordTool,
        "reload_guard" => UserActionKind::ReloadGuard,
        "repair_guard" => UserActionKind::RepairGuard,
        _ => UserActionKind::ReloadRequired,
    }
}

pub(in crate::connection_command) fn agent_result_status(
    status: ConnectionStatus,
) -> AgentResultStatus {
    match status {
        ConnectionStatus::Complete => AgentResultStatus::Complete,
        ConnectionStatus::ActionRequired => AgentResultStatus::ActionRequired,
        ConnectionStatus::Failed => AgentResultStatus::Failed,
    }
}

fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(version: &str) -> Verification {
        Verification {
            config_target: "/tmp/codex/config.toml".to_owned(),
            managed_config: ManagedConfigStatus::Match,
            managed_config_details: "matches".to_owned(),
            host_executable: HostExecutableStatus::Available,
            executable_path: Some("/opt/codex/bin/codex".to_owned()),
            host_version: Some(version.to_owned()),
            host_executable_code: "host_executable_available".to_owned(),
            host_executable_details: "version probe passed".to_owned(),
            project_trust: None,
        }
    }

    fn managed_session(version: &str, required_tools_present: bool) -> McpRuntimeSessionRecord {
        McpRuntimeSessionRecord {
            runtime_session_id: "mcp_runtime_fixture".to_owned(),
            connection_internal_id: "connection_fixture".to_owned(),
            session_source: volicord_types::McpRuntimeSessionSource::ManagedHost,
            connection_integration_revision: "revision_current".to_owned(),
            observed_host_executable_version: None,
            client_name: Some("codex".to_owned()),
            client_version: Some(version.to_owned()),
            negotiated_protocol_version: Some("2025-11-25".to_owned()),
            process_id: 42,
            process_started_at: "2026-07-18T00:00:00Z".to_owned(),
            initialize_completed_at: Some("2026-07-18T00:00:01Z".to_owned()),
            initialized_notification_at: Some("2026-07-18T00:00:02Z".to_owned()),
            tools_list_observed_at: Some("2026-07-18T00:00:03Z".to_owned()),
            required_tools_present: Some(required_tools_present),
            last_safe_read_only_tool_call_at: Some("2026-07-18T00:00:04Z".to_owned()),
            last_observed_at: "2026-07-18T00:00:04Z".to_owned(),
            terminal_protocol_failure_code: None,
            terminal_protocol_failure_details: None,
            graceful_close_at: None,
        }
    }

    #[test]
    fn arbitrary_future_version_can_complete_managed_host_checks() {
        let host = host("999.123-preview+custom");
        let session = managed_session("999.123-preview+custom", true);

        let session_checks =
            host_session_checks(&host, "revision_current", Some(&session), Some(&session))
                .expect("valid checks");

        assert!(session_checks
            .iter()
            .all(|check| check.status() == ConnectionCheckStatus::Passed));
        let mut checks = vec![
            managed_config_check(&host).expect("managed config check"),
            host_executable_check(&host).expect("host executable check"),
            project_trust_check(&host).expect("project trust check"),
            canonical_check(
                "mcp_server",
                ConnectionCheckStatus::Passed,
                "mcp_server_ready",
                "MCP server passed",
                None,
                None,
            )
            .expect("MCP check"),
        ];
        checks.extend(session_checks);
        for id in ["guard_files", "guard_observation"] {
            checks.push(
                canonical_check(
                    id,
                    ConnectionCheckStatus::Passed,
                    &format!("{id}_passed"),
                    "Guard check passed",
                    None,
                    None,
                )
                .expect("Guard check"),
            );
        }
        let report = ConnectionVerificationReport::try_new(
            current_timestamp(),
            checks.clone(),
            actions_for_checks(&checks).expect("actions"),
        )
        .expect("canonical report");
        assert_eq!(report.status(), ConnectionStatus::Complete);
    }

    #[test]
    fn host_version_change_requires_new_observation_without_rejection() {
        let host = host("1000.0-new-host");
        let session = managed_session("999.123-preview+custom", true);

        let checks = host_session_checks(&host, "revision_current", Some(&session), Some(&session))
            .expect("valid checks");

        assert!(checks
            .iter()
            .all(|check| check.status() == ConnectionCheckStatus::Pending));
        assert_eq!(checks[0].code(), Some("host_version_observation_stale"));
    }

    #[test]
    fn old_revision_and_cli_preflight_observations_remain_action_required() {
        let host = host("future");
        let mut old = managed_session("future", true);
        old.connection_integration_revision = "revision_old".to_owned();
        let stale =
            host_session_checks(&host, "revision_current", None, Some(&old)).expect("stale checks");
        assert!(stale
            .iter()
            .all(|check| check.status() == ConnectionCheckStatus::Pending));
        assert_eq!(stale[0].code(), Some("host_session_revision_stale"));

        old.session_source = volicord_types::McpRuntimeSessionSource::CliPreflight;
        let cli = host_session_checks(&host, "revision_current", Some(&old), Some(&old))
            .expect("CLI-preflight checks");
        assert!(cli
            .iter()
            .all(|check| check.status() == ConnectionCheckStatus::Pending));
        assert_eq!(cli[0].code(), Some("host_session_not_observed"));
    }

    #[test]
    fn actual_current_protocol_incompatibility_fails_only_demonstrated_checks() {
        let host = host("future");
        let mut session = managed_session("future", true);
        session.last_safe_read_only_tool_call_at = None;
        session.terminal_protocol_failure_code = Some("protocol_contract_mismatch".to_owned());
        session.terminal_protocol_failure_details = Some("read-only call failed".to_owned());
        let checks = host_session_checks(&host, "revision_current", Some(&session), Some(&session))
            .expect("protocol checks");

        assert_eq!(checks[0].status(), ConnectionCheckStatus::Passed);
        assert_eq!(checks[1].status(), ConnectionCheckStatus::Passed);
        assert_eq!(checks[2].status(), ConnectionCheckStatus::Failed);
        assert_eq!(checks[2].code(), Some("tool_round_trip_failed"));
    }

    #[test]
    fn initialize_failure_does_not_invent_tool_surface_or_call_failures() {
        let host = host("future");
        let mut session = managed_session("future", true);
        session.initialize_completed_at = None;
        session.initialized_notification_at = None;
        session.tools_list_observed_at = None;
        session.required_tools_present = None;
        session.last_safe_read_only_tool_call_at = None;
        session.terminal_protocol_failure_code = Some("mcp_transport_failure".to_owned());
        session.terminal_protocol_failure_details = Some("initialize failed".to_owned());
        let checks = host_session_checks(&host, "revision_current", Some(&session), Some(&session))
            .expect("protocol checks");

        assert_eq!(checks[0].status(), ConnectionCheckStatus::Failed);
        assert_eq!(checks[1].status(), ConnectionCheckStatus::Pending);
        assert_eq!(checks[2].status(), ConnectionCheckStatus::Pending);
    }

    #[test]
    fn tool_discovery_failure_does_not_invent_a_tool_call_failure() {
        let host = host("future");
        let mut session = managed_session("future", true);
        session.tools_list_observed_at = None;
        session.required_tools_present = None;
        session.last_safe_read_only_tool_call_at = None;
        session.terminal_protocol_failure_code = Some("mcp_transport_failure".to_owned());
        session.terminal_protocol_failure_details = Some("tools/list failed".to_owned());
        let checks = host_session_checks(&host, "revision_current", Some(&session), Some(&session))
            .expect("protocol checks");

        assert_eq!(checks[0].status(), ConnectionCheckStatus::Passed);
        assert_eq!(checks[1].status(), ConnectionCheckStatus::Failed);
        assert_eq!(checks[2].status(), ConnectionCheckStatus::Pending);
    }

    #[test]
    fn fresh_setup_without_host_observation_is_action_required() {
        let host = host("unlisted-future-version");
        let mut checks = vec![
            managed_config_check(&host).expect("managed config check"),
            host_executable_check(&host).expect("host executable check"),
            project_trust_check(&host).expect("project trust check"),
            canonical_check(
                "mcp_server",
                ConnectionCheckStatus::Passed,
                "mcp_server_ready",
                "MCP server passed",
                None,
                None,
            )
            .expect("MCP check"),
        ];
        checks.extend(
            host_session_checks(&host, "revision_current", None, None)
                .expect("pending host checks"),
        );
        let report = ConnectionVerificationReport::try_new(
            current_timestamp(),
            checks.clone(),
            actions_for_checks(&checks).expect("actions"),
        )
        .expect("canonical report");

        assert_eq!(report.status(), ConnectionStatus::ActionRequired);
        assert_eq!(
            report
                .actions()
                .iter()
                .map(ConnectionAction::id)
                .collect::<Vec<_>>(),
            vec!["observe_codex"]
        );
    }

    #[test]
    fn managed_config_failures_keep_precise_codes() {
        let cases = [
            (ManagedConfigStatus::Missing, "managed_config_missing"),
            (
                ManagedConfigStatus::Unmanaged,
                "managed_config_ownership_conflict",
            ),
            (ManagedConfigStatus::Changed, "managed_config_mismatch"),
            (ManagedConfigStatus::Malformed, "managed_config_malformed"),
            (
                ManagedConfigStatus::Unavailable,
                "managed_config_unavailable",
            ),
        ];
        for (status, expected_code) in cases {
            let mut host = host("future");
            host.managed_config = status;
            let check = managed_config_check(&host).expect("managed config check");
            assert_eq!(check.status(), ConnectionCheckStatus::Failed);
            assert_eq!(check.code(), Some(expected_code));
        }
    }

    #[test]
    fn unavailable_executable_is_a_failed_behavioral_check() {
        let mut host = host("future");
        host.host_executable = HostExecutableStatus::Unavailable;
        host.host_executable_code = "host_executable_probe_failed".to_owned();
        host.host_version = None;
        let check = host_executable_check(&host).expect("host executable check");
        assert_eq!(check.status(), ConnectionCheckStatus::Failed);
        assert_eq!(check.code(), Some("host_executable_probe_failed"));
    }

    #[test]
    fn aggregation_and_user_actions_are_deterministic() {
        let checks = vec![
            canonical_check(
                "tool_round_trip",
                ConnectionCheckStatus::Pending,
                "tool_round_trip_not_observed",
                "Tool call pending",
                None,
                None,
            )
            .expect("tool check"),
            canonical_check(
                "managed_config",
                ConnectionCheckStatus::Failed,
                "managed_config_malformed",
                "Config malformed",
                None,
                None,
            )
            .expect("config check"),
            canonical_check(
                "host_session",
                ConnectionCheckStatus::Pending,
                "host_session_not_observed",
                "Host session pending",
                None,
                None,
            )
            .expect("host check"),
            canonical_check(
                "mcp_server",
                ConnectionCheckStatus::Failed,
                "mcp_server_protocol_failed",
                "MCP failed",
                None,
                None,
            )
            .expect("MCP check"),
        ];
        let first = actions_for_checks(&checks).expect("actions");
        let second = actions_for_checks(&checks).expect("repeat actions");
        assert_eq!(first, second);
        assert_eq!(
            first.iter().map(ConnectionAction::id).collect::<Vec<_>>(),
            vec![
                "observe_codex",
                "repair_managed_config",
                "repair_mcp_server",
            ]
        );
        let report = ConnectionVerificationReport::try_new(current_timestamp(), checks, first)
            .expect("canonical report");
        assert_eq!(report.status(), ConnectionStatus::Failed);
    }
}
