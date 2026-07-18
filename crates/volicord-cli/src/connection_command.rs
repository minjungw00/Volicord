use std::{
    collections::{BTreeMap, BTreeSet},
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
        list_connection_projects_for_diagnostics, remove_agent_connection_if_unused,
        remove_connection_project, set_connection_mode, staged_connection_migration_state,
        update_agent_connection_verification_report, AgentConnectionRecord,
        AgentConnectionRegistration, ConnectionProjectRecord, ConnectionProjectRegistration,
        PendingHostCleanupError, StagedConnectionMigrationState, SupersededConnectionProject,
        CONNECTION_INTENT_PERSONAL, CONNECTION_INTENT_SHARED, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT, HOST_SCOPE_USER,
    },
    bootstrap::{
        ensure_project_for_repo, initialize_runtime_home, installation_profile,
        project_record_by_repo_root, write_installation_profile, InstallationProfileRecord,
        InstallationProfileRegistration, RepoProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::CoreProjectStore,
    guards::{
        guard_health_record, guard_observation_summary, list_guard_installations,
        GuardInstallationRecord,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    workflow_records::ProjectWorkflowPolicyAuthorityApply,
    StoreError,
};
use volicord_types::{
    canonical_json_sha256, canonical_json_string, guard_manifest_from_json,
    ConnectionVerificationError, IntegrationProfile, ProjectId, PromptCaptureStatus, UtcTimestamp,
};

use crate::cli::{
    ConnectionAddArgs, ConnectionArgs, ConnectionCommand, ConnectionListArgs, ConnectionModeArgs,
    ConnectionRemoveArgs, ConnectionSelectArgs, InitArgs,
};
use crate::guard_integration::audit::{
    guard_file_findings_for_installation, guard_manifest_binding_valid_for_installation,
    GuardFileFindings,
};
use crate::guard_integration::{
    apply_guard_integration, apply_guard_migration_protection, guard_installation_upsert,
    plan_guard_integration, record_guard_installation, FilePlanStatus, GuardIntegrationError,
    GuardIntegrationPlan, GuardIntegrationPlanRequest,
};
use crate::host_integration::{
    codex::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest},
    verification::Verification,
    ConnectionIntent, HostAdapter, HostConfigError, HostIntegrationFileKind, HostKind, HostPlan,
    HostPlanRequest, HostRemoveRequest, HostScope, HostTarget, InstallationProfile, PlannedChange,
    ProjectContext, UserAction, UserActionKind,
};
use crate::{
    registration::ADMIN_METADATA_JSON,
    setup_command::{is_executable_file, path_text as setup_path_text, runtime_home_id_for_path},
};

mod args;
mod mcp_process;
mod output;
mod persisted_state;
mod selection;
mod service;
mod verification;

pub use mcp_process::{
    ConnectionProcess, ConnectionProcessOutput, McpLaunch, McpVerification,
    ProductionConnectionProcess,
};

use args::{
    connection_output_format, init_options, init_output_format, InitMode, OutputFormat,
    ParsedConnectionOptions, ParsedInitOptions,
};
use mcp_process::mcp_launch_from_host_plan;
use output::{
    render_connection_output, render_connection_plan_output,
    render_connection_remove_dry_run_output, render_connections_output,
    render_current_connection_output, render_init_output, CommandOperation, ConnectionOutput,
    ConnectionPlanOutput, ConnectionRemovePlan, InitOutput,
};
use persisted_state::{
    decode_persisted_object, persisted_object_state_json,
    PERSISTED_CONNECTION_METADATA_CORRUPT_REASON,
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
    connection_status_actions, current_status_report, effective_connection_report,
    verify_connection, AgentResultStatus, VerificationReport,
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
    let rendered = render_init_output(InitOutput {
        format: init_output_format(&parsed),
        dry_run: outcome.dry_run,
        host_kind: outcome.host_kind,
        host_scope: outcome.host_scope,
        runtime_home: &outcome.runtime_home,
        repo_root: &outcome.repo_root,
        connection_id: &outcome.connection_id,
        project_id: outcome.project_id.as_deref(),
        host_plan: &outcome.host_plan,
        verification: outcome.verification.as_ref(),
        current_report: outcome.current_report.as_ref(),
        integration: &outcome.integration,
        profile_action: outcome.profile_action,
    })?;
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
    let parsed = ParsedConnectionOptions::from(args);
    match provision_connection(
        ProvisionConnectionRequest {
            parsed: &parsed,
            current_dir,
        },
        process,
    )? {
        ConnectionProvisioningOutcome::DryRun(plan) => {
            let plan = *plan;
            render_connection_plan_output(ConnectionPlanOutput {
                format: connection_output_format(&parsed),
                action: "connection_add",
                status: AgentResultStatus::ActionRequired,
                runtime_home: &plan.runtime_home,
                connection_id: &plan.connection_id,
                host_kind: plan.host_kind,
                intent: plan.intent,
                host_scope: plan.host_scope,
                mode: &plan.mode,
                enabled: true,
                repo_root: Some(&plan.repo_root),
                plan: &plan.host_plan,
                projects_remaining: None,
                user_actions: plan.host_plan.user_actions.clone(),
            })
        }
        ConnectionProvisioningOutcome::Applied(outcome) => {
            let outcome = *outcome;
            render_connection_output(ConnectionOutput {
                format: connection_output_format(&parsed),
                action: "connected",
                status: outcome.verification.status(),
                runtime_home: &outcome.runtime_home,
                host_kind: parse_host_kind(&outcome.connection.host_kind)?,
                guard_state: outcome.guard_state,
                connection: &outcome.connection,
                projects: &outcome.projects,
                affected_repo_root: Some(&outcome.affected_repo_root),
                verification: Some(&outcome.verification),
                current_report: None,
                current_host: None,
                plan: Some(&outcome.host_plan),
                user_actions: connection_status_actions(None, &outcome.verification.report),
            })
        }
    }
}

pub fn run_connections_command(
    args: ConnectionListArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = ParsedConnectionOptions::from(args);
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
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
    let parsed = ParsedConnectionOptions::from(args);
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, projects) = select_connection_for_diagnostics(&runtime_home, &selector)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
    let mut report = effective_connection_report(&connection)?;
    let persisted_metadata_corrupt = decode_persisted_object(&connection.metadata_json).is_none();
    if persisted_metadata_corrupt {
        report = verification::connection_metadata_failure_report(&report)?;
        let rendered = render_current_connection_output(
            connection_output_format(&parsed),
            CommandOperation::Status,
            &runtime_home,
            &connection,
            &selected_project.project.repo_root,
            &report,
        )?;
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
    let rendered = render_current_connection_output(
        connection_output_format(&parsed),
        CommandOperation::Status,
        &runtime_home,
        &connection,
        &selected_project.project.repo_root,
        &report,
    )?;
    command_output_result(rendered.status, rendered.output)
}

fn command_connection_verify(
    args: ConnectionSelectArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = ParsedConnectionOptions::from(args);
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (mut connection, projects) = select_connection_for_diagnostics(&runtime_home, &selector)?;
    if decode_persisted_object(&connection.metadata_json).is_none() {
        return Err(ConnectionCommandError::runtime(format!(
            "{PERSISTED_CONNECTION_METADATA_CORRUPT_REASON}: connection verification cannot repair Agent Connection registration metadata; recreate or repair the registration before retrying"
        )));
    }
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
    connection = update_agent_connection_verification_report(
        &runtime_home,
        &connection.connection_internal_id,
        &host_plan.fingerprint,
        Some(&verification.report),
    )?;
    let rendered = render_current_connection_output(
        connection_output_format(&parsed),
        CommandOperation::Verify,
        &runtime_home,
        &connection,
        &selected_repo_root,
        &verification.report,
    )?;
    command_output_result(rendered.status, rendered.output)
}

fn command_connection_mode(
    args: ConnectionModeArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let mode = args.mode.as_str().to_owned();
    let parsed = ParsedConnectionOptions::from(args);
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, _) = select_connection(&runtime_home, &selector)?;
    let connection = set_connection_mode(&runtime_home, &connection.connection_internal_id, &mode)?;
    let actions = vec![UserAction::new(
        UserActionKind::ReloadRequired,
        "Restart or reload the host so it refreshes the Volicord tool list for the selected mode",
    )];
    let projects = list_connection_projects(&runtime_home, &connection.connection_internal_id)?;
    render_connection_output(ConnectionOutput {
        format: connection_output_format(&parsed),
        action: "mode_updated",
        status: AgentResultStatus::ActionRequired,
        runtime_home: &runtime_home,
        host_kind: parse_host_kind(&connection.host_kind)?,
        guard_state: guard_state_for_connection(&runtime_home, &connection, &projects)?,
        user_actions: actions,
        connection: &connection,
        projects: &projects,
        affected_repo_root: None,
        verification: None,
        current_report: None,
        current_host: None,
        plan: None,
    })
}

fn command_connection_remove(
    args: ConnectionRemoveArgs,
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let parsed = ParsedConnectionOptions::from(args);
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
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
        let plan = host_plan
            .as_ref()
            .map(ConnectionRemovePlan::Host)
            .unwrap_or(ConnectionRemovePlan::MembershipOnly);
        return render_connection_remove_dry_run_output(
            connection_output_format(&parsed),
            &runtime_home,
            &connection,
            &projects,
            selected_project,
            plan,
            remaining_count,
        );
    }

    remove_connection_project(
        &runtime_home,
        &connection.connection_internal_id,
        &selected_project.project_id,
    )?;
    let remaining_projects =
        list_connection_projects(&runtime_home, &connection.connection_internal_id)?;
    if remaining_projects.is_empty() {
        if let Some(host_plan) = &host_plan {
            remove_host_configuration(host_plan, &connection, process)?;
        }
        remove_agent_connection_if_unused(&runtime_home, &connection.connection_internal_id)?;
    }
    render_connection_output(ConnectionOutput {
        format: connection_output_format(&parsed),
        action: "removed",
        status: AgentResultStatus::Complete,
        runtime_home: &runtime_home,
        host_kind: parse_host_kind(&connection.host_kind)?,
        guard_state: guard_state_for_connection(&runtime_home, &connection, &remaining_projects)?,
        user_actions: Vec::new(),
        connection: &connection,
        projects: &remaining_projects,
        affected_repo_root: Some(&selected_project.project.repo_root),
        verification: None,
        current_report: None,
        current_host: None,
        plan: host_plan.as_ref(),
    })
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

fn public_host_display_name(host_kind: HostKind) -> &'static str {
    match host_kind {
        HostKind::Codex => "Codex",
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

fn required_installation_profile(
    runtime_home: &Path,
) -> Result<InstallationProfileRecord, ConnectionCommandError> {
    installation_profile(runtime_home)?.ok_or_else(|| {
        ConnectionCommandError::runtime(format!(
            "SETUP_REQUIRED: installation profile is missing for Runtime Home {}; run `volicord init --host <host> --repo <path>` from the Product Repository to initialize Volicord.",
            runtime_home.display()
        ))
    })
}

struct InitProfilePlan {
    volicord_command: PathBuf,
    volicord_mcp_command: PathBuf,
    bin_dir: PathBuf,
    metadata_json: String,
}

fn init_runtime_home_path(
    parsed: &ParsedInitOptions,
    current_dir: &Path,
    process: &impl ConnectionProcess,
) -> Result<PathBuf, ConnectionCommandError> {
    if let Some(path) = &parsed.runtime_home {
        Ok(path.clone())
    } else {
        resolve_runtime_home(|name| process.env_var(name), current_dir).map_err(Into::into)
    }
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
    let metadata = parse_metadata(&connection.metadata_json)?;
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

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuardOperationalState {
    mode_state: String,
    guard_profile_state: String,
    installation_state: String,
    configuration_state: String,
    observation_state: String,
    effective_state: String,
    generated_config_verified: bool,
    direct_file_write_matcher_coverage: bool,
    files_state: String,
    agents_block_state: String,
    policy_file_state: String,
    rule_instruction_state: String,
    hook_config_state: String,
    hook_observed_state: String,
    last_observed_at: Option<String>,
    last_guard_event_at: Option<String>,
    prompt_capture_state: String,
    missing_files: Vec<String>,
    stale_files: Vec<String>,
    broken_files: Vec<String>,
    missing_required_hooks: Vec<String>,
    unresolved_blockers: Vec<String>,
}

impl GuardOperationalState {
    fn not_configured() -> Self {
        Self {
            mode_state: "not_configured".to_owned(),
            guard_profile_state: "not_configured".to_owned(),
            installation_state: "not_configured".to_owned(),
            configuration_state: "absent".to_owned(),
            observation_state: "not_observed".to_owned(),
            effective_state: "inactive".to_owned(),
            generated_config_verified: false,
            direct_file_write_matcher_coverage: false,
            files_state: "not_configured".to_owned(),
            agents_block_state: "not_configured".to_owned(),
            policy_file_state: "not_configured".to_owned(),
            rule_instruction_state: "not_configured".to_owned(),
            hook_config_state: "not_configured".to_owned(),
            hook_observed_state: "not_observed".to_owned(),
            last_observed_at: None,
            last_guard_event_at: None,
            prompt_capture_state: PromptCaptureStatus::NotConfigured.as_str().to_owned(),
            missing_files: Vec::new(),
            stale_files: Vec::new(),
            broken_files: Vec::new(),
            missing_required_hooks: Vec::new(),
            unresolved_blockers: Vec::new(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "selected_profile": &self.guard_profile_state,
            "control_surface": self.control_surface_json(),
            "installation": &self.installation_state,
            "configuration_health": &self.configuration_state,
            "observation_health": &self.observation_state,
            "effective_health": &self.effective_state,
            "generated_config_verified": self.generated_config_verified,
            "cooperative_pre_tool_warning_available": self.cooperative_pre_tool_warning_available(),
            "cooperative_pre_tool_denial_available": self.cooperative_pre_tool_denial_available(),
            "post_tool_correlation_available": self.post_tool_correlation_available(),
            "direct_file_write_matcher_coverage": self.direct_file_write_matcher_coverage,
            "bypass_detection_active": self.bypass_detection_active(),
            "files": &self.files_state,
            "agents_managed_block": &self.agents_block_state,
            "volicord_policy_file": &self.policy_file_state,
            "rule_instruction_config": &self.rule_instruction_state,
            "hook_config": &self.hook_config_state,
            "required_hook_phases": self.required_hook_phases_state(),
            "hook_observed": &self.hook_observed_state,
            "guard_observed": self.guard_observed(),
            "last_observed_at": &self.last_observed_at,
            "last_guard_event_at": &self.last_guard_event_at,
            "prompt_capture": &self.prompt_capture_state,
            "prompt_capture_available": self.prompt_capture_available(),
            "missing_files": &self.missing_files,
            "stale_files": &self.stale_files,
            "broken_files": &self.broken_files,
            "missing_required_hooks": &self.missing_required_hooks,
            "unresolved_blockers": &self.unresolved_blockers,
        })
    }

    fn guard_observed(&self) -> bool {
        self.hook_observed_state == "observed"
    }

    fn control_surface_json(&self) -> Value {
        json!({
            "selected_profile": self.selected_profile(),
            "host_hooks_active": self.host_hook_guard_available(),
            "cooperative_pre_tool_warning_available": self.cooperative_pre_tool_warning_available(),
            "cooperative_pre_tool_denial_available": self.cooperative_pre_tool_denial_available(),
            "unrecorded_changes_detectable": self.post_tool_correlation_available(),
            "actor_identity_provable": false,
            "os_enforced": false,
        })
    }

    fn selected_profile(&self) -> &str {
        self.guard_profile_state.as_str()
    }

    fn guard_hooks_applicable(&self) -> bool {
        self.mode_state == IntegrationProfile::Record.as_str()
            && self.guard_profile_state == IntegrationProfile::Record.as_str()
    }

    fn host_hook_guard_available(&self) -> bool {
        self.guard_hooks_applicable()
            && self.effective_state == "active"
            && self.missing_required_hooks.is_empty()
            && self.generated_config_verified
            && self.direct_file_write_matcher_coverage
    }

    fn cooperative_pre_tool_warning_available(&self) -> bool {
        self.host_hook_guard_available()
    }

    fn cooperative_pre_tool_denial_available(&self) -> bool {
        self.host_hook_guard_available()
    }

    fn post_tool_correlation_available(&self) -> bool {
        self.host_hook_guard_available()
    }

    fn bypass_detection_active(&self) -> bool {
        false
    }

    fn prompt_capture_available(&self) -> bool {
        matches!(
            self.prompt_capture_state.as_str(),
            "configured" | "observed" | "active"
        )
    }

    fn required_hook_phases_state(&self) -> &'static str {
        if self.missing_required_hooks.is_empty() {
            "configured"
        } else {
            "missing"
        }
    }
}

fn guard_state_for_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> Result<GuardOperationalState, ConnectionCommandError> {
    let connection_id = &connection.connection_internal_id;
    let mut installations = Vec::new();
    for project in projects {
        installations.extend(list_guard_installations(
            runtime_home,
            connection_id,
            Some(&project.project_id),
        )?);
    }
    if installations.is_empty() {
        installations = list_guard_installations(runtime_home, connection_id, None)?;
    }
    if installations.is_empty() {
        return Ok(GuardOperationalState::not_configured());
    }

    let mut findings = GuardFileFindings::default();
    let mut every_observation_current = true;
    let mut any_incompatible_observation = false;
    let mut every_prompt_capture_observed = true;
    let mut last_observed_at = None;
    for installation in &installations {
        let installation_findings =
            guard_file_findings_for_installation(runtime_home, installation, connection, projects);
        findings.merge(installation_findings);
        let binding_is_current =
            guard_manifest_binding_valid_for_installation(installation, connection, projects);
        let observation =
            guard_observation_summary(runtime_home, &installation.project_id, installation)?;
        let observation_is_current =
            binding_is_current && observation.all_required_phases_observed();
        any_incompatible_observation |= !observation.incompatible_event_ids.is_empty();
        every_observation_current &= observation_is_current;
        every_prompt_capture_observed &=
            observation_is_current && observation.prompt_capture_observed();
        last_observed_at = max_optional_utc_timestamp(
            last_observed_at,
            observation.last_observed_at.as_deref(),
            "guard_events.occurred_at",
        )?;
    }
    findings.sort_dedup();

    let observed = every_observation_current;
    let prompt_capture_observed = every_prompt_capture_observed;
    let mode_state = guard_mode_state(&installations);
    let installation_state = if !findings.broken_files.is_empty() {
        "broken"
    } else if !findings.missing_files.is_empty() {
        "files_missing"
    } else if !findings.stale_files.is_empty() {
        "stale"
    } else if !findings.missing_required_hooks.is_empty() {
        "degraded"
    } else {
        "configured"
    };

    let hook_observed_state = if any_incompatible_observation {
        "failed"
    } else if observed {
        "observed"
    } else {
        "not_observed"
    };
    let configuration_state = guard_configuration_state(
        installation_state,
        !findings.missing_required_hooks.is_empty(),
    );
    let observation_state = guard_observation_state(hook_observed_state);
    let effective_state =
        guard_effective_state(&mode_state, &configuration_state, &observation_state);
    let prompt_capture_state = if !findings.prompt_capture_host_supported {
        PromptCaptureStatus::UnsupportedByHost.as_str()
    } else if !findings.prompt_capture_configured {
        PromptCaptureStatus::NotConfigured.as_str()
    } else if matches!(
        installation_state,
        "broken" | "stale" | "degraded" | "files_missing"
    ) {
        PromptCaptureStatus::Degraded.as_str()
    } else if prompt_capture_observed {
        PromptCaptureStatus::Active.as_str()
    } else if observed {
        PromptCaptureStatus::Observed.as_str()
    } else {
        PromptCaptureStatus::Configured.as_str()
    };

    let generated_config_verified = findings.generated_config_verified();
    let direct_file_write_matcher_coverage = findings.direct_file_write_matcher_coverage();
    let files_state = if !findings.broken_files.is_empty() {
        "broken"
    } else if !findings.missing_files.is_empty() {
        "missing"
    } else if !findings.stale_files.is_empty() {
        "stale"
    } else {
        "installed"
    }
    .to_owned();
    let agents_block_state = findings
        .kind_state(HostIntegrationFileKind::AgentsManagedBlock)
        .to_owned();
    let policy_file_state = findings
        .kind_state(HostIntegrationFileKind::VolicordPolicy)
        .to_owned();
    let rule_instruction_state = findings.rule_instruction_state(false);
    let hook_config_state = findings.hook_config_state(false);
    let required_hooks_missing = !findings.missing_required_hooks.is_empty();
    let unresolved_blockers = guard_blockers_for_state(
        &mode_state,
        installation_state,
        observed,
        required_hooks_missing,
    );

    Ok(GuardOperationalState {
        mode_state,
        guard_profile_state: IntegrationProfile::Record.as_str().to_owned(),
        installation_state: installation_state.to_owned(),
        configuration_state,
        observation_state,
        effective_state,
        generated_config_verified,
        direct_file_write_matcher_coverage,
        files_state,
        agents_block_state,
        policy_file_state,
        rule_instruction_state,
        hook_config_state,
        hook_observed_state: hook_observed_state.to_owned(),
        last_observed_at: last_observed_at.map(|timestamp| timestamp.to_canonical_string()),
        last_guard_event_at: last_guard_event_for_projects(runtime_home, connection_id, projects)?,
        prompt_capture_state: prompt_capture_state.to_owned(),
        missing_files: findings.missing_files,
        stale_files: findings.stale_files,
        broken_files: findings.broken_files,
        missing_required_hooks: findings.missing_required_hooks,
        unresolved_blockers,
    })
}
fn guard_mode_state(installations: &[GuardInstallationRecord]) -> String {
    let mut modes = installations
        .iter()
        .map(|installation| {
            guard_manifest_from_json(&installation.manifest_json)
                .map(|manifest| manifest.integration_profile.as_str().to_owned())
                .unwrap_or_else(|_| "invalid".to_owned())
        })
        .collect::<Vec<_>>();
    modes.sort_unstable();
    modes.dedup();
    if modes.len() == 1 {
        modes[0].clone()
    } else {
        "mixed".to_owned()
    }
}

fn guard_configuration_state(installation_state: &str, missing_required_hooks: bool) -> String {
    if missing_required_hooks
        && !matches!(
            installation_state,
            "not_configured" | "files_missing" | "stale" | "broken"
        )
    {
        return "degraded".to_owned();
    }
    match installation_state {
        "not_configured" | "files_missing" => "absent",
        "active" | "configured" => "configured",
        "reload_required" => "reload_required",
        "degraded" => "degraded",
        "stale" => "stale",
        "broken" => "broken",
        other => other,
    }
    .to_owned()
}

fn guard_observation_state(hook_observed_state: &str) -> String {
    match hook_observed_state {
        "observed" => "observed",
        "failed" => "failed",
        "disabled" => "not_observed",
        _ => "not_observed",
    }
    .to_owned()
}

fn guard_effective_state(
    _guard_mode: &str,
    configuration_state: &str,
    observation_state: &str,
) -> String {
    match configuration_state {
        "absent" => "inactive",
        "broken" => "broken",
        "stale" | "degraded" => "degraded",
        "configured" if observation_state == "failed" => "degraded",
        "configured" if observation_state == "observed" => "active",
        "configured" | "reload_required" => "action_required",
        _ => "action_required",
    }
    .to_owned()
}

fn guard_blockers_for_state(
    _guard_mode: &str,
    installation_state: &str,
    host_hook_observed: bool,
    required_hooks_missing: bool,
) -> Vec<String> {
    match installation_state {
        "not_configured" | "files_missing" => vec!["guard_not_installed".to_owned()],
        "reload_required" => vec!["guard_reload_required".to_owned()],
        "configured" if !host_hook_observed => vec!["guard_not_observed".to_owned()],
        "configured" => Vec::new(),
        "active" if !host_hook_observed => vec!["guard_not_observed".to_owned()],
        "stale" => vec!["guard_stale".to_owned()],
        "broken" => vec!["guard_broken".to_owned()],
        "degraded" if required_hooks_missing => vec!["guard_required_hooks_missing".to_owned()],
        "degraded" => vec!["guard_degraded".to_owned()],
        _ => Vec::new(),
    }
}

fn last_guard_event_for_projects(
    runtime_home: &Path,
    connection_id: &str,
    projects: &[ConnectionProjectRecord],
) -> Result<Option<String>, ConnectionCommandError> {
    let mut latest = None;
    for project in projects {
        if let Some(event) =
            guard_health_record(runtime_home, &project.project_id, connection_id)?.latest_event
        {
            latest = max_optional_utc_timestamp(
                latest,
                Some(&event.occurred_at),
                "guard_events.occurred_at",
            )?;
        }
    }
    Ok(latest.map(|timestamp| timestamp.to_canonical_string()))
}

fn max_optional_utc_timestamp(
    current: Option<UtcTimestamp>,
    candidate: Option<&str>,
    owner_field: &str,
) -> Result<Option<UtcTimestamp>, ConnectionCommandError> {
    let Some(candidate) = candidate else {
        return Ok(current);
    };
    let candidate = canonical_utc_timestamp(candidate).ok_or_else(|| {
        ConnectionCommandError::runtime(format!(
            "stored {owner_field} is not a canonical four-digit RFC 3339 UTC instant"
        ))
    })?;
    Ok(Some(match current {
        Some(current) => current.max(candidate),
        None => candidate,
    }))
}

fn canonical_utc_timestamp(value: &str) -> Option<UtcTimestamp> {
    let timestamp = UtcTimestamp::parse(value).ok()?;
    timestamp.ensure_canonical_rfc3339_representable().ok()?;
    Some(timestamp)
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

fn parse_metadata(text: &str) -> Result<BTreeMap<String, String>, ConnectionCommandError> {
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
    object
        .iter()
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
}

#[cfg(test)]
mod persisted_metadata_tests {
    use std::{collections::BTreeMap, ffi::OsString, io, path::PathBuf};

    use volicord_store::{
        agent_connections::agent_connection_record,
        sqlite::{open_registry_database, registry_db_path},
    };
    use volicord_test_support::core_fixtures::CoreFixture;

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
        assert!(parse_metadata("{").is_err());
        assert!(parse_metadata("[]").is_err());
        assert!(parse_metadata(r#"{"created_by":42}"#).is_err());
        assert!(parse_metadata("{}").expect("empty typed map").is_empty());
    }

    #[test]
    fn corrupt_verification_report_degrades_list_then_verify_repairs_it(
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
                json: true,
            },
            &repo_root,
            &mut process,
        )?;
        let list: Value = serde_json::from_str(&list)?;
        assert_eq!(list["status"], "degraded");
        assert!(list["connections"][0]["verification_report"].is_null());

        let select_args = || ConnectionSelectArgs {
            host: Some(crate::cli::CodexHost::Codex),
            repo: Some(repo_root.clone()),
            shared: false,
            json: true,
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
                    shared: false,
                    json: true,
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
}
