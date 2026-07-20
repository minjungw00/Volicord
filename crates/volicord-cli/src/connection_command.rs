use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{
        activate_staged_connection, add_connection_project, agent_connection_record,
        complete_pending_host_cleanup, connection_metadata_has_pending_host_cleanup,
        connection_metadata_has_pending_host_cleanup_for_project, ensure_agent_connection,
        ensure_staged_agent_connection, list_agent_connections,
        list_agent_connections_for_diagnostics, list_connection_projects,
        list_connection_projects_for_diagnostics, list_connection_projects_read_only,
        remove_connection_project, replace_agent_connection_verification_report_if_revision,
        staged_connection_migration_state, transition_connection_mode, AgentConnectionRecord,
        AgentConnectionRegistration, ConnectionModeGuardManifestRebind, ConnectionModeTransition,
        ConnectionModeTransitionKind, ConnectionProjectRecord, ConnectionProjectRegistration,
        PendingHostCleanupError, StagedConnectionMigrationState, SupersededConnectionProject,
        CONNECTION_INTENT_PERSONAL, CONNECTION_INTENT_SHARED, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT, HOST_SCOPE_USER,
    },
    bootstrap::{
        ensure_project_for_repo, initialize_runtime_home, installation_profile,
        installation_profile_read_only, project_record_by_repo_root_read_only,
        write_installation_profile, InstallationProfileRecord, InstallationProfileRegistration,
        RepoProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::CoreProjectStore,
    guards::{list_guard_installations, GuardInstallationRecord},
    operational_sessions::connection_integration_revision,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    workflow_records::ProjectWorkflowPolicyAuthorityApply,
    StoreError,
};
use volicord_types::{
    canonical_json_sha256, canonical_json_string, guard_manifest_from_json,
    ConnectionVerificationError, ConnectionVerificationReport, IntegrationProfile,
    IntegrationRevision, ProjectId,
};

use crate::cli::{
    ConnectionAddArgs, ConnectionArgs, ConnectionCommand, ConnectionListArgs, ConnectionModeArgs,
    ConnectionRemoveArgs, ConnectionSelectArgs, InitArgs,
};
use crate::guard_integration::audit::guard_manifest_binding_valid_for_installation;
use crate::guard_integration::hooks::shell_word;
use crate::guard_integration::{
    apply_guard_integration, apply_guard_migration_protection, guard_installation_upsert,
    plan_guard_integration, record_guard_installation, GuardIntegrationError, GuardIntegrationPlan,
    GuardIntegrationPlanRequest,
};
use crate::host_integration::{
    codex::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest},
    ConnectionIntent, HostAdapter, HostConfigError, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, ProjectContext,
};
use crate::{
    registration::ADMIN_METADATA_JSON,
    setup_command::{is_executable_file, path_text as setup_path_text, runtime_home_id_for_path},
};

mod args;
mod mcp_process;
mod output;
mod persisted_state;
mod planning;
mod selection;
mod service;
mod verification;

pub use mcp_process::{
    ConnectionProcess, ConnectionProcessOutput, McpLaunch, McpVerification,
    ProductionConnectionProcess,
};

use args::{
    absolute_path, connection_add_options, connection_list_options, connection_mode_options,
    connection_output_format, connection_remove_options, connection_select_options, init_options,
    init_output_format, InitMode, OutputFormat, ParsedConnectionOptions, ParsedInitOptions,
};
use mcp_process::mcp_launch_from_host_plan;
use output::{
    render_command_report, render_connections_output, CommandConnection, CommandOperation,
    ConnectionCommandReport,
};
use persisted_state::{decode_persisted_object, PERSISTED_CONNECTION_METADATA_CORRUPT_REASON};
use planning::{
    plan_init_changes, InitPlannedChanges, PlannedChangeOperation, PlannedConnectionChange,
    PlannedConnectionChangeKind,
};
use selection::{
    connection_for_host_target, connection_selector, host_scope_for_intent,
    resolve_connection_host, resolve_connection_repo_root, select_connection,
    select_connection_for_diagnostics, selected_connection_project,
};
use service::{
    provision_connection, provision_init, ConnectionProvisioningOutcome, InitProvisioningRequest,
    ProvisionConnectionRequest,
};
use verification::{
    current_status_report, effective_connection_report, verify_connection, VerificationReport,
};

const PATH_ENV: &str = "PATH";
const AGENT_METADATA_CREATED_BY: &str = "volicord_cli_agent_connection";
const AGENT_RUNTIME_HOME_ID: &str = "runtime_home_agent";
const INIT_METADATA_CREATED_BY: &str = "volicord_cli_init";
const DEFAULT_MCP_COMMAND: &str = "volicord";
const DEFAULT_SERVER_NAME: &str = "volicord";
const INSTALLATION_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionCommandError {
    Usage(String),
    Runtime(String),
    FailureOutput(String),
}

impl ConnectionCommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for ConnectionCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::FailureOutput(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for ConnectionCommandError {}

impl From<StoreError> for ConnectionCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for ConnectionCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<GuardIntegrationError> for ConnectionCommandError {
    fn from(error: GuardIntegrationError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<HostConfigError> for ConnectionCommandError {
    fn from(error: HostConfigError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<ConnectionVerificationError> for ConnectionCommandError {
    fn from(error: ConnectionVerificationError) -> Self {
        Self::runtime(error.to_string())
    }
}

pub fn run_init_command(
    args: InitArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = init_options(args, current_dir);
    let outcome = provision_init(
        InitProvisioningRequest {
            parsed: &parsed,
            current_dir,
        },
        process,
    )?;
    let connection = CommandConnection::new(
        &outcome.connection_id,
        outcome.host_kind.as_str(),
        outcome.host_scope.as_str(),
        &outcome.mode,
        &outcome.repo_root,
        host_target_text(&outcome.host_plan.target),
    );
    let report = if outcome.dry_run {
        ConnectionCommandReport::setup_dry_run(
            CommandOperation::Init,
            &outcome.runtime_home,
            connection,
            outcome.current_report.as_ref(),
            outcome.planned_changes,
            &outcome.host_plan.actions,
        )?
    } else {
        let verification = outcome.verification.as_ref().ok_or_else(|| {
            ConnectionCommandError::runtime(
                "applied init requires one canonical verification report",
            )
        })?;
        ConnectionCommandReport::from_verification(
            CommandOperation::Init,
            Some(true),
            &outcome.runtime_home,
            connection,
            &verification.report,
        )
    };
    let rendered = render_command_report(init_output_format(&parsed), &report)?;
    command_output_result(rendered.status, rendered.output)
}

fn command_output_result(
    status: volicord_types::ConnectionStatus,
    rendered_output: String,
) -> Result<String, ConnectionCommandError> {
    if status == volicord_types::ConnectionStatus::Failed {
        Err(ConnectionCommandError::FailureOutput(rendered_output))
    } else {
        Ok(rendered_output)
    }
}

pub fn run_connect_command(
    args: ConnectionAddArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = connection_add_options(args, current_dir);
    match provision_connection(
        ProvisionConnectionRequest {
            parsed: &parsed,
            current_dir,
        },
        process,
    )? {
        ConnectionProvisioningOutcome::DryRun(plan) => {
            let plan = *plan;
            let report = ConnectionCommandReport::setup_dry_run(
                CommandOperation::Add,
                &plan.runtime_home,
                CommandConnection::new(
                    &plan.connection_id,
                    plan.host_kind.as_str(),
                    plan.host_scope.as_str(),
                    &plan.effective_mode,
                    &plan.repo_root,
                    host_target_text(&plan.host_plan.target),
                ),
                plan.current_report.as_ref(),
                plan.planned_changes,
                &plan.host_plan.actions,
            )?;
            let rendered = render_command_report(connection_output_format(&parsed), &report)?;
            command_output_result(rendered.status, rendered.output)
        }
        ConnectionProvisioningOutcome::Applied(outcome) => {
            let outcome = *outcome;
            let report = ConnectionCommandReport::from_verification(
                CommandOperation::Add,
                Some(true),
                &outcome.runtime_home,
                CommandConnection::new(
                    &outcome.connection.connection_internal_id,
                    &outcome.connection.host_kind,
                    &outcome.connection.host_scope,
                    &outcome.connection.mode,
                    &outcome.affected_repo_root,
                    &outcome.connection.config_target,
                ),
                &outcome.verification.report,
            );
            let rendered = render_command_report(connection_output_format(&parsed), &report)?;
            command_output_result(rendered.status, rendered.output)
        }
    }
}

pub fn run_connections_command(
    args: ConnectionListArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = connection_list_options(args, current_dir);
    let SelectedConnectionRuntimeHome {
        path: runtime_home, ..
    } = selected_connection_runtime_home_read_only(
        parsed.explicit_runtime_home.as_deref(),
        |name| process.env_var(name),
        current_dir,
    )?;
    let repo_root = parsed
        .repo
        .as_deref()
        .map(|repo| resolve_connection_repo_root(current_dir, Some(repo)))
        .transpose()?;
    let mut rows = Vec::new();
    for connection in list_agent_connections_for_diagnostics(&runtime_home)? {
        let projects = list_connection_projects_for_diagnostics(
            &runtime_home,
            &connection.connection_internal_id,
        )?;
        if repo_root.as_ref().is_none_or(|repo_root| {
            projects
                .iter()
                .any(|project| project.project.repo_root == *repo_root)
        }) {
            rows.push((connection, projects));
        }
    }
    render_connections_output(connection_output_format(&parsed), &rows)
}

pub fn run_connection_command(
    args: ConnectionArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    match args.command {
        ConnectionCommand::Add(options) => run_connect_command(options, current_dir, process),
        ConnectionCommand::List(options) => run_connections_command(options, current_dir, process),
        ConnectionCommand::Status(options) => {
            command_connection_status(options, current_dir, process)
        }
        ConnectionCommand::Verify(options) => {
            command_connection_verify(options, current_dir, process)
        }
        ConnectionCommand::Mode(options) => command_connection_mode(options, current_dir, process),
        ConnectionCommand::Remove(options) => {
            command_connection_remove(options, current_dir, process)
        }
    }
}

fn command_connection_status(
    args: ConnectionSelectArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = connection_select_options(args, current_dir);
    let SelectedConnectionRuntimeHome {
        path: runtime_home, ..
    } = selected_connection_runtime_home_read_only(
        parsed.explicit_runtime_home.as_deref(),
        |name| process.env_var(name),
        current_dir,
    )?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, projects) = select_connection_for_diagnostics(&runtime_home, &selector)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
    let mut report = effective_connection_report(&connection)?;
    let persisted_metadata_corrupt = decode_persisted_object(&connection.metadata_json).is_none();
    if persisted_metadata_corrupt {
        report = verification::connection_metadata_failure_report(&report)?;
        let report = ConnectionCommandReport::from_verification(
            CommandOperation::Status,
            None,
            &runtime_home,
            CommandConnection::new(
                &connection.connection_internal_id,
                &connection.host_kind,
                &connection.host_scope,
                &connection.mode,
                &selected_project.project.repo_root,
                &connection.config_target,
            ),
            &report,
        );
        let rendered = render_command_report(connection_output_format(&parsed), &report)?;
        return command_output_result(rendered.status, rendered.output);
    }
    let host_plan =
        existing_host_plan(&connection, &runtime_home, process, Some(selected_project))?;
    let (_, report) = current_status_report(
        &runtime_home,
        &connection,
        Some(&host_plan),
        &projects,
        process,
    )?;
    let report = ConnectionCommandReport::from_verification(
        CommandOperation::Status,
        None,
        &runtime_home,
        CommandConnection::new(
            &connection.connection_internal_id,
            &connection.host_kind,
            &connection.host_scope,
            &connection.mode,
            &selected_project.project.repo_root,
            &connection.config_target,
        ),
        &report,
    );
    let rendered = render_command_report(connection_output_format(&parsed), &report)?;
    command_output_result(rendered.status, rendered.output)
}

fn command_connection_verify(
    args: ConnectionSelectArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = connection_select_options(args, current_dir);
    let SelectedConnectionRuntimeHome {
        path: runtime_home, ..
    } = selected_connection_runtime_home_read_only(
        parsed.explicit_runtime_home.as_deref(),
        |name| process.env_var(name),
        current_dir,
    )?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (mut connection, projects) = select_connection_for_diagnostics(&runtime_home, &selector)?;
    if decode_persisted_object(&connection.metadata_json).is_none() {
        return Err(ConnectionCommandError::runtime(format!(
            "{PERSISTED_CONNECTION_METADATA_CORRUPT_REASON}: connection verification cannot repair Agent Connection registration metadata; recreate or repair the registration before retrying"
        )));
    }
    let expected_integration_revision = connection_integration_revision(&connection)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
    let selected_repo_root = selected_project.project.repo_root.clone();
    let host_plan =
        existing_host_plan(&connection, &runtime_home, process, Some(selected_project))?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&selected_project.project.repo_root));
    let verification = verify_connection(
        &runtime_home,
        &connection,
        &host_plan,
        &launch,
        Some(&selected_project.project_id),
        process,
    )?;
    connection = persist_connection_verification_report(
        &runtime_home,
        &connection.connection_internal_id,
        &expected_integration_revision,
        Some(&verification.report),
    )?;
    let report = ConnectionCommandReport::from_verification(
        CommandOperation::Verify,
        None,
        &runtime_home,
        CommandConnection::new(
            &connection.connection_internal_id,
            &connection.host_kind,
            &connection.host_scope,
            &connection.mode,
            &selected_repo_root,
            &connection.config_target,
        ),
        &verification.report,
    );
    let rendered = render_command_report(connection_output_format(&parsed), &report)?;
    command_output_result(rendered.status, rendered.output)
}

fn persist_connection_verification_report(
    runtime_home: &Path,
    connection_internal_id: &str,
    expected_integration_revision: &IntegrationRevision,
    verification_report: Option<&ConnectionVerificationReport>,
) -> Result<AgentConnectionRecord, ConnectionCommandError> {
    match replace_agent_connection_verification_report_if_revision(
        runtime_home,
        connection_internal_id,
        expected_integration_revision,
        verification_report,
    ) {
        Ok(connection) => Ok(connection),
        Err(StoreError::Conflict { .. }) => Err(ConnectionCommandError::runtime(
            "CONNECTION_VERIFICATION_CONFLICT: the Agent Connection changed while verification was running; rerun `volicord connection verify` against the current Connection revision",
        )),
        Err(error) => Err(ConnectionCommandError::from(error)),
    }
}

fn command_connection_mode(
    args: ConnectionModeArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let mode = args.mode.as_str().to_owned();
    let parsed = connection_mode_options(args, current_dir);
    let SelectedConnectionRuntimeHome {
        path: runtime_home, ..
    } = selected_connection_runtime_home_read_only(
        parsed.explicit_runtime_home.as_deref(),
        |name| process.env_var(name),
        current_dir,
    )?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, projects) = select_connection(&runtime_home, &selector)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
    let previous_mode = connection.mode.clone();
    let expected_revision = connection_integration_revision(&connection)?;
    let guard_manifests = if connection.mode == mode {
        Vec::new()
    } else {
        preflight_mode_guard_rebinds(&runtime_home, &connection, &projects, &mode)?
    };
    let outcome = transition_connection_mode(
        &runtime_home,
        ConnectionModeTransition {
            connection_internal_id: connection.connection_internal_id.clone(),
            expected_mode: connection.mode.clone(),
            expected_integration_revision: expected_revision,
            mode,
            guard_manifests,
        },
    )?;
    let report = ConnectionCommandReport::mode_transition(
        &runtime_home,
        CommandConnection::new(
            &outcome.connection.connection_internal_id,
            &outcome.connection.host_kind,
            &outcome.connection.host_scope,
            &outcome.connection.mode,
            &selected_project.project.repo_root,
            &outcome.connection.config_target,
        ),
        outcome.kind == ConnectionModeTransitionKind::Updated,
        previous_mode,
        outcome.connection.mode.clone(),
        outcome.previous_integration_revision.as_str().to_owned(),
        outcome.current_integration_revision.as_str().to_owned(),
        outcome.rebound_guard_installation_ids.clone(),
    )?;
    let rendered = render_command_report(connection_output_format(&parsed), &report)?;
    command_output_result(rendered.status, rendered.output)
}

fn preflight_mode_guard_rebinds(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    mode: &str,
) -> Result<Vec<ConnectionModeGuardManifestRebind>, ConnectionCommandError> {
    let mut candidate_connection = connection.clone();
    candidate_connection.mode = mode.to_owned();
    candidate_connection.integration_generation = candidate_connection
        .integration_generation
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "Agent Connection integration generation is exhausted".to_owned(),
        })?;
    let candidate_revision = connection_integration_revision(&candidate_connection)?;
    let mut rebinds = Vec::with_capacity(projects.len());

    for project in projects {
        let repair =
            owning_init_repair_command(connection, &project.project.repo_root, runtime_home);
        let installations = list_guard_installations(
            runtime_home,
            &connection.connection_internal_id,
            Some(&project.project_id),
        )
        .map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "cannot change the Connection mode because the Guard Installation for {} is malformed or unavailable: {error}; repair it by rerunning `{repair}`",
                project.project.repo_root.display()
            ))
        })?;
        let [installation] = installations.as_slice() else {
            return Err(ConnectionCommandError::runtime(format!(
                "cannot change the Connection mode because {} must have exactly one current Guard Installation; repair it by rerunning `{repair}`",
                project.project.repo_root.display()
            )));
        };
        if !guard_manifest_binding_valid_for_installation(installation, connection, projects) {
            return Err(ConnectionCommandError::runtime(format!(
                "cannot change the Connection mode because Guard Installation {} is not owned by the selected Connection and project; repair it by rerunning `{repair}`",
                installation.guard_installation_id
            )));
        }
        let mut manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "cannot change the Connection mode because Guard Installation {} is malformed: {error}; repair it by rerunning `{repair}`",
                installation.guard_installation_id
            ))
        })?;
        manifest.integration_revision = candidate_revision.clone();
        let manifest_json = serde_json::to_string(&manifest).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "could not construct the candidate Guard manifest for {}: {error}",
                installation.guard_installation_id
            ))
        })?;
        let candidate_installation = GuardInstallationRecord {
            manifest_json: manifest_json.clone(),
            ..installation.clone()
        };
        if !guard_manifest_binding_valid_for_installation(
            &candidate_installation,
            &candidate_connection,
            projects,
        ) {
            return Err(ConnectionCommandError::runtime(format!(
                "candidate Guard manifest {} does not match the requested Connection revision; repair the current installation by rerunning `{repair}`",
                installation.guard_installation_id
            )));
        }
        rebinds.push(ConnectionModeGuardManifestRebind {
            guard_installation_id: installation.guard_installation_id.clone(),
            project_id: project.project_id.clone(),
            expected_manifest_json: installation.manifest_json.clone(),
            manifest_json,
        });
    }
    Ok(rebinds)
}

fn owning_init_repair_command(
    connection: &AgentConnectionRecord,
    repo_root: &Path,
    runtime_home: &Path,
) -> String {
    let shared = if connection.intent == CONNECTION_INTENT_SHARED {
        " --shared"
    } else {
        ""
    };
    let repo_root = shell_word(&path_text(repo_root));
    let runtime_home = shell_word(&path_text(runtime_home));
    format!(
        "volicord init{shared} --host codex --repo {repo_root} --profile record --home {runtime_home}"
    )
}

fn command_connection_remove(
    args: ConnectionRemoveArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = connection_remove_options(args, current_dir);
    let SelectedConnectionRuntimeHome {
        path: runtime_home, ..
    } = selected_connection_runtime_home_read_only(
        parsed.explicit_runtime_home.as_deref(),
        |name| process.env_var(name),
        current_dir,
    )?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, projects) = select_connection(&runtime_home, &selector)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
    let remaining_count = projects.len().saturating_sub(1);
    let host_plan = if remaining_count == 0 {
        Some(existing_host_plan(
            &connection,
            &runtime_home,
            process,
            Some(selected_project),
        )?)
    } else {
        None
    };
    if parsed.dry_run {
        let mut planned_changes = vec![PlannedConnectionChange::new(
            PlannedConnectionChangeKind::ConnectionMembership,
            PlannedChangeOperation::Remove,
            path_text(&selected_project.project.repo_root),
        )];
        for installation in list_guard_installations(
            &runtime_home,
            &connection.connection_internal_id,
            Some(&selected_project.project_id),
        )? {
            planned_changes.push(PlannedConnectionChange::new(
                PlannedConnectionChangeKind::GuardRegistrySetup,
                PlannedChangeOperation::Remove,
                installation.guard_installation_id,
            ));
        }
        if remaining_count == 0 {
            planned_changes.push(PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ManagedHostConfiguration,
                PlannedChangeOperation::Remove,
                &connection.config_target,
            ));
        }
        planning::canonicalize_planned_changes(&mut planned_changes);
        let report = ConnectionCommandReport::removal_dry_run(
            &runtime_home,
            CommandConnection::new(
                &connection.connection_internal_id,
                &connection.host_kind,
                &connection.host_scope,
                &connection.mode,
                &selected_project.project.repo_root,
                &connection.config_target,
            ),
            planned_changes,
        )?;
        let rendered = render_command_report(connection_output_format(&parsed), &report)?;
        return command_output_result(rendered.status, rendered.output);
    }

    if let Some(host_plan) = &host_plan {
        remove_host_configuration(host_plan, &connection, process)?;
    }
    let removal_outcome = remove_connection_project(
        &runtime_home,
        &connection.connection_internal_id,
        &selected_project.project_id,
    )?;
    let report = ConnectionCommandReport::removal(
        &runtime_home,
        CommandConnection::new(
            &connection.connection_internal_id,
            &connection.host_kind,
            &connection.host_scope,
            &connection.mode,
            &selected_project.project.repo_root,
            &connection.config_target,
        ),
        removal_outcome.membership_removed,
        removal_outcome.connection_removed,
        removal_outcome.remaining_project_count,
    )?;
    let rendered = render_command_report(connection_output_format(&parsed), &report)?;
    command_output_result(rendered.status, rendered.output)
}

fn resolve_init_repo_root(
    current_dir: &Path,
    repo: &Path,
    _host_kind: HostKind,
    init_mode: InitMode,
) -> Result<PathBuf, ConnectionCommandError> {
    match resolve_connection_repo_root(current_dir, Some(repo)) {
        Ok(repo_root) => Ok(repo_root),
        Err(ConnectionCommandError::Runtime(message))
            if init_mode == InitMode::Record
                && message.contains("no Git repository root found") =>
        {
            resolve_explicit_record_repo_root(current_dir, repo)
        }
        Err(error) => Err(error),
    }
}

fn resolve_explicit_record_repo_root(
    current_dir: &Path,
    repo: &Path,
) -> Result<PathBuf, ConnectionCommandError> {
    let absolute = if repo.is_absolute() {
        repo.to_path_buf()
    } else {
        current_dir.join(repo)
    };
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            absolute.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            canonical.display()
        ))
    })?;
    if !metadata.is_dir() {
        return Err(ConnectionCommandError::runtime(format!(
            "record-profile Product Repository path must be a directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn connection_intent_from_flags(
    parsed: &ParsedConnectionOptions,
) -> Result<ConnectionIntent, ConnectionCommandError> {
    if parsed.shared {
        Ok(ConnectionIntent::Shared)
    } else {
        Ok(ConnectionIntent::Personal)
    }
}

fn public_host_label(host_kind: HostKind) -> &'static str {
    match host_kind {
        HostKind::Codex => "codex",
    }
}

fn intent_flag_suffix(intent: ConnectionIntent) -> &'static str {
    match intent {
        ConnectionIntent::Personal => "",
        ConnectionIntent::Shared => " --shared",
    }
}

fn public_host_name_text(host_kind: &str) -> &str {
    match host_kind {
        HOST_KIND_CODEX => "codex",
        other => other,
    }
}

fn public_mode_text(mode: &str) -> &str {
    match mode {
        CONNECTION_MODE_READ_ONLY => "read-only",
        CONNECTION_MODE_WORKFLOW => "workflow",
        other => other,
    }
}

fn parse_host_kind(value: &str) -> Result<HostKind, ConnectionCommandError> {
    match value {
        HOST_KIND_CODEX => Ok(HostKind::Codex),
        other => Err(ConnectionCommandError::usage(format!(
            "unsupported host in Agent Connection registry: {other}"
        ))),
    }
}

fn parse_host_scope(value: &str) -> Result<HostScope, ConnectionCommandError> {
    match value {
        HOST_SCOPE_USER => Ok(HostScope::User),
        HOST_SCOPE_PROJECT => Ok(HostScope::Project),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown scope: {other}"
        ))),
    }
}

fn parse_connection_intent(value: &str) -> Result<ConnectionIntent, ConnectionCommandError> {
    match value {
        CONNECTION_INTENT_PERSONAL => Ok(ConnectionIntent::Personal),
        CONNECTION_INTENT_SHARED => Ok(ConnectionIntent::Shared),
        other => Err(ConnectionCommandError::runtime(format!(
            "unknown connection intent in registry: {other}"
        ))),
    }
}

fn required_connection_installation_profile_read_only(
    runtime_home: &Path,
) -> Result<InstallationProfileRecord, ConnectionCommandError> {
    match installation_profile_read_only(runtime_home) {
        Ok(Some(profile)) => Ok(profile),
        Ok(None) => Err(ConnectionCommandError::runtime(
            connection_setup_required_message(runtime_home),
        )),
        Err(error) => Err(ConnectionCommandError::runtime(format!(
            "{error}; {}",
            connection_setup_required_message(runtime_home)
        ))),
    }
}

fn connection_setup_required_message(runtime_home: &Path) -> String {
    let runtime_home_argument = shell_word(&path_text(runtime_home));
    if runtime_home.exists() {
        format!(
            "SETUP_REQUIRED: installation profile is missing for Runtime Home {}; run `volicord init --host <host> --repo <path> --home {runtime_home_argument}` from the Product Repository to initialize Volicord.",
            runtime_home.display(),
        )
    } else {
        format!(
            "RUNTIME_HOME_MISSING: Runtime Home {} is missing; run `volicord init --host <host> --repo <path> --home {runtime_home_argument}` from the Product Repository to initialize Volicord.",
            runtime_home.display(),
        )
    }
}

struct InitProfilePlan {
    volicord_command: PathBuf,
    volicord_mcp_command: PathBuf,
    bin_dir: PathBuf,
    metadata_json: String,
}

fn selected_runtime_home_path<F>(
    explicit_runtime_home: Option<&Path>,
    env_var: F,
    current_dir: &Path,
) -> Result<PathBuf, RuntimeHomeResolutionError>
where
    F: Fn(&str) -> Option<OsString>,
{
    if let Some(path) = explicit_runtime_home {
        Ok(absolute_path(current_dir, path.to_path_buf()))
    } else {
        resolve_runtime_home(env_var, current_dir)
    }
}

struct SelectedConnectionRuntimeHome {
    path: PathBuf,
    installation_profile: InstallationProfileRecord,
}

fn selected_connection_runtime_home_read_only<F>(
    explicit_runtime_home: Option<&Path>,
    env_var: F,
    current_dir: &Path,
) -> Result<SelectedConnectionRuntimeHome, ConnectionCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let path = selected_runtime_home_path(explicit_runtime_home, env_var, current_dir)?;
    let installation_profile = required_connection_installation_profile_read_only(&path)?;
    Ok(SelectedConnectionRuntimeHome {
        path,
        installation_profile,
    })
}

fn init_profile_plan(
    parsed: &ParsedInitOptions,
    runtime_home: &Path,
    existing: Option<&InstallationProfileRecord>,
    process: &impl ConnectionProcess,
) -> Result<InitProfilePlan, ConnectionCommandError> {
    let current_exe = canonical_existing_file(
        &process
            .current_exe()
            .map_err(ConnectionCommandError::runtime)?,
        "volicord command",
    )?;
    let existing_volicord_command = existing.and_then(|profile| {
        let command = Path::new(&profile.volicord_command);
        command
            .is_absolute()
            .then(|| {
                canonical_existing_executable(command, "installation profile volicord command")
            })
            .and_then(Result::ok)
    });
    let volicord_command_source = if existing_volicord_command.is_some() {
        "existing_profile"
    } else if existing.is_some() {
        "current_exe_repair"
    } else {
        "current_exe"
    };
    let volicord_command = existing_volicord_command.unwrap_or_else(|| current_exe.clone());
    let volicord_mcp_command = match &parsed.mcp_command {
        Some(path) => canonical_existing_executable(path, "MCP launch command")?,
        None => existing
            .map(|profile| PathBuf::from(&profile.volicord_mcp_command))
            .unwrap_or(current_exe),
    };
    let bin_dir = volicord_command
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| runtime_home.join("bin"));
    let metadata_json = serde_json::to_string(&json!({
        "created_by": INIT_METADATA_CREATED_BY,
        "volicord_command_source": volicord_command_source,
        "volicord_mcp_command_source": if parsed.mcp_command.is_some() {
            "explicit"
        } else if existing.is_some() {
            "existing_profile"
        } else {
            "current_exe"
        },
    }))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    Ok(InitProfilePlan {
        volicord_command,
        volicord_mcp_command,
        bin_dir,
        metadata_json,
    })
}

fn ensure_init_installation_profile(
    runtime_home: &Path,
    plan: &InitProfilePlan,
) -> Result<InstallationProfileRecord, ConnectionCommandError> {
    write_installation_profile(
        runtime_home,
        InstallationProfileRegistration {
            installation_id: INSTALLATION_ID.to_owned(),
            volicord_command: setup_path_text(&plan.volicord_command),
            volicord_mcp_command: setup_path_text(&plan.volicord_mcp_command),
            bin_dir: plan.bin_dir.clone(),
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json: plan.metadata_json.clone(),
        },
    )
    .map_err(Into::into)
}

fn canonical_existing_file(
    path: &Path,
    label: &'static str,
) -> Result<PathBuf, ConnectionCommandError> {
    let metadata = fs::metadata(path).map_err(|error| {
        ConnectionCommandError::runtime(format!("{label} is not accessible: {error}"))
    })?;
    if !metadata.is_file() {
        return Err(ConnectionCommandError::runtime(format!(
            "{label} must be a file: {}",
            path.display()
        )));
    }
    Ok(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn canonical_existing_executable(
    path: &Path,
    label: &'static str,
) -> Result<PathBuf, ConnectionCommandError> {
    let path = canonical_existing_file(path, label)?;
    if is_executable_file(&path) {
        Ok(path)
    } else {
        Err(ConnectionCommandError::runtime(format!(
            "{label} must be executable: {}",
            path.display()
        )))
    }
}

fn installation_profile_context<'a>(
    runtime_home: &'a Path,
    profile: &'a InstallationProfileRecord,
) -> InstallationProfile<'a> {
    InstallationProfile {
        runtime_home,
        volicord_command: Path::new(&profile.volicord_command),
        volicord_mcp_command: Path::new(&profile.volicord_mcp_command),
        default_connection_mode: &profile.default_connection_mode,
    }
}

fn enforce_single_project_scope(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    project_id: &str,
) -> Result<(), ConnectionCommandError> {
    let scope = parse_host_scope(&connection.host_scope)?;
    if scope != HostScope::Project {
        return Ok(());
    }
    let projects = list_connection_projects(runtime_home, &connection.connection_internal_id)?;
    if projects
        .iter()
        .any(|project| project.project_id != project_id)
    {
        return Err(ConnectionCommandError::runtime(
            "shared Agent Connections may allow only one project",
        ));
    }
    Ok(())
}

fn connection_target_hint(
    _host_kind: HostKind,
    scope: HostScope,
    repo_root: Option<&Path>,
    process: &impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    match scope {
        HostScope::User => {
            let path = codex_home(process)?.join("config.toml");
            Ok(path_text(&path))
        }
        HostScope::Project => {
            let repo_root = repo_root.ok_or_else(|| {
                ConnectionCommandError::usage("Codex shared connection requires --repo PATH")
            })?;
            Ok(path_text(&repo_root.join(".codex").join("config.toml")))
        }
    }
}

struct BuildHostPlanRequest<'a> {
    host_kind: HostKind,
    connection_intent: ConnectionIntent,
    connection_id: &'a str,
    repo_root: Option<&'a Path>,
    project_id: Option<&'a str>,
    project_name: Option<&'a str>,
    installation_profile: InstallationProfile<'a>,
    mode: &'a str,
    expected_fingerprint: Option<&'a str>,
}

fn build_host_plan(
    request: BuildHostPlanRequest<'_>,
    process: &impl ConnectionProcess,
) -> Result<HostPlan, ConnectionCommandError> {
    let project = request.repo_root.map(|repo_root| ProjectContext {
        project_id: request.project_id.unwrap_or(""),
        project_name: request.project_name.unwrap_or(""),
        repo_root,
    });
    let plan_request = HostPlanRequest {
        host_kind: request.host_kind,
        connection_intent: request.connection_intent,
        project,
        installation_profile: request.installation_profile,
        connection_id: request.connection_id,
        mode: request.mode,
        expected_fingerprint: request.expected_fingerprint,
    };
    if request.host_kind != HostKind::Codex {
        return Err(ConnectionCommandError::usage(
            "only Codex managed connections are supported",
        ));
    }
    let adapter = CodexAdapter::new(codex_environment(process));
    adapter.plan(plan_request).map_err(Into::into)
}

fn apply_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    process: &impl ConnectionProcess,
) -> Result<(), ConnectionCommandError> {
    if host_kind != HostKind::Codex {
        return Err(ConnectionCommandError::usage(
            "only Codex managed connections are supported",
        ));
    }
    let mut adapter = CodexAdapter::new(codex_environment(process));
    adapter.apply(plan)?;
    Ok(())
}

fn remove_host_configuration(
    plan: &HostPlan,
    connection: &AgentConnectionRecord,
    process: &impl ConnectionProcess,
) -> Result<(), ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let request = HostRemoveRequest {
        host_kind,
        connection_intent: parse_connection_intent(&connection.intent)?,
        host_scope: parse_host_scope(&connection.host_scope)?,
        mode: connection.mode.clone(),
        server_name: connection.server_name.clone(),
        target: plan.target.clone(),
        expected_fingerprint: connection.managed_fingerprint.clone(),
    };
    if host_kind != HostKind::Codex {
        return Err(ConnectionCommandError::usage(
            "only Codex managed connections are supported",
        ));
    }
    let mut adapter = CodexAdapter::new(codex_environment(process));
    adapter.remove(request)?;
    Ok(())
}

fn existing_host_plan(
    connection: &AgentConnectionRecord,
    runtime_home: &Path,
    process: &impl ConnectionProcess,
    selected_project: Option<&ConnectionProjectRecord>,
) -> Result<HostPlan, ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let host_scope = parse_host_scope(&connection.host_scope)?;
    let connection_intent = parse_connection_intent(&connection.intent)?;
    let metadata = parse_metadata(
        &connection.metadata_json,
        selected_project.map(|project| project.project_id.as_str()),
    )?;
    let mcp_command = metadata
        .get("mcp_command")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MCP_COMMAND));
    let runtime_home_for_entry = metadata
        .get("host_runtime_home")
        .map(PathBuf::from)
        .or_else(|| runtime_home_for_host_config(host_scope, runtime_home).map(Path::to_path_buf));
    if host_kind != HostKind::Codex {
        return Err(ConnectionCommandError::usage(
            "only Codex managed connections are supported",
        ));
    }
    let adapter = CodexAdapter::new(codex_environment(process));
    adapter
        .plan_existing(CodexExistingPlanRequest {
            connection_intent,
            scope: host_scope,
            connection_id: &connection.connection_internal_id,
            project_id: selected_project.map(|project| project.project_id.as_str()),
            server_name: &connection.server_name,
            config_target: Path::new(&connection.config_target),
            mcp_command: &mcp_command,
            runtime_home: runtime_home_for_entry.as_deref(),
            mode: &connection.mode,
        })
        .map_err(Into::into)
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hex_bytes(&hasher.finalize());
    format!("{prefix}_{}", &digest[..16])
}

fn connection_project_planning_facts(
    runtime_home: &Path,
    connection_id: Option<&str>,
    project_id: Option<&str>,
    guard_installation_id: &str,
) -> Result<(bool, bool), ConnectionCommandError> {
    let (Some(connection_id), Some(project_id)) = (connection_id, project_id) else {
        return Ok((false, false));
    };
    let membership_exists = list_connection_projects_read_only(runtime_home, connection_id)?
        .iter()
        .any(|membership| membership.project_id == project_id);
    let guard_installation_exists =
        list_guard_installations(runtime_home, connection_id, Some(project_id))?
            .iter()
            .any(|installation| installation.guard_installation_id == guard_installation_id);
    Ok((membership_exists, guard_installation_exists))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn connection_metadata_json(
    plan: &HostPlan,
    mcp_command: &Path,
    runtime_home: &Path,
) -> Result<String, ConnectionCommandError> {
    let mut value = json!({
        "created_by": AGENT_METADATA_CREATED_BY,
        "mcp_command": path_text(mcp_command),
        "connection_intent": plan.connection_intent.as_str(),
        "mode": plan.mode.as_str(),
    });
    let object = value
        .as_object_mut()
        .expect("metadata should be object immediately after construction");
    if let Some(host_runtime_home) = runtime_home_for_host_config(plan.host_scope, runtime_home) {
        object.insert(
            "host_runtime_home".to_owned(),
            Value::String(path_text(host_runtime_home)),
        );
    }
    match &plan.target {
        HostTarget::File(path) => {
            object.insert("target_kind".to_owned(), Value::String("file".to_owned()));
            object.insert("target_path".to_owned(), Value::String(path_text(path)));
        }
        HostTarget::Export(path) => {
            object.insert("target_kind".to_owned(), Value::String("export".to_owned()));
            object.insert("target_path".to_owned(), Value::String(path_text(path)));
        }
        HostTarget::ExternalCli { program, cwd } => {
            object.insert(
                "target_kind".to_owned(),
                Value::String("external_cli".to_owned()),
            );
            object.insert(
                "external_program".to_owned(),
                Value::String(program.clone()),
            );
            if let Some(cwd) = cwd {
                object.insert("external_cwd".to_owned(), Value::String(path_text(cwd)));
            }
        }
    }
    serde_json::to_string(&value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn metadata_json_base() -> Result<String, ConnectionCommandError> {
    serde_json::to_string(&json!({ "created_by": AGENT_METADATA_CREATED_BY }))
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn parse_metadata(
    text: &str,
    pending_cleanup_project_id: Option<&str>,
) -> Result<BTreeMap<String, String>, ConnectionCommandError> {
    let value = serde_json::from_str::<Value>(text).map_err(|_| {
        ConnectionCommandError::runtime(
            "PERSISTED_CONNECTION_METADATA_CORRUPT: metadata_json is not valid JSON",
        )
    })?;
    let object = value.as_object().ok_or_else(|| {
        ConnectionCommandError::runtime(
            "PERSISTED_CONNECTION_METADATA_CORRUPT: metadata_json is not an object",
        )
    })?;
    let has_valid_pending_cleanup = pending_cleanup_project_id.is_some_and(|project_id| {
        connection_metadata_has_pending_host_cleanup_for_project(text, project_id)
    });
    object
        .iter()
        .filter(|(key, _)| !(has_valid_pending_cleanup && key.as_str() == "pending_host_cleanup"))
        .map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), value.to_owned()))
                .ok_or_else(|| {
                    ConnectionCommandError::runtime(format!(
                        "PERSISTED_CONNECTION_METADATA_CORRUPT: metadata_json.{key} is not a string"
                    ))
                })
        })
        .collect()
}

fn host_target_text(target: &HostTarget) -> String {
    match target {
        HostTarget::File(path) | HostTarget::Export(path) => path_text(path),
        HostTarget::ExternalCli { program, cwd } => cwd
            .as_ref()
            .map(|cwd| format!("{program} cwd={}", path_text(cwd)))
            .unwrap_or_else(|| program.clone()),
    }
}

fn runtime_home_for_host_config(scope: HostScope, runtime_home: &Path) -> Option<&Path> {
    match scope {
        HostScope::User => Some(runtime_home),
        HostScope::Project => None,
    }
}

fn deterministic_connection_id(
    host_kind: HostKind,
    scope: HostScope,
    project_id: Option<&str>,
    config_target: &str,
    server_name: &str,
) -> String {
    let key = json!({
        "host_kind": host_kind.as_str(),
        "host_scope": scope.as_str(),
        "project_id": project_id,
        "config_target": config_target,
        "server_name": server_name,
    })
    .to_string();
    let label = match (scope, project_id) {
        (HostScope::Project, Some(project_id)) => {
            format!(
                "{}_{}_{}_{}",
                host_kind.as_str(),
                scope.as_str(),
                project_id,
                server_name
            )
        }
        _ => format!("{}_{}_{}", host_kind.as_str(), scope.as_str(), server_name),
    };
    let mut sanitized = sanitize_identifier(&label);
    let suffix = short_hash(&key);
    let max_label = 48usize.saturating_sub(suffix.len() + 6);
    if sanitized.len() > max_label {
        sanitized.truncate(max_label);
        sanitized = sanitized.trim_end_matches('_').to_owned();
    }
    if sanitized.is_empty() {
        format!("conn_{suffix}")
    } else {
        format!("conn_{sanitized}_{suffix}")
    }
}

fn sanitize_identifier(input: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;
    for ch in input.chars().flat_map(char::to_lowercase) {
        let next = if ch.is_ascii_alphanumeric() {
            Some(ch)
        } else if ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == ':' {
            Some('_')
        } else {
            None
        };
        if let Some(ch) = next {
            if ch == '_' {
                if last_underscore {
                    continue;
                }
                last_underscore = true;
            } else {
                last_underscore = false;
            }
            out.push(ch);
        }
    }
    out.trim_matches('_').to_owned()
}

fn short_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut text = String::new();
    for byte in digest.iter().take(6) {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn codex_environment(process: &impl ConnectionProcess) -> CodexEnvironment {
    CodexEnvironment {
        home: process.env_var("HOME").map(PathBuf::from),
        codex_home: process.env_var("CODEX_HOME").map(PathBuf::from),
        path: process.env_var(PATH_ENV),
    }
}

fn codex_home(process: &impl ConnectionProcess) -> Result<PathBuf, ConnectionCommandError> {
    if let Some(path) = process.env_var("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = process.env_var("HOME").ok_or_else(|| {
        ConnectionCommandError::runtime("Codex user configuration requires CODEX_HOME or HOME")
    })?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod init_status_tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn failed_init_uses_failure_output_channel() {
        let output = "rendered failed init".to_owned();

        assert_eq!(
            command_output_result(volicord_types::ConnectionStatus::Failed, output.clone()),
            Err(ConnectionCommandError::FailureOutput(output))
        );
    }

    #[test]
    fn non_failure_init_statuses_use_success_channel() {
        for status in [
            volicord_types::ConnectionStatus::Complete,
            volicord_types::ConnectionStatus::ActionRequired,
        ] {
            let output = format!("rendered {} init", status.as_str());
            assert_eq!(command_output_result(status, output.clone()), Ok(output));
        }
    }

    #[test]
    fn explicit_runtime_home_precedes_environment_and_is_made_absolute() {
        let current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let explicit = Path::new("explicit-runtime-home");
        let selected = selected_runtime_home_path(
            Some(explicit),
            |name| (name == "VOLICORD_HOME").then(|| OsString::from("environment-runtime-home")),
            &current_dir,
        )
        .expect("an explicit Runtime Home should select directly");

        assert_eq!(selected, current_dir.join(explicit));
    }

    #[test]
    fn runtime_home_selection_keeps_environment_then_platform_default_precedence() {
        let current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let from_environment = selected_runtime_home_path(
            None,
            |name| match name {
                "VOLICORD_HOME" => Some(OsString::from("environment-runtime-home")),
                "HOME" => Some(OsString::from("platform-home")),
                _ => None,
            },
            &current_dir,
        )
        .expect("VOLICORD_HOME should resolve");
        assert_eq!(
            from_environment,
            current_dir.join("environment-runtime-home")
        );

        let from_default = selected_runtime_home_path(
            None,
            |name| (name == "HOME").then(|| OsString::from("platform-home")),
            &current_dir,
        )
        .expect("the platform default should resolve");
        assert_eq!(from_default, current_dir.join("platform-home/.volicord"));
    }
}

#[cfg(test)]
mod persisted_metadata_tests {
    use std::{collections::BTreeMap, ffi::OsString, io, path::PathBuf};

    use volicord_store::{
        agent_connections::{agent_connection_record, list_connection_projects},
        guards::{list_guard_installations, upsert_guard_installation, GuardInstallationUpsert},
        sqlite::{open_registry_database, registry_db_path},
    };
    use volicord_test_support::{core_fixtures::CoreFixture, test_guard_manifest_json};

    use super::*;

    #[derive(Debug)]
    struct DiagnosticProcess {
        runtime_home: PathBuf,
        preflight_calls: usize,
        stdio_calls: usize,
    }

    impl ConnectionProcess for DiagnosticProcess {
        fn env_var(&self, name: &str) -> Option<OsString> {
            (name == "VOLICORD_HOME").then(|| self.runtime_home.clone().into_os_string())
        }

        fn current_exe(&self) -> Result<PathBuf, String> {
            Err("current executable is not used by diagnostic commands".to_owned())
        }

        fn run_preflight(
            &mut self,
            _launch: &McpLaunch,
            _runtime_home: &Path,
            _connection_id: &str,
            _project_id: Option<&str>,
        ) -> Result<ConnectionProcessOutput, String> {
            self.preflight_calls += 1;
            Ok(ConnectionProcessOutput {
                success: false,
                status_code: Some(1),
                stdout: String::new(),
                stderr: "fixture preflight unavailable".to_owned(),
            })
        }

        fn verify_mcp_stdio(
            &mut self,
            _launch: &McpLaunch,
            _runtime_home: &Path,
            _connection_id: &str,
            _mode: &str,
        ) -> Result<McpVerification, String> {
            self.stdio_calls += 1;
            Ok(McpVerification::failed("fixture handshake unavailable"))
        }
    }

    #[derive(Debug)]
    struct ModeTransitionDuringVerificationProcess {
        runtime_home: PathBuf,
        transitioned: bool,
    }

    impl ConnectionProcess for ModeTransitionDuringVerificationProcess {
        fn env_var(&self, name: &str) -> Option<OsString> {
            (name == "VOLICORD_HOME").then(|| self.runtime_home.clone().into_os_string())
        }

        fn current_exe(&self) -> Result<PathBuf, String> {
            Err("current executable is not used by diagnostic commands".to_owned())
        }

        fn run_preflight(
            &mut self,
            _launch: &McpLaunch,
            runtime_home: &Path,
            connection_id: &str,
            _project_id: Option<&str>,
        ) -> Result<ConnectionProcessOutput, String> {
            let connection = agent_connection_record(runtime_home, connection_id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "fixture connection disappeared".to_owned())?;
            let projects = list_connection_projects(runtime_home, connection_id)
                .map_err(|error| error.to_string())?;
            let expected_integration_revision =
                connection_integration_revision(&connection).map_err(|error| error.to_string())?;
            let guard_manifests = preflight_mode_guard_rebinds(
                runtime_home,
                &connection,
                &projects,
                CONNECTION_MODE_READ_ONLY,
            )
            .map_err(|error| error.to_string())?;
            transition_connection_mode(
                runtime_home,
                ConnectionModeTransition {
                    connection_internal_id: connection_id.to_owned(),
                    expected_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    expected_integration_revision,
                    mode: CONNECTION_MODE_READ_ONLY.to_owned(),
                    guard_manifests,
                },
            )
            .map_err(|error| error.to_string())?;
            self.transitioned = true;
            Ok(ConnectionProcessOutput {
                success: false,
                status_code: Some(1),
                stdout: String::new(),
                stderr: "fixture preflight unavailable after mode transition".to_owned(),
            })
        }

        fn verify_mcp_stdio(
            &mut self,
            _launch: &McpLaunch,
            _runtime_home: &Path,
            _connection_id: &str,
            _mode: &str,
        ) -> Result<McpVerification, String> {
            Ok(McpVerification::failed("fixture handshake unavailable"))
        }
    }

    fn tree_snapshot(root: &Path) -> io::Result<BTreeMap<PathBuf, Vec<u8>>> {
        fn visit(
            root: &Path,
            path: &Path,
            snapshot: &mut BTreeMap<PathBuf, Vec<u8>>,
        ) -> io::Result<()> {
            let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(std::fs::DirEntry::file_name);
            for entry in entries {
                let path = entry.path();
                let relative = path
                    .strip_prefix(root)
                    .expect("snapshot entry remains under its root")
                    .to_path_buf();
                let file_type = entry.file_type()?;
                if file_type.is_dir() {
                    snapshot.insert(relative, vec![0]);
                    visit(root, &path, snapshot)?;
                } else if file_type.is_symlink() {
                    let mut bytes = vec![2];
                    bytes.extend_from_slice(fs::read_link(&path)?.to_string_lossy().as_bytes());
                    snapshot.insert(relative, bytes);
                } else {
                    let mut bytes = vec![1];
                    bytes.extend_from_slice(&fs::read(path)?);
                    snapshot.insert(relative, bytes);
                }
            }
            Ok(())
        }

        let mut snapshot = BTreeMap::new();
        if root.exists() {
            visit(root, root, &mut snapshot)?;
        }
        Ok(snapshot)
    }

    #[test]
    fn stored_connection_metadata_never_defaults_after_decode_failure() {
        assert!(parse_metadata("{", None).is_err());
        assert!(parse_metadata("[]", None).is_err());
        assert!(parse_metadata(r#"{"created_by":42}"#, None).is_err());
        assert!(parse_metadata("{}", None)
            .expect("empty typed map")
            .is_empty());
        let pending_cleanup = r#"{
            "created_by":"volicord_cli_agent_connection",
            "pending_host_cleanup":{
                "project_id":"project_fixture",
                "replacement_connection_id":"conn_replacement"
            }
        }"#;
        assert_eq!(
            parse_metadata(pending_cleanup, Some("project_fixture"))
                .expect("exact pending cleanup marker")
                .get("created_by")
                .map(String::as_str),
            Some("volicord_cli_agent_connection")
        );
        assert!(parse_metadata(pending_cleanup, Some("project_other")).is_err());
    }

    #[test]
    fn corrupt_verification_report_is_a_list_issue_then_verify_repairs_it(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("connection-corrupt-verification-report")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let damaged = r#"{"status":"not_verified"}"#;
        open_registry_database(registry_db_path(fixture.runtime_home_path()))?.execute(
            "UPDATE agent_connections
                SET verification_report_json = ?2
              WHERE connection_internal_id = ?1",
            (fixture.connection_id(), damaged),
        )?;
        assert!(
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id()).is_err()
        );

        let mut process = DiagnosticProcess {
            runtime_home: fixture.runtime_home_path().to_path_buf(),
            preflight_calls: 0,
            stdio_calls: 0,
        };
        let list = run_connections_command(
            ConnectionListArgs {
                repo: Some(repo_root.clone()),
                runtime_home: crate::cli::RuntimeHomeArgs::default(),
                json: true,
            },
            &repo_root,
            &mut process,
        )?;
        let list: Value = serde_json::from_str(&list)?;
        assert!(list.get("status").is_none());
        assert!(list["connections"][0]["verification_report"].is_null());
        assert_eq!(
            list["connections"][0]["issues"][0]["kind"],
            "verification_report_corrupt"
        );

        let select_args = || ConnectionSelectArgs {
            host: Some(crate::cli::CodexHost::Codex),
            repo: Some(repo_root.clone()),
            runtime_home: crate::cli::RuntimeHomeArgs::default(),
            shared: false,
            output: crate::cli::ConnectionReportOutputArgs {
                json: true,
                verbose: false,
            },
        };
        let verification = run_connection_command(
            ConnectionArgs {
                command: ConnectionCommand::Verify(select_args()),
            },
            &repo_root,
            &mut process,
        );
        let output = match verification {
            Err(ConnectionCommandError::FailureOutput(output)) => output,
            other => panic!("failed verify must use the operational output channel: {other:?}"),
        };
        let output: Value = serde_json::from_str(&output)?;
        let repaired =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
                .expect("verification should preserve the selected connection");
        let persisted = repaired
            .verification_report()?
            .expect("verify should persist one canonical report");
        assert_eq!(output["status"], persisted.status().as_str());
        assert_eq!(output["checks"], serde_json::to_value(persisted.checks())?);
        assert_eq!(
            output["actions"],
            serde_json::to_value(persisted.actions())?
        );
        Ok(())
    }

    #[test]
    fn connection_status_is_read_only_and_does_not_probe_processes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("connection-status-read-only")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let mut process = DiagnosticProcess {
            runtime_home: fixture.runtime_home_path().to_path_buf(),
            preflight_calls: 0,
            stdio_calls: 0,
        };
        let registry_path = volicord_store::sqlite::registry_db_path(fixture.runtime_home_path());
        let registry_before = fs::read(&registry_path)?;
        let runtime_before = tree_snapshot(fixture.runtime_home_path())?;
        let repository_before = tree_snapshot(&repo_root)?;

        let output = run_connection_command(
            ConnectionArgs {
                command: ConnectionCommand::Status(ConnectionSelectArgs {
                    host: Some(crate::cli::CodexHost::Codex),
                    repo: Some(repo_root.clone()),
                    runtime_home: crate::cli::RuntimeHomeArgs::default(),
                    shared: false,
                    output: crate::cli::ConnectionReportOutputArgs {
                        json: true,
                        verbose: false,
                    },
                }),
            },
            &repo_root,
            &mut process,
        );

        let output = match output {
            Err(ConnectionCommandError::FailureOutput(output)) => output,
            other => panic!("failed status must use the operational output channel: {other:?}"),
        };
        let output: Value = serde_json::from_str(&output)?;
        assert_eq!(output["operation"], "status");
        assert_eq!(output["dry_run"], false);
        assert_eq!(output["status"], "failed");
        assert_eq!(process.preflight_calls, 0);
        assert_eq!(process.stdio_calls, 0);
        assert_eq!(fs::read(registry_path)?, registry_before);
        assert_eq!(tree_snapshot(fixture.runtime_home_path())?, runtime_before);
        assert_eq!(tree_snapshot(&repo_root)?, repository_before);
        Ok(())
    }

    #[test]
    fn verification_rejects_a_report_after_a_concurrent_mode_transition(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("connection-verify-mode-transition")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let guard_installation_id = "guard_verify_mode_transition";
        upsert_guard_installation(
            fixture.runtime_home_path(),
            GuardInstallationUpsert {
                guard_installation_id: guard_installation_id.to_owned(),
                connection_internal_id: fixture.connection_id().to_owned(),
                project_id: fixture.project_id().to_owned(),
                manifest_json: test_guard_manifest_json(
                    fixture.runtime_home_path(),
                    &repo_root,
                    fixture.project_id(),
                    fixture.connection_id(),
                    guard_installation_id,
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                ),
            },
        )?;
        let before = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
            .expect("R1 fixture connection");
        let revision_r1 = connection_integration_revision(&before)?;
        let generation_r1 = before.integration_generation;
        let select_args = || ConnectionSelectArgs {
            host: Some(crate::cli::CodexHost::Codex),
            repo: Some(repo_root.clone()),
            runtime_home: crate::cli::RuntimeHomeArgs::default(),
            shared: false,
            output: crate::cli::ConnectionReportOutputArgs {
                json: true,
                verbose: false,
            },
        };
        let mut transitioning_process = ModeTransitionDuringVerificationProcess {
            runtime_home: fixture.runtime_home_path().to_path_buf(),
            transitioned: false,
        };

        let error = run_connection_command(
            ConnectionArgs {
                command: ConnectionCommand::Verify(select_args()),
            },
            &repo_root,
            &mut transitioning_process,
        )
        .expect_err("stale R1 report persistence must fail");
        assert!(transitioning_process.transitioned);
        assert!(matches!(error, ConnectionCommandError::Runtime(_)));
        assert!(error
            .to_string()
            .contains("CONNECTION_VERIFICATION_CONFLICT"));
        assert!(error
            .to_string()
            .contains("rerun `volicord connection verify`"));

        let after_transition =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
                .expect("R2 fixture connection");
        let revision_r2 = connection_integration_revision(&after_transition)?;
        assert_ne!(revision_r2, revision_r1);
        assert_eq!(after_transition.mode, CONNECTION_MODE_READ_ONLY);
        assert_eq!(after_transition.integration_generation, generation_r1 + 1);
        assert!(after_transition.verification_report_json.is_none());
        let guard_after_transition = list_guard_installations(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            Some(fixture.project_id()),
        )?
        .into_iter()
        .next()
        .expect("R2 Guard Installation");
        assert_eq!(
            guard_manifest_from_json(&guard_after_transition.manifest_json)?.integration_revision,
            revision_r2
        );

        let mut retry_process = DiagnosticProcess {
            runtime_home: fixture.runtime_home_path().to_path_buf(),
            preflight_calls: 0,
            stdio_calls: 0,
        };
        let retry = run_connection_command(
            ConnectionArgs {
                command: ConnectionCommand::Verify(select_args()),
            },
            &repo_root,
            &mut retry_process,
        );
        assert!(matches!(
            retry,
            Err(ConnectionCommandError::FailureOutput(_))
        ));
        let after_retry =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
                .expect("verified R2 fixture connection");
        assert_eq!(connection_integration_revision(&after_retry)?, revision_r2);
        assert_eq!(after_retry.mode, CONNECTION_MODE_READ_ONLY);
        assert_eq!(after_retry.integration_generation, generation_r1 + 1);
        assert!(after_retry.verification_report_json.is_some());
        assert_eq!(
            list_guard_installations(
                fixture.runtime_home_path(),
                fixture.connection_id(),
                Some(fixture.project_id()),
            )?
            .into_iter()
            .next()
            .expect("R2 Guard Installation after retry"),
            guard_after_transition
        );
        Ok(())
    }
}
