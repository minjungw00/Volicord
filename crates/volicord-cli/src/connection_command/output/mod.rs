use super::*;

mod json;
mod report;
mod summary;
mod text;

use crate::disclosure::cooperative_host_decision_disclosure_json;
use crate::guard_integration::files::RetirementPlanStatus;
use json::{actions_json_values, checks_json, connection_json};
use report::{
    render_command_report, CommandConnection, ConnectionCommandReport, PlannedConnectionChange,
    RenderedCommandReport,
};
use summary::connection_diagnostic_summary_card;
use text::{render_compact_connection_text, render_compact_plan_text};

pub(super) use json::connection_states_json;
pub(super) use report::CommandOperation;
pub(super) use text::{render_connection_remove_dry_run_output, render_connections_output};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepoFileChangeStatus {
    Created,
    Updated,
    Removed,
    PlannedCreate,
    PlannedUpdate,
    PlannedRemove,
}

impl RepoFileChangeStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Removed => "removed",
            Self::PlannedCreate => "planned_create",
            Self::PlannedUpdate => "planned_update",
            Self::PlannedRemove => "planned_remove",
        }
    }

    fn text_verb(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Removed => "removed",
            Self::PlannedCreate => "would create",
            Self::PlannedUpdate => "would update",
            Self::PlannedRemove => "would remove",
        }
    }

    fn is_actual(self) -> bool {
        matches!(self, Self::Created | Self::Updated | Self::Removed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoFileChange {
    status: RepoFileChangeStatus,
    path: String,
}

pub(super) struct InitOutput<'a> {
    pub(super) format: OutputFormat,
    pub(super) dry_run: bool,
    pub(super) host_kind: HostKind,
    pub(super) host_scope: HostScope,
    pub(super) runtime_home: &'a Path,
    pub(super) repo_root: &'a Path,
    pub(super) connection_id: &'a str,
    pub(super) project_id: Option<&'a str>,
    pub(super) host_plan: &'a HostPlan,
    pub(super) verification: Option<&'a VerificationReport>,
    pub(super) current_report: Option<&'a volicord_types::ConnectionVerificationReport>,
    pub(super) integration: &'a GuardIntegrationPlan,
    pub(super) profile_action: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrimaryNextAction {
    id: String,
    instruction: String,
    command: Option<String>,
}

impl PrimaryNextAction {
    fn new(id: impl Into<String>, instruction: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            instruction: instruction.into(),
            command: None,
        }
    }

    fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    fn to_json(&self) -> Value {
        json!({
            "id": &self.id,
            "instruction": &self.instruction,
            "command": &self.command,
        })
    }
}

pub(super) struct ConnectionOutput<'a> {
    pub(super) format: OutputFormat,
    pub(super) action: &'a str,
    pub(super) status: AgentResultStatus,
    pub(super) runtime_home: &'a Path,
    pub(super) host_kind: HostKind,
    pub(super) guard_state: GuardOperationalState,
    pub(super) connection: &'a AgentConnectionRecord,
    pub(super) projects: &'a [ConnectionProjectRecord],
    pub(super) affected_repo_root: Option<&'a Path>,
    pub(super) verification: Option<&'a VerificationReport>,
    pub(super) current_report: Option<volicord_types::ConnectionVerificationReport>,
    pub(super) current_host: Option<Verification>,
    pub(super) plan: Option<&'a HostPlan>,
    pub(super) removal_outcome: Option<&'a ConnectionProjectRemovalOutcome>,
    pub(super) user_actions: Vec<UserAction>,
}

pub(super) struct ConnectionPlanOutput<'a> {
    pub(super) format: OutputFormat,
    pub(super) action: &'a str,
    pub(super) status: AgentResultStatus,
    pub(super) runtime_home: &'a Path,
    pub(super) connection_id: &'a str,
    pub(super) host_kind: HostKind,
    pub(super) intent: ConnectionIntent,
    pub(super) host_scope: HostScope,
    pub(super) mode: &'a str,
    pub(super) enabled: bool,
    pub(super) repo_root: Option<&'a Path>,
    pub(super) plan: &'a HostPlan,
    pub(super) projects_remaining: Option<usize>,
    pub(super) user_actions: Vec<UserAction>,
}

pub(super) enum ConnectionRemovePlan<'a> {
    Host(&'a HostPlan),
    MembershipOnly,
}

fn user_action_id(kind: UserActionKind) -> &'static str {
    match kind {
        UserActionKind::HostTrustRequired => "host_trust_required",
        UserActionKind::RepairManagedConfig => "repair_managed_config",
        UserActionKind::InstallOrRepairCodex => "install_or_repair_codex",
        UserActionKind::RepairMcpServer => "repair_mcp_server",
        UserActionKind::ReloadHost => "reload_host",
        UserActionKind::UseVolicordTool => "use_volicord_tool",
        UserActionKind::ReloadGuard => "reload_guard",
        UserActionKind::RepairGuard => "repair_guard",
        UserActionKind::ReloadRequired => "reload_required",
    }
}

pub(super) fn render_connection_output(
    data: ConnectionOutput<'_>,
) -> Result<String, ConnectionCommandError> {
    let project_ids = data
        .projects
        .iter()
        .map(|project| project.project_id.clone())
        .collect::<Vec<_>>();
    let target = data
        .plan
        .map(|plan| host_target_text(&plan.target))
        .unwrap_or_else(|| data.connection.config_target.clone());
    let planned_change = data.plan.map(|plan| planned_change_text(plan.change));
    let mcp_config_state = connection_mcp_config_state(
        data.connection,
        data.verification,
        data.plan,
        data.current_host.as_ref(),
    );
    let primary_next_action = primary_connection_action(
        &data.user_actions,
        data.verification,
        data.current_host.as_ref(),
        &data.guard_state,
        Some(data.connection),
        data.projects,
    );
    let host_display = connection_host_display_name(data.connection);
    let summary_card = connection_diagnostic_summary_card(
        data.action,
        &data.guard_state,
        &host_display,
        primary_next_action.as_ref(),
    );
    match data.format {
        OutputFormat::Text => {
            render_compact_connection_text(&data, &mcp_config_state, primary_next_action.as_ref())
        }
        OutputFormat::Json => {
            let projected_user_actions = Some(data.user_actions.as_slice());
            let displayed_report = data
                .verification
                .map(|verification| &verification.report)
                .or(data.current_report.as_ref());
            let mut value = json!({
                "action": data.action,
                "status": data.status.as_str(),
                "disclosure": cooperative_host_decision_disclosure_json(),
                "runtime_home": path_text(data.runtime_home),
                "states": connection_states_json(
                    data.status.as_str(),
                    project_registration_state(data.projects),
                    mcp_config_state.as_str(),
                    &data.guard_state,
                    has_reload_action(&data.user_actions),
                ),
                "connection": connection_json(data.connection, &project_ids, projected_user_actions),
                "target": target,
                "planned_change": planned_change,
                "checks": checks_json(
                    data.connection,
                    data.verification,
                    data.current_report.as_ref(),
                    &data.guard_state,
                ),
                "actions": displayed_report
                    .and_then(|report| serde_json::to_value(report.actions()).ok())
                    .unwrap_or_else(|| actions_json_values(&data.user_actions)),
                "primary_next_action": primary_next_action.as_ref().map(|action| action.to_json()),
                "host_hook": data.guard_state.to_json(),
                "verification": displayed_report
                    .and_then(|report| serde_json::to_value(report).ok()),
            });
            if let Some(card) = &summary_card {
                value
                    .as_object_mut()
                    .expect("connection output should be a JSON object")
                    .insert(
                        "summary_card".to_owned(),
                        serde_json::to_value(card).expect("summary card should serialize to JSON"),
                    );
            }
            if let Some(outcome) = data.removal_outcome {
                let object = value
                    .as_object_mut()
                    .expect("connection output should be a JSON object");
                object.insert(
                    "membership_removed".to_owned(),
                    Value::Bool(outcome.membership_removed),
                );
                object.insert(
                    "connection_removed".to_owned(),
                    Value::Bool(outcome.connection_removed),
                );
                object.insert(
                    "remaining_project_count".to_owned(),
                    Value::from(outcome.remaining_project_count),
                );
            }
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

pub(super) fn render_connection_mode_output(
    format: OutputFormat,
    runtime_home: &Path,
    outcome: &volicord_store::agent_connections::ConnectionModeTransitionOutcome,
    projects: &[ConnectionProjectRecord],
    actions: &[UserAction],
) -> Result<String, ConnectionCommandError> {
    let changed = outcome.kind == ConnectionModeTransitionKind::Updated;
    let action = if changed {
        "mode_updated"
    } else {
        "mode_unchanged"
    };
    let status = if changed {
        AgentResultStatus::ActionRequired
    } else {
        AgentResultStatus::Complete
    };
    match format {
        OutputFormat::Text => {
            let mut output = format!(
                "Agent Connection mode {} for Codex\n\nStatus:\n  Result: {}\n  Mode: {}\n  Previous integration revision: {}\n  Current integration revision: {}\n  Guard manifests rebound: {}\n\nRepositories:\n",
                if changed { "updated" } else { "unchanged" },
                status.as_str(),
                public_mode_text(&outcome.connection.mode),
                outcome.previous_integration_revision.as_str(),
                outcome.current_integration_revision.as_str(),
                outcome.rebound_guard_installation_ids.len(),
            );
            for project in projects {
                output.push_str(&format!("  {}\n", project.project.repo_root.display()));
            }
            output.push_str("\nNext:\n");
            if let Some(next) = actions.first() {
                output.push_str(&format!("  1. {}\n", next.message));
            } else {
                output.push_str("  none\n");
            }
            Ok(output)
        }
        OutputFormat::Json => {
            let project_ids = projects
                .iter()
                .map(|project| project.project_id.clone())
                .collect::<Vec<_>>();
            let value = json!({
                "action": action,
                "status": status.as_str(),
                "runtime_home": path_text(runtime_home),
                "connection": {
                    "connection_id": outcome.connection.connection_internal_id,
                    "host_kind": outcome.connection.host_kind,
                    "connection_intent": outcome.connection.intent,
                    "host_scope": outcome.connection.host_scope,
                    "mode": outcome.connection.mode,
                    "enabled": outcome.connection.enabled,
                    "connected_projects": project_ids,
                },
                "transition": if changed { "updated" } else { "unchanged" },
                "previous_integration_revision": outcome.previous_integration_revision,
                "current_integration_revision": outcome.current_integration_revision,
                "rebound_guard_installation_ids": outcome.rebound_guard_installation_ids,
                "actions": actions_json_values(actions),
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

pub(super) fn render_current_connection_output(
    format: OutputFormat,
    operation: CommandOperation,
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    repository: &Path,
    verification: &volicord_types::ConnectionVerificationReport,
) -> Result<RenderedCommandReport, ConnectionCommandError> {
    let report = ConnectionCommandReport::from_verification(
        operation,
        None,
        runtime_home,
        CommandConnection::new(
            &connection.connection_internal_id,
            &connection.host_kind,
            &connection.host_scope,
            &connection.mode,
            repository,
            &connection.config_target,
        ),
        verification,
    );
    render_command_report(format, &report)
}

pub(super) fn render_connection_plan_output(
    data: ConnectionPlanOutput<'_>,
) -> Result<String, ConnectionCommandError> {
    let target = host_target_text(&data.plan.target);
    let planned_change = planned_change_text(data.plan.change);
    let guard_state = GuardOperationalState::not_configured();
    let primary_next_action =
        primary_connection_action(&data.user_actions, None, None, &guard_state, None, &[]);
    let project_state = data.repo_root.map(|_| "planned").unwrap_or("not_selected");
    match data.format {
        OutputFormat::Text => Ok(render_compact_plan_text(&data)),
        OutputFormat::Json => {
            let connected_repositories = data
                .repo_root
                .into_iter()
                .map(path_text)
                .collect::<Vec<_>>();
            let value = json!({
                "action": data.action,
                "status": data.status.as_str(),
                "operation_mode": "dry_run",
                "disclosure": cooperative_host_decision_disclosure_json(),
                "runtime_home": path_text(data.runtime_home),
                "states": connection_states_json(
                    "planned",
                    project_state,
                    &format!("planned_{planned_change}"),
                    &guard_state,
                    has_reload_action(&data.user_actions),
                ),
                "connection": {
                    "connection_id": data.connection_id,
                    "host_kind": data.host_kind.as_str(),
                    "connection_intent": data.intent.as_str(),
                    "host_scope": data.host_scope.as_str(),
                    "mode": data.mode,
                    "enabled": data.enabled,
                    "connected_repositories": connected_repositories,
                    "server_name": data.plan.server_name,
                    "config_target": target,
                },
                "target": target,
                "planned_change": planned_change,
                "remaining_connected_projects": data.projects_remaining,
                "checks": [{
                    "id": "host_plan",
                    "status": "passed",
                    "summary": "host plan was built"
                }],
                "actions": actions_json_values(&data.user_actions),
                "primary_next_action": primary_next_action.map(|action| action.to_json()),
                "host_hook": guard_state.to_json(),
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

pub(super) fn render_init_output(
    data: InitOutput<'_>,
) -> Result<RenderedCommandReport, ConnectionCommandError> {
    let target = host_target_text(&data.host_plan.target);
    let connection = CommandConnection::new(
        data.connection_id,
        data.host_kind.as_str(),
        data.host_scope.as_str(),
        CONNECTION_MODE_WORKFLOW,
        data.repo_root,
        &target,
    );
    let report = if data.dry_run {
        ConnectionCommandReport::dry_run(
            data.runtime_home,
            connection,
            data.current_report,
            init_planned_changes(&data, &target),
            &data.host_plan.user_actions,
        )?
    } else {
        let verification = data.verification.ok_or_else(|| {
            ConnectionCommandError::runtime(
                "applied init requires one canonical verification report",
            )
        })?;
        ConnectionCommandReport::from_verification(
            CommandOperation::Init,
            Some(true),
            data.runtime_home,
            connection,
            &verification.report,
        )
    };
    render_command_report(data.format, &report)
}

fn init_planned_changes(data: &InitOutput<'_>, target: &str) -> Vec<PlannedConnectionChange> {
    let mut changes = Vec::new();
    if data.profile_action == "planned" {
        changes.push(PlannedConnectionChange::new(
            "create",
            path_text(data.runtime_home),
        ));
    }
    if data.project_id.is_none() {
        changes.push(PlannedConnectionChange::new(
            "register",
            path_text(data.repo_root),
        ));
    }
    if data.host_plan.change != PlannedChange::Noop {
        changes.push(PlannedConnectionChange::new(
            planned_change_text(data.host_plan.change),
            target,
        ));
    }
    for change in init_repo_file_changes(data) {
        if !change.status.is_actual() {
            changes.push(PlannedConnectionChange::new(
                change.status.as_str(),
                change.path,
            ));
        }
    }
    changes.sort_by(|left, right| {
        left.target()
            .cmp(right.target())
            .then_with(|| left.change().cmp(right.change()))
    });
    changes.dedup();
    changes
}

fn init_repo_file_changes(data: &InitOutput<'_>) -> Vec<RepoFileChange> {
    let mut changes = BTreeMap::new();
    if let Some(status) = repo_file_change_from_host_plan(data.host_plan.change, data.dry_run) {
        if let Some(path) = repo_relative_host_target_path(data.host_plan, data.repo_root) {
            insert_repo_file_change(&mut changes, path, status);
        }
    }
    for file in &data.integration.generated_files {
        if let Some(status) = repo_file_change_from_file_status(file.status) {
            if let Some(path) = repo_relative_path(&file.path, data.repo_root) {
                insert_repo_file_change(&mut changes, path, status);
            }
        }
    }
    for file in &data.integration.retired_files {
        if let Some(status) = repo_file_change_from_retirement_status(file.status) {
            if let Some(path) = repo_relative_path(&file.path, data.repo_root) {
                insert_repo_file_change(&mut changes, path, status);
            }
        }
    }
    changes
        .into_iter()
        .map(|(path, status)| RepoFileChange { status, path })
        .collect()
}

fn repo_file_change_from_retirement_status(
    status: RetirementPlanStatus,
) -> Option<RepoFileChangeStatus> {
    match status {
        RetirementPlanStatus::PlannedRemove => Some(RepoFileChangeStatus::PlannedRemove),
        RetirementPlanStatus::PlannedUpdate => Some(RepoFileChangeStatus::PlannedUpdate),
        RetirementPlanStatus::Removed => Some(RepoFileChangeStatus::Removed),
        RetirementPlanStatus::Updated => Some(RepoFileChangeStatus::Updated),
        RetirementPlanStatus::Unchanged => None,
    }
}

fn repo_file_change_from_host_plan(
    change: PlannedChange,
    dry_run: bool,
) -> Option<RepoFileChangeStatus> {
    match (dry_run, change) {
        (true, PlannedChange::Create) => Some(RepoFileChangeStatus::PlannedCreate),
        (true, PlannedChange::Update) => Some(RepoFileChangeStatus::PlannedUpdate),
        (false, PlannedChange::Create) => Some(RepoFileChangeStatus::Created),
        (false, PlannedChange::Update) => Some(RepoFileChangeStatus::Updated),
        _ => None,
    }
}

fn repo_file_change_from_file_status(status: FilePlanStatus) -> Option<RepoFileChangeStatus> {
    match status {
        FilePlanStatus::PlannedCreate => Some(RepoFileChangeStatus::PlannedCreate),
        FilePlanStatus::PlannedUpdate => Some(RepoFileChangeStatus::PlannedUpdate),
        FilePlanStatus::Created => Some(RepoFileChangeStatus::Created),
        FilePlanStatus::Updated => Some(RepoFileChangeStatus::Updated),
        FilePlanStatus::Unchanged => None,
    }
}

fn repo_relative_host_target_path(plan: &HostPlan, repo_root: &Path) -> Option<String> {
    match &plan.target {
        HostTarget::File(path) | HostTarget::Export(path) => repo_relative_path(path, repo_root),
        HostTarget::ExternalCli { .. } => None,
    }
}

fn repo_relative_path(path: &Path, repo_root: &Path) -> Option<String> {
    let relative = path.strip_prefix(repo_root).ok()?;
    relative.components().next()?;
    Some(path_text(relative))
}

fn insert_repo_file_change(
    changes: &mut BTreeMap<String, RepoFileChangeStatus>,
    path: String,
    status: RepoFileChangeStatus,
) {
    changes
        .entry(path)
        .and_modify(|existing| *existing = merge_repo_file_change_status(*existing, status))
        .or_insert(status);
}

fn merge_repo_file_change_status(
    existing: RepoFileChangeStatus,
    new: RepoFileChangeStatus,
) -> RepoFileChangeStatus {
    match (existing, new) {
        (RepoFileChangeStatus::Created, _) | (RepoFileChangeStatus::PlannedCreate, _) => existing,
        (_, RepoFileChangeStatus::Created) | (_, RepoFileChangeStatus::PlannedCreate) => new,
        (RepoFileChangeStatus::Removed, _) | (RepoFileChangeStatus::PlannedRemove, _) => existing,
        (_, RepoFileChangeStatus::Removed) | (_, RepoFileChangeStatus::PlannedRemove) => new,
        _ => existing,
    }
}

fn planned_change_text(change: PlannedChange) -> &'static str {
    match change {
        PlannedChange::Create => "create",
        PlannedChange::Update => "update",
        PlannedChange::Remove => "remove",
        PlannedChange::Noop => "noop",
        PlannedChange::ExternalCommand => "external_command",
    }
}

pub(super) fn display_project_roots(projects: &[ConnectionProjectRecord]) -> String {
    projects
        .iter()
        .map(|project| path_text(&project.project.repo_root))
        .collect::<Vec<_>>()
        .join(",")
}

fn project_registration_state(projects: &[ConnectionProjectRecord]) -> &'static str {
    if projects.is_empty() {
        "not_connected"
    } else {
        "registered"
    }
}

fn connection_mcp_config_state(
    connection: &AgentConnectionRecord,
    verification: Option<&VerificationReport>,
    plan: Option<&HostPlan>,
    current_host: Option<&Verification>,
) -> String {
    if let Some(verification) = verification {
        return verification.host.managed_config.as_str().to_owned();
    }
    if let Some(host) = current_host {
        return host.managed_config.as_str().to_owned();
    }
    if let Some(plan) = plan {
        return planned_change_text(plan.change).to_owned();
    }
    connection
        .verification_report()
        .ok()
        .flatten()
        .and_then(|report| {
            report
                .checks()
                .iter()
                .find(|check| check.id().as_str() == "host")
                .and_then(|check| check.details())
                .and_then(|details| details.as_object().get("managed_config"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn has_reload_action(actions: &[UserAction]) -> bool {
    actions
        .iter()
        .any(|action| action.kind == UserActionKind::ReloadRequired)
}

fn primary_connection_action(
    actions: &[UserAction],
    verification: Option<&VerificationReport>,
    current_host: Option<&Verification>,
    guard_state: &GuardOperationalState,
    connection: Option<&AgentConnectionRecord>,
    projects: &[ConnectionProjectRecord],
) -> Option<PrimaryNextAction> {
    if let Some(verification) = verification {
        match verification.host.managed_config.as_str() {
            "missing" => {
                return Some(connection_repair_action(
                    "mcp_config_missing",
                    "Reinstall missing MCP configuration.",
                    connection,
                    projects,
                ));
            }
            "unmanaged" => {
                return Some(connection_repair_action(
                    "mcp_config_changed",
                    "Review the unmanaged MCP configuration entry and repair it if Volicord should manage it.",
                    connection,
                    projects,
                ));
            }
            "changed" => {
                return Some(connection_repair_action(
                    "mcp_config_changed",
                    "Review the changed MCP configuration and repair it if Volicord should manage it.",
                    connection,
                    projects,
                ));
            }
            "malformed" => {
                return Some(connection_repair_action(
                    "mcp_config_malformed",
                    "Repair the malformed MCP configuration.",
                    connection,
                    projects,
                ));
            }
            _ => {}
        }
        if verification.host.host_executable.as_str() == "unavailable" {
            return Some(PrimaryNextAction::new(
                "path_binary_not_found",
                verification.host.host_executable_details.clone(),
            ));
        }
    }
    if verification.is_none() {
        if let Some(host) = current_host {
            match host.managed_config.as_str() {
                "missing" => {
                    return Some(connection_repair_action(
                        "mcp_config_missing",
                        "Reinstall missing MCP configuration.",
                        connection,
                        projects,
                    ));
                }
                "unmanaged" => {
                    return Some(connection_repair_action(
                        "mcp_config_changed",
                        "Review the unmanaged MCP configuration entry and repair it if Volicord should manage it.",
                        connection,
                        projects,
                    ));
                }
                "changed" => {
                    return Some(connection_repair_action(
                        "mcp_config_changed",
                        "Review the changed MCP configuration and repair it if Volicord should manage it.",
                        connection,
                        projects,
                    ));
                }
                "malformed" => {
                    return Some(connection_repair_action(
                        "mcp_config_malformed",
                        "Repair the malformed MCP configuration.",
                        connection,
                        projects,
                    ));
                }
                _ => {}
            }
        }
        if let Some(connection) = connection {
            let stored_host_details =
                connection
                    .verification_report()
                    .ok()
                    .flatten()
                    .and_then(|report| {
                        report
                            .checks()
                            .iter()
                            .find(|check| check.id().as_str() == "host")
                            .and_then(|check| check.details())
                            .map(|details| details.as_object().clone())
                    });
            match connection_mcp_config_state(connection, None, None, None).as_str() {
                "missing" => {
                    return Some(connection_repair_action(
                        "mcp_config_missing",
                        "Reinstall missing MCP configuration.",
                        Some(connection),
                        projects,
                    ));
                }
                "unmanaged" => {
                    return Some(connection_repair_action(
                        "mcp_config_changed",
                        "Review the unmanaged MCP configuration entry and repair it if Volicord should manage it.",
                        Some(connection),
                        projects,
                    ));
                }
                "changed" => {
                    return Some(connection_repair_action(
                        "mcp_config_changed",
                        "Review the changed MCP configuration and repair it if Volicord should manage it.",
                        Some(connection),
                        projects,
                    ));
                }
                "malformed" => {
                    return Some(connection_repair_action(
                        "mcp_config_malformed",
                        "Repair the malformed MCP configuration.",
                        Some(connection),
                        projects,
                    ));
                }
                _ => {}
            }
            if stored_host_details
                .as_ref()
                .and_then(|host| host.get("executable"))
                .and_then(Value::as_str)
                == Some("unavailable")
            {
                return Some(PrimaryNextAction::new(
                    "path_binary_not_found",
                    stored_host_details
                        .as_ref()
                        .and_then(|host| host.get("failure_reason"))
                        .and_then(Value::as_str)
                        .unwrap_or(
                            "Install or repair the host executable so it is available on PATH.",
                        ),
                ));
            }
        }
    }
    if guard_state.guard_hooks_applicable() {
        if guard_state.installation_state == "files_missing" {
            return Some(connection_repair_action(
                "guard_files_missing",
                "Run init again to reinstall missing Codex Record Guard files.",
                connection,
                projects,
            ));
        }
        if guard_state.installation_state == "stale" {
            return Some(connection_repair_action(
                "guard_files_stale",
                "Run init again to refresh stale Codex Record Guard files.",
                connection,
                projects,
            ));
        }
        if guard_state.installation_state == "broken" {
            return Some(connection_repair_action(
                "guard_files_broken",
                "Repair broken Codex Record Guard files, then run init again.",
                connection,
                projects,
            ));
        }
        if guard_state.installation_state == "degraded" {
            return Some(guard_degraded_action(connection, projects));
        }
        if guard_state.installation_state == "reload_required" {
            if let Some(action) = actions
                .iter()
                .find(|action| action.kind == UserActionKind::ReloadRequired)
            {
                let mut primary =
                    PrimaryNextAction::new(user_action_id(action.kind), action.message.clone());
                attach_connection_verify_command(&mut primary, connection, projects);
                return Some(primary);
            }
        }
    }
    if connection.is_none() {
        if let Some(action) = actions
            .iter()
            .find(|action| action.kind == UserActionKind::ReloadRequired)
        {
            let mut primary =
                PrimaryNextAction::new(user_action_id(action.kind), action.message.clone());
            attach_connection_verify_command(&mut primary, connection, projects);
            return Some(primary);
        }
    }
    prioritized_connection_action(actions).map(|action| {
        let mut primary =
            PrimaryNextAction::new(user_action_id(action.kind), action.message.clone());
        attach_connection_verify_command(&mut primary, connection, projects);
        primary
    })
}

fn prioritized_connection_action(actions: &[UserAction]) -> Option<&UserAction> {
    [
        UserActionKind::HostTrustRequired,
        UserActionKind::RepairManagedConfig,
        UserActionKind::InstallOrRepairCodex,
        UserActionKind::RepairMcpServer,
        UserActionKind::ReloadHost,
        UserActionKind::UseVolicordTool,
        UserActionKind::ReloadGuard,
        UserActionKind::RepairGuard,
        UserActionKind::ReloadRequired,
    ]
    .into_iter()
    .find_map(|kind| actions.iter().find(|action| action.kind == kind))
    .or_else(|| actions.first())
}

fn connection_host_display_name(connection: &AgentConnectionRecord) -> String {
    parse_host_kind(&connection.host_kind)
        .map(public_host_display_name)
        .unwrap_or_else(|_| public_host_name_text(&connection.host_kind))
        .to_owned()
}

fn attach_connection_verify_command(
    action: &mut PrimaryNextAction,
    connection: Option<&AgentConnectionRecord>,
    projects: &[ConnectionProjectRecord],
) {
    if !next_action_should_verify(&action.id) {
        return;
    }
    let Some(command) = connection_verify_command(connection, projects) else {
        return;
    };
    set_verify_command(action, command);
}

fn next_action_should_verify(id: &str) -> bool {
    matches!(
        id,
        "host_trust_required"
            | "project_approval_required"
            | "reload_required"
            | "managed_host_startup_not_observed"
            | "managed_host_tools_list_not_observed"
            | "active_tool_exposure_unconfirmed"
            | "managed_host_storage_degraded"
    )
}

fn set_verify_command(action: &mut PrimaryNextAction, command: String) {
    action.command = Some(command);
}

fn connection_verify_command(
    connection: Option<&AgentConnectionRecord>,
    projects: &[ConnectionProjectRecord],
) -> Option<String> {
    let connection = connection?;
    let project = projects.first()?;
    let intent = parse_connection_intent(&connection.intent).ok()?;
    Some(format!(
        "volicord connection verify {}{} --repo {}",
        public_host_name_text(&connection.host_kind),
        intent_flag_suffix(intent),
        project.project.repo_root.display()
    ))
}

fn guard_degraded_action(
    connection: Option<&AgentConnectionRecord>,
    projects: &[ConnectionProjectRecord],
) -> PrimaryNextAction {
    let Some(connection) = connection else {
        return PrimaryNextAction::new(
            "guard_capability_degraded",
            "Repair the required Codex Record Guard hook configuration before rerunning init.",
        );
    };
    let Some(project) = projects.first() else {
        return PrimaryNextAction::new(
            "guard_capability_degraded",
            "Repair the required Codex Record Guard hook configuration before rerunning init.",
        );
    };
    let host = public_host_name_text(&connection.host_kind);
    let intent = parse_connection_intent(&connection.intent).unwrap_or(ConnectionIntent::Personal);
    let command = format!(
        "volicord init --host {}{} --repo {}",
        host,
        intent_flag_suffix(intent),
        project.project.repo_root.display()
    );
    PrimaryNextAction::new(
        "guard_capability_degraded",
        "Repair the required Codex Record Guard hook configuration before rerunning init.",
    )
    .with_command(command)
}

fn connection_repair_action(
    id: &'static str,
    fallback: &'static str,
    connection: Option<&AgentConnectionRecord>,
    projects: &[ConnectionProjectRecord],
) -> PrimaryNextAction {
    let Some(connection) = connection else {
        return PrimaryNextAction::new(id, fallback);
    };
    let Some(project) = projects.first() else {
        return PrimaryNextAction::new(id, fallback);
    };
    let host = public_host_name_text(&connection.host_kind);
    let command = if connection.intent == ConnectionIntent::Shared.as_str() {
        format!(
            "volicord init --host {} --shared --repo {}",
            host,
            project.project.repo_root.display()
        )
    } else {
        format!(
            "volicord connection add {}{} --repo {}",
            host,
            intent_flag_suffix(
                parse_connection_intent(&connection.intent).unwrap_or(ConnectionIntent::Personal)
            ),
            project.project.repo_root.display()
        )
    };
    let instruction = repair_instruction(id, fallback);
    PrimaryNextAction::new(id, instruction).with_command(command)
}

fn repair_instruction(id: &str, fallback: &str) -> String {
    match id {
        "mcp_config_missing" => "Reinstall missing MCP configuration.".to_owned(),
        "mcp_config_changed" => {
            "Review the changed MCP configuration and repair it if Volicord should manage it."
                .to_owned()
        }
        "mcp_config_malformed" => "Repair the malformed MCP configuration.".to_owned(),
        "guard_files_missing" => "Reinstall missing Codex Record Guard files.".to_owned(),
        "guard_files_stale" => "Refresh stale Codex Record Guard files.".to_owned(),
        "guard_files_broken" => "Repair broken Codex Record Guard files.".to_owned(),
        _ => fallback.to_owned(),
    }
}
