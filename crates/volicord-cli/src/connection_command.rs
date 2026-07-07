use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, list_agent_connections,
        list_connection_projects, remove_agent_connection_if_unused, remove_connection_project,
        set_connection_mode, update_agent_connection_verification_report, AgentConnectionRecord,
        AgentConnectionRegistration, ConnectionProjectRecord, ConnectionProjectRegistration,
        CONNECTION_INTENT_GLOBAL, CONNECTION_INTENT_PERSONAL, CONNECTION_INTENT_SHARED,
        CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW, HOST_KIND_CLAUDE_CODE,
        HOST_KIND_CODEX, HOST_KIND_GENERIC, HOST_SCOPE_EXPORT, HOST_SCOPE_LOCAL,
        HOST_SCOPE_PROJECT, HOST_SCOPE_USER, VERIFIED_STATUS_NOT_VERIFIED,
    },
    bootstrap::{
        ensure_project_for_repo, initialize_runtime_home, installation_profile,
        project_record_by_repo_root, write_installation_profile, InstallationProfileRecord,
        InstallationProfileRegistration, RepoProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    guards::{
        guard_health_record, list_guard_installations, upsert_guard_installation,
        GuardInstallationRecord, GuardInstallationUpsert,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    session_watch::{snapshot_product_repository, WatchSnapshotOptions},
    StoreError,
};
use volicord_types::{GuardInstallationStatus, IntegrationProfile, PromptCaptureStatus};

#[cfg(test)]
use crate::guard_integration::audit::guard_file_findings;
use crate::guard_integration::audit::{
    all_recorded_values_true, combine_optional_file_states, file_state_rank,
    guard_file_findings_for_installation, hook_wrapper_comment_value, hook_wrapper_exec_command,
    is_volicord_codex_hook_config, policy_hash, required_guard_phase_names, script_is_executable,
    sha256_text, GuardFileFindings, HookWrapperResolutionStatus, ManagedJsonProjection,
    CODEX_DISPATCH_WRAPPER, HOOK_WRAPPER_MARKER,
};
use crate::host_integration::{
    claude_code::{self, ClaudeCodeAdapter, ProductionCommandRunner},
    codex::{self, CodexAdapter, CodexEnvironment, CodexExistingPlanRequest},
    contracts::{
        contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
    },
    generic::{GenericAdapter, USER_MANAGED_CONFIGURATION_GUIDANCE},
    host_capabilities,
    verification::{
        ActiveToolExposureStatus, HostMcpCommandDiagnostic, HostMcpCommandLaunchMode,
        HostRuntimeDiagnostic, HostRuntimeObservationStatus, ManagedHostStorageDiagnostic,
        ProjectTrustStatus, Verification,
    },
    ConnectionIntent, HostAdapter, HostCapabilities, HostConfigError, HostIntegrationFileKind,
    HostKind, HostLifecyclePhase, HostPlan, HostPlanRequest, HostRemoveRequest, HostScope,
    HostTarget, InstallationProfile, ManagedServerEntry, PlannedChange, ProjectContext, UserAction,
    UserActionKind, REQUIRED_GUARD_PHASES,
};
use crate::{
    disclosure::detective_observation_disclosure_json,
    managed_block::{self, ManagedBlockError, ManagedBlockWrite},
    registration::ADMIN_METADATA_JSON,
    setup_command::{is_executable_file, path_text as setup_path_text, runtime_home_id_for_path},
};

mod args;
mod mcp_process;
mod output;
mod selection;
mod service;
mod verification;

pub use args::{connect_usage, connection_usage, connections_usage, init_usage};
pub use mcp_process::{
    ConnectionProcess, ConnectionProcessOutput, McpLaunch, McpVerification,
    ProductionConnectionProcess,
};

use args::{
    connection_add_usage, connection_list_usage, connection_mode_usage, connection_output_format,
    connection_remove_usage, connection_status_usage, connection_verify_usage, init_output_format,
    is_help_request, parse_connection_options, parse_init_options, parse_public_host_kind,
    parse_user_connection_mode, InitMode, OutputFormat, ParsedConnectionOptions, ParsedInitOptions,
};
use mcp_process::mcp_launch_from_host_plan;
use output::{
    detailed_verification_report_json, generated_files_json, hook_path_safety_json,
    hook_root_resolution_json, host_hook_commands_json, render_connection_output,
    render_connection_plan_output, render_connection_remove_dry_run_output,
    render_connections_output, render_init_output, ConnectionOutput, ConnectionPlanOutput,
    ConnectionRemovePlan, InitOutput,
};
use selection::{
    connection_for_host_target, connection_selector, host_scope_for_intent,
    resolve_connection_host, resolve_connection_repo_root, select_connection,
    selected_connection_project,
};
use service::{
    provision_connection, provision_init, ConnectionProvisioningOutcome, InitProvisioningRequest,
    ProvisionConnectionRequest,
};
use verification::{
    connection_status_actions, current_status_host_diagnostic, effective_tool_mode_check_status,
    host_mcp_command_check_status, status_from_store, status_with_current_diagnostics,
    storage_read_check_status, storage_write_check_status, verify_connection, AgentResultStatus,
    McpPreflightDiagnostics, VerificationReport, VerificationStep,
};

const PATH_ENV: &str = "PATH";
const AGENT_METADATA_CREATED_BY: &str = "volicord_cli_agent_connection";
const AGENT_RUNTIME_HOME_ID: &str = "runtime_home_agent";
const INIT_METADATA_CREATED_BY: &str = "volicord_cli_init";
const DEFAULT_MCP_COMMAND: &str = "volicord";
const DEFAULT_SERVER_NAME: &str = "volicord";
const INSTALLATION_ID: &str = "default";
const VOLICORD_POLICY_SCHEMA: &str = "volicord-policy-v1";
const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
const AGENTS_FILE: &str = "AGENTS.md";
const GUIDANCE_START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE v1 -->";
const GUIDANCE_END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE v1 -->";
const CODEX_RULE_START_MARKER: &str = "# BEGIN VOLICORD MANAGED CODEX RULES v1";
const CODEX_RULE_END_MARKER: &str = "# END VOLICORD MANAGED CODEX RULES v1";

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

impl From<HostConfigError> for ConnectionCommandError {
    fn from(error: HostConfigError) -> Self {
        Self::runtime(error.to_string())
    }
}

pub fn run_init_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(init_usage());
    }
    let parsed = parse_init_options(args, current_dir)?;
    let outcome = provision_init(
        InitProvisioningRequest {
            parsed: &parsed,
            current_dir,
        },
        process,
    )?;
    render_init_output(InitOutput {
        format: init_output_format(&parsed),
        status: outcome.status,
        host_kind: outcome.host_kind,
        init_mode: outcome.init_mode,
        runtime_home: &outcome.runtime_home,
        repo_root: &outcome.repo_root,
        connection_id: &outcome.connection_id,
        project_id: outcome.project_id.as_deref(),
        host_plan: &outcome.host_plan,
        verification: outcome.verification.as_ref(),
        integration: &outcome.integration,
        guard_installation: outcome.guard_installation.as_ref(),
        profile_action: outcome.profile_action,
    })
}

pub fn run_connect_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connection_add_usage());
    }
    let parsed = parse_connection_options(
        args,
        &["repo", "shared", "global", "read-only", "dry-run", "json"],
        1,
    )?;
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
                status: AgentResultStatus::DryRun,
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
                status: outcome.verification.status,
                runtime_home: &outcome.runtime_home,
                guard_state: outcome.guard_state,
                connection: &outcome.connection,
                projects: &outcome.projects,
                affected_repo_root: Some(&outcome.affected_repo_root),
                verification: Some(&outcome.verification),
                current_host: None,
                plan: Some(&outcome.host_plan),
                user_actions: outcome.verification.host.user_actions.clone(),
            })
        }
    }
}

pub fn run_connections_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connection_list_usage());
    }
    let parsed = parse_connection_options(args, &["repo", "json"], 0)?;
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let repo_root = parsed
        .repo
        .as_deref()
        .map(|repo| resolve_connection_repo_root(current_dir, Some(repo)))
        .transpose()?;
    let mut rows = Vec::new();
    for connection in list_agent_connections(&runtime_home)? {
        let projects = list_connection_projects(&runtime_home, &connection.connection_internal_id)?;
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
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(connection_usage());
    };
    if matches!(subcommand, "-h" | "--help" | "help") {
        if args.len() == 1 {
            return Ok(connection_usage());
        }
        return Err(ConnectionCommandError::usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            connection_usage()
        )));
    }
    match subcommand {
        "add" => run_connect_command(&args[1..], current_dir, process),
        "list" => run_connections_command(&args[1..], current_dir, process),
        "status" => command_connection_status(&args[1..], current_dir, process),
        "verify" => command_connection_verify(&args[1..], current_dir, process),
        "mode" => command_connection_mode(&args[1..], current_dir, process),
        "remove" => command_connection_remove(&args[1..], current_dir, process),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown connection command: {other}\n\n{}",
            connection_usage()
        ))),
    }
}

fn command_connection_status(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connection_status_usage());
    }
    let parsed = parse_connection_options(args, &["repo", "shared", "global", "json"], 1)?;
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, projects) = select_connection(&runtime_home, &selector)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
    let host_plan =
        existing_host_plan(&connection, &runtime_home, process, Some(selected_project))?;
    let current_host = current_status_host_diagnostic(
        &runtime_home,
        &connection,
        Some(&host_plan),
        &projects,
        process,
    )?;
    let user_actions = connection_status_actions(&connection, current_host.as_ref());
    let status = status_with_current_diagnostics(
        status_from_store(&connection.last_verification_status),
        &user_actions,
        current_host.as_ref(),
    );
    render_connection_output(ConnectionOutput {
        format: connection_output_format(&parsed),
        action: "status",
        status,
        runtime_home: &runtime_home,
        guard_state: guard_state_for_connection(
            &runtime_home,
            &connection.connection_internal_id,
            &projects,
        )?,
        user_actions,
        connection: &connection,
        projects: &projects,
        affected_repo_root: None,
        verification: None,
        current_host,
        plan: None,
    })
}

fn command_connection_verify(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connection_verify_usage());
    }
    let parsed = parse_connection_options(args, &["repo", "shared", "global", "json"], 1)?;
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (mut connection, projects) = select_connection(&runtime_home, &selector)?;
    let selected_project = selected_connection_project(&projects, selector.repo_root())?;
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
        verification.status.store_status(),
        &host_plan.fingerprint,
        &detailed_verification_report_json(&verification)?,
        &user_actions_json(&verification.host.user_actions)?,
    )?;
    let projects = list_connection_projects(&runtime_home, &connection.connection_internal_id)?;
    render_connection_output(ConnectionOutput {
        format: connection_output_format(&parsed),
        action: "verified",
        status: verification.status,
        runtime_home: &runtime_home,
        guard_state: guard_state_for_connection(
            &runtime_home,
            &connection.connection_internal_id,
            &projects,
        )?,
        user_actions: verification.host.user_actions.clone(),
        connection: &connection,
        projects: &projects,
        affected_repo_root: None,
        verification: Some(&verification),
        current_host: None,
        plan: Some(&host_plan),
    })
}

fn command_connection_mode(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connection_mode_usage());
    }
    let parsed = parse_connection_options(args, &["repo", "shared", "global", "json"], 2)?;
    let (host_kind, mode) = mode_positionals(&parsed, process)?;
    let parsed = ParsedConnectionOptions {
        host_kind: Some(host_kind),
        ..parsed
    };
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let selector = connection_selector(&parsed, current_dir, process)?;
    let (connection, _) = select_connection(&runtime_home, &selector)?;
    let mut connection =
        set_connection_mode(&runtime_home, &connection.connection_internal_id, &mode)?;
    let mut actions = stored_or_default_user_actions(
        &connection,
        parse_host_kind(&connection.host_kind)?,
        parse_host_scope(&connection.host_scope)?,
    );
    actions.push(UserAction::new(
        UserActionKind::ReloadRequired,
        "Restart or reload the host so it refreshes the Volicord tool list for the selected mode",
    ));
    connection = update_agent_connection_verification_report(
        &runtime_home,
        &connection.connection_internal_id,
        &connection.last_verification_status,
        &connection.managed_fingerprint,
        &connection.last_verification_report_json,
        &user_actions_json(&actions)?,
    )?;
    let projects = list_connection_projects(&runtime_home, &connection.connection_internal_id)?;
    render_connection_output(ConnectionOutput {
        format: connection_output_format(&parsed),
        action: "mode_updated",
        status: status_from_store(&connection.last_verification_status),
        runtime_home: &runtime_home,
        guard_state: guard_state_for_connection(
            &runtime_home,
            &connection.connection_internal_id,
            &projects,
        )?,
        user_actions: actions,
        connection: &connection,
        projects: &projects,
        affected_repo_root: None,
        verification: None,
        current_host: None,
        plan: None,
    })
}

fn command_connection_remove(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connection_remove_usage());
    }
    let parsed =
        parse_connection_options(args, &["repo", "shared", "global", "dry-run", "json"], 1)?;
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
        guard_state: guard_state_for_connection(
            &runtime_home,
            &connection.connection_internal_id,
            &remaining_projects,
        )?,
        user_actions: Vec::new(),
        connection: &connection,
        projects: &remaining_projects,
        affected_repo_root: Some(&selected_project.project.repo_root),
        verification: None,
        current_host: None,
        plan: host_plan.as_ref(),
    })
}

fn resolve_init_repo_root(
    current_dir: &Path,
    repo: &Path,
    host_kind: HostKind,
    init_mode: InitMode,
) -> Result<PathBuf, ConnectionCommandError> {
    match resolve_connection_repo_root(current_dir, Some(repo)) {
        Ok(repo_root) => Ok(repo_root),
        Err(ConnectionCommandError::Runtime(message))
            if init_mode == InitMode::Detective
                && message.contains("no Git repository root found") =>
        {
            Err(ConnectionCommandError::runtime(
                observe_hook_root_unsupported_message(host_kind, repo),
            ))
        }
        Err(error) => Err(error),
    }
}

fn connection_intent_from_flags(
    parsed: &ParsedConnectionOptions,
) -> Result<ConnectionIntent, ConnectionCommandError> {
    if parsed.shared && parsed.global {
        return Err(ConnectionCommandError::usage(
            "--shared and --global are mutually exclusive",
        ));
    }
    if parsed.shared {
        Ok(ConnectionIntent::Shared)
    } else if parsed.global {
        Ok(ConnectionIntent::Global)
    } else {
        Ok(ConnectionIntent::Personal)
    }
}

fn mode_positionals(
    parsed: &ParsedConnectionOptions,
    process: &impl ConnectionProcess,
) -> Result<(HostKind, String), ConnectionCommandError> {
    match parsed.positionals.as_slice() {
        [mode] => {
            if let Ok(mode) = parse_user_connection_mode(mode) {
                Ok((resolve_connection_host(None, process)?, mode))
            } else {
                Err(ConnectionCommandError::usage(
                    "missing mode; use `workflow` or `read-only`",
                ))
            }
        }
        [host, mode] => Ok((
            parse_public_host_kind(host)?,
            parse_user_connection_mode(mode)?,
        )),
        [] => Err(ConnectionCommandError::usage(
            "missing mode; use `workflow` or `read-only`",
        )),
        _ => Err(ConnectionCommandError::usage("unexpected mode arguments")),
    }
}

fn public_host_label(host_kind: HostKind) -> &'static str {
    match host_kind {
        HostKind::Codex => "codex",
        HostKind::ClaudeCode => "claude-code",
        HostKind::Generic => "generic",
    }
}

fn public_host_display_name(host_kind: HostKind) -> &'static str {
    match host_kind {
        HostKind::Codex => "Codex",
        HostKind::ClaudeCode => "Claude Code",
        HostKind::Generic => "generic host",
    }
}

fn intent_flag_suffix(intent: ConnectionIntent) -> &'static str {
    match intent {
        ConnectionIntent::Personal => "",
        ConnectionIntent::Shared => " --shared",
        ConnectionIntent::Global => " --global",
    }
}

fn public_host_name_text(host_kind: &str) -> &str {
    match host_kind {
        HOST_KIND_CODEX => "codex",
        HOST_KIND_CLAUDE_CODE => "claude-code",
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
        "claude-code" | HOST_KIND_CLAUDE_CODE => Ok(HostKind::ClaudeCode),
        HOST_KIND_GENERIC => Ok(HostKind::Generic),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown host: {other}"
        ))),
    }
}

fn parse_host_scope(value: &str) -> Result<HostScope, ConnectionCommandError> {
    match value {
        HOST_SCOPE_USER => Ok(HostScope::User),
        HOST_SCOPE_PROJECT => Ok(HostScope::Project),
        HOST_SCOPE_LOCAL => Ok(HostScope::Local),
        HOST_SCOPE_EXPORT => Ok(HostScope::Export),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown scope: {other}"
        ))),
    }
}

fn parse_connection_intent(value: &str) -> Result<ConnectionIntent, ConnectionCommandError> {
    match value {
        CONNECTION_INTENT_PERSONAL => Ok(ConnectionIntent::Personal),
        CONNECTION_INTENT_SHARED => Ok(ConnectionIntent::Shared),
        CONNECTION_INTENT_GLOBAL => Ok(ConnectionIntent::Global),
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
    let volicord_command = existing
        .map(|profile| PathBuf::from(&profile.volicord_command))
        .unwrap_or_else(|| current_exe.clone());
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
        "volicord_command_source": if existing.is_some() { "existing_profile" } else { "current_exe" },
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
    if !matches!(scope, HostScope::Project | HostScope::Local) {
        return Ok(());
    }
    let projects = list_connection_projects(runtime_home, &connection.connection_internal_id)?;
    if projects
        .iter()
        .any(|project| project.project_id != project_id)
    {
        return Err(ConnectionCommandError::runtime(
            "project and local Agent Connections may allow only one project",
        ));
    }
    Ok(())
}

fn connection_target_hint(
    host_kind: HostKind,
    scope: HostScope,
    repo_root: Option<&Path>,
    process: &impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    match (host_kind, scope) {
        (HostKind::Codex, HostScope::User) => {
            let path = codex_home(process)?.join("config.toml");
            Ok(path_text(&path))
        }
        (HostKind::Codex, HostScope::Project) => {
            let repo_root = repo_root.ok_or_else(|| {
                ConnectionCommandError::usage("Codex shared connection requires --repo PATH")
            })?;
            Ok(path_text(&repo_root.join(".codex").join("config.toml")))
        }
        (HostKind::ClaudeCode, HostScope::Project) => {
            let repo_root = repo_root.ok_or_else(|| {
                ConnectionCommandError::usage("Claude Code shared connection requires --repo PATH")
            })?;
            Ok(path_text(&repo_root.join(".mcp.json")))
        }
        (HostKind::ClaudeCode, HostScope::Local) => {
            let repo_root = repo_root.ok_or_else(|| {
                ConnectionCommandError::usage(
                    "Claude Code personal connection requires --repo PATH",
                )
            })?;
            Ok(format!("claude local {}", path_text(repo_root)))
        }
        (HostKind::ClaudeCode, HostScope::User) => Ok("claude user".to_owned()),
        (HostKind::Generic, _) => Err(ConnectionCommandError::usage(
            USER_MANAGED_CONFIGURATION_GUIDANCE,
        )),
        _ => Err(ConnectionCommandError::usage(
            "host and scope must match the supported Agent Connection matrix",
        )),
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
    match request.host_kind {
        HostKind::Codex => {
            let adapter = CodexAdapter::new(codex_environment(process));
            adapter.plan(plan_request).map_err(Into::into)
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter.plan(plan_request).map_err(Into::into)
        }
        HostKind::Generic => Err(ConnectionCommandError::usage(
            USER_MANAGED_CONFIGURATION_GUIDANCE,
        )),
    }
}

fn apply_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    process: &impl ConnectionProcess,
) -> Result<(), ConnectionCommandError> {
    match host_kind {
        HostKind::Codex => {
            let mut adapter = CodexAdapter::new(codex_environment(process));
            adapter.apply(plan)?;
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter.apply(plan)?;
        }
        HostKind::Generic => {
            let mut adapter = GenericAdapter;
            adapter.apply(plan)?;
        }
    }
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
    match host_kind {
        HostKind::Codex => {
            let mut adapter = CodexAdapter::new(codex_environment(process));
            adapter.remove(request)?;
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter.remove(request)?;
        }
        HostKind::Generic => {
            let mut adapter = GenericAdapter;
            adapter.remove(request)?;
        }
    }
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
    let metadata = parse_metadata(&connection.metadata_json);
    let mcp_command = metadata
        .get("mcp_command")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MCP_COMMAND));
    let runtime_home_for_entry = metadata
        .get("host_runtime_home")
        .map(PathBuf::from)
        .or_else(|| runtime_home_for_host_config(host_scope, runtime_home).map(Path::to_path_buf));
    match host_kind {
        HostKind::Codex => {
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
        _ => Ok(manual_existing_host_plan(
            connection,
            host_kind,
            connection_intent,
            host_scope,
            &mcp_command,
            runtime_home_for_entry.as_deref(),
            &metadata,
        )),
    }
}

fn manual_existing_host_plan(
    connection: &AgentConnectionRecord,
    host_kind: HostKind,
    connection_intent: ConnectionIntent,
    host_scope: HostScope,
    mcp_command: &Path,
    runtime_home: Option<&Path>,
    metadata: &BTreeMap<String, String>,
) -> HostPlan {
    let target = match metadata.get("target_kind").map(String::as_str) {
        Some("file") => HostTarget::File(PathBuf::from(
            metadata
                .get("target_path")
                .cloned()
                .unwrap_or_else(|| connection.config_target.clone()),
        )),
        Some("export") => HostTarget::Export(PathBuf::from(
            metadata
                .get("target_path")
                .cloned()
                .unwrap_or_else(|| connection.config_target.clone()),
        )),
        Some("external_cli") => HostTarget::ExternalCli {
            program: metadata
                .get("external_program")
                .cloned()
                .unwrap_or_else(|| "claude".to_owned()),
            cwd: metadata.get("external_cwd").map(PathBuf::from),
        },
        _ if host_kind == HostKind::Generic => {
            HostTarget::Export(PathBuf::from(&connection.config_target))
        }
        _ => HostTarget::File(PathBuf::from(&connection.config_target)),
    };
    HostPlan {
        host_kind,
        connection_intent,
        host_scope,
        mode: connection.mode.clone(),
        server_name: connection.server_name.clone(),
        target,
        entry: ManagedServerEntry::new(
            &connection.connection_internal_id,
            mcp_command,
            runtime_home,
        ),
        change: PlannedChange::Noop,
        fingerprint: connection.managed_fingerprint.clone(),
        conflicts: Vec::new(),
        user_actions: stored_or_default_user_actions(connection, host_kind, host_scope),
        file_snapshot: None,
    }
}

fn stored_or_default_user_actions(
    connection: &AgentConnectionRecord,
    host_kind: HostKind,
    host_scope: HostScope,
) -> Vec<UserAction> {
    let parsed = serde_json::from_str::<Vec<UserAction>>(&connection.last_user_actions_json)
        .unwrap_or_default();
    if !parsed.is_empty() {
        return parsed;
    }
    match (host_kind, host_scope) {
        (HostKind::ClaudeCode, HostScope::Project) => vec![UserAction::new(
            UserActionKind::ProjectApprovalRequired,
            "Claude Code requires user approval before project-scoped .mcp.json servers load",
        )],
        (HostKind::Generic, HostScope::Export) => vec![UserAction::new(
            UserActionKind::HostTrustRequired,
            "Configure the external MCP host manually after a supported Agent Connection exists; Volicord does not write generic host configuration",
        )],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone)]
struct GuardIntegrationPlan {
    generated_files: Vec<GeneratedFilePlan>,
    host_hook_commands: Vec<HostHookCommand>,
    policy: Value,
    policy_hash: String,
    guard_installation_id: String,
    guard_profile: String,
    managed_source: String,
    managed_bundle_hash: Option<String>,
    managed_verification_status: String,
    native_host_output_adapter: String,
    native_host_output_adapter_verified: bool,
    bash_shell_mutation_coverage: bool,
    direct_file_write_matcher_coverage: bool,
    capabilities: HostCapabilities,
    missing_required_hooks: Vec<HostLifecyclePhase>,
}

#[derive(Debug, Clone)]
struct GeneratedFilePlan {
    kind: HostIntegrationFileKind,
    path: PathBuf,
    content: String,
    status: FilePlanStatus,
    write_kind: GeneratedFileWriteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedFileWriteKind {
    Block {
        start_marker: &'static str,
        end_marker: &'static str,
        require_existing_marker: bool,
    },
    Json,
    ExactJson,
    JsonProjection {
        projection: ManagedJsonProjection,
    },
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilePlanStatus {
    PlannedCreate,
    PlannedUpdate,
    Unchanged,
    Created,
    Updated,
}

impl FilePlanStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::PlannedCreate => "planned_create",
            Self::PlannedUpdate => "planned_update",
            Self::Unchanged => "unchanged",
            Self::Created => "created",
            Self::Updated => "updated",
        }
    }
}

#[derive(Debug, Clone)]
struct GuardCommandSpec {
    command: String,
    args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostHookCommand {
    host_kind: HostKind,
    phase: HostLifecyclePhase,
    generated_command_shape: HostHookCommandShape,
    expected_wrapper_path: PathBuf,
    expected_phase_wrapper_path: PathBuf,
    root_resolution_basis: HookRootResolutionBasis,
    hook_command_path_basis: HookCommandPathBasis,
    cwd_independent: bool,
    subdirectory_safe: bool,
    wrapper_resolution_status: HookWrapperResolutionStatus,
    verification: HostHookCommandVerification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostHookCommandShape {
    ShellCommandString(String),
    Exec { command: String, args: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookRootResolutionBasis {
    GitWorkTree,
    ClaudeProjectDir,
}

impl HookRootResolutionBasis {
    fn as_str(self) -> &'static str {
        match self {
            Self::GitWorkTree => "git_work_tree",
            Self::ClaudeProjectDir => "claude_project_dir",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookCommandPathBasis {
    GitRootRuntime,
    ClaudeProjectDir,
}

impl HookCommandPathBasis {
    fn as_str(self) -> &'static str {
        match self {
            Self::GitRootRuntime => "git_root_runtime",
            Self::ClaudeProjectDir => "claude_project_dir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostHookCommandVerification {
    basis_verified_by: String,
    host_contract_source: String,
}

impl HostHookCommand {
    fn command_line(&self) -> String {
        match &self.generated_command_shape {
            HostHookCommandShape::ShellCommandString(command) => command.clone(),
            HostHookCommandShape::Exec { command, args } => guard_command_line(&GuardCommandSpec {
                command: command.clone(),
                args: args.clone(),
            }),
        }
    }

    fn command_shape_name(&self) -> &'static str {
        match &self.generated_command_shape {
            HostHookCommandShape::ShellCommandString(_) => "shell_command_string",
            HostHookCommandShape::Exec { .. } => "exec_form",
        }
    }
}

fn plan_guard_integration(
    host_kind: HostKind,
    init_mode: InitMode,
    runtime_home: &Path,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
) -> Result<GuardIntegrationPlan, ConnectionCommandError> {
    if init_mode != InitMode::Record {
        ensure_observe_profile_supported_on_platform(host_kind)?;
    }
    let capabilities = host_capabilities(host_kind);
    let missing_required_hooks = if init_mode == InitMode::Record {
        Vec::new()
    } else {
        capabilities.missing_required_hook_phases()
    };
    if init_mode != InitMode::Record && !missing_required_hooks.is_empty() {
        return Err(ConnectionCommandError::runtime(
            observe_hooks_unsupported_message(host_kind, &missing_required_hooks),
        ));
    }
    if init_mode != InitMode::Record {
        ensure_observe_session_watcher_supported(runtime_home, repo_root, host_kind)?;
    }
    let policy_guard_commands = guard_command_specs(
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        init_mode,
        None,
    );
    let policy = policy_json(
        host_kind,
        init_mode,
        repo_root,
        connection_id,
        guard_installation_id,
        mcp_entry,
        &policy_guard_commands,
    );
    let policy_hash =
        policy_hash(&policy).map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let guard_commands = guard_command_specs(
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        init_mode,
        Some(&policy_hash),
    );
    let host_hook_commands = if init_mode != InitMode::Record
        && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
    {
        host_hook_command_specs(host_kind, repo_root)?
    } else {
        BTreeMap::new()
    };
    let mut generated_files = Vec::new();
    let agents_path = repo_root.join(AGENTS_FILE);
    generated_files.push(plan_managed_block_file(
        HostIntegrationFileKind::AgentsManagedBlock,
        &agents_path,
        &agents_guidance_block(),
        GUIDANCE_START_MARKER,
        GUIDANCE_END_MARKER,
        false,
    )?);
    let policy_path = repo_root.join(VOLICORD_POLICY_FILE);
    generated_files.push(plan_policy_file(&policy_path, &policy)?);
    if host_kind == HostKind::Codex && init_mode != InitMode::Record {
        generated_files.push(plan_codex_dispatch_wrapper_file(repo_root)?);
        generated_files.extend(plan_hook_wrapper_files(
            repo_root,
            host_kind,
            &guard_commands,
        )?);
        generated_files.push(plan_codex_hook_file(repo_root, &host_hook_commands)?);
        generated_files.push(plan_codex_rule_file(repo_root, &host_hook_commands)?);
    }
    if host_kind == HostKind::ClaudeCode {
        generated_files.push(plan_claude_mcp_file(
            repo_root,
            DEFAULT_SERVER_NAME,
            mcp_entry,
        )?);
    }
    if host_kind == HostKind::ClaudeCode && init_mode != InitMode::Record {
        generated_files.extend(plan_hook_wrapper_files(
            repo_root,
            host_kind,
            &guard_commands,
        )?);
        let command_lines = host_hook_command_lines(&host_hook_commands);
        generated_files.push(plan_claude_project_settings_file(
            repo_root,
            &host_hook_commands,
        )?);
        let rule_path = claude_code::project_rule_path(repo_root);
        let rule_block = managed_guidance_block(&claude_code::project_rule_block(
            VOLICORD_POLICY_FILE,
            &command_lines,
        ));
        generated_files.push(plan_managed_block_file(
            HostIntegrationFileKind::HostRuleInstruction,
            &rule_path,
            &rule_block,
            GUIDANCE_START_MARKER,
            GUIDANCE_END_MARKER,
            true,
        )?);
    }
    let managed_status = managed_status_for_init_mode(init_mode);
    Ok(GuardIntegrationPlan {
        generated_files,
        host_hook_commands: host_hook_commands.into_values().collect(),
        policy,
        policy_hash,
        guard_installation_id: guard_installation_id.to_owned(),
        guard_profile: guard_profile_for_init_mode(init_mode).to_owned(),
        managed_source: managed_source_for_init_mode(init_mode).to_owned(),
        managed_bundle_hash: None,
        managed_verification_status: managed_status.to_owned(),
        native_host_output_adapter: native_host_output_adapter(host_kind, init_mode).to_owned(),
        native_host_output_adapter_verified: native_host_output_adapter_verified(
            host_kind, init_mode,
        ),
        bash_shell_mutation_coverage: bash_shell_mutation_coverage(host_kind, init_mode),
        direct_file_write_matcher_coverage: direct_file_write_matcher_coverage(
            host_kind, init_mode,
        ),
        capabilities,
        missing_required_hooks,
    })
}

#[cfg(not(windows))]
fn ensure_observe_profile_supported_on_platform(
    _host_kind: HostKind,
) -> Result<(), ConnectionCommandError> {
    Ok(())
}

#[cfg(windows)]
fn ensure_observe_profile_supported_on_platform(
    host_kind: HostKind,
) -> Result<(), ConnectionCommandError> {
    Err(ConnectionCommandError::runtime(format!(
        "DETECTIVE_WINDOWS_UNSUPPORTED: native Windows supports the record profile for {}, but detective profile is not supported because Windows host-hook wrappers and session watcher behavior are not implemented and tested. Use --profile record on native Windows, or run Volicord in WSL2, Linux, or macOS where the selected host hook contract is supported.",
        public_host_label(host_kind)
    )))
}

fn guard_profile_for_init_mode(init_mode: InitMode) -> &'static str {
    match init_mode {
        InitMode::Record => "record",
        InitMode::Detective => "detective",
    }
}

fn managed_source_for_init_mode(init_mode: InitMode) -> &'static str {
    match init_mode {
        InitMode::Record => "not_applicable",
        InitMode::Detective => "host_hooks",
    }
}

fn managed_status_for_init_mode(init_mode: InitMode) -> &'static str {
    match init_mode {
        InitMode::Record | InitMode::Detective => "not_applicable",
    }
}

fn native_host_output_adapter(host_kind: HostKind, init_mode: InitMode) -> &'static str {
    match (host_kind, init_mode) {
        (HostKind::Codex, InitMode::Detective) => "codex",
        (HostKind::ClaudeCode, InitMode::Detective) => "claude-code",
        _ => "none",
    }
}

fn native_host_output_adapter_verified(host_kind: HostKind, init_mode: InitMode) -> bool {
    native_host_output_adapter(host_kind, init_mode) != "none"
}

fn bash_shell_mutation_coverage(host_kind: HostKind, init_mode: InitMode) -> bool {
    matches!(init_mode, InitMode::Detective)
        && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
}

fn direct_file_write_matcher_coverage(host_kind: HostKind, init_mode: InitMode) -> bool {
    matches!(init_mode, InitMode::Detective)
        && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
}

fn observe_hooks_unsupported_message(
    host_kind: HostKind,
    missing_required_hooks: &[HostLifecyclePhase],
) -> String {
    format!(
        "DETECTIVE_HOOKS_UNSUPPORTED: {} detective init requires supported host lifecycle hook configuration, but this adapter does not know verified project-local hook support for: {}. AGENTS.md and {VOLICORD_POLICY_FILE} are not host hook configuration. Use --profile record for record-only setup, or prepare a supported host, platform, and configuration for detective before rerunning init.",
        public_host_label(host_kind),
        lifecycle_phase_names(missing_required_hooks).join(", ")
    )
}

fn observe_hook_root_unsupported_message(host_kind: HostKind, repo_root: &Path) -> String {
    format!(
        "DETECTIVE_HOOK_ROOT_UNSUPPORTED: {} detective init requires a Git work tree root for supported host hook configuration, but no Git repository root was found from {}. Use --profile record for record-only setup, or prepare a supported host, platform, and configuration for detective before rerunning init.",
        public_host_label(host_kind),
        repo_root.display()
    )
}

fn ensure_observe_session_watcher_supported(
    runtime_home: &Path,
    repo_root: &Path,
    host_kind: HostKind,
) -> Result<(), ConnectionCommandError> {
    snapshot_product_repository(runtime_home, repo_root, WatchSnapshotOptions::default()).map_err(
        |error| {
            ConnectionCommandError::runtime(format!(
                "DETECTIVE_WATCHER_UNSUPPORTED: {} detective init requires session watcher support for the selected Product Repository, but the watcher snapshot check failed: {error}. Use --profile record for record-only setup, or prepare a supported host, platform, and repository configuration for detective before rerunning init.",
                public_host_label(host_kind)
            ))
        },
    )?;
    Ok(())
}

fn apply_guard_integration(
    mut plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, ConnectionCommandError> {
    for file in &mut plan.generated_files {
        file.status = match file.write_kind {
            GeneratedFileWriteKind::Block {
                start_marker,
                end_marker,
                require_existing_marker,
            } => write_managed_markdown_file(
                &file.path,
                &file.content,
                start_marker,
                end_marker,
                require_existing_marker,
            )?,
            GeneratedFileWriteKind::Json => {
                write_managed_json_file(&file.path, &file.policy_value()?)?
            }
            GeneratedFileWriteKind::ExactJson => {
                write_managed_exact_json_file(&file.path, &file.policy_value()?, file.kind)?
            }
            GeneratedFileWriteKind::JsonProjection { projection } => {
                write_managed_json_projection_file(&file.path, &file.policy_value()?, projection)?
            }
            GeneratedFileWriteKind::Script => {
                write_managed_script_file(&file.path, &file.content, file.kind)?
            }
        };
    }
    Ok(plan)
}

impl GeneratedFilePlan {
    fn policy_value(&self) -> Result<Value, ConnectionCommandError> {
        serde_json::from_str::<Value>(&self.content)
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
    }
}

fn plan_managed_block_file(
    kind: HostIntegrationFileKind,
    path: &Path,
    block: &str,
    start_marker: &'static str,
    end_marker: &'static str,
    require_existing_marker: bool,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let content = block.to_owned();
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            if require_existing_marker && !existing.contains(start_marker) {
                return Err(ConnectionCommandError::runtime(format!(
                    "{} already exists without a Volicord-managed block: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
            let updated = managed_block::apply_managed_block_with_markers(
                &existing,
                &content,
                start_marker,
                end_marker,
            )
            .map_err(managed_block_conflict)?;
            if updated == existing {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::Block {
            start_marker,
            end_marker,
            require_existing_marker,
        },
    })
}

fn plan_policy_file(
    path: &Path,
    policy: &Value,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let mut content = serde_json::to_string_pretty(policy)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    content.push('\n');
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            let value = serde_json::from_str::<Value>(&existing).map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "existing policy file is not valid JSON: {} ({error})",
                    path.display()
                ))
            })?;
            if !is_volicord_policy(&value) {
                return Err(ConnectionCommandError::runtime(format!(
                    "policy file already exists without Volicord ownership metadata: {}",
                    path.display()
                )));
            }
            if existing == content {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind: HostIntegrationFileKind::VolicordPolicy,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::Json,
    })
}

fn plan_hook_wrapper_files(
    repo_root: &Path,
    host_kind: HostKind,
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
) -> Result<Vec<GeneratedFilePlan>, ConnectionCommandError> {
    REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| {
            let guard_command = guard_commands.get(phase.policy_key()).ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "missing generated host-hook command for {}",
                    phase.policy_key()
                ))
            })?;
            plan_hook_wrapper_file(repo_root, host_kind, *phase, guard_command)
        })
        .collect()
}

fn plan_hook_wrapper_file(
    repo_root: &Path,
    host_kind: HostKind,
    phase: HostLifecyclePhase,
    guard_command: &GuardCommandSpec,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let relative_path = hook_wrapper_relative_path(host_kind, phase)?;
    let path = repo_root.join(&relative_path);
    let content = hook_wrapper_script_content(host_kind, phase, guard_command);
    let status = match fs::read_to_string(&path) {
        Ok(existing) => {
            if existing == content {
                if script_is_executable(&path) {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if existing.contains(HOOK_WRAPPER_MARKER) {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(ConnectionCommandError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    HostIntegrationFileKind::HostHookWrapper.as_str(),
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind: HostIntegrationFileKind::HostHookWrapper,
        path,
        content,
        status,
        write_kind: GeneratedFileWriteKind::Script,
    })
}

fn hook_wrapper_relative_path(
    host_kind: HostKind,
    phase: HostLifecyclePhase,
) -> Result<PathBuf, ConnectionCommandError> {
    let base = match host_kind {
        HostKind::Codex => PathBuf::from(".codex").join("hooks"),
        HostKind::ClaudeCode => PathBuf::from(".claude").join("hooks"),
        HostKind::Generic => {
            return Err(ConnectionCommandError::runtime(
                "generic host integrations do not define hook wrapper paths",
            ));
        }
    };
    Ok(base.join(format!("volicord-{}.sh", phase.command_name())))
}

fn codex_dispatch_wrapper_relative_path() -> PathBuf {
    PathBuf::from(CODEX_DISPATCH_WRAPPER)
}

fn plan_codex_dispatch_wrapper_file(
    repo_root: &Path,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let path = repo_root.join(codex_dispatch_wrapper_relative_path());
    let content = codex_dispatch_wrapper_script_content();
    plan_managed_script_file(&path, &content, HostIntegrationFileKind::HostHookDispatch)
}

fn host_hook_command_specs(
    host_kind: HostKind,
    repo_root: &Path,
) -> Result<BTreeMap<String, HostHookCommand>, ConnectionCommandError> {
    if host_kind == HostKind::Codex && !repo_has_git_marker(repo_root)? {
        return Err(ConnectionCommandError::runtime(
            observe_hook_root_unsupported_message(host_kind, repo_root),
        ));
    }
    REQUIRED_GUARD_PHASES
        .into_iter()
        .map(|phase| {
            let command = host_hook_command_spec(host_kind, repo_root, phase)?;
            Ok((phase.policy_key().to_owned(), command))
        })
        .collect()
}

fn host_hook_command_spec(
    host_kind: HostKind,
    repo_root: &Path,
    phase: HostLifecyclePhase,
) -> Result<HostHookCommand, ConnectionCommandError> {
    let relative_path = hook_wrapper_relative_path(host_kind, phase)?;
    let relative = path_text(&relative_path);
    match host_kind {
        HostKind::Codex => {
            let dispatch_relative = codex_dispatch_wrapper_relative_path();
            let dispatch_relative_text = path_text(&dispatch_relative);
            let expected_wrapper_path = repo_root.join(&dispatch_relative);
            let expected_phase_wrapper_path = repo_root.join(&relative_path);
            let script = format!(
                "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/{dispatch_relative_text}\" {}",
                phase.command_name()
            );
            Ok(HostHookCommand {
                host_kind,
                phase,
                generated_command_shape: HostHookCommandShape::ShellCommandString(format!(
                    "sh -c {}",
                    shell_word(&script)
                )),
                expected_wrapper_path,
                expected_phase_wrapper_path,
                root_resolution_basis: HookRootResolutionBasis::GitWorkTree,
                hook_command_path_basis: HookCommandPathBasis::GitRootRuntime,
                cwd_independent: true,
                subdirectory_safe: true,
                wrapper_resolution_status: HookWrapperResolutionStatus::Ok,
                verification: HostHookCommandVerification {
                    basis_verified_by: "repo_root_git_marker".to_owned(),
                    host_contract_source: "codex_hook_command_string".to_owned(),
                },
            })
        }
        HostKind::ClaudeCode => {
            let expected_wrapper_path = repo_root.join(&relative_path);
            Ok(HostHookCommand {
                host_kind,
                phase,
                generated_command_shape: HostHookCommandShape::Exec {
                    command: format!("${{CLAUDE_PROJECT_DIR}}/{relative}"),
                    args: Vec::new(),
                },
                expected_wrapper_path,
                expected_phase_wrapper_path: repo_root.join(&relative_path),
                root_resolution_basis: HookRootResolutionBasis::ClaudeProjectDir,
                hook_command_path_basis: HookCommandPathBasis::ClaudeProjectDir,
                cwd_independent: true,
                subdirectory_safe: true,
                wrapper_resolution_status: HookWrapperResolutionStatus::Ok,
                verification: HostHookCommandVerification {
                    basis_verified_by: "verified_claude_project_dir_placeholder".to_owned(),
                    host_contract_source: "claude_code_hook_exec_form".to_owned(),
                },
            })
        }
        HostKind::Generic => Err(ConnectionCommandError::runtime(
            "generic host integrations do not define hook commands",
        )),
    }
}

fn repo_has_git_marker(repo_root: &Path) -> Result<bool, ConnectionCommandError> {
    repo_root.join(".git").try_exists().map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "failed to inspect Git repository marker {}: {error}",
            repo_root.join(".git").display()
        ))
    })
}

fn hook_wrapper_script_content(
    host_kind: HostKind,
    phase: HostLifecyclePhase,
    guard_command: &GuardCommandSpec,
) -> String {
    let command_line = guard_command_line(guard_command);
    let connection_id = arg_after(&guard_command.args, "--connection").unwrap_or("unknown");
    let guard_installation_id =
        arg_after(&guard_command.args, "--guard-installation").unwrap_or("unknown");
    let policy_hash = arg_after(&guard_command.args, "--policy-hash").unwrap_or("unknown");
    let host_output = arg_after(&guard_command.args, "--host-output").unwrap_or("none");
    format!(
        "#!/bin/sh\n# {HOOK_WRAPPER_MARKER}\n# host_kind={}\n# phase={}\n# connection_id={connection_id}\n# guard_installation_id={guard_installation_id}\n# policy_hash={policy_hash}\n# host_output={host_output}\nexec {command_line}\n",
        public_host_label(host_kind),
        phase.policy_key(),
    )
}

fn codex_dispatch_wrapper_script_content() -> String {
    format!(
        concat!(
            "#!/bin/sh\n",
            "# {}\n",
            "# host_kind=codex\n",
            "# phase=dispatch\n",
            "# script_role=codex_dispatch\n",
            "if [ \"$#\" -ne 1 ]; then\n",
            "    printf '%s\\n' 'volicord dispatch: expected one host-hook phase argument' >&2\n",
            "    exit 64\n",
            "fi\n",
            "phase=$1\n",
            "case \"$phase\" in\n",
            "    session-start|pre-tool|post-tool|prompt-capture|stop) ;;\n",
            "    *)\n",
            "        printf '%s\\n' \"volicord dispatch: unsupported host-hook phase: $phase\" >&2\n",
            "        exit 64\n",
            "        ;;\n",
            "esac\n",
            "root=$(git rev-parse --show-toplevel 2>/dev/null) || {{\n",
            "    printf '%s\\n' 'volicord dispatch: failed to resolve Git work-tree root' >&2\n",
            "    exit 70\n",
            "}}\n",
            "case \"$root\" in\n",
            "    /*) ;;\n",
            "    *)\n",
            "        printf '%s\\n' 'volicord dispatch: resolved Git work-tree root is not absolute' >&2\n",
            "        exit 70\n",
            "        ;;\n",
            "esac\n",
            "wrapper=\"$root/.codex/hooks/volicord-$phase.sh\"\n",
            "if [ ! -f \"$wrapper\" ]; then\n",
            "    printf '%s\\n' \"volicord dispatch: missing phase wrapper: $wrapper\" >&2\n",
            "    exit 70\n",
            "fi\n",
            "if [ ! -x \"$wrapper\" ]; then\n",
            "    printf '%s\\n' \"volicord dispatch: phase wrapper is not executable: $wrapper\" >&2\n",
            "    exit 70\n",
            "fi\n",
            "exec \"$wrapper\"\n",
        ),
        HOOK_WRAPPER_MARKER
    )
}

fn arg_after<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn plan_codex_hook_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let contract = contract_for(HostKind::Codex).ok_or_else(|| {
        ConnectionCommandError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Codex host integration contract is available",
        )
    })?;
    let hooks = REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| {
            let event = hook_event_for_phase(contract, *phase).ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "DETECTIVE_HOOKS_UNSUPPORTED: Codex contract is missing {} hook event data",
                    phase.capability_name()
                ))
            })?;
            let hook_command = hook_commands.get(phase.policy_key()).ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "missing generated hook command for {}",
                    phase.policy_key()
                ))
            })?;
            let mut group = serde_json::Map::new();
            if !event.write_matcher_tokens.is_empty() {
                group.insert(
                    "matcher".to_owned(),
                    Value::String(event.write_matcher_tokens.join("|")),
                );
            } else if *phase == HostLifecyclePhase::SessionStart {
                group.insert(
                    "matcher".to_owned(),
                    Value::String("startup|resume".to_owned()),
                );
            }
            group.insert(
                "hooks".to_owned(),
                Value::Array(vec![codex_hook_handler_value(*phase, hook_command)?]),
            );
            Ok::<(String, Value), ConnectionCommandError>((
                event.event_name.to_owned(),
                Value::Array(vec![Value::Object(group)]),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    let value = json!({ "hooks": hooks });
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    validate_contract_config(HostKind::Codex, HostContractConfigKind::HookConfig, &text).map_err(
        |error| {
            ConnectionCommandError::runtime(format!(
                "generated Codex hook config does not match the verified contract: {error}"
            ))
        },
    )?;
    plan_managed_exact_json_file(
        HostIntegrationFileKind::HostHookConfig,
        &codex::project_hooks_path(repo_root),
        &value,
    )
}

fn codex_hook_handler_value(
    phase: HostLifecyclePhase,
    command: &HostHookCommand,
) -> Result<Value, ConnectionCommandError> {
    let HostHookCommandShape::ShellCommandString(command_text) = &command.generated_command_shape
    else {
        return Err(ConnectionCommandError::runtime(
            "Codex hook command generation requires command-string form",
        ));
    };
    let mut handler = serde_json::Map::new();
    handler.insert("type".to_owned(), Value::String("command".to_owned()));
    handler.insert("command".to_owned(), Value::String(command_text.clone()));
    handler.insert("timeout".to_owned(), Value::Number(30.into()));
    let status_message = match phase {
        HostLifecyclePhase::SessionStart => Some("Checking Volicord session"),
        HostLifecyclePhase::PreTool => Some("Checking Volicord write"),
        HostLifecyclePhase::PostTool => Some("Recording Volicord write"),
        HostLifecyclePhase::UserPromptSubmit | HostLifecyclePhase::Stop => None,
    };
    if let Some(status_message) = status_message {
        handler.insert(
            "statusMessage".to_owned(),
            Value::String(status_message.to_owned()),
        );
    }
    Ok(Value::Object(handler))
}

fn plan_claude_mcp_file(
    repo_root: &Path,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let value = claude_mcp_projection(server_name, entry);
    plan_managed_json_projection_file(
        HostIntegrationFileKind::HostMcpConfig,
        &repo_root.join(".mcp.json"),
        &value,
        ManagedJsonProjection::ClaudeCodeMcpEntry,
    )
}

fn plan_claude_project_settings_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let value = claude_settings_hooks_projection(hook_commands)?;
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    validate_contract_config(
        HostKind::ClaudeCode,
        HostContractConfigKind::ProjectSettings,
        &text,
    )
    .map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "generated Claude Code settings hooks do not match the verified contract: {error}"
        ))
    })?;
    plan_managed_json_projection_file(
        HostIntegrationFileKind::HostHookConfig,
        &claude_code::project_settings_path(repo_root),
        &value,
        ManagedJsonProjection::ClaudeCodeSettingsHooks,
    )
}

fn claude_mcp_projection(server_name: &str, entry: &ManagedServerEntry) -> Value {
    let mut servers = serde_json::Map::new();
    servers.insert(server_name.to_owned(), entry.to_json_value());
    let mut root = serde_json::Map::new();
    root.insert("mcpServers".to_owned(), Value::Object(servers));
    Value::Object(root)
}

fn claude_settings_hooks_projection(
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<Value, ConnectionCommandError> {
    let contract = contract_for(HostKind::ClaudeCode).ok_or_else(|| {
        ConnectionCommandError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Claude Code host integration contract is available",
        )
    })?;
    let hooks = REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| {
            let event = hook_event_for_phase(contract, *phase).ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "DETECTIVE_HOOKS_UNSUPPORTED: Claude Code contract is missing {} hook event data",
                    phase.capability_name()
                ))
            })?;
            let hook_command = hook_commands.get(phase.policy_key()).ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "missing generated hook command for {}",
                    phase.policy_key()
                ))
            })?;
            Ok::<(String, Value), ConnectionCommandError>((
                event.event_name.to_owned(),
                Value::Array(vec![claude_hook_group_value(
                    *phase,
                    event.write_matcher_tokens,
                    hook_command,
                )?]),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    Ok(json!({ "hooks": hooks }))
}

fn claude_hook_group_value(
    phase: HostLifecyclePhase,
    write_matcher_tokens: &[&str],
    command: &HostHookCommand,
) -> Result<Value, ConnectionCommandError> {
    let mut group = serde_json::Map::new();
    if !write_matcher_tokens.is_empty() {
        group.insert(
            "matcher".to_owned(),
            Value::String(write_matcher_tokens.join("|")),
        );
    } else if phase == HostLifecyclePhase::SessionStart {
        group.insert(
            "matcher".to_owned(),
            Value::String("startup|resume".to_owned()),
        );
    }
    group.insert(
        "hooks".to_owned(),
        Value::Array(vec![claude_hook_handler_value(phase, command)?]),
    );
    Ok(Value::Object(group))
}

fn claude_hook_handler_value(
    phase: HostLifecyclePhase,
    command: &HostHookCommand,
) -> Result<Value, ConnectionCommandError> {
    let HostHookCommandShape::Exec { command, args } = &command.generated_command_shape else {
        return Err(ConnectionCommandError::runtime(
            "Claude Code hook command generation requires exec-form command and args",
        ));
    };
    let mut handler = serde_json::Map::new();
    handler.insert("type".to_owned(), Value::String("command".to_owned()));
    handler.insert("command".to_owned(), Value::String(command.clone()));
    handler.insert(
        "args".to_owned(),
        Value::Array(args.iter().cloned().map(Value::String).collect()),
    );
    handler.insert("timeout".to_owned(), Value::Number(30.into()));
    let status_message = match phase {
        HostLifecyclePhase::SessionStart => Some("Checking Volicord session"),
        HostLifecyclePhase::PreTool => Some("Checking Volicord write"),
        HostLifecyclePhase::PostTool => Some("Recording Volicord write"),
        HostLifecyclePhase::UserPromptSubmit | HostLifecyclePhase::Stop => None,
    };
    if let Some(status_message) = status_message {
        handler.insert(
            "statusMessage".to_owned(),
            Value::String(status_message.to_owned()),
        );
    }
    Ok(Value::Object(handler))
}

fn plan_codex_rule_file(
    repo_root: &Path,
    hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let command_lines = host_hook_command_lines(hook_commands)
        .into_iter()
        .map(|(_, command)| command)
        .collect::<Vec<_>>();
    let mut body = String::from(
        "prefix_rule(\n    pattern = [\".codex\", \"hooks\"],\n    decision = \"prompt\",\n    justification = \"Volicord hook wrappers record local lifecycle events.\",\n    match = [\n",
    );
    for command in command_lines {
        body.push_str("        ");
        body.push_str(&starlark_string(&command));
        body.push_str(",\n");
    }
    body.push_str("    ],\n)\n");
    validate_contract_config(HostKind::Codex, HostContractConfigKind::RuleConfig, &body).map_err(
        |error| {
            ConnectionCommandError::runtime(format!(
                "generated Codex rule config does not match the verified contract: {error}"
            ))
        },
    )?;
    let block = format!("{CODEX_RULE_START_MARKER}\n{body}{CODEX_RULE_END_MARKER}\n");
    plan_managed_block_file(
        HostIntegrationFileKind::HostRuleInstruction,
        &codex::project_rule_path(repo_root),
        &block,
        CODEX_RULE_START_MARKER,
        CODEX_RULE_END_MARKER,
        true,
    )
}

fn starlark_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn plan_managed_exact_json_file(
    kind: HostIntegrationFileKind,
    path: &Path,
    value: &Value,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    content.push('\n');
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            let existing_value = serde_json::from_str::<Value>(&existing).map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "existing {} is not valid JSON: {} ({error})",
                    kind.as_str(),
                    path.display()
                ))
            })?;
            if existing_value == *value {
                if existing == content {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if kind == HostIntegrationFileKind::HostHookConfig
                && is_volicord_codex_hook_config(&existing_value)
            {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(ConnectionCommandError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::ExactJson,
    })
}

fn plan_managed_json_projection_file(
    kind: HostIntegrationFileKind,
    path: &Path,
    value: &Value,
    projection: ManagedJsonProjection,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let mut content = canonical_json_text(value)?;
    content.push('\n');
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            let existing_value = serde_json::from_str::<Value>(&existing).map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "existing {} is not valid JSON: {} ({error})",
                    kind.as_str(),
                    path.display()
                ))
            })?;
            let merged = managed_json_projection_merge(&existing_value, value, projection)?;
            if merged == existing_value {
                FilePlanStatus::Unchanged
            } else {
                FilePlanStatus::PlannedUpdate
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::JsonProjection { projection },
    })
}

fn write_managed_markdown_file(
    path: &Path,
    block: &str,
    start_marker: &'static str,
    end_marker: &'static str,
    require_existing_marker: bool,
) -> Result<FilePlanStatus, ConnectionCommandError> {
    if require_existing_marker && path.exists() {
        let existing = fs::read_to_string(path).map_err(|error| {
            ConnectionCommandError::runtime(format!("failed to read {}: {error}", path.display()))
        })?;
        if !existing.contains(start_marker) {
            return Err(ConnectionCommandError::runtime(format!(
                "{} already exists without a Volicord-managed block",
                path.display()
            )));
        }
    }
    match managed_block::write_managed_block_with_markers(path, block, start_marker, end_marker)
        .map_err(|error| {
            ConnectionCommandError::runtime(format!("failed to write {}: {error}", path.display()))
        })? {
        Ok(ManagedBlockWrite::Created(_)) => Ok(FilePlanStatus::Created),
        Ok(ManagedBlockWrite::Updated(_)) => Ok(FilePlanStatus::Updated),
        Ok(ManagedBlockWrite::Unchanged(_)) => Ok(FilePlanStatus::Unchanged),
        Err(error) => Err(managed_block_conflict(error)),
    }
}

fn write_managed_json_file(
    path: &Path,
    value: &Value,
) -> Result<FilePlanStatus, ConnectionCommandError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    content.push('\n');
    let planned = plan_policy_file(path, value)?;
    if planned.status == FilePlanStatus::Unchanged {
        return Ok(FilePlanStatus::Unchanged);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to create {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, content).map_err(|error| {
        ConnectionCommandError::runtime(format!("failed to write {}: {error}", path.display()))
    })?;
    Ok(match planned.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => other,
    })
}

fn write_managed_exact_json_file(
    path: &Path,
    value: &Value,
    kind: HostIntegrationFileKind,
) -> Result<FilePlanStatus, ConnectionCommandError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    content.push('\n');
    let planned = plan_managed_exact_json_file(kind, path, value)?;
    if planned.status == FilePlanStatus::Unchanged {
        return Ok(FilePlanStatus::Unchanged);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to create {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, content).map_err(|error| {
        ConnectionCommandError::runtime(format!("failed to write {}: {error}", path.display()))
    })?;
    Ok(match planned.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => other,
    })
}

fn write_managed_json_projection_file(
    path: &Path,
    value: &Value,
    projection: ManagedJsonProjection,
) -> Result<FilePlanStatus, ConnectionCommandError> {
    let mut existed = true;
    let existing = match fs::read_to_string(path) {
        Ok(text) => {
            let value = serde_json::from_str::<Value>(&text).map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "existing JSON configuration is not valid JSON: {} ({error})",
                    path.display()
                ))
            })?;
            Some(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            existed = false;
            None
        }
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    let current = existing.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let merged = managed_json_projection_merge(&current, value, projection)?;
    if merged == current {
        return Ok(FilePlanStatus::Unchanged);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to create {}: {error}",
                parent.display()
            ))
        })?;
    }
    let mut text = serde_json::to_string_pretty(&merged)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    text.push('\n');
    fs::write(path, text).map_err(|error| {
        ConnectionCommandError::runtime(format!("failed to write {}: {error}", path.display()))
    })?;
    Ok(if existed {
        FilePlanStatus::Updated
    } else {
        FilePlanStatus::Created
    })
}

fn write_managed_script_file(
    path: &Path,
    content: &str,
    kind: HostIntegrationFileKind,
) -> Result<FilePlanStatus, ConnectionCommandError> {
    let planned = plan_managed_script_file(path, content, kind)?;
    if planned.status != FilePlanStatus::Unchanged {
        let existing_matches = fs::read_to_string(path)
            .map(|existing| existing == content)
            .unwrap_or(false);
        if !existing_matches {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    ConnectionCommandError::runtime(format!(
                        "failed to create {}: {error}",
                        parent.display()
                    ))
                })?;
            }
            fs::write(path, content).map_err(|error| {
                ConnectionCommandError::runtime(format!(
                    "failed to write {}: {error}",
                    path.display()
                ))
            })?;
        }
        set_script_executable(path)?;
    }
    Ok(match planned.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => other,
    })
}

fn plan_managed_script_file(
    path: &Path,
    content: &str,
    kind: HostIntegrationFileKind,
) -> Result<GeneratedFilePlan, ConnectionCommandError> {
    let status = match fs::read_to_string(path) {
        Ok(existing) => {
            if existing == content {
                if script_is_executable(path) {
                    FilePlanStatus::Unchanged
                } else {
                    FilePlanStatus::PlannedUpdate
                }
            } else if existing.contains(HOOK_WRAPPER_MARKER) {
                FilePlanStatus::PlannedUpdate
            } else {
                return Err(ConnectionCommandError::runtime(format!(
                    "{} already exists with unmanaged content: {}",
                    kind.as_str(),
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FilePlanStatus::PlannedCreate,
        Err(error) => {
            return Err(ConnectionCommandError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    Ok(GeneratedFilePlan {
        kind,
        path: path.to_path_buf(),
        content: content.to_owned(),
        status,
        write_kind: GeneratedFileWriteKind::Script,
    })
}

#[cfg(unix)]
fn set_script_executable(path: &Path) -> Result<(), ConnectionCommandError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to inspect {} permissions: {error}",
                path.display()
            ))
        })?
        .permissions();
    let mode = permissions.mode();
    if mode & 0o100 == 0 {
        permissions.set_mode(mode | 0o755);
        fs::set_permissions(path, permissions).map_err(|error| {
            ConnectionCommandError::runtime(format!(
                "failed to make {} executable: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_script_executable(_path: &Path) -> Result<(), ConnectionCommandError> {
    Ok(())
}

fn canonical_json_text(value: &Value) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(value).map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn managed_json_projection_merge(
    current: &Value,
    desired: &Value,
    projection: ManagedJsonProjection,
) -> Result<Value, ConnectionCommandError> {
    let merged = match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => {
            merge_claude_settings_hooks(current, desired)
        }
        ManagedJsonProjection::ClaudeCodeMcpEntry => merge_claude_mcp_entry(current, desired),
    }?;
    validate_managed_json_projection_config(projection, &merged)?;
    Ok(merged)
}

fn validate_managed_json_projection_config(
    projection: ManagedJsonProjection,
    value: &Value,
) -> Result<(), ConnectionCommandError> {
    let text = serde_json::to_string(value)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let (kind, label) = match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => (
            HostContractConfigKind::ProjectSettings,
            "merged Claude Code project settings",
        ),
        ManagedJsonProjection::ClaudeCodeMcpEntry => (
            HostContractConfigKind::McpConfig,
            "merged Claude Code MCP config",
        ),
    };
    validate_contract_config(HostKind::ClaudeCode, kind, &text).map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "{label} do not match the verified contract: {error}"
        ))
    })
}

fn merge_claude_mcp_entry(
    current: &Value,
    desired: &Value,
) -> Result<Value, ConnectionCommandError> {
    let mut object = current.as_object().cloned().ok_or_else(|| {
        ConnectionCommandError::runtime("Claude Code .mcp.json must be a JSON object")
    })?;
    let desired_servers = desired
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| ConnectionCommandError::runtime("managed MCP projection is invalid"))?;
    let servers = object
        .entry("mcpServers".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            ConnectionCommandError::runtime("Claude Code .mcp.json mcpServers must be an object")
        })?;
    for (name, entry) in desired_servers {
        servers.insert(name.clone(), entry.clone());
    }
    Ok(Value::Object(object))
}

fn merge_claude_settings_hooks(
    current: &Value,
    desired: &Value,
) -> Result<Value, ConnectionCommandError> {
    let mut root = current.as_object().cloned().ok_or_else(|| {
        ConnectionCommandError::runtime("Claude Code settings must be a JSON object")
    })?;
    let desired_hooks = desired
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ConnectionCommandError::runtime("managed Claude Code hook projection is invalid")
        })?;
    let hooks = root
        .entry("hooks".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            ConnectionCommandError::runtime("Claude Code settings hooks must be an object")
        })?;
    for phase in REQUIRED_GUARD_PHASES {
        let event_name = claude_event_name(phase)?;
        let desired_groups = desired_hooks
            .get(event_name)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "managed Claude Code hook projection is missing {event_name}"
                ))
            })?;
        let desired_group = desired_groups.first().cloned().ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "managed Claude Code hook projection has no {event_name} group"
            ))
        })?;
        let desired_handler = claude_managed_group_signature(&desired_group, event_name)?;
        let existing_groups = hooks
            .remove(event_name)
            .map(|value| {
                value.as_array().cloned().ok_or_else(|| {
                    ConnectionCommandError::runtime(format!(
                        "Claude Code settings hook event {event_name} must be an array"
                    ))
                })
            })
            .transpose()?
            .unwrap_or_default();
        let mut preserved_groups = Vec::new();
        for group in existing_groups {
            if let Some(group) =
                remove_claude_managed_handlers(phase, event_name, &desired_handler, group)?
            {
                preserved_groups.push(group);
            }
        }
        preserved_groups.push(desired_group);
        hooks.insert(event_name.to_owned(), Value::Array(preserved_groups));
    }
    Ok(Value::Object(root))
}

fn remove_claude_managed_handlers(
    phase: HostLifecyclePhase,
    event_name: &str,
    desired_handler: &ClaudeHookHandlerSignature,
    group: Value,
) -> Result<Option<Value>, ConnectionCommandError> {
    let mut object = group.as_object().cloned().ok_or_else(|| {
        ConnectionCommandError::runtime(format!(
            "Claude Code settings hook group for {event_name} must be an object"
        ))
    })?;
    let handlers = object
        .remove("hooks")
        .ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "Claude Code settings hook group for {event_name} must contain hooks"
            ))
        })?
        .as_array()
        .cloned()
        .ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "Claude Code settings hook handlers for {event_name} must be an array"
            ))
        })?;
    let mut kept = Vec::new();
    let mut removed = 0usize;
    for handler in handlers {
        if is_exact_claude_managed_handler(&handler, desired_handler)
            || is_legacy_claude_managed_handler(phase, &handler)
        {
            removed += 1;
        } else if looks_like_conflicting_claude_managed_handler(phase, &handler, desired_handler) {
            return Err(ConnectionCommandError::runtime(format!(
                "Claude Code settings contain a conflicting Volicord-managed {event_name} hook entry"
            )));
        } else {
            kept.push(handler);
        }
    }
    if removed == 0 {
        object.insert("hooks".to_owned(), Value::Array(kept));
        return Ok(Some(Value::Object(object)));
    }
    if kept.is_empty() {
        return Ok(None);
    }
    object.insert("hooks".to_owned(), Value::Array(kept));
    Ok(Some(Value::Object(object)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaudeHookHandlerSignature {
    command: String,
    args: Option<Vec<String>>,
}

fn claude_managed_group_signature(
    group: &Value,
    event_name: &str,
) -> Result<ClaudeHookHandlerSignature, ConnectionCommandError> {
    let handler = group
        .get("hooks")
        .and_then(Value::as_array)
        .and_then(|handlers| handlers.first())
        .ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "managed Claude Code hook projection is missing {event_name} handler"
            ))
        })?;
    let command = handler
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "managed Claude Code hook projection is missing {event_name} command"
            ))
        })?;
    let args = match handler.get("args") {
        Some(value) => {
            let values = value.as_array().ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "managed Claude Code hook projection has non-array {event_name} args"
                ))
            })?;
            Some(
                values
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        ConnectionCommandError::runtime(format!(
                            "managed Claude Code hook projection has non-string {event_name} args"
                        ))
                    })?,
            )
        }
        None => None,
    };
    Ok(ClaudeHookHandlerSignature { command, args })
}

fn is_exact_claude_managed_handler(handler: &Value, desired: &ClaudeHookHandlerSignature) -> bool {
    handler.as_object().is_some_and(|object| {
        object.get("type").and_then(Value::as_str) == Some("command")
            && object
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| command == desired.command)
            && hook_handler_args(object) == desired.args
    })
}

fn is_legacy_claude_managed_handler(phase: HostLifecyclePhase, handler: &Value) -> bool {
    handler.as_object().is_some_and(|object| {
        object.get("type").and_then(Value::as_str) == Some("command")
            && object
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| {
                    let legacy_direct = command
                        .contains(&format!("volicord _hook {}", phase.command_name()))
                        && command.contains("--connection")
                        && command.contains("--guard-installation")
                        && (command.contains("--host claude-code")
                            || command.contains("--host claude_code"))
                        && (command.contains("--host-output claude-code")
                            || command.contains("--host-output claude_code"));
                    let legacy_wrapper = command.contains(&format!(
                        ".claude/hooks/volicord-{}.sh",
                        phase.command_name()
                    ));
                    legacy_direct || legacy_wrapper
                })
    })
}

fn looks_like_conflicting_claude_managed_handler(
    phase: HostLifecyclePhase,
    handler: &Value,
    desired: &ClaudeHookHandlerSignature,
) -> bool {
    handler.as_object().is_some_and(|object| {
        object
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| {
                (command != desired.command || hook_handler_args(object) != desired.args)
                    && ((command.contains("volicord _hook")
                        && command.contains(phase.command_name())
                        && (command.contains("--host claude-code")
                            || command.contains("--host claude_code")
                            || command.contains("--guard-installation")))
                        || command.contains(&format!(
                            ".claude/hooks/volicord-{}.sh",
                            phase.command_name()
                        )))
            })
    })
}

fn hook_handler_args(object: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    object
        .get("args")
        .and_then(Value::as_array)
        .and_then(|args| {
            args.iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
}

fn claude_event_name(phase: HostLifecyclePhase) -> Result<&'static str, ConnectionCommandError> {
    let contract = contract_for(HostKind::ClaudeCode).ok_or_else(|| {
        ConnectionCommandError::runtime(
            "DETECTIVE_HOOKS_UNSUPPORTED: no Claude Code host integration contract is available",
        )
    })?;
    hook_event_for_phase(contract, phase)
        .map(|event| event.event_name)
        .ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "DETECTIVE_HOOKS_UNSUPPORTED: Claude Code contract is missing {} hook event data",
                phase.capability_name()
            ))
        })
}

fn managed_block_conflict(error: ManagedBlockError) -> ConnectionCommandError {
    match error {
        ManagedBlockError::Unterminated { start_marker } => ConnectionCommandError::runtime(
            format!("managed block starting with {start_marker} is missing its end marker"),
        ),
        ManagedBlockError::Duplicate { start_marker } => ConnectionCommandError::runtime(format!(
            "multiple managed blocks starting with {start_marker} were found"
        )),
    }
}

fn is_volicord_policy(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(VOLICORD_POLICY_SCHEMA)
        && value.get("managed_by").and_then(Value::as_str) == Some("volicord")
}

fn agents_guidance_block() -> String {
    format!(
        "{GUIDANCE_START_MARKER}\n# Volicord\n\n- Check Volicord status before planning: `volicord.status`.\n- Start a task before planning implementation: `volicord.intake`.\n- Prepare write before product-file changes: `volicord.prepare_write`.\n- Request user judgment through Volicord: `volicord.request_user_judgment`; the user records decisions through the `User Channel`.\n- Check close before claiming completion: `volicord.check_close`.\n- If Volicord tools are unavailable, say so explicitly and do not imply Volicord state was updated.\n{GUIDANCE_END_MARKER}\n"
    )
}

fn managed_guidance_block(body: &str) -> String {
    format!("{GUIDANCE_START_MARKER}\n{body}{GUIDANCE_END_MARKER}\n")
}

fn guard_command_specs(
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host_kind: HostKind,
    init_mode: InitMode,
    policy_hash: Option<&str>,
) -> BTreeMap<String, GuardCommandSpec> {
    REQUIRED_GUARD_PHASES
        .into_iter()
        .map(|phase| {
            let mut args = vec![
                "_hook".to_owned(),
                phase.command_name().to_owned(),
                "--repo".to_owned(),
                path_text(repo_root),
                "--connection".to_owned(),
                connection_id.to_owned(),
                "--guard-installation".to_owned(),
                guard_installation_id.to_owned(),
                "--host".to_owned(),
                public_host_label(host_kind).to_owned(),
                "--integration-profile".to_owned(),
                init_mode.profile_value().to_owned(),
            ];
            if let Some(policy_hash) = policy_hash {
                args.push("--policy-hash".to_owned());
                args.push(policy_hash.to_owned());
            }
            match (host_kind, init_mode) {
                (HostKind::Codex, InitMode::Detective) => {
                    args.push("--host-output".to_owned());
                    args.push("codex".to_owned());
                }
                (HostKind::ClaudeCode, InitMode::Detective) => {
                    args.push("--host-output".to_owned());
                    args.push("claude-code".to_owned());
                }
                _ => {
                    args.push("--output".to_owned());
                    args.push("volicord-json".to_owned());
                }
            }
            (
                phase.policy_key().to_owned(),
                GuardCommandSpec {
                    command: DEFAULT_MCP_COMMAND.to_owned(),
                    args,
                },
            )
        })
        .collect()
}

fn host_hook_command_lines(commands: &BTreeMap<String, HostHookCommand>) -> Vec<(String, String)> {
    commands
        .iter()
        .map(|(phase, spec)| (phase.clone(), spec.command_line()))
        .collect()
}

fn guard_command_line(spec: &GuardCommandSpec) -> String {
    let mut words = Vec::with_capacity(spec.args.len() + 1);
    words.push(shell_word(&spec.command));
    words.extend(spec.args.iter().map(|arg| shell_word(arg)));
    words.join(" ")
}

fn shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn policy_json(
    host_kind: HostKind,
    init_mode: InitMode,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
) -> Value {
    let commands = guard_commands
        .iter()
        .map(|(phase, spec)| {
            (
                phase.clone(),
                json!({
                    "command": &spec.command,
                    "args": &spec.args,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "schema": VOLICORD_POLICY_SCHEMA,
        "managed_by": "volicord",
        "host": public_host_label(host_kind),
        "repo_root": path_text(repo_root),
        "connection_id": connection_id,
        "guard_installation_id": guard_installation_id,
        "selected_profile": init_mode.profile_value(),
        "mcp": {
            "command": &mcp_entry.command,
            "args": &mcp_entry.args,
            "env": &mcp_entry.env,
        },
        "host_hook": {
            "enabled": init_mode != InitMode::Record,
            "commands": commands,
        },
    })
}

fn record_guard_installation(
    runtime_home: &Path,
    host_kind: HostKind,
    init_mode: InitMode,
    installation_status: GuardInstallationStatus,
    connection_id: &str,
    project_id: &str,
    integration: &GuardIntegrationPlan,
) -> Result<GuardInstallationRecord, ConnectionCommandError> {
    let now = current_timestamp();
    upsert_guard_installation(
        runtime_home,
        GuardInstallationUpsert {
            guard_installation_id: integration.guard_installation_id.clone(),
            connection_internal_id: connection_id.to_owned(),
            project_id: Some(project_id.to_owned()),
            host_kind: host_kind.as_str().to_owned(),
            guard_mode: init_mode.guard_value().to_owned(),
            host_capability_json: host_hook_capability_json(integration)?,
            installation_status: installation_status.as_str().to_owned(),
            installed_at: (init_mode != InitMode::Record).then_some(now.clone()),
            last_checked_at: now,
            first_seen_at: None,
            last_seen_at: None,
            last_seen_phase: None,
            observed_host_kind: None,
            observed_policy_hash: None,
            observed_binary_version: None,
            metadata_json: serde_json::to_string(&json!({
                "created_by": INIT_METADATA_CREATED_BY,
                "policy_file": VOLICORD_POLICY_FILE,
                "selected_profile": integration.guard_profile,
                "required_phases": required_guard_phase_names(),
                "observation_status": if init_mode == InitMode::Record {
                    "disabled"
                } else {
                    "not_observed"
                },
            }))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        },
    )
    .map_err(Into::into)
}

fn host_hook_capability_json(
    plan: &GuardIntegrationPlan,
) -> Result<String, ConnectionCommandError> {
    let capabilities = serde_json::to_value(plan.capabilities)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    serde_json::to_string(&json!({
        "schema": "volicord-host-hook-capability-v1",
        "policy_hash": plan.policy_hash,
        "selected_profile": plan.guard_profile,
        "native_host_output_adapter": plan.native_host_output_adapter,
        "native_host_output_adapter_verified": plan.native_host_output_adapter_verified,
        "bash_shell_mutation_coverage": plan.bash_shell_mutation_coverage,
        "direct_file_write_matcher_coverage": plan.direct_file_write_matcher_coverage,
        "host_capabilities": capabilities,
        "required_hook_phases": required_guard_phase_names(),
        "missing_required_hooks": lifecycle_phase_names(&plan.missing_required_hooks),
        "prompt_capture": plan.capabilities.user_prompt_submit_hook
            && guard_has_prompt_capture_commands(&plan.policy),
        "files": generated_files_json(&plan.generated_files),
        "host_hook_commands": host_hook_commands_json(&plan.host_hook_commands),
        "hook_root_resolution": hook_root_resolution_json(&plan.host_hook_commands),
        "hook_path_safety": hook_path_safety_json(&plan.host_hook_commands),
        "commands": plan.policy["host_hook"]["commands"].clone(),
    }))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn initial_guard_installation_status(
    init_mode: InitMode,
    host_plan: &HostPlan,
    integration: &GuardIntegrationPlan,
) -> GuardInstallationStatus {
    if init_mode == InitMode::Record {
        GuardInstallationStatus::Configured
    } else if !integration.missing_required_hooks.is_empty() {
        GuardInstallationStatus::Degraded
    } else if host_plan.change != PlannedChange::Noop
        || integration.generated_files.iter().any(|file| {
            matches!(
                file.status,
                FilePlanStatus::Created | FilePlanStatus::Updated
            )
        })
    {
        GuardInstallationStatus::ReloadRequired
    } else {
        GuardInstallationStatus::Configured
    }
}

fn lifecycle_phase_names(phases: &[HostLifecyclePhase]) -> Vec<&'static str> {
    phases.iter().map(|phase| phase.capability_name()).collect()
}

fn guard_has_prompt_capture_commands(policy: &Value) -> bool {
    policy
        .get("host_hook")
        .and_then(|guard| guard.get("commands"))
        .and_then(|commands| commands.get("prompt_capture"))
        .is_some()
}

fn init_first_run_user_actions(
    existing: &[UserAction],
    host_kind: HostKind,
    init_mode: InitMode,
) -> Vec<UserAction> {
    let mut actions = existing.to_vec();
    if host_kind == HostKind::Codex && init_mode != InitMode::Record {
        let codex_first_run_hook_trust_hint = UserAction::new(
            UserActionKind::HostTrustRequired,
            "Review and trust Codex project hook commands before relying on Volicord detective host hooks",
        );
        if !actions.contains(&codex_first_run_hook_trust_hint) {
            actions.push(codex_first_run_hook_trust_hint);
        }
    }
    actions.push(UserAction::new(
        UserActionKind::ReloadRequired,
        format!(
            "Restart or reload {} so it loads the Volicord MCP and host hook configuration",
            public_host_label(host_kind)
        ),
    ));
    actions
}

fn current_timestamp() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true)
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
    native_host_output_adapter_verified: bool,
    hook_path_safety_state: String,
    hook_commands_cwd_independent: bool,
    hook_commands_subdirectory_safe: bool,
    hook_path_safety_details: Vec<Value>,
    bash_shell_mutation_coverage: bool,
    direct_file_write_matcher_coverage: bool,
    files_state: String,
    managed_source_state: String,
    managed_bundle_hash: Option<String>,
    managed_verification_state: String,
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
            native_host_output_adapter_verified: false,
            hook_path_safety_state: "not_checked".to_owned(),
            hook_commands_cwd_independent: false,
            hook_commands_subdirectory_safe: false,
            hook_path_safety_details: Vec::new(),
            bash_shell_mutation_coverage: false,
            direct_file_write_matcher_coverage: false,
            files_state: "not_configured".to_owned(),
            managed_source_state: "not_configured".to_owned(),
            managed_bundle_hash: None,
            managed_verification_state: "not_configured".to_owned(),
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

    fn planned(init_mode: InitMode, integration: &GuardIntegrationPlan) -> Self {
        let installation_state = "planned".to_owned();
        let observation_state = if init_mode == InitMode::Record {
            "disabled".to_owned()
        } else {
            "not_observed".to_owned()
        };
        let configuration_state = guard_configuration_state(
            &installation_state,
            !integration.missing_required_hooks.is_empty(),
        );
        let effective_state = guard_effective_state(
            init_mode.guard_value(),
            &configuration_state,
            &observation_state,
        );
        Self {
            mode_state: init_mode.guard_value().to_owned(),
            guard_profile_state: integration.guard_profile.clone(),
            installation_state,
            configuration_state,
            observation_state: observation_state.clone(),
            effective_state,
            generated_config_verified: false,
            native_host_output_adapter_verified: integration.native_host_output_adapter_verified,
            hook_path_safety_state: planned_hook_path_safety_state(init_mode, integration),
            hook_commands_cwd_independent: integration
                .host_hook_commands
                .iter()
                .all(|command| command.cwd_independent),
            hook_commands_subdirectory_safe: integration
                .host_hook_commands
                .iter()
                .all(|command| command.subdirectory_safe),
            hook_path_safety_details: Vec::new(),
            bash_shell_mutation_coverage: integration.bash_shell_mutation_coverage,
            direct_file_write_matcher_coverage: integration.direct_file_write_matcher_coverage,
            files_state: if init_mode == InitMode::Record {
                "disabled".to_owned()
            } else {
                "planned".to_owned()
            },
            managed_source_state: integration.managed_source.clone(),
            managed_bundle_hash: integration.managed_bundle_hash.clone(),
            managed_verification_state: integration.managed_verification_status.clone(),
            agents_block_state: generated_file_kind_state(
                &integration.generated_files,
                HostIntegrationFileKind::AgentsManagedBlock,
            ),
            policy_file_state: generated_file_kind_state(
                &integration.generated_files,
                HostIntegrationFileKind::VolicordPolicy,
            ),
            rule_instruction_state: planned_rule_instruction_state(init_mode, integration),
            hook_config_state: planned_hook_config_state(init_mode, integration),
            hook_observed_state: observation_state,
            last_observed_at: None,
            last_guard_event_at: None,
            prompt_capture_state: planned_prompt_capture_state(init_mode, integration).to_owned(),
            missing_files: Vec::new(),
            stale_files: Vec::new(),
            broken_files: Vec::new(),
            missing_required_hooks: lifecycle_phase_names(&integration.missing_required_hooks)
                .into_iter()
                .map(str::to_owned)
                .collect(),
            unresolved_blockers: Vec::new(),
        }
    }

    fn init(health: &str, init_mode: InitMode, integration: &GuardIntegrationPlan) -> Self {
        let missing_required_hooks = lifecycle_phase_names(&integration.missing_required_hooks)
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let hook_observed_state = if init_mode == InitMode::Record {
            "disabled".to_owned()
        } else if health == GuardInstallationStatus::Active.as_str() {
            "observed".to_owned()
        } else {
            "not_observed".to_owned()
        };
        let configuration_state =
            guard_configuration_state(health, !missing_required_hooks.is_empty());
        let observation_state = guard_observation_state(&hook_observed_state);
        let effective_state = guard_effective_state(
            init_mode.guard_value(),
            &configuration_state,
            &observation_state,
        );
        let required_hooks_missing = !missing_required_hooks.is_empty();
        Self {
            mode_state: init_mode.guard_value().to_owned(),
            guard_profile_state: integration.guard_profile.clone(),
            installation_state: health.to_owned(),
            configuration_state,
            observation_state,
            effective_state,
            generated_config_verified: init_mode != InitMode::Record
                && integration
                    .generated_files
                    .iter()
                    .all(|file| file.status == FilePlanStatus::Unchanged),
            native_host_output_adapter_verified: integration.native_host_output_adapter_verified,
            hook_path_safety_state: planned_hook_path_safety_state(init_mode, integration),
            hook_commands_cwd_independent: integration
                .host_hook_commands
                .iter()
                .all(|command| command.cwd_independent),
            hook_commands_subdirectory_safe: integration
                .host_hook_commands
                .iter()
                .all(|command| command.subdirectory_safe),
            hook_path_safety_details: Vec::new(),
            bash_shell_mutation_coverage: integration.bash_shell_mutation_coverage,
            direct_file_write_matcher_coverage: integration.direct_file_write_matcher_coverage,
            files_state: if init_mode == InitMode::Record {
                "disabled".to_owned()
            } else {
                "installed".to_owned()
            },
            managed_source_state: integration.managed_source.clone(),
            managed_bundle_hash: integration.managed_bundle_hash.clone(),
            managed_verification_state: integration.managed_verification_status.clone(),
            agents_block_state: generated_file_kind_state(
                &integration.generated_files,
                HostIntegrationFileKind::AgentsManagedBlock,
            ),
            policy_file_state: generated_file_kind_state(
                &integration.generated_files,
                HostIntegrationFileKind::VolicordPolicy,
            ),
            rule_instruction_state: planned_rule_instruction_state(init_mode, integration),
            hook_config_state: planned_hook_config_state(init_mode, integration),
            hook_observed_state: hook_observed_state.clone(),
            last_observed_at: None,
            last_guard_event_at: None,
            prompt_capture_state: init_prompt_capture_state(
                init_mode,
                integration,
                health,
                &hook_observed_state,
            )
            .to_owned(),
            missing_files: Vec::new(),
            stale_files: Vec::new(),
            broken_files: Vec::new(),
            missing_required_hooks,
            unresolved_blockers: guard_blockers_for_state(
                init_mode.guard_value(),
                health,
                health == GuardInstallationStatus::Active.as_str(),
                required_hooks_missing,
            ),
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
            "native_host_output_adapter_verified": self.native_host_output_adapter_verified,
            "hook_path_safety": &self.hook_path_safety_state,
            "hook_commands_cwd_independent": self.hook_commands_cwd_independent,
            "hook_commands_subdirectory_safe": self.hook_commands_subdirectory_safe,
            "hook_path_safety_details": &self.hook_path_safety_details,
            "cooperative_pre_tool_warning_available": self.cooperative_pre_tool_warning_available(),
            "cooperative_pre_tool_denial_available": self.cooperative_pre_tool_denial_available(),
            "post_tool_correlation_available": self.post_tool_correlation_available(),
            "bash_shell_mutation_coverage": self.bash_shell_mutation_coverage,
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
            "local_web_consent_available": false,
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
            "session_watcher_active": false,
            "cooperative_pre_tool_warning_available": self.cooperative_pre_tool_warning_available(),
            "cooperative_pre_tool_denial_available": self.cooperative_pre_tool_denial_available(),
            "unrecorded_changes_detectable": false,
            "actor_identity_provable": false,
            "os_enforced": false,
        })
    }

    fn selected_profile(&self) -> &str {
        match self.guard_profile_state.as_str() {
            "detective" => "detective",
            _ => "record",
        }
    }

    fn detective_hooks_applicable(&self) -> bool {
        matches!(self.mode_state.as_str(), "detective" | "mixed")
            || matches!(self.guard_profile_state.as_str(), "detective" | "mixed")
    }

    fn host_hook_guard_available(&self) -> bool {
        self.mode_state == IntegrationProfile::Detective.as_str()
            && self.effective_state == "active"
            && self.missing_required_hooks.is_empty()
            && self.generated_config_verified
            && self.native_host_output_adapter_verified
            && self.hook_path_safety_state == HookWrapperResolutionStatus::Ok.as_str()
            && self.hook_commands_cwd_independent
            && self.hook_commands_subdirectory_safe
            && self.bash_shell_mutation_coverage
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
        if self.mode_state == IntegrationProfile::Record.as_str() {
            "disabled"
        } else if self.missing_required_hooks.is_empty() {
            "configured"
        } else {
            "missing"
        }
    }
}

fn generated_file_kind_state(files: &[GeneratedFilePlan], kind: HostIntegrationFileKind) -> String {
    files
        .iter()
        .filter(|file| file.kind == kind)
        .map(|file| file.status.as_str())
        .reduce(combine_file_states)
        .unwrap_or("not_configured")
        .to_owned()
}

fn combine_file_states(left: &'static str, right: &'static str) -> &'static str {
    if file_state_rank(right) > file_state_rank(left) {
        right
    } else {
        left
    }
}

fn planned_rule_instruction_state(
    init_mode: InitMode,
    integration: &GuardIntegrationPlan,
) -> String {
    if init_mode == InitMode::Record {
        return "not_applicable".to_owned();
    }
    let state = generated_file_kind_state(
        &integration.generated_files,
        HostIntegrationFileKind::HostRuleInstruction,
    );
    if state != "not_configured" {
        state
    } else if integration.capabilities.rule_file_support {
        "not_configured".to_owned()
    } else {
        "unsupported_by_host".to_owned()
    }
}

fn planned_hook_config_state(init_mode: InitMode, integration: &GuardIntegrationPlan) -> String {
    if init_mode == InitMode::Record {
        return "disabled".to_owned();
    }
    let config_state = generated_file_kind_state(
        &integration.generated_files,
        HostIntegrationFileKind::HostHookConfig,
    );
    let dispatch_state = generated_file_kind_state(
        &integration.generated_files,
        HostIntegrationFileKind::HostHookDispatch,
    );
    let wrapper_state = generated_file_kind_state(
        &integration.generated_files,
        HostIntegrationFileKind::HostHookWrapper,
    );
    let state = combine_optional_file_states(
        &combine_optional_file_states(&config_state, &dispatch_state),
        &wrapper_state,
    );
    if state != "not_configured" {
        state
    } else if integration.missing_required_hooks.is_empty() {
        "not_recorded".to_owned()
    } else {
        "missing_required_hooks".to_owned()
    }
}

fn planned_hook_path_safety_state(
    init_mode: InitMode,
    integration: &GuardIntegrationPlan,
) -> String {
    if init_mode == InitMode::Record {
        return "not_applicable".to_owned();
    }
    if integration.host_hook_commands.is_empty() {
        return HookWrapperResolutionStatus::MetadataMissing
            .as_str()
            .to_owned();
    }
    if integration.host_hook_commands.iter().all(|command| {
        command.cwd_independent
            && command.subdirectory_safe
            && command.wrapper_resolution_status == HookWrapperResolutionStatus::Ok
    }) {
        HookWrapperResolutionStatus::Ok.as_str().to_owned()
    } else {
        HookWrapperResolutionStatus::RelativePathUnsafe
            .as_str()
            .to_owned()
    }
}

fn planned_prompt_capture_state(
    init_mode: InitMode,
    integration: &GuardIntegrationPlan,
) -> &'static str {
    if init_mode == InitMode::Record {
        return PromptCaptureStatus::NotConfigured.as_str();
    }
    if !integration.capabilities.user_prompt_submit_hook {
        return PromptCaptureStatus::UnsupportedByHost.as_str();
    }
    if !guard_has_prompt_capture_commands(&integration.policy)
        || integration
            .missing_required_hooks
            .contains(&HostLifecyclePhase::UserPromptSubmit)
    {
        return PromptCaptureStatus::NotConfigured.as_str();
    }
    if !integration.missing_required_hooks.is_empty() {
        return PromptCaptureStatus::Degraded.as_str();
    }
    PromptCaptureStatus::Configured.as_str()
}

fn init_prompt_capture_state(
    init_mode: InitMode,
    integration: &GuardIntegrationPlan,
    installation_status: &str,
    hook_observed_state: &str,
) -> &'static str {
    let planned = planned_prompt_capture_state(init_mode, integration);
    if !matches!(
        planned,
        "configured" | "observed" | "active" | "reload_required"
    ) {
        return planned;
    }
    match installation_status {
        "active" if hook_observed_state == "observed" => PromptCaptureStatus::Observed.as_str(),
        "active" => PromptCaptureStatus::Configured.as_str(),
        "reload_required" => PromptCaptureStatus::ReloadRequired.as_str(),
        "configured" => PromptCaptureStatus::Configured.as_str(),
        "degraded" | "stale" | "broken" => PromptCaptureStatus::Degraded.as_str(),
        _ => PromptCaptureStatus::Unavailable.as_str(),
    }
}

fn guard_state_for_connection(
    runtime_home: &Path,
    connection_id: &str,
    projects: &[ConnectionProjectRecord],
) -> Result<GuardOperationalState, ConnectionCommandError> {
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

    let mut file_findings = GuardFileFindings::default();
    let mut prompt_capture_configured = false;
    let mut prompt_capture_host_supported = false;
    let mut prompt_capture_observed = false;
    let prompt_capture_disabled = installations
        .iter()
        .all(|installation| installation.guard_mode == IntegrationProfile::Record.as_str());
    let mut observed = false;
    let mut last_observed_at = None;
    for installation in &installations {
        let findings = guard_file_findings_for_installation(installation, projects);
        file_findings.merge(findings);
        if installation.last_seen_at.is_some() {
            observed = true;
            last_observed_at = max_optional_text(
                last_observed_at,
                installation.last_seen_at.as_deref().map(str::to_owned),
            );
        }
        if installation.last_seen_phase.as_deref() == Some("prompt_capture") {
            prompt_capture_observed = true;
        }
        if installation.guard_mode != IntegrationProfile::Record.as_str()
            && file_findings.prompt_capture_configured
        {
            prompt_capture_configured = true;
        }
        prompt_capture_host_supported |= file_findings.prompt_capture_host_supported;
    }
    file_findings.sort_dedup();
    let guard_profile_state = guard_profile_state_for_installations(&installations, &file_findings);
    let managed_source_state =
        managed_source_state_for_installations(&installations, &file_findings);
    let managed_bundle_hash = managed_bundle_hash_for_findings(&file_findings);
    let managed_verification_state =
        managed_verification_state_for_installations(&installations, &file_findings);
    let hook_path_safety_state = if prompt_capture_disabled {
        "not_applicable".to_owned()
    } else {
        file_findings.hook_path_safety_state()
    };
    let hook_commands_cwd_independent =
        all_recorded_values_true(&file_findings.hook_cwd_independent_values);
    let hook_commands_subdirectory_safe =
        all_recorded_values_true(&file_findings.hook_subdirectory_safe_values);
    let hook_path_safety_details = file_findings.hook_path_safety_details.clone();

    if !file_findings.broken_files.is_empty() {
        let mode_state = guard_mode_state(&installations);
        let installation_state = GuardInstallationStatus::Broken.as_str();
        let hook_observed_state = if prompt_capture_disabled {
            "disabled"
        } else if observed {
            "observed"
        } else {
            "not_observed"
        };
        let configuration_state = guard_configuration_state(
            installation_state,
            !file_findings.missing_required_hooks.is_empty(),
        );
        let observation_state = guard_observation_state(hook_observed_state);
        let effective_state =
            guard_effective_state(&mode_state, &configuration_state, &observation_state);
        let required_hooks_missing = !file_findings.missing_required_hooks.is_empty();
        return Ok(GuardOperationalState {
            mode_state: mode_state.clone(),
            guard_profile_state,
            installation_state: installation_state.to_owned(),
            configuration_state,
            observation_state,
            effective_state,
            generated_config_verified: false,
            native_host_output_adapter_verified: file_findings
                .native_host_output_adapter_verified(),
            hook_path_safety_state: hook_path_safety_state.clone(),
            hook_commands_cwd_independent,
            hook_commands_subdirectory_safe,
            hook_path_safety_details: hook_path_safety_details.clone(),
            bash_shell_mutation_coverage: file_findings.bash_shell_mutation_coverage(),
            direct_file_write_matcher_coverage: file_findings.direct_file_write_matcher_coverage(),
            files_state: "broken".to_owned(),
            managed_source_state,
            managed_bundle_hash,
            managed_verification_state,
            agents_block_state: file_findings
                .kind_state(HostIntegrationFileKind::AgentsManagedBlock)
                .to_owned(),
            policy_file_state: file_findings
                .kind_state(HostIntegrationFileKind::VolicordPolicy)
                .to_owned(),
            rule_instruction_state: file_findings.rule_instruction_state(prompt_capture_disabled),
            hook_config_state: file_findings.hook_config_state(prompt_capture_disabled),
            hook_observed_state: hook_observed_state.to_owned(),
            last_observed_at,
            last_guard_event_at: last_guard_event_for_projects(
                runtime_home,
                connection_id,
                projects,
            )?,
            prompt_capture_state: PromptCaptureStatus::Degraded.as_str().to_owned(),
            missing_files: file_findings.missing_files,
            stale_files: file_findings.stale_files,
            broken_files: file_findings.broken_files,
            missing_required_hooks: file_findings.missing_required_hooks,
            unresolved_blockers: guard_blockers_for_state(
                &mode_state,
                GuardInstallationStatus::Broken.as_str(),
                observed,
                required_hooks_missing,
            ),
        });
    }

    if !file_findings.missing_files.is_empty() {
        let mode_state = guard_mode_state(&installations);
        let installation_state = "files_missing";
        let hook_observed_state = if prompt_capture_disabled {
            "disabled"
        } else if observed {
            "observed"
        } else {
            "not_observed"
        };
        let configuration_state = guard_configuration_state(
            installation_state,
            !file_findings.missing_required_hooks.is_empty(),
        );
        let observation_state = guard_observation_state(hook_observed_state);
        let effective_state =
            guard_effective_state(&mode_state, &configuration_state, &observation_state);
        return Ok(GuardOperationalState {
            mode_state,
            guard_profile_state,
            installation_state: installation_state.to_owned(),
            configuration_state,
            observation_state,
            effective_state,
            generated_config_verified: false,
            native_host_output_adapter_verified: file_findings
                .native_host_output_adapter_verified(),
            hook_path_safety_state: hook_path_safety_state.clone(),
            hook_commands_cwd_independent,
            hook_commands_subdirectory_safe,
            hook_path_safety_details: hook_path_safety_details.clone(),
            bash_shell_mutation_coverage: file_findings.bash_shell_mutation_coverage(),
            direct_file_write_matcher_coverage: file_findings.direct_file_write_matcher_coverage(),
            files_state: "missing".to_owned(),
            managed_source_state,
            managed_bundle_hash,
            managed_verification_state,
            agents_block_state: file_findings
                .kind_state(HostIntegrationFileKind::AgentsManagedBlock)
                .to_owned(),
            policy_file_state: file_findings
                .kind_state(HostIntegrationFileKind::VolicordPolicy)
                .to_owned(),
            rule_instruction_state: file_findings.rule_instruction_state(prompt_capture_disabled),
            hook_config_state: file_findings.hook_config_state(prompt_capture_disabled),
            hook_observed_state: hook_observed_state.to_owned(),
            last_observed_at,
            last_guard_event_at: last_guard_event_for_projects(
                runtime_home,
                connection_id,
                projects,
            )?,
            prompt_capture_state: PromptCaptureStatus::NotConfigured.as_str().to_owned(),
            missing_files: file_findings.missing_files,
            stale_files: file_findings.stale_files,
            broken_files: file_findings.broken_files,
            missing_required_hooks: file_findings.missing_required_hooks,
            unresolved_blockers: vec!["guard_not_installed".to_owned()],
        });
    }

    if !file_findings.stale_files.is_empty() {
        let mode_state = guard_mode_state(&installations);
        let installation_state = GuardInstallationStatus::Stale.as_str();
        let hook_observed_state = if prompt_capture_disabled {
            "disabled"
        } else if observed {
            "observed"
        } else {
            "not_observed"
        };
        let configuration_state = guard_configuration_state(
            installation_state,
            !file_findings.missing_required_hooks.is_empty(),
        );
        let observation_state = guard_observation_state(hook_observed_state);
        let effective_state =
            guard_effective_state(&mode_state, &configuration_state, &observation_state);
        let required_hooks_missing = !file_findings.missing_required_hooks.is_empty();
        return Ok(GuardOperationalState {
            mode_state: mode_state.clone(),
            guard_profile_state,
            installation_state: installation_state.to_owned(),
            configuration_state,
            observation_state,
            effective_state,
            generated_config_verified: false,
            native_host_output_adapter_verified: file_findings
                .native_host_output_adapter_verified(),
            hook_path_safety_state: hook_path_safety_state.clone(),
            hook_commands_cwd_independent,
            hook_commands_subdirectory_safe,
            hook_path_safety_details: hook_path_safety_details.clone(),
            bash_shell_mutation_coverage: file_findings.bash_shell_mutation_coverage(),
            direct_file_write_matcher_coverage: file_findings.direct_file_write_matcher_coverage(),
            files_state: "stale".to_owned(),
            managed_source_state,
            managed_bundle_hash,
            managed_verification_state,
            agents_block_state: file_findings
                .kind_state(HostIntegrationFileKind::AgentsManagedBlock)
                .to_owned(),
            policy_file_state: file_findings
                .kind_state(HostIntegrationFileKind::VolicordPolicy)
                .to_owned(),
            rule_instruction_state: file_findings.rule_instruction_state(prompt_capture_disabled),
            hook_config_state: file_findings.hook_config_state(prompt_capture_disabled),
            hook_observed_state: hook_observed_state.to_owned(),
            last_observed_at,
            last_guard_event_at: last_guard_event_for_projects(
                runtime_home,
                connection_id,
                projects,
            )?,
            prompt_capture_state: PromptCaptureStatus::Degraded.as_str().to_owned(),
            missing_files: file_findings.missing_files,
            stale_files: file_findings.stale_files,
            broken_files: file_findings.broken_files,
            missing_required_hooks: file_findings.missing_required_hooks,
            unresolved_blockers: guard_blockers_for_state(
                &mode_state,
                GuardInstallationStatus::Stale.as_str(),
                observed,
                required_hooks_missing,
            ),
        });
    }

    let installation_state = if installations.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::Broken.as_str()
    }) {
        GuardInstallationStatus::Broken.as_str()
    } else if installations.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::Stale.as_str()
    }) {
        GuardInstallationStatus::Stale.as_str()
    } else if !file_findings.missing_required_hooks.is_empty() {
        GuardInstallationStatus::Degraded.as_str()
    } else if installations.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::ReloadRequired.as_str()
    }) {
        GuardInstallationStatus::ReloadRequired.as_str()
    } else if installations.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::Degraded.as_str()
    }) {
        GuardInstallationStatus::Degraded.as_str()
    } else if installations.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::Active.as_str()
    }) {
        GuardInstallationStatus::Active.as_str()
    } else if installations.iter().all(|installation| {
        installation.installation_status == GuardInstallationStatus::Configured.as_str()
    }) {
        GuardInstallationStatus::Configured.as_str()
    } else {
        installations[0].installation_status.as_str()
    };
    let prompt_capture_state = if prompt_capture_disabled {
        PromptCaptureStatus::NotConfigured.as_str()
    } else if !prompt_capture_host_supported {
        PromptCaptureStatus::UnsupportedByHost.as_str()
    } else if !prompt_capture_configured {
        PromptCaptureStatus::NotConfigured.as_str()
    } else if matches!(installation_state, "broken" | "stale" | "degraded") {
        PromptCaptureStatus::Degraded.as_str()
    } else if installation_state == GuardInstallationStatus::ReloadRequired.as_str() {
        PromptCaptureStatus::ReloadRequired.as_str()
    } else if installation_state == GuardInstallationStatus::Active.as_str()
        && prompt_capture_observed
    {
        PromptCaptureStatus::Active.as_str()
    } else if installation_state == GuardInstallationStatus::Active.as_str() && observed {
        PromptCaptureStatus::Observed.as_str()
    } else if installation_state == GuardInstallationStatus::Configured.as_str()
        || installation_state == GuardInstallationStatus::Active.as_str()
    {
        PromptCaptureStatus::Configured.as_str()
    } else {
        PromptCaptureStatus::Unavailable.as_str()
    };
    let mode_state = guard_mode_state(&installations);
    let hook_observed_state = if prompt_capture_disabled {
        "disabled"
    } else if observed {
        "observed"
    } else {
        "not_observed"
    };
    let configuration_state = guard_configuration_state(
        installation_state,
        !file_findings.missing_required_hooks.is_empty(),
    );
    let observation_state = guard_observation_state(hook_observed_state);
    let effective_state =
        guard_effective_state(&mode_state, &configuration_state, &observation_state);
    let required_hooks_missing = !file_findings.missing_required_hooks.is_empty();
    Ok(GuardOperationalState {
        mode_state: mode_state.clone(),
        guard_profile_state,
        installation_state: installation_state.to_owned(),
        configuration_state,
        observation_state,
        effective_state,
        generated_config_verified: file_findings.generated_config_verified(),
        native_host_output_adapter_verified: file_findings.native_host_output_adapter_verified(),
        hook_path_safety_state,
        hook_commands_cwd_independent,
        hook_commands_subdirectory_safe,
        hook_path_safety_details,
        bash_shell_mutation_coverage: file_findings.bash_shell_mutation_coverage(),
        direct_file_write_matcher_coverage: file_findings.direct_file_write_matcher_coverage(),
        files_state: if prompt_capture_disabled {
            "not_configured".to_owned()
        } else {
            "installed".to_owned()
        },
        managed_source_state,
        managed_bundle_hash,
        managed_verification_state,
        agents_block_state: file_findings
            .kind_state(HostIntegrationFileKind::AgentsManagedBlock)
            .to_owned(),
        policy_file_state: file_findings
            .kind_state(HostIntegrationFileKind::VolicordPolicy)
            .to_owned(),
        rule_instruction_state: file_findings.rule_instruction_state(prompt_capture_disabled),
        hook_config_state: file_findings.hook_config_state(prompt_capture_disabled),
        hook_observed_state: hook_observed_state.to_owned(),
        last_observed_at,
        last_guard_event_at: last_guard_event_for_projects(runtime_home, connection_id, projects)?,
        prompt_capture_state: prompt_capture_state.to_owned(),
        missing_files: file_findings.missing_files,
        stale_files: file_findings.stale_files,
        broken_files: file_findings.broken_files,
        missing_required_hooks: file_findings.missing_required_hooks,
        unresolved_blockers: guard_blockers_for_state(
            &mode_state,
            installation_state,
            observed,
            required_hooks_missing,
        ),
    })
}

fn guard_mode_state(installations: &[GuardInstallationRecord]) -> String {
    let mut modes = installations
        .iter()
        .map(|installation| installation.guard_mode.as_str())
        .collect::<Vec<_>>();
    modes.sort_unstable();
    modes.dedup();
    if modes.len() == 1 {
        modes[0].to_owned()
    } else {
        "mixed".to_owned()
    }
}

fn guard_profile_state_for_installations(
    installations: &[GuardInstallationRecord],
    findings: &GuardFileFindings,
) -> String {
    if let Some(value) = single_or_mixed(&findings.guard_profiles) {
        return value;
    }
    match guard_mode_state(installations).as_str() {
        "record" => "record",
        "detective" => "detective",
        _ => "mixed",
    }
    .to_owned()
}

fn managed_source_state_for_installations(
    installations: &[GuardInstallationRecord],
    findings: &GuardFileFindings,
) -> String {
    if let Some(value) = single_or_mixed(&findings.managed_sources) {
        return value;
    }
    match guard_profile_state_for_installations(installations, findings).as_str() {
        "record" => "not_applicable",
        "detective" => "host_hooks",
        "mixed" => "mixed",
        _ => "unknown",
    }
    .to_owned()
}

fn managed_bundle_hash_for_findings(findings: &GuardFileFindings) -> Option<String> {
    single_or_mixed(&findings.managed_bundle_hashes)
}

fn managed_verification_state_for_installations(
    installations: &[GuardInstallationRecord],
    findings: &GuardFileFindings,
) -> String {
    if let Some(value) = single_or_mixed(&findings.managed_verification_statuses) {
        return value;
    }
    match guard_profile_state_for_installations(installations, findings).as_str() {
        "record" | "detective" => "not_applicable",
        "mixed" => "mixed",
        _ => "unknown",
    }
    .to_owned()
}

fn single_or_mixed(values: &[String]) -> Option<String> {
    match values {
        [] => None,
        [value] => Some(value.clone()),
        _ => Some("mixed".to_owned()),
    }
}

fn guard_configuration_state(installation_state: &str, missing_required_hooks: bool) -> String {
    if missing_required_hooks
        && !matches!(
            installation_state,
            "not_configured" | "files_missing" | "stale" | "broken"
        )
    {
        return GuardInstallationStatus::Degraded.as_str().to_owned();
    }
    match installation_state {
        "not_configured" | "files_missing" => GuardInstallationStatus::Absent.as_str(),
        "active" | "configured" => GuardInstallationStatus::Configured.as_str(),
        "reload_required" => GuardInstallationStatus::ReloadRequired.as_str(),
        "degraded" => GuardInstallationStatus::Degraded.as_str(),
        "stale" => GuardInstallationStatus::Stale.as_str(),
        "broken" => GuardInstallationStatus::Broken.as_str(),
        other => other,
    }
    .to_owned()
}

fn guard_observation_state(hook_observed_state: &str) -> String {
    match hook_observed_state {
        "observed" => "observed",
        "disabled" => "not_observed",
        _ => "not_observed",
    }
    .to_owned()
}

fn guard_effective_state(
    guard_mode: &str,
    configuration_state: &str,
    observation_state: &str,
) -> String {
    if guard_mode == IntegrationProfile::Record.as_str() {
        return "inactive".to_owned();
    }
    match configuration_state {
        "absent" => "inactive",
        "broken" => "broken",
        "stale" | "degraded" => "degraded",
        "configured" if observation_state == "observed" => "active",
        "configured" | "reload_required" => "action_required",
        _ => "action_required",
    }
    .to_owned()
}

fn guard_blockers_for_state(
    guard_mode: &str,
    installation_state: &str,
    host_hook_observed: bool,
    required_hooks_missing: bool,
) -> Vec<String> {
    if guard_mode == IntegrationProfile::Record.as_str() {
        return Vec::new();
    }
    match installation_state {
        "not_configured" | "files_missing" => vec!["guard_not_installed".to_owned()],
        "reload_required" => vec!["guard_reload_required".to_owned()],
        "configured" => vec!["guard_not_observed".to_owned()],
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
            latest = max_optional_text(latest, Some(event.occurred_at));
        }
    }
    Ok(latest)
}

fn max_optional_text(current: Option<String>, candidate: Option<String>) -> Option<String> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.max(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

fn user_actions_json(
    actions: &[crate::host_integration::UserAction],
) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(actions)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
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

fn parse_metadata(text: &str) -> BTreeMap<String, String> {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| {
            value.as_object().map(|object| {
                object
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_owned()))
                    })
                    .collect()
            })
        })
        .unwrap_or_default()
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
        HostScope::User | HostScope::Local | HostScope::Export => Some(runtime_home),
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
        (HostScope::Project | HostScope::Local, Some(project_id)) => {
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
mod tests {
    use std::{
        ffi::OsString,
        process::{Command, Stdio},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use volicord_store::agent_connections::VERIFIED_STATUS_COMPLETE;
    use volicord_types::RECONCILE_CHANGES_TOOL_NAME;

    fn plan_guard_integration_for_test(
        host_kind: HostKind,
        init_mode: InitMode,
        repo_root: &Path,
        connection_id: &str,
        guard_installation_id: &str,
        mcp_entry: &ManagedServerEntry,
    ) -> Result<GuardIntegrationPlan, ConnectionCommandError> {
        let runtime_home = repo_root.with_file_name(format!(
            "{}-runtime-home",
            repo_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("volicord-test")
        ));
        plan_guard_integration(
            host_kind,
            init_mode,
            &runtime_home,
            repo_root,
            connection_id,
            guard_installation_id,
            mcp_entry,
        )
    }

    #[test]
    fn generated_connection_id_includes_connection_unit_inputs() {
        let first = deterministic_connection_id(
            HostKind::Codex,
            HostScope::Project,
            Some("project_a"),
            "/repo/.codex/config.toml",
            "volicord",
        );
        let second = deterministic_connection_id(
            HostKind::Codex,
            HostScope::Project,
            Some("project_b"),
            "/repo/.codex/config.toml",
            "volicord",
        );

        assert!(first.starts_with("conn_codex_project_project_a_"));
        assert_ne!(first, second);
    }

    #[test]
    fn public_connection_mode_parses_user_labels() {
        assert_eq!(
            parse_user_connection_mode("read-only").unwrap(),
            CONNECTION_MODE_READ_ONLY
        );
        assert_eq!(
            parse_user_connection_mode("workflow").unwrap(),
            CONNECTION_MODE_WORKFLOW
        );
        assert!(parse_user_connection_mode("read_only").is_err());
    }

    #[test]
    fn host_scope_mapping_uses_connection_intent_support_matrix() {
        assert_eq!(
            host_scope_for_intent(HostKind::Codex, ConnectionIntent::Personal).unwrap(),
            HostScope::User
        );
        assert_eq!(
            host_scope_for_intent(HostKind::Codex, ConnectionIntent::Shared).unwrap(),
            HostScope::Project
        );
        assert_eq!(
            host_scope_for_intent(HostKind::ClaudeCode, ConnectionIntent::Global).unwrap(),
            HostScope::User
        );

        let error = host_scope_for_intent(HostKind::Codex, ConnectionIntent::Global).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("codex does not support --global"));
        assert!(message.contains("supported connection intents: personal, shared"));
    }

    #[test]
    fn mcp_tool_validation_matches_public_connection_modes() {
        let workflow_tools = mcp_process::workflow_required_tool_names()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert!(
            mcp_process::validate_tools_for_mode(CONNECTION_MODE_WORKFLOW, &workflow_tools).is_ok()
        );
        assert!(workflow_tools
            .iter()
            .any(|tool| tool == RECONCILE_CHANGES_TOOL_NAME));

        let read_only_tools = mcp_process::read_only_required_tool_names()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert!(
            mcp_process::validate_tools_for_mode(CONNECTION_MODE_READ_ONLY, &read_only_tools)
                .is_ok()
        );
        assert!(!read_only_tools
            .iter()
            .any(|tool| tool == RECONCILE_CHANGES_TOOL_NAME));

        let missing_reconcile_workflow_tools = mcp_process::workflow_required_tool_names()
            .filter(|tool| *tool != RECONCILE_CHANGES_TOOL_NAME)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let error = mcp_process::validate_tools_for_mode(
            CONNECTION_MODE_WORKFLOW,
            &missing_reconcile_workflow_tools,
        )
        .unwrap_err();
        assert!(error.contains(RECONCILE_CHANGES_TOOL_NAME));

        let stale_read_only_tools = vec![
            "volicord.status".to_owned(),
            "volicord.close_task".to_owned(),
            "volicord.list_projects".to_owned(),
        ];
        let error =
            mcp_process::validate_tools_for_mode(CONNECTION_MODE_READ_ONLY, &stale_read_only_tools)
                .unwrap_err();
        assert!(error.contains("volicord.check_close"));
    }

    #[test]
    fn detective_integration_plan_rejects_missing_generic_hooks(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("guard-capabilities-reject")?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let error = plan_guard_integration_for_test(
            HostKind::Generic,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("default detective init should reject missing host hook support");

        assert!(error.to_string().contains("DETECTIVE_HOOKS_UNSUPPORTED"));
        assert!(error.to_string().contains("--profile record"));
        assert!(error.to_string().contains("supported host"));
        assert!(error.to_string().contains("AGENTS.md"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn detective_profile_is_rejected_on_native_windows() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("windows-detective-reject")?;
        fs::create_dir_all(repo.join(".git"))?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);

        let error = plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("native Windows detective init should fail before planning hook files");

        assert!(error.to_string().contains("DETECTIVE_WINDOWS_UNSUPPORTED"));
        assert!(error.to_string().contains("--profile record"));
        assert!(error.to_string().contains("WSL2"));
        assert!(!repo.join(".codex/hooks.json").exists());
        assert!(!repo.join(VOLICORD_POLICY_FILE).exists());
        Ok(())
    }

    #[test]
    fn codex_guarded_integration_rejects_non_git_root() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex non git")?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let error = plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("Codex detective host hooks should require a Git root strategy");

        assert!(error
            .to_string()
            .contains("DETECTIVE_HOOK_ROOT_UNSUPPORTED"));
        assert!(error.to_string().contains("Git work tree root"));
        assert!(error.to_string().contains("--profile record"));
        assert!(error.to_string().contains("supported host"));
        assert!(!repo.join(".codex/hooks.json").exists());
        assert!(!repo.join(VOLICORD_POLICY_FILE).exists());

        Ok(())
    }

    #[test]
    fn codex_guarded_integration_plan_generates_required_hook_files(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("guard capabilities")?;
        fs::create_dir_all(repo.join(".git"))?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let plan = plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?;
        let applied = apply_guard_integration(plan)?;

        assert!(applied.missing_required_hooks.is_empty());
        assert_eq!(
            initial_guard_installation_status(
                InitMode::Detective,
                &host_plan_stub(&entry),
                &applied
            ),
            GuardInstallationStatus::ReloadRequired
        );
        let capability: Value = serde_json::from_str(&host_hook_capability_json(&applied)?)?;
        assert_eq!(capability["prompt_capture"], true);
        assert_eq!(capability["selected_profile"], "detective");
        assert_eq!(capability["native_host_output_adapter"], "codex");
        assert_eq!(capability["native_host_output_adapter_verified"], true);
        assert_eq!(capability["bash_shell_mutation_coverage"], true);
        assert_eq!(capability["direct_file_write_matcher_coverage"], true);
        assert_eq!(capability["hook_root_resolution"]["basis"], "git_work_tree");
        assert_eq!(
            capability["hook_root_resolution"]["all_cwd_independent"],
            true
        );
        assert_eq!(capability["hook_path_safety"]["overall_status"], "ok");
        assert_eq!(capability["hook_path_safety"]["all_cwd_independent"], true);
        assert_eq!(
            capability["hook_path_safety"]["all_subdirectory_safe"],
            true
        );
        assert_eq!(
            capability["host_hook_commands"]
                .as_array()
                .expect("host hook commands should be recorded")
                .len(),
            REQUIRED_GUARD_PHASES.len()
        );
        let pre_tool_command = capability["host_hook_commands"]
            .as_array()
            .expect("host hook commands should be recorded")
            .iter()
            .find(|command| command["phase"] == "pre_tool_hook")
            .expect("pre-tool command should be recorded");
        assert_eq!(
            pre_tool_command["hook_command_path_basis"],
            "git_root_runtime"
        );
        assert_eq!(pre_tool_command["cwd_independent"], true);
        assert_eq!(pre_tool_command["subdirectory_safe"], true);
        assert_eq!(pre_tool_command["wrapper_resolution_status"], "ok");
        assert_eq!(
            capability["missing_required_hooks"]
                .as_array()
                .expect("missing hooks should be an array")
                .len(),
            0
        );
        let generated_files = generated_files_json(&applied.generated_files);
        let generated_files = generated_files
            .as_array()
            .expect("generated files should be an array");
        assert!(generated_files
            .iter()
            .any(|file| file["kind"] == "host_hook_config"));
        assert_eq!(
            generated_files
                .iter()
                .filter(|file| file["kind"] == "host_hook_dispatch")
                .count(),
            1
        );
        assert_eq!(
            generated_files
                .iter()
                .filter(|file| file["kind"] == "host_hook_wrapper")
                .count(),
            REQUIRED_GUARD_PHASES.len()
        );
        assert!(generated_files
            .iter()
            .any(|file| file["kind"] == "host_rule_instruction"));
        let hooks_text = fs::read_to_string(repo.join(".codex/hooks.json"))?;
        assert!(!hooks_text.contains("\"command\": \".codex/hooks/"));
        assert!(hooks_text.contains("sh -c"));
        assert!(hooks_text.contains("git rev-parse --show-toplevel"));
        assert!(hooks_text.contains(".codex/hooks/volicord-dispatch.sh"));
        assert!(hooks_text.contains("session-start"));
        assert!(hooks_text.contains("pre-tool"));
        assert!(hooks_text.contains("post-tool"));
        assert!(hooks_text.contains("prompt-capture"));
        assert!(hooks_text.contains("stop"));
        assert!(!hooks_text.contains("volicord _hook "));
        assert!(hooks_text.contains(
            "Bash|apply_patch|Edit|Write|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*"
        ));
        assert!(!hooks_text.contains("--json"));
        let dispatch_wrapper_path = repo.join(".codex/hooks/volicord-dispatch.sh");
        let dispatch_wrapper = fs::read_to_string(&dispatch_wrapper_path)?;
        assert!(dispatch_wrapper.contains(HOOK_WRAPPER_MARKER));
        assert!(dispatch_wrapper.contains("phase=dispatch"));
        assert!(dispatch_wrapper.contains("git rev-parse --show-toplevel"));
        assert!(dispatch_wrapper.contains(".codex/hooks/volicord-$phase.sh"));
        assert!(dispatch_wrapper.contains("exec \"$wrapper\""));
        assert!(script_is_executable(&dispatch_wrapper_path));
        let pre_tool_wrapper_path = repo.join(".codex/hooks/volicord-pre-tool.sh");
        let pre_tool_wrapper = fs::read_to_string(&pre_tool_wrapper_path)?;
        assert!(pre_tool_wrapper.contains(HOOK_WRAPPER_MARKER));
        assert!(pre_tool_wrapper.contains("exec volicord _hook pre-tool"));
        assert!(pre_tool_wrapper.contains(&format!("--repo {}", shell_word(&path_text(&repo)))));
        assert!(pre_tool_wrapper.contains("--connection conn_alpha"));
        assert!(pre_tool_wrapper.contains("--guard-installation guard_installation_alpha"));
        assert!(pre_tool_wrapper.contains("--host codex"));
        assert!(pre_tool_wrapper.contains("--policy-hash"));
        assert!(pre_tool_wrapper.contains(
            capability["policy_hash"]
                .as_str()
                .expect("capability should include policy hash")
        ));
        assert!(pre_tool_wrapper.contains("--host-output codex"));
        assert!(script_is_executable(&pre_tool_wrapper_path));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn codex_dispatch_executes_from_subdirectory_and_preserves_host_protocol(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write as _;

        let repo = temp_dir("codex dispatch repo spaces")?;
        init_real_git_repo(&repo)?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        apply_guard_integration(plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        let bin_dir = repo.join("fake bin");
        write_fake_guard_volicord(&bin_dir)?;
        let subdir = repo.join("nested dir").join("inner");
        fs::create_dir_all(&subdir)?;
        let hooks: Value =
            serde_json::from_str(&fs::read_to_string(repo.join(".codex/hooks.json"))?)?;
        let command = hooks["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .expect("PreToolUse command should be present");
        assert!(command.contains(CODEX_DISPATCH_WRAPPER));
        assert!(command.contains("pre-tool"));

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&subdir)
            .env("PATH", path_with_prefix(&bin_dir)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().expect("stdin should be piped");
        stdin.write_all(b"payload via stdin")?;
        drop(stdin);
        let output = child.wait_with_output()?;

        assert_eq!(output.status.code(), Some(37));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "stdout:payload via stdin\n"
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "stderr:guard reached\n"
        );

        let invalid = Command::new(repo.join(CODEX_DISPATCH_WRAPPER))
            .arg("bad-phase")
            .current_dir(&subdir)
            .output()?;
        assert!(!invalid.status.success());
        assert_eq!(String::from_utf8_lossy(&invalid.stdout), "");
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("unsupported host-hook phase"));
        Ok(())
    }

    #[test]
    fn claude_guarded_integration_generates_hooks_mcp_and_rules(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-guarded")?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let plan = plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?;
        let applied = apply_guard_integration(plan)?;

        assert!(applied.missing_required_hooks.is_empty());
        let capability: Value = serde_json::from_str(&host_hook_capability_json(&applied)?)?;
        assert_eq!(capability["prompt_capture"], true);
        assert_eq!(
            capability["hook_root_resolution"]["basis"],
            "claude_project_dir"
        );
        assert_eq!(
            capability["hook_root_resolution"]["all_cwd_independent"],
            true
        );
        assert_eq!(capability["hook_path_safety"]["overall_status"], "ok");
        assert_eq!(capability["hook_path_safety"]["all_cwd_independent"], true);
        assert_eq!(
            capability["hook_path_safety"]["all_subdirectory_safe"],
            true
        );
        let pre_tool_command = capability["host_hook_commands"]
            .as_array()
            .expect("host hook commands should be recorded")
            .iter()
            .find(|command| command["phase"] == "pre_tool_hook")
            .expect("pre-tool command should be recorded");
        assert_eq!(
            pre_tool_command["hook_command_path_basis"],
            "claude_project_dir"
        );
        assert_eq!(pre_tool_command["cwd_independent"], true);
        assert_eq!(pre_tool_command["subdirectory_safe"], true);
        assert_eq!(pre_tool_command["wrapper_resolution_status"], "ok");
        assert_eq!(
            capability["missing_required_hooks"]
                .as_array()
                .expect("missing hooks should be an array")
                .len(),
            0
        );
        let generated_files = generated_files_json(&applied.generated_files);
        let generated_files = generated_files
            .as_array()
            .expect("generated files should be an array");
        assert!(generated_files
            .iter()
            .any(|file| file["kind"] == "host_mcp_config"));
        assert!(generated_files
            .iter()
            .any(|file| file["kind"] == "host_hook_config"));
        assert_eq!(
            generated_files
                .iter()
                .filter(|file| file["kind"] == "host_hook_wrapper")
                .count(),
            REQUIRED_GUARD_PHASES.len()
        );
        assert!(generated_files.iter().any(|file| {
            file["kind"] == "host_hook_config"
                && file["ownership"] == "managed_json_projection"
                && file["managed_projection"] == "claude_code_settings_hooks"
        }));

        let mcp_text = fs::read_to_string(repo.join(".mcp.json"))?;
        assert!(mcp_text.contains("\"volicord\""));
        assert!(mcp_text.contains("\"mcp\""));
        assert!(mcp_text.contains("\"--stdio\""));
        assert!(mcp_text.contains("\"--connection\""));
        let settings_text = fs::read_to_string(repo.join(".claude/settings.json"))?;
        for command in [
            "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-session-start.sh",
            "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh",
            "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-post-tool.sh",
            "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-prompt-capture.sh",
            "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-stop.sh",
        ] {
            assert!(settings_text.contains(command), "missing {command}");
        }
        assert!(!settings_text.contains("\"command\": \".claude/hooks/"));
        assert!(settings_text.contains("\"args\": []"));
        assert!(!settings_text.contains("volicord _hook "));
        let pre_tool_wrapper_path = repo.join(".claude/hooks/volicord-pre-tool.sh");
        let pre_tool_wrapper = fs::read_to_string(&pre_tool_wrapper_path)?;
        assert!(pre_tool_wrapper.contains(HOOK_WRAPPER_MARKER));
        assert!(pre_tool_wrapper.contains("exec volicord _hook pre-tool"));
        assert!(pre_tool_wrapper.contains("--host claude-code"));
        assert!(pre_tool_wrapper.contains("--host-output claude-code"));
        assert!(pre_tool_wrapper.contains("--guard-installation guard_installation_alpha"));
        assert!(pre_tool_wrapper.contains("--policy-hash"));
        assert!(pre_tool_wrapper.contains(
            capability["policy_hash"]
                .as_str()
                .expect("capability should include policy hash")
        ));
        assert!(script_is_executable(&pre_tool_wrapper_path));
        assert!(settings_text.contains(
            "\"matcher\": \"Bash|Edit|Write|MultiEdit|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*\""
        ));
        assert!(fs::read_to_string(repo.join(".claude/rules/volicord.md"))?
            .contains("Configured local detective host-hook commands"));

        let again = plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?;
        let applied_again = apply_guard_integration(again)?;
        let settings_again = fs::read_to_string(repo.join(".claude/settings.json"))?;
        assert_eq!(settings_text, settings_again);
        assert_eq!(
            settings_again
                .matches("${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-")
                .count(),
            REQUIRED_GUARD_PHASES.len()
        );
        assert!(applied_again
            .generated_files
            .iter()
            .any(|file| file.kind == HostIntegrationFileKind::HostHookConfig
                && file.status == FilePlanStatus::Unchanged));
        assert_eq!(
            applied_again
                .generated_files
                .iter()
                .filter(|file| file.kind == HostIntegrationFileKind::HostHookWrapper
                    && file.status == FilePlanStatus::Unchanged)
                .count(),
            REQUIRED_GUARD_PHASES.len()
        );
        Ok(())
    }

    #[test]
    fn claude_settings_merge_preserves_unmanaged_hooks_and_keys(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-settings-preserve")?;
        fs::create_dir_all(repo.join(".claude"))?;
        fs::write(
            repo.join(".claude/settings.json"),
            r#"{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "theme": "dark",
  "permissions": {
    "ask": ["Bash"]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "echo keep",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
"#,
        )?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let applied = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        let settings: Value =
            serde_json::from_str(&fs::read_to_string(repo.join(".claude/settings.json"))?)?;

        assert_eq!(settings["theme"], "dark");
        assert_eq!(settings["permissions"]["ask"][0], "Bash");
        let pre_tool = settings["hooks"]["PreToolUse"]
            .as_array()
            .expect("PreToolUse should be an array");
        assert!(pre_tool.iter().any(|group| group["matcher"] == "Bash"));
        assert!(pre_tool.iter().any(|group| {
            group["matcher"]
                == "Bash|Edit|Write|MultiEdit|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*"
                && group["hooks"][0]["command"]
                    .as_str()
                    .is_some_and(|command| command
                        == "${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh")
                && group["hooks"][0]["args"]
                    .as_array()
                    .is_some_and(|args| args.is_empty())
        }));

        let capability_json = host_hook_capability_json(&applied)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.is_empty());
        assert!(findings.broken_files.is_empty());
        Ok(())
    }

    #[test]
    fn claude_settings_conflicting_managed_entry_is_rejected(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-settings-conflict")?;
        fs::create_dir_all(repo.join(".claude"))?;
        fs::write(
            repo.join(".claude/settings.json"),
            r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "volicord _hook pre-tool --host claude-code --host-output claude-code",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
"#,
        )?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let error = plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("conflicting managed hook should be rejected");

        assert!(error.to_string().contains("conflicting Volicord-managed"));
        Ok(())
    }

    #[test]
    fn guarded_integration_rejects_unmanaged_hook_wrapper() -> Result<(), Box<dyn std::error::Error>>
    {
        let repo = temp_dir("hook-wrapper-conflict")?;
        fs::create_dir_all(repo.join(".git"))?;
        let wrapper_path = repo.join(".codex/hooks/volicord-pre-tool.sh");
        fs::create_dir_all(wrapper_path.parent().expect("wrapper should have parent"))?;
        fs::write(&wrapper_path, "#!/bin/sh\nexec echo user-owned\n")?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);

        let error = plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("unmanaged hook wrapper should be rejected");

        assert!(error
            .to_string()
            .contains("host_hook_wrapper already exists with unmanaged content"));
        assert!(error.to_string().contains(&path_text(&wrapper_path)));
        Ok(())
    }

    #[test]
    fn guarded_integration_rejects_unmanaged_codex_dispatch_wrapper(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("hook-dispatch-conflict")?;
        fs::create_dir_all(repo.join(".git"))?;
        let dispatch_path = repo.join(".codex/hooks/volicord-dispatch.sh");
        fs::create_dir_all(dispatch_path.parent().expect("dispatch should have parent"))?;
        fs::write(&dispatch_path, "#!/bin/sh\nexec echo user-owned\n")?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);

        let error = plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("unmanaged dispatch wrapper should be rejected");

        assert!(error
            .to_string()
            .contains("host_hook_dispatch already exists with unmanaged content"));
        assert!(error.to_string().contains(&path_text(&dispatch_path)));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn guarded_integration_rerun_repairs_hook_wrapper_executable_bit(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let repo = temp_dir("hook-wrapper-executable-repair")?;
        fs::create_dir_all(repo.join(".git"))?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let applied = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        let wrapper_path = repo.join(".codex/hooks/volicord-pre-tool.sh");
        let dispatch_path = repo.join(".codex/hooks/volicord-dispatch.sh");
        assert!(script_is_executable(&wrapper_path));
        assert!(script_is_executable(&dispatch_path));
        let capability_json = host_hook_capability_json(&applied)?;

        let mut permissions = fs::metadata(&wrapper_path)?.permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&wrapper_path, permissions)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.contains(&path_text(&wrapper_path)));

        let mut permissions = fs::metadata(&dispatch_path)?.permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&dispatch_path, permissions)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.contains(&path_text(&dispatch_path)));

        let repaired = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        assert!(script_is_executable(&wrapper_path));
        assert!(script_is_executable(&dispatch_path));
        assert!(repaired.generated_files.iter().any(|file| {
            file.kind == HostIntegrationFileKind::HostHookWrapper
                && file.path == wrapper_path
                && file.status == FilePlanStatus::Updated
        }));
        assert!(repaired.generated_files.iter().any(|file| {
            file.kind == HostIntegrationFileKind::HostHookDispatch
                && file.path == dispatch_path
                && file.status == FilePlanStatus::Updated
        }));
        Ok(())
    }

    #[test]
    fn claude_settings_merge_rejects_invalid_preserved_settings(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-settings-invalid")?;
        fs::create_dir_all(repo.join(".claude"))?;
        fs::write(
            repo.join(".claude/settings.json"),
            r#"{
  "permissions": []
}
"#,
        )?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let error = plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )
        .expect_err("invalid preserved settings should be rejected");

        assert!(error
            .to_string()
            .contains("merged Claude Code project settings do not match the verified contract"));
        Ok(())
    }

    #[test]
    fn claude_guard_file_verification_ignores_unmanaged_settings_changes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-guard-file-verify")?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let applied = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        let capability_json = host_hook_capability_json(&applied)?;

        let settings_path = repo.join(".claude/settings.json");
        let mut settings: Value = serde_json::from_str(&fs::read_to_string(&settings_path)?)?;
        settings["theme"] = Value::String("light".to_owned());
        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.is_empty());

        settings["hooks"]["PreToolUse"] = Value::Array(Vec::new());
        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.contains(&path_text(&settings_path)));
        Ok(())
    }

    #[test]
    fn guard_file_verification_detects_stale_policy_and_duplicate_markers(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("guard-file-verify")?;
        fs::create_dir_all(repo.join(".git"))?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let plan = plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?;
        let applied = apply_guard_integration(plan)?;
        let capability_json = host_hook_capability_json(&applied)?;

        let findings = guard_file_findings(&capability_json);
        assert!(findings.missing_files.is_empty());
        assert!(findings.stale_files.is_empty());
        assert!(findings.broken_files.is_empty());
        assert!(findings.missing_required_hooks.is_empty());

        let wrapper_path = repo.join(".codex/hooks/volicord-pre-tool.sh");
        let wrapper_text = fs::read_to_string(&wrapper_path)?;
        fs::write(
            &wrapper_path,
            wrapper_text.replace("--host-output codex", "--host-output claude-code"),
        )?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.contains(&path_text(&wrapper_path)));
        assert_eq!(
            findings.kind_state(HostIntegrationFileKind::HostHookWrapper),
            "stale"
        );

        fs::remove_file(&wrapper_path)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.missing_files.contains(&path_text(&wrapper_path)));
        assert_eq!(findings.hook_config_state(false), "missing");

        fs::write(&wrapper_path, &wrapper_text)?;
        set_script_executable(&wrapper_path)?;
        let dispatch_path = repo.join(".codex/hooks/volicord-dispatch.sh");
        let dispatch_text = fs::read_to_string(&dispatch_path)?;
        fs::remove_file(&dispatch_path)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.missing_files.contains(&path_text(&dispatch_path)));
        assert_eq!(findings.hook_config_state(false), "missing");

        fs::write(&dispatch_path, &dispatch_text)?;
        set_script_executable(&dispatch_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&wrapper_path)?.permissions();
            permissions.set_mode(0o644);
            fs::set_permissions(&wrapper_path, permissions)?;
            let findings = guard_file_findings(&capability_json);
            assert!(findings.stale_files.contains(&path_text(&wrapper_path)));
        }

        let policy_path = repo.join(VOLICORD_POLICY_FILE);
        let policy_text = fs::read_to_string(&policy_path)?;
        fs::write(
            &policy_path,
            policy_text.replace("conn_alpha", "conn_changed"),
        )?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.stale_files.contains(&path_text(&policy_path)));

        let hooks_path = repo.join(".codex/hooks.json");
        fs::write(&hooks_path, r#"{"hooks":{"SessionStart":[]}}"#)?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings.broken_files.contains(&path_text(&hooks_path)));

        fs::write(
            repo.join(AGENTS_FILE),
            format!(
                "{GUIDANCE_START_MARKER}\nfirst\n{GUIDANCE_END_MARKER}\n{GUIDANCE_START_MARKER}\nsecond\n{GUIDANCE_END_MARKER}\n"
            ),
        )?;
        let findings = guard_file_findings(&capability_json);
        assert!(findings
            .broken_files
            .contains(&path_text(&repo.join(AGENTS_FILE))));
        Ok(())
    }

    #[test]
    fn guard_file_verification_reports_hook_path_safety_failures(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("hook-path-safety-verify")?;
        fs::create_dir_all(repo.join(".git"))?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let applied = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        let capability_json = host_hook_capability_json(&applied)?;
        let capability: Value = serde_json::from_str(&capability_json)?;
        let findings = guard_file_findings(&capability_json);
        assert_eq!(findings.hook_path_safety_state(), "ok");
        assert!(findings.generated_config_verified());

        let hooks_path = repo.join(".codex/hooks.json");
        let hooks_text = fs::read_to_string(&hooks_path)?;
        let mut hooks_json: Value = serde_json::from_str(&hooks_text)?;
        hooks_json["hooks"]["SessionStart"][0]["hooks"][0]["command"] =
            json!(".codex/hooks/volicord-dispatch.sh session-start");
        fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_json)?)?;
        let findings = guard_file_findings(&capability_json);
        assert_eq!(findings.hook_path_safety_state(), "relative_path_unsafe");
        assert!(hook_path_status_recorded(&findings, "relative_path_unsafe"));
        assert!(findings.stale_files.contains(&path_text(&hooks_path)));
        assert!(!findings.generated_config_verified());
        fs::write(&hooks_path, hooks_text)?;

        let mut stale_absolute_capability = capability.clone();
        let stale_command = format!(
            "sh -c 'exec \"{}\" session-start'",
            path_text(&repo.join("stale-root/.codex/hooks/volicord-dispatch.sh"))
        );
        stale_absolute_capability["host_hook_commands"][0]["command"] = json!(stale_command);
        let findings = guard_file_findings(&stale_absolute_capability.to_string());
        assert_eq!(findings.hook_path_safety_state(), "absolute_path_stale");
        assert!(hook_path_status_recorded(&findings, "absolute_path_stale"));
        assert!(!findings.generated_config_verified());

        let wrapper_path = repo.join(".codex/hooks/volicord-pre-tool.sh");
        let wrapper_text = fs::read_to_string(&wrapper_path)?;
        fs::remove_file(&wrapper_path)?;
        let findings = guard_file_findings(&capability_json);
        assert_eq!(findings.hook_path_safety_state(), "wrapper_missing");
        assert!(hook_path_status_recorded(&findings, "wrapper_missing"));
        assert!(findings.missing_files.contains(&path_text(&wrapper_path)));
        fs::write(&wrapper_path, &wrapper_text)?;
        set_script_executable(&wrapper_path)?;

        fs::write(
            &wrapper_path,
            wrapper_text.replace("# host_output=codex", "# host_output=claude-code"),
        )?;
        set_script_executable(&wrapper_path)?;
        let findings = guard_file_findings(&capability_json);
        assert_eq!(findings.hook_path_safety_state(), "host_output_mismatch");
        assert!(hook_path_status_recorded(&findings, "host_output_mismatch"));
        assert!(findings.stale_files.contains(&path_text(&wrapper_path)));
        Ok(())
    }

    #[test]
    fn claude_guard_state_becomes_active_after_synthetic_observation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let runtime_home = temp_dir("claude-guard-runtime")?;
        let repo = temp_dir("claude-guard-detective")?;
        fs::create_dir_all(repo.join(".git"))?;
        initialize_runtime_home(&runtime_home, "runtime_home_test", "{}")?;
        let project = ensure_project_for_repo(
            &runtime_home,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let entry = ManagedServerEntry::new("conn_alpha", Path::new("volicord"), None);
        let integration = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::ClaudeCode,
            InitMode::Detective,
            &repo,
            "conn_alpha",
            "guard_installation_alpha",
            &entry,
        )?)?;
        ensure_agent_connection(
            &runtime_home,
            AgentConnectionRegistration {
                connection_internal_id: "conn_alpha".to_owned(),
                host_kind: HostKind::ClaudeCode.as_str().to_owned(),
                intent: ConnectionIntent::Shared.as_str().to_owned(),
                host_scope: HostScope::Project.as_str().to_owned(),
                server_name: DEFAULT_SERVER_NAME.to_owned(),
                config_target: path_text(&repo.join(".mcp.json")),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: "fingerprint".to_owned(),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_connection_project(
            &runtime_home,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_alpha".to_owned(),
                project_id: project.project_id.clone(),
            },
        )?;
        upsert_guard_installation(
            &runtime_home,
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_alpha".to_owned(),
                connection_internal_id: "conn_alpha".to_owned(),
                project_id: Some(project.project_id.clone()),
                host_kind: HostKind::ClaudeCode.as_str().to_owned(),
                guard_mode: IntegrationProfile::Detective.as_str().to_owned(),
                host_capability_json: host_hook_capability_json(&integration)?,
                installation_status: GuardInstallationStatus::ReloadRequired.as_str().to_owned(),
                installed_at: Some("2026-07-01T00:00:00Z".to_owned()),
                last_checked_at: "2026-07-01T00:00:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        volicord_store::guards::observe_guard_installation(
            &runtime_home,
            volicord_store::guards::GuardInstallationObservation {
                guard_installation_id: "guard_installation_alpha".to_owned(),
                connection_internal_id: "conn_alpha".to_owned(),
                project_id: project.project_id.clone(),
                host_kind: HostKind::ClaudeCode.as_str().to_owned(),
                guard_mode: IntegrationProfile::Detective.as_str().to_owned(),
                observed_policy_hash: integration.policy_hash.clone(),
                observed_binary_version: Some("test".to_owned()),
                observed_phase: "session_start".to_owned(),
                observed_at: "2026-07-01T00:01:00Z".to_owned(),
            },
        )?;
        let projects = list_connection_projects(&runtime_home, "conn_alpha")?;
        let guard_state = guard_state_for_connection(&runtime_home, "conn_alpha", &projects)?;

        assert_eq!(guard_state.installation_state, "active");
        assert_eq!(guard_state.hook_observed_state, "observed");
        assert_eq!(guard_state.effective_state, "active");
        assert_eq!(guard_state.selected_profile(), "detective");
        let control_surface = guard_state.control_surface_json();
        assert_eq!(control_surface["selected_profile"], "detective");
        assert_eq!(control_surface["host_hooks_active"], true);
        assert_eq!(
            control_surface["cooperative_pre_tool_warning_available"],
            true
        );
        assert_eq!(
            control_surface["cooperative_pre_tool_denial_available"],
            true
        );
        assert_eq!(control_surface["actor_identity_provable"], false);
        assert_eq!(control_surface["os_enforced"], false);
        assert!(guard_state.cooperative_pre_tool_warning_available());
        assert!(guard_state.cooperative_pre_tool_denial_available());
        assert!(guard_state.post_tool_correlation_available());
        assert!(guard_state.generated_config_verified);
        assert!(guard_state.native_host_output_adapter_verified);
        assert!(guard_state.bash_shell_mutation_coverage);
        assert!(guard_state.direct_file_write_matcher_coverage);
        assert!(!guard_state.bypass_detection_active());
        assert_eq!(guard_state.managed_source_state, "host_hooks");
        assert_eq!(guard_state.managed_bundle_hash, None);
        assert_eq!(guard_state.managed_verification_state, "not_applicable");
        assert_eq!(guard_state.prompt_capture_state, "observed");
        Ok(())
    }

    #[test]
    fn guard_state_downgrades_when_required_shell_matcher_is_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let runtime_home = temp_dir("codex-missing-shell-matcher-runtime")?;
        let repo = temp_dir("codex-missing-shell-matcher-repo")?;
        fs::create_dir_all(repo.join(".git"))?;
        initialize_runtime_home(&runtime_home, "runtime_home_test", "{}")?;
        let project = ensure_project_for_repo(
            &runtime_home,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let entry = ManagedServerEntry::new("conn_codex_missing_bash", Path::new("volicord"), None);
        let integration = apply_guard_integration(plan_guard_integration_for_test(
            HostKind::Codex,
            InitMode::Detective,
            &repo,
            "conn_codex_missing_bash",
            "guard_installation_missing_bash",
            &entry,
        )?)?;

        let hooks_path = repo.join(".codex/hooks.json");
        let hooks_without_bash = fs::read_to_string(&hooks_path)?.replace("Bash|", "");
        fs::write(&hooks_path, &hooks_without_bash)?;
        let mut capability: Value =
            serde_json::from_str(&host_hook_capability_json(&integration)?)?;
        let hook_file = capability["files"]
            .as_array_mut()
            .and_then(|files| {
                files
                    .iter_mut()
                    .find(|file| file["kind"] == HostIntegrationFileKind::HostHookConfig.as_str())
            })
            .expect("capability should record hook config file");
        hook_file["content_hash"] = Value::String(sha256_text(&hooks_without_bash));

        ensure_agent_connection(
            &runtime_home,
            AgentConnectionRegistration {
                connection_internal_id: "conn_codex_missing_bash".to_owned(),
                host_kind: HostKind::Codex.as_str().to_owned(),
                intent: ConnectionIntent::Shared.as_str().to_owned(),
                host_scope: HostScope::Project.as_str().to_owned(),
                server_name: DEFAULT_SERVER_NAME.to_owned(),
                config_target: path_text(&repo.join(".codex/config.toml")),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: "fingerprint".to_owned(),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_connection_project(
            &runtime_home,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_codex_missing_bash".to_owned(),
                project_id: project.project_id.clone(),
            },
        )?;
        upsert_guard_installation(
            &runtime_home,
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_missing_bash".to_owned(),
                connection_internal_id: "conn_codex_missing_bash".to_owned(),
                project_id: Some(project.project_id.clone()),
                host_kind: HostKind::Codex.as_str().to_owned(),
                guard_mode: IntegrationProfile::Detective.as_str().to_owned(),
                host_capability_json: serde_json::to_string(&capability)?,
                installation_status: GuardInstallationStatus::ReloadRequired.as_str().to_owned(),
                installed_at: Some("2026-07-01T00:00:00Z".to_owned()),
                last_checked_at: "2026-07-01T00:00:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        volicord_store::guards::observe_guard_installation(
            &runtime_home,
            volicord_store::guards::GuardInstallationObservation {
                guard_installation_id: "guard_installation_missing_bash".to_owned(),
                connection_internal_id: "conn_codex_missing_bash".to_owned(),
                project_id: project.project_id.clone(),
                host_kind: HostKind::Codex.as_str().to_owned(),
                guard_mode: IntegrationProfile::Detective.as_str().to_owned(),
                observed_policy_hash: integration.policy_hash.clone(),
                observed_binary_version: Some("test".to_owned()),
                observed_phase: "session_start".to_owned(),
                observed_at: "2026-07-01T00:01:00Z".to_owned(),
            },
        )?;

        let projects = list_connection_projects(&runtime_home, "conn_codex_missing_bash")?;
        let guard_state =
            guard_state_for_connection(&runtime_home, "conn_codex_missing_bash", &projects)?;

        assert_eq!(guard_state.hook_config_state, "stale");
        assert_eq!(guard_state.effective_state, "degraded");
        assert!(guard_state.stale_files.contains(&path_text(&hooks_path)));
        assert_eq!(guard_state.selected_profile(), "detective");
        let control_surface = guard_state.control_surface_json();
        assert_eq!(control_surface["host_hooks_active"], false);
        assert_eq!(
            control_surface["cooperative_pre_tool_warning_available"],
            false
        );
        assert_eq!(
            control_surface["cooperative_pre_tool_denial_available"],
            false
        );
        assert_eq!(control_surface["os_enforced"], false);
        assert!(!guard_state.post_tool_correlation_available());
        assert!(!guard_state.generated_config_verified);
        assert!(!guard_state.bash_shell_mutation_coverage);
        Ok(())
    }

    fn host_plan_stub(entry: &ManagedServerEntry) -> HostPlan {
        HostPlan {
            host_kind: HostKind::Codex,
            connection_intent: ConnectionIntent::Shared,
            host_scope: HostScope::Project,
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            server_name: DEFAULT_SERVER_NAME.to_owned(),
            target: HostTarget::File(PathBuf::from("/repo/.codex/config.toml")),
            entry: entry.clone(),
            change: PlannedChange::Noop,
            fingerprint: "fingerprint".to_owned(),
            conflicts: Vec::new(),
            user_actions: Vec::new(),
            file_snapshot: None,
        }
    }

    #[cfg(unix)]
    fn init_real_git_repo(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(repo)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git init failed\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    #[cfg(unix)]
    fn write_fake_guard_volicord(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        let path = dir.join("volicord");
        fs::write(
            &path,
            "#!/bin/sh\ninput=$(cat)\nprintf 'stdout:%s\\n' \"$input\"\nprintf 'stderr:guard reached\\n' >&2\nexit 37\n",
        )?;
        set_script_executable(&path)?;
        Ok(path)
    }

    #[cfg(unix)]
    fn path_with_prefix(prefix: &Path) -> Result<OsString, Box<dyn std::error::Error>> {
        let mut paths = vec![prefix.to_path_buf()];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        Ok(std::env::join_paths(paths)?)
    }

    fn hook_path_status_recorded(findings: &GuardFileFindings, status: &str) -> bool {
        findings
            .hook_path_safety_statuses
            .iter()
            .any(|recorded| recorded == status)
            || findings.hook_path_safety_details.iter().any(|detail| {
                detail
                    .get("wrapper_resolution_status")
                    .and_then(Value::as_str)
                    == Some(status)
            })
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
