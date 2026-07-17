use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use volicord_store::agent_connections::{
    list_connection_projects_for_diagnostics, AgentConnectionRecord, ConnectionProjectRecord,
    VERIFIED_STATUS_ACTION_REQUIRED, VERIFIED_STATUS_COMPLETE, VERIFIED_STATUS_FAILED,
    VERIFIED_STATUS_NOT_VERIFIED,
};
use volicord_types::HostVerificationReceipt;

use crate::host_integration::{
    codex::{self, CodexAdapter},
    verification::{
        ActiveToolExposureStatus, CliMcpStepStatus, CliMcpVerification, HostMcpCommandDiagnostic,
        HostMcpCommandLaunchMode, HostRuntimeDiagnostic, HostRuntimeObservationStatus,
        ManagedConfigStatus, ProjectTrustStatus, StorageCapability, Verification,
        VerificationStatus,
    },
    HostAdapter, HostKind, HostPlan, HostScope, ManagedServerEntry, UserAction, UserActionKind,
};

use super::mcp_process::{run_connection_preflight, ConnectionProcess, McpLaunch, McpVerification};
use super::persisted_user_actions::{decode_persisted_user_actions, PersistedUserActions};
use super::{codex_environment, parse_host_kind, parse_host_scope, ConnectionCommandError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connection_command) enum AgentResultStatus {
    Complete,
    ActionRequired,
    Failed,
    NotVerified,
    DryRun,
}

impl AgentResultStatus {
    pub(in crate::connection_command) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Failed => "failed",
            Self::NotVerified => "not_verified",
            Self::DryRun => "dry_run",
        }
    }

    pub(in crate::connection_command) fn store_status(self) -> &'static str {
        match self {
            Self::Complete => VERIFIED_STATUS_COMPLETE,
            Self::ActionRequired => VERIFIED_STATUS_ACTION_REQUIRED,
            Self::Failed => VERIFIED_STATUS_FAILED,
            Self::NotVerified | Self::DryRun => VERIFIED_STATUS_NOT_VERIFIED,
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
    pub(in crate::connection_command) details: String,
    pub(in crate::connection_command) preflight_diagnostics: Option<McpPreflightDiagnostics>,
}

impl VerificationStep {
    pub(in crate::connection_command) fn passed(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Passed,
            details: details.into(),
            preflight_diagnostics: None,
        }
    }

    pub(in crate::connection_command) fn failed(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Failed,
            details: details.into(),
            preflight_diagnostics: None,
        }
    }

    pub(in crate::connection_command) fn skipped(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Skipped,
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

    fn storage_capability(&self) -> StorageCapability {
        StorageCapability::from_read_write_status(
            &self.storage_read,
            &self.storage_write,
            &self.effective_tool_mode,
        )
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
    pub(in crate::connection_command) status: AgentResultStatus,
    pub(in crate::connection_command) host: Verification,
    pub(in crate::connection_command) preflight: VerificationStep,
    pub(in crate::connection_command) handshake: VerificationStep,
    pub(in crate::connection_command) tools: Vec<String>,
    pub(in crate::connection_command) receipt: Option<HostVerificationReceipt>,
}

pub(in crate::connection_command) fn verify_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    launch: &McpLaunch,
    project_id: Option<&str>,
    process: &mut impl ConnectionProcess,
) -> Result<VerificationReport, ConnectionCommandError> {
    let persisted_user_actions = decode_persisted_user_actions(&connection.last_user_actions_json);
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let mut host = verify_host_plan(host_kind, host_plan, process)?;
    let projects =
        list_connection_projects_for_diagnostics(runtime_home, &connection.connection_internal_id)?;
    host = attach_current_host_runtime_diagnostics(
        runtime_home,
        connection,
        host_plan,
        &projects,
        host,
    );
    let mut regenerated_actions = host.user_actions.clone();
    for action in persisted_user_actions
        .actions_for_verification_repair()
        .iter()
        .filter(|action| action.kind == UserActionKind::ReloadRequired)
        .cloned()
    {
        push_unique_action(&mut regenerated_actions, action);
    }
    host = host.with_user_actions(regenerated_actions);
    let preflight = run_connection_preflight(
        process,
        launch,
        runtime_home,
        &connection.connection_internal_id,
        project_id,
        &connection.mode,
    );
    let handshake = if host.mcp_handshake_allowed && preflight.status == StepStatus::Passed {
        match process.verify_mcp_stdio(
            launch,
            runtime_home,
            &connection.connection_internal_id,
            &connection.mode,
        ) {
            Ok(verification) => verification,
            Err(error) => McpVerification::failed(error),
        }
    } else if !host.mcp_handshake_allowed {
        McpVerification {
            step: VerificationStep::skipped("host state does not allow direct MCP handshake"),
            tools: Vec::new(),
        }
    } else {
        McpVerification {
            step: VerificationStep::skipped("MCP preflight failed"),
            tools: Vec::new(),
        }
    };
    let cli_mcp = cli_mcp_verification(&preflight, &handshake.step, &handshake.tools);
    let status = aggregate_verification_status(
        &host,
        &cli_mcp,
        host_plan_requires_active_tool_exposure(host_plan),
    );
    let receipt = None;
    Ok(VerificationReport {
        status,
        host,
        preflight,
        handshake: handshake.step,
        tools: handshake.tools,
        receipt,
    })
}

pub(in crate::connection_command) fn current_status_host_diagnostic(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: Option<&HostPlan>,
    projects: &[ConnectionProjectRecord],
    process: &impl ConnectionProcess,
) -> Result<Option<Verification>, ConnectionCommandError> {
    let Some(host_plan) = host_plan else {
        return Ok(None);
    };
    if host_plan.host_kind != HostKind::Codex {
        return Ok(None);
    }
    let mut host = Verification::new(
        VerificationStatus::NotVerified,
        "Codex status diagnostics were read without running MCP verification",
    );
    let host_policy_overlay = if host_plan.host_kind == HostKind::Codex {
        let evaluation = codex::managed_identity_evaluation_for_plan(host_plan)?;
        host = host.with_managed_config(evaluation.status);
        evaluation.host_policy_overlay
    } else {
        let managed_config = stored_host_managed_config(connection)
            .as_deref()
            .and_then(managed_config_status_from_str)
            .unwrap_or(ManagedConfigStatus::Unknown);
        host = host.with_managed_config(managed_config);
        None
    };
    if let Some(overlay) = host_policy_overlay {
        host = host.with_host_policy_overlay(overlay);
    }
    if host.managed_config == ManagedConfigStatus::Match {
        host = host.with_mcp_handshake_allowed(true);
    }
    if parse_host_scope(&connection.host_scope)? == HostScope::Project {
        if let Some(project) = projects.first() {
            let trust = codex::project_trust_diagnostic(
                &codex_environment(process),
                &project.project.repo_root,
            );
            if trust.status == ProjectTrustStatus::Untrusted {
                host = host.with_user_actions(vec![UserAction::new(
                    UserActionKind::HostTrustRequired,
                    "Codex project trust is untrusted in the Codex user configuration",
                )]);
            }
            host = host.with_project_trust(trust);
        }
    }
    Ok(Some(attach_current_host_runtime_diagnostics(
        runtime_home,
        connection,
        host_plan,
        projects,
        host,
    )))
}

pub(in crate::connection_command) fn status_with_current_diagnostics(
    status: AgentResultStatus,
    actions: &[UserAction],
    current_host: Option<&Verification>,
) -> AgentResultStatus {
    if current_host.is_some_and(|host| {
        matches!(
            host.managed_config,
            ManagedConfigStatus::Missing
                | ManagedConfigStatus::Unmanaged
                | ManagedConfigStatus::Changed
                | ManagedConfigStatus::Malformed
        )
    }) {
        return AgentResultStatus::ActionRequired;
    }
    if status == AgentResultStatus::Complete {
        if let Some(runtime) = current_host.and_then(|host| host.host_runtime.as_ref()) {
            if !managed_runtime_confirms_active_exposure(runtime) {
                return AgentResultStatus::ActionRequired;
            }
        }
    }
    if status == AgentResultStatus::Complete && !actions.is_empty() {
        AgentResultStatus::ActionRequired
    } else {
        status
    }
}

pub(in crate::connection_command) fn connection_status_actions(
    current_host: Option<&Verification>,
    persisted: &PersistedUserActions,
) -> Option<Vec<UserAction>> {
    let mut actions = match current_host {
        Some(host) => host.user_actions.clone(),
        None => persisted.actions()?.to_vec(),
    };
    if let Some(persisted_actions) = persisted.actions() {
        for action in persisted_actions
            .iter()
            .filter(|action| action.kind == UserActionKind::ReloadRequired)
            .cloned()
        {
            push_unique_action(&mut actions, action);
        }
    }
    Some(actions)
}

pub(in crate::connection_command) fn status_from_store(value: &str) -> AgentResultStatus {
    match value {
        VERIFIED_STATUS_COMPLETE => AgentResultStatus::Complete,
        VERIFIED_STATUS_ACTION_REQUIRED => AgentResultStatus::ActionRequired,
        VERIFIED_STATUS_FAILED => AgentResultStatus::Failed,
        _ => AgentResultStatus::NotVerified,
    }
}

fn verify_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    process: &impl ConnectionProcess,
) -> Result<Verification, ConnectionCommandError> {
    if host_kind != HostKind::Codex {
        return Err(ConnectionCommandError::usage(
            "only Codex managed connections are supported",
        ));
    }
    if let Some(verification) = process.verify_host_plan(plan) {
        return verification.map_err(ConnectionCommandError::runtime);
    }
    let mut adapter = CodexAdapter::new(codex_environment(process));
    adapter.verify(plan).map_err(Into::into)
}

fn cli_mcp_verification(
    preflight: &VerificationStep,
    handshake: &VerificationStep,
    tools: &[String],
) -> CliMcpVerification {
    let storage_capability = preflight
        .preflight_diagnostics
        .as_ref()
        .map(McpPreflightDiagnostics::storage_capability)
        .unwrap_or(StorageCapability::Unknown);
    let effective_tool_mode = preflight
        .preflight_diagnostics
        .as_ref()
        .map(|diagnostics| diagnostics.effective_tool_mode.clone());
    CliMcpVerification::new(
        cli_mcp_step_status(preflight.status),
        cli_mcp_step_status(handshake.status),
        cli_mcp_tools_list_status(handshake.status, tools),
        storage_capability,
        effective_tool_mode,
    )
}

fn cli_mcp_step_status(status: StepStatus) -> CliMcpStepStatus {
    match status {
        StepStatus::Passed => CliMcpStepStatus::Passed,
        StepStatus::Failed => CliMcpStepStatus::Failed,
        StepStatus::Skipped => CliMcpStepStatus::Skipped,
    }
}

fn cli_mcp_tools_list_status(status: StepStatus, _tools: &[String]) -> CliMcpStepStatus {
    cli_mcp_step_status(status)
}

fn stored_host_managed_config(connection: &AgentConnectionRecord) -> Option<String> {
    serde_json::from_str::<Value>(&connection.last_verification_report_json)
        .ok()
        .and_then(|report| {
            report
                .get("host")
                .and_then(|host| host.get("managed_config"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

fn managed_config_status_from_str(value: &str) -> Option<ManagedConfigStatus> {
    match value {
        "match" => Some(ManagedConfigStatus::Match),
        "unmanaged" => Some(ManagedConfigStatus::Unmanaged),
        "missing" => Some(ManagedConfigStatus::Missing),
        "changed" => Some(ManagedConfigStatus::Changed),
        "malformed" => Some(ManagedConfigStatus::Malformed),
        "not_applicable" => Some(ManagedConfigStatus::NotApplicable),
        "unknown" => Some(ManagedConfigStatus::Unknown),
        _ => None,
    }
}

fn attach_current_host_runtime_diagnostics(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    projects: &[ConnectionProjectRecord],
    host: Verification,
) -> Verification {
    if host_plan.host_kind != HostKind::Codex || host_plan.host_scope != HostScope::Project {
        return host;
    }
    let runtime = host_runtime_observation(runtime_home, connection, projects);
    let command = host_mcp_command_diagnostic(&host_plan.entry, &runtime);
    let mut actions = host.user_actions.clone();
    if let Some(action) = managed_runtime_action(&host, &runtime) {
        push_unique_action(&mut actions, action);
    }
    host.with_host_runtime(runtime)
        .with_host_mcp_command(command)
        .with_user_actions(actions)
}

fn managed_runtime_action(
    host: &Verification,
    runtime: &HostRuntimeDiagnostic,
) -> Option<UserAction> {
    if host.managed_config.as_str() != "match" || !host.mcp_handshake_allowed {
        return None;
    }
    if host.user_actions.iter().any(|action| {
        matches!(
            action.kind,
            UserActionKind::HostTrustRequired | UserActionKind::ProjectApprovalRequired
        )
    }) {
        return None;
    }
    match (
        runtime.managed_host_startup,
        runtime.managed_host_tools_list,
        runtime.managed_host_tool_call,
    ) {
        (HostRuntimeObservationStatus::NotObserved, _, _) => Some(UserAction::new(
            UserActionKind::ManagedHostStartupNotObserved,
            managed_host_startup_not_observed_action_message(),
        )),
        (HostRuntimeObservationStatus::Observed, HostRuntimeObservationStatus::NotObserved, _) => {
            Some(UserAction::new(
                UserActionKind::ManagedHostToolsListNotObserved,
                managed_host_tools_list_not_observed_action_message(),
            ))
        }
        (
            HostRuntimeObservationStatus::Observed,
            HostRuntimeObservationStatus::Observed,
            HostRuntimeObservationStatus::NotObserved,
        ) => Some(UserAction::new(
            UserActionKind::ActiveToolExposureUnconfirmed,
            active_tool_exposure_unconfirmed_action_message(),
        )),
        (
            HostRuntimeObservationStatus::Observed,
            HostRuntimeObservationStatus::Observed,
            HostRuntimeObservationStatus::Observed,
        ) if managed_host_storage_is_degraded(runtime) => Some(UserAction::new(
            UserActionKind::ManagedHostStorageDegraded,
            managed_host_storage_degraded_action_message(),
        )),
        _ if runtime.active_tool_exposure != ActiveToolExposureStatus::Confirmed => {
            Some(UserAction::new(
                UserActionKind::ActiveToolExposureUnconfirmed,
                active_tool_exposure_unconfirmed_action_message(),
            ))
        }
        _ => None,
    }
}

fn managed_host_startup_not_observed_action_message() -> &'static str {
    "Restart, reload, resume, or start a new Codex session in this repository so Codex starts the managed Volicord MCP server."
}

fn managed_host_tools_list_not_observed_action_message() -> &'static str {
    "Check Codex MCP startup/tool-list logs; Volicord has observed managed startup but not managed tools/list."
}

fn active_tool_exposure_unconfirmed_action_message() -> &'static str {
    "Confirm active Codex tool exposure or invoke a read-only Volicord tool from the active Codex session."
}

fn managed_host_storage_degraded_action_message() -> &'static str {
    "Repair managed Codex host storage read/write capability or switch the connection to a compatible read-only mode."
}

fn managed_host_storage_is_degraded(runtime: &HostRuntimeDiagnostic) -> bool {
    let Some(storage) = &runtime.managed_host_storage else {
        return false;
    };
    storage_read_check_status(&storage.storage_read) != "passed"
        || storage_write_check_status(&storage.storage_write, &storage.effective_tool_mode)
            != "passed"
        || !matches!(
            effective_tool_mode_check_status(&storage.effective_tool_mode),
            "passed"
        )
}

fn push_unique_action(actions: &mut Vec<UserAction>, action: UserAction) {
    if !actions.iter().any(|existing| existing.kind == action.kind) {
        actions.push(action);
    }
}

fn host_runtime_observation(
    _runtime_home: &Path,
    _connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> HostRuntimeDiagnostic {
    if projects.is_empty() {
        return HostRuntimeDiagnostic {
            status: HostRuntimeObservationStatus::Unknown,
            managed_host_startup: HostRuntimeObservationStatus::Unknown,
            managed_host_tools_list: HostRuntimeObservationStatus::Unknown,
            managed_host_tool_call: HostRuntimeObservationStatus::Unknown,
            active_tool_exposure: ActiveToolExposureStatus::Unknown,
            managed_host_storage: None,
            details: "No connected project was available for managed Codex lifecycle observation"
                .to_owned(),
            last_observed_at: None,
        };
    }
    HostRuntimeDiagnostic {
        status: HostRuntimeObservationStatus::NotObserved,
        managed_host_startup: HostRuntimeObservationStatus::NotObserved,
        managed_host_tools_list: HostRuntimeObservationStatus::NotObserved,
        managed_host_tool_call: HostRuntimeObservationStatus::NotObserved,
        active_tool_exposure: ActiveToolExposureStatus::Unconfirmed,
        managed_host_storage: None,
        details:
            "Volicord has not received a current managed Codex runtime probe for this connection"
                .to_owned(),
        last_observed_at: None,
    }
}

fn host_mcp_command_diagnostic(
    entry: &ManagedServerEntry,
    runtime: &HostRuntimeDiagnostic,
) -> HostMcpCommandDiagnostic {
    let command = entry.command.trim();
    if command.is_empty() {
        return HostMcpCommandDiagnostic {
            mode: HostMcpCommandLaunchMode::Malformed,
            command: None,
            risk: Some("command_missing".to_owned()),
            details: "Host MCP command is empty".to_owned(),
        };
    }
    let path = Path::new(command);
    if path.is_absolute() {
        return HostMcpCommandDiagnostic {
            mode: HostMcpCommandLaunchMode::AbsolutePath,
            command: Some(command.to_owned()),
            risk: None,
            details: format!("Host MCP command uses absolute path {command}"),
        };
    }
    if path.components().count() == 1 {
        let risk = (runtime.status == HostRuntimeObservationStatus::NotObserved)
            .then(|| "host_path_unconfirmed".to_owned());
        return HostMcpCommandDiagnostic {
            mode: HostMcpCommandLaunchMode::PathResolved,
            command: Some(command.to_owned()),
            risk,
            details: format!("Host MCP command uses {command} from the Codex host PATH"),
        };
    }
    HostMcpCommandDiagnostic {
        mode: HostMcpCommandLaunchMode::Unknown,
        command: Some(command.to_owned()),
        risk: Some("launch_mode_unknown".to_owned()),
        details: format!(
            "Host MCP command is non-absolute and not a simple PATH command: {command}"
        ),
    }
}

fn aggregate_verification_status(
    host: &Verification,
    cli_mcp: &CliMcpVerification,
    requires_active_tool_exposure: bool,
) -> AgentResultStatus {
    if cli_mcp.has_failed_step() {
        return AgentResultStatus::Failed;
    }
    if let Some(runtime) = host.host_runtime.as_ref() {
        return match host.status {
            VerificationStatus::Complete
                if cli_mcp.handshake_passed()
                    && host.user_actions.is_empty()
                    && managed_runtime_confirms_active_exposure(runtime) =>
            {
                AgentResultStatus::Complete
            }
            VerificationStatus::Complete | VerificationStatus::ActionRequired
                if cli_mcp.handshake_passed() =>
            {
                AgentResultStatus::ActionRequired
            }
            VerificationStatus::NotVerified => AgentResultStatus::NotVerified,
            _ => AgentResultStatus::Failed,
        };
    }
    if requires_active_tool_exposure {
        return match host.status {
            VerificationStatus::Complete | VerificationStatus::ActionRequired
                if cli_mcp.handshake_passed() =>
            {
                AgentResultStatus::ActionRequired
            }
            VerificationStatus::NotVerified => AgentResultStatus::NotVerified,
            _ => AgentResultStatus::Failed,
        };
    }
    match host.status {
        VerificationStatus::Complete
            if cli_mcp.handshake_passed() && host.user_actions.is_empty() =>
        {
            AgentResultStatus::Complete
        }
        VerificationStatus::Complete | VerificationStatus::ActionRequired
            if cli_mcp.handshake_passed() =>
        {
            AgentResultStatus::ActionRequired
        }
        VerificationStatus::NotVerified => AgentResultStatus::NotVerified,
        _ => AgentResultStatus::Failed,
    }
}

fn managed_runtime_confirms_active_exposure(runtime: &HostRuntimeDiagnostic) -> bool {
    runtime.managed_host_startup == HostRuntimeObservationStatus::Observed
        && runtime.managed_host_tools_list == HostRuntimeObservationStatus::Observed
        && runtime.managed_host_tool_call == HostRuntimeObservationStatus::Observed
        && runtime.active_tool_exposure == ActiveToolExposureStatus::Confirmed
}

fn host_plan_requires_active_tool_exposure(host_plan: &HostPlan) -> bool {
    host_plan.host_kind == HostKind::Codex && host_plan.host_scope == HostScope::Project
}

pub(in crate::connection_command) fn storage_read_check_status(value: &str) -> &'static str {
    match value {
        "passed" => "passed",
        "failed" => "failed",
        "skipped" => "skipped",
        _ => "unknown",
    }
}

pub(in crate::connection_command) fn storage_write_check_status(
    value: &str,
    effective_tool_mode: &str,
) -> &'static str {
    match value {
        "passed" => "passed",
        "readonly" if effective_tool_mode == "read_only" => "passed",
        "readonly" => "action_required",
        "failed" => "failed",
        "skipped" => "skipped",
        _ => "unknown",
    }
}

pub(in crate::connection_command) fn effective_tool_mode_check_status(value: &str) -> &'static str {
    match value {
        "workflow" | "read_only" => "passed",
        "read_only_degraded" => "action_required",
        "unavailable" => "failed",
        _ => "unknown",
    }
}

pub(in crate::connection_command) fn host_mcp_command_check_status(
    command: &HostMcpCommandDiagnostic,
) -> &'static str {
    if command.mode == HostMcpCommandLaunchMode::Malformed {
        "failed"
    } else if command.risk.is_some() {
        "warning"
    } else {
        "passed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_actions_preserve_corrupt_persisted_state_as_unknown() {
        assert!(connection_status_actions(None, &PersistedUserActions::Corrupt).is_none());

        let current = Verification::new(VerificationStatus::NotVerified, "current diagnostics")
            .with_user_actions(vec![UserAction::new(
                UserActionKind::ManagedHostStartupNotObserved,
                "start Codex",
            )]);
        let actions = connection_status_actions(Some(&current), &PersistedUserActions::Corrupt)
            .expect("current typed diagnostics replace unknown persisted actions");
        assert_eq!(actions, current.user_actions);
    }

    #[test]
    fn connection_verify_managed_tools_list_without_tool_call_is_unconfirmed() {
        let cli_mcp = passed_cli_mcp_verification();
        let status = aggregate_verification_status(
            &Verification::configured_ready("ready").with_host_runtime(runtime_diagnostic(
                HostRuntimeObservationStatus::Observed,
                HostRuntimeObservationStatus::Observed,
                HostRuntimeObservationStatus::NotObserved,
            )),
            &cli_mcp,
            true,
        );

        assert_eq!(status, AgentResultStatus::ActionRequired);
    }

    #[test]
    fn connection_verify_managed_tool_call_marks_connection_complete() {
        let cli_mcp = passed_cli_mcp_verification();
        let status = aggregate_verification_status(
            &Verification::configured_ready("ready").with_host_runtime(runtime_diagnostic(
                HostRuntimeObservationStatus::Observed,
                HostRuntimeObservationStatus::Observed,
                HostRuntimeObservationStatus::Observed,
            )),
            &cli_mcp,
            true,
        );

        assert_eq!(status, AgentResultStatus::Complete);
    }

    #[test]
    fn connection_verify_tool_call_without_managed_lifecycle_cannot_complete() {
        let cli_mcp = passed_cli_mcp_verification();
        let status = aggregate_verification_status(
            &Verification::configured_ready("ready").with_host_runtime(runtime_diagnostic(
                HostRuntimeObservationStatus::NotObserved,
                HostRuntimeObservationStatus::NotObserved,
                HostRuntimeObservationStatus::Observed,
            )),
            &cli_mcp,
            true,
        );

        assert_eq!(status, AgentResultStatus::ActionRequired);
    }

    #[test]
    fn connection_verify_codex_project_without_runtime_cannot_complete() {
        let cli_mcp = passed_cli_mcp_verification();
        let status =
            aggregate_verification_status(&Verification::configured_ready("ready"), &cli_mcp, true);

        assert_eq!(status, AgentResultStatus::ActionRequired);
    }

    #[test]
    fn cli_mcp_verification_separates_handshake_tools_and_storage() {
        let preflight = VerificationStep::passed("CLI MCP preflight passed")
            .with_preflight_diagnostics(Some(McpPreflightDiagnostics {
                storage_read: "passed".to_owned(),
                storage_write: "readonly".to_owned(),
                effective_tool_mode: "read_only".to_owned(),
            }));
        let handshake = VerificationStep::passed("tools/list returned 1 tools");

        let verification =
            cli_mcp_verification(&preflight, &handshake, &["volicord.status".to_owned()]);

        assert_eq!(verification.preflight, CliMcpStepStatus::Passed);
        assert_eq!(verification.handshake, CliMcpStepStatus::Passed);
        assert_eq!(verification.tools_list, CliMcpStepStatus::Passed);
        assert_eq!(verification.storage_capability, StorageCapability::ReadOnly);
        assert_eq!(
            verification.effective_tool_mode.as_deref(),
            Some("read_only")
        );
        assert!(!verification.has_failed_step());
    }

    #[test]
    fn unknown_active_tool_exposure_has_action() {
        let action = managed_runtime_action(
            &Verification::configured_ready("ready"),
            &HostRuntimeDiagnostic {
                status: HostRuntimeObservationStatus::Unknown,
                managed_host_startup: HostRuntimeObservationStatus::Unknown,
                managed_host_tools_list: HostRuntimeObservationStatus::Unknown,
                managed_host_tool_call: HostRuntimeObservationStatus::Unknown,
                active_tool_exposure: ActiveToolExposureStatus::Unknown,
                managed_host_storage: None,
                details: "runtime unavailable".to_owned(),
                last_observed_at: None,
            },
        )
        .expect("unknown active exposure should require an action");

        assert_eq!(action.kind, UserActionKind::ActiveToolExposureUnconfirmed);
    }

    #[test]
    fn connection_verify_path_unconfirmed_is_secondary_without_launch_failure() {
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"));
        let command = host_mcp_command_diagnostic(
            &entry,
            &runtime_diagnostic(
                HostRuntimeObservationStatus::NotObserved,
                HostRuntimeObservationStatus::NotObserved,
                HostRuntimeObservationStatus::NotObserved,
            ),
        );

        assert_eq!(command.risk.as_deref(), Some("host_path_unconfirmed"));
        assert_eq!(host_mcp_command_check_status(&command), "warning");
    }

    fn runtime_diagnostic(
        startup: HostRuntimeObservationStatus,
        tools_list: HostRuntimeObservationStatus,
        tool_call: HostRuntimeObservationStatus,
    ) -> HostRuntimeDiagnostic {
        let active_tool_exposure = match tool_call {
            HostRuntimeObservationStatus::Observed => ActiveToolExposureStatus::Confirmed,
            HostRuntimeObservationStatus::NotObserved => ActiveToolExposureStatus::Unconfirmed,
            HostRuntimeObservationStatus::Unknown => ActiveToolExposureStatus::Unknown,
        };
        HostRuntimeDiagnostic {
            status: if startup == HostRuntimeObservationStatus::Observed {
                HostRuntimeObservationStatus::Observed
            } else {
                HostRuntimeObservationStatus::NotObserved
            },
            managed_host_startup: startup,
            managed_host_tools_list: tools_list,
            managed_host_tool_call: tool_call,
            active_tool_exposure,
            managed_host_storage: None,
            details: "test runtime".to_owned(),
            last_observed_at: None,
        }
    }

    fn passed_cli_mcp_verification() -> CliMcpVerification {
        let preflight = VerificationStep::passed("CLI MCP preflight passed")
            .with_preflight_diagnostics(Some(McpPreflightDiagnostics {
                storage_read: "passed".to_owned(),
                storage_write: "passed".to_owned(),
                effective_tool_mode: "workflow".to_owned(),
            }));
        let handshake = VerificationStep::passed("CLI MCP handshake passed");
        cli_mcp_verification(&preflight, &handshake, &["volicord.status".to_owned()])
    }
}
