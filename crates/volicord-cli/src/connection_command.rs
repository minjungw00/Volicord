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
        HOST_SCOPE_PROJECT, HOST_SCOPE_USER, VERIFIED_STATUS_ACTION_REQUIRED,
        VERIFIED_STATUS_COMPLETE, VERIFIED_STATUS_FAILED, VERIFIED_STATUS_NOT_VERIFIED,
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
    session_watch::{
        latest_watch_baseline_for_connection, snapshot_product_repository, WatchSnapshotOptions,
    },
    StoreError,
};
use volicord_types::{
    GuardInstallationStatus, IntegrationProfile, PromptCaptureStatus, SummaryCard,
};

use crate::host_integration::{
    claude_code::{self, ClaudeCodeAdapter, ProductionCommandRunner},
    codex::{self, CodexAdapter, CodexEnvironment, CodexExistingPlanRequest},
    contracts::{
        contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
    },
    format_supported_connection_intents,
    generic::{GenericAdapter, USER_MANAGED_CONFIGURATION_GUIDANCE},
    host_capabilities, supports_connection_intent,
    verification::{
        HostMcpCommandDiagnostic, HostMcpCommandLaunchMode, HostRuntimeDiagnostic,
        HostRuntimeObservationStatus, ManagedConfigStatus, ProjectTrustStatus, Verification,
        VerificationStatus,
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
    summary_card::DIAGNOSTIC_SUMMARY_GUARANTEE,
};

mod args;
mod mcp_process;

pub use args::{connect_usage, connection_usage, connections_usage, init_usage};
pub use mcp_process::{
    ConnectionProcess, ConnectionProcessOutput, McpLaunch, McpVerification,
    ProductionConnectionProcess,
};

use args::{
    absolute_path, connection_add_usage, connection_list_usage, connection_mode_usage,
    connection_output_format, connection_remove_usage, connection_status_usage,
    connection_verify_usage, init_output_format, is_help_request, parse_connection_options,
    parse_init_options, parse_public_host_kind, parse_user_connection_mode, InitMode, OutputFormat,
    ParsedConnectionOptions, ParsedInitOptions,
};
use mcp_process::{mcp_launch_from_host_plan, run_connection_preflight};

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
const HOOK_WRAPPER_MARKER: &str = "VOLICORD_MANAGED_HOOK_WRAPPER v1";
const CODEX_DISPATCH_WRAPPER: &str = ".codex/hooks/volicord-dispatch.sh";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentResultStatus {
    Complete,
    ActionRequired,
    Failed,
    NotVerified,
    DryRun,
}

impl AgentResultStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Failed => "failed",
            Self::NotVerified => "not_verified",
            Self::DryRun => "dry_run",
        }
    }

    fn store_status(self) -> &'static str {
        match self {
            Self::Complete => VERIFIED_STATUS_COMPLETE,
            Self::ActionRequired => VERIFIED_STATUS_ACTION_REQUIRED,
            Self::Failed => VERIFIED_STATUS_FAILED,
            Self::NotVerified | Self::DryRun => VERIFIED_STATUS_NOT_VERIFIED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

impl StepStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone)]
struct VerificationStep {
    status: StepStatus,
    details: String,
}

impl VerificationStep {
    fn passed(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Passed,
            details: details.into(),
        }
    }

    fn failed(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Failed,
            details: details.into(),
        }
    }

    fn skipped(details: impl Into<String>) -> Self {
        Self {
            status: StepStatus::Skipped,
            details: details.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct VerificationReport {
    status: AgentResultStatus,
    host: Verification,
    preflight: VerificationStep,
    handshake: VerificationStep,
    tools: Vec<String>,
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
    let host_kind = parsed
        .host_kind
        .ok_or_else(|| ConnectionCommandError::usage("--host is required"))?;
    let repo = parsed
        .repo
        .as_deref()
        .ok_or_else(|| ConnectionCommandError::usage("--repo is required"))?;
    let repo_root = resolve_init_repo_root(current_dir, repo, host_kind, parsed.mode)?;
    let runtime_home = init_runtime_home_path(&parsed, current_dir, process)?;
    let existing_profile = installation_profile(&runtime_home)?;
    let profile_plan =
        init_profile_plan(&parsed, &runtime_home, existing_profile.as_ref(), process)?;
    let intent = ConnectionIntent::Shared;
    let host_scope = host_scope_for_intent(host_kind, intent)?;
    let mode = CONNECTION_MODE_WORKFLOW;
    let server_name = DEFAULT_SERVER_NAME.to_owned();
    let target_hint = connection_target_hint(host_kind, host_scope, Some(&repo_root), process)?;
    let existing = connection_for_host_target(
        &runtime_home,
        host_kind,
        intent,
        host_scope,
        &target_hint,
        &server_name,
    )?;
    let connection_internal_id = existing
        .as_ref()
        .map(|connection| connection.connection_internal_id.clone())
        .unwrap_or_else(|| {
            deterministic_connection_id(
                host_kind,
                host_scope,
                Some(&path_text(&repo_root)),
                &target_hint,
                &server_name,
            )
        });
    let project_hint = project_record_by_repo_root(&runtime_home, &repo_root)
        .ok()
        .flatten();
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let installation_context = InstallationProfile {
        runtime_home: &runtime_home,
        volicord_command: &profile_plan.volicord_command,
        volicord_mcp_command: &profile_plan.volicord_mcp_command,
        default_connection_mode: CONNECTION_MODE_WORKFLOW,
    };
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind,
            connection_intent: intent,
            connection_id: &connection_internal_id,
            repo_root: Some(&repo_root),
            project_id: project_hint
                .as_ref()
                .map(|project| project.project_id.as_str())
                .or(Some("planned_project")),
            project_name: project_hint
                .as_ref()
                .map(|project| project.project_name.as_str())
                .or(Some("planned project")),
            installation_profile: installation_context,
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    if let Some(conflict) = host_plan.conflicts.first() {
        return Err(ConnectionCommandError::runtime(conflict.message.clone()));
    }
    let repo_root_key = path_text(&repo_root);
    let planned_guard_installation_id = stable_id(
        "guard_installation",
        &[
            &connection_internal_id,
            &repo_root_key,
            parsed.mode.guard_value(),
        ],
    );
    let integration_plan = plan_guard_integration(
        host_kind,
        parsed.mode,
        &runtime_home,
        &repo_root,
        &connection_internal_id,
        &planned_guard_installation_id,
        &host_plan.entry,
    )?;

    if parsed.dry_run {
        return render_init_output(InitOutput {
            format: init_output_format(&parsed),
            status: AgentResultStatus::DryRun,
            host_kind,
            init_mode: parsed.mode,
            runtime_home: &runtime_home,
            repo_root: &repo_root,
            connection_id: &connection_internal_id,
            project_id: project_hint
                .as_ref()
                .map(|project| project.project_id.as_str()),
            host_plan: &host_plan,
            verification: None,
            integration: &integration_plan,
            guard_installation: None,
            profile_action: if existing_profile.is_some() {
                "reused"
            } else {
                "planned"
            },
        });
    }

    let runtime_home_id = runtime_home_id_for_path(&runtime_home)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    initialize_runtime_home(&runtime_home, &runtime_home_id, ADMIN_METADATA_JSON)?;
    let profile = ensure_init_installation_profile(&runtime_home, &profile_plan)?;
    let project = ensure_project_for_repo(
        &runtime_home,
        RepoProjectRegistration {
            project_name: None,
            project_alias: None,
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: metadata_json_base()?,
        },
    )?;
    let existing = connection_for_host_target(
        &runtime_home,
        host_kind,
        intent,
        host_scope,
        &target_hint,
        &server_name,
    )?;
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind,
            connection_intent: intent,
            connection_id: &connection_internal_id,
            repo_root: Some(&project.repo_root),
            project_id: Some(&project.project_id),
            project_name: Some(&project.project_name),
            installation_profile: installation_profile_context(&runtime_home, &profile),
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    if let Some(conflict) = host_plan.conflicts.first() {
        return Err(ConnectionCommandError::runtime(conflict.message.clone()));
    }
    let integration_plan = plan_guard_integration(
        host_kind,
        parsed.mode,
        &runtime_home,
        &project.repo_root,
        &connection_internal_id,
        &planned_guard_installation_id,
        &host_plan.entry,
    )?;
    let mcp_command = PathBuf::from(&host_plan.entry.command);
    let metadata_json = connection_metadata_json(&host_plan, &mcp_command, &runtime_home)?;
    let mut connection = ensure_agent_connection(
        &runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: connection_internal_id.clone(),
            host_kind: host_kind.as_str().to_owned(),
            intent: intent.as_str().to_owned(),
            host_scope: host_scope.as_str().to_owned(),
            server_name: host_plan.server_name.clone(),
            config_target: host_target_text(&host_plan.target),
            mode: mode.to_owned(),
            enabled: true,
            managed_fingerprint: host_plan.fingerprint.clone(),
            last_verification_status: existing
                .as_ref()
                .map(|record| record.last_verification_status.clone())
                .unwrap_or_else(|| VERIFIED_STATUS_NOT_VERIFIED.to_owned()),
            last_verification_report_json: existing
                .as_ref()
                .map(|record| record.last_verification_report_json.clone())
                .unwrap_or_else(|| "{}".to_owned()),
            last_user_actions_json: user_actions_json(&host_plan.user_actions)?,
            metadata_json,
        },
    )?;
    enforce_single_project_scope(&runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &runtime_home,
        ConnectionProjectRegistration {
            connection_internal_id: connection.connection_internal_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;
    apply_host_plan(host_kind, &host_plan, process)?;
    let integration_plan = apply_guard_integration(integration_plan)?;
    let installation_status =
        initial_guard_installation_status(parsed.mode, &host_plan, &integration_plan);
    let guard_installation = record_guard_installation(
        &runtime_home,
        host_kind,
        parsed.mode,
        installation_status,
        &connection.connection_internal_id,
        &project.project_id,
        &integration_plan,
    )?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&project.repo_root));
    let verification = verify_connection(
        &runtime_home,
        &connection,
        &host_plan,
        &launch,
        Some(&project.project_id),
        process,
    )?;
    let user_actions =
        init_first_run_user_actions(&verification.host.user_actions, host_kind, parsed.mode);
    connection = update_agent_connection_verification_report(
        &runtime_home,
        &connection.connection_internal_id,
        verification.status.store_status(),
        &host_plan.fingerprint,
        &detailed_verification_report_json(&verification)?,
        &user_actions_json(&user_actions)?,
    )?;
    let status = if verification.status == AgentResultStatus::Complete && user_actions.is_empty() {
        AgentResultStatus::Complete
    } else if verification.status == AgentResultStatus::Failed {
        AgentResultStatus::Failed
    } else {
        AgentResultStatus::ActionRequired
    };
    let _ = connection;
    render_init_output(InitOutput {
        format: init_output_format(&parsed),
        status,
        host_kind,
        init_mode: parsed.mode,
        runtime_home: &runtime_home,
        repo_root: &project.repo_root,
        connection_id: &connection_internal_id,
        project_id: Some(&project.project_id),
        host_plan: &host_plan,
        verification: Some(&verification),
        integration: &integration_plan,
        guard_installation: Some(&guard_installation),
        profile_action: if existing_profile.is_some() {
            "reused"
        } else {
            "created"
        },
    })
}

#[derive(Debug, Clone)]
struct ConnectionSelector {
    host_kind: HostKind,
    intent: Option<ConnectionIntent>,
    host_scope: Option<HostScope>,
    repo_root: PathBuf,
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
    let host_kind = resolve_connection_host(parsed.host_kind, process)?;
    let intent = connection_intent_from_flags(&parsed)?;
    let host_scope = host_scope_for_intent(host_kind, intent)?;
    let mode = if parsed.read_only {
        CONNECTION_MODE_READ_ONLY
    } else {
        CONNECTION_MODE_WORKFLOW
    };
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), current_dir)?;
    let installation_profile = required_installation_profile(&runtime_home)?;
    let repo_root = resolve_connection_repo_root(current_dir, parsed.repo.as_deref())?;
    let server_name = DEFAULT_SERVER_NAME.to_owned();
    let target_hint = connection_target_hint(host_kind, host_scope, Some(&repo_root), process)?;
    let existing = connection_for_host_target(
        &runtime_home,
        host_kind,
        intent,
        host_scope,
        &target_hint,
        &server_name,
    )?;
    let connection_internal_id = existing
        .as_ref()
        .map(|connection| connection.connection_internal_id.clone())
        .unwrap_or_else(|| {
            deterministic_connection_id(
                host_kind,
                host_scope,
                Some(&path_text(&repo_root)),
                &target_hint,
                &server_name,
            )
        });
    let project_hint = project_record_by_repo_root(&runtime_home, &repo_root)
        .ok()
        .flatten();
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind,
            connection_intent: intent,
            connection_id: &connection_internal_id,
            repo_root: Some(&repo_root),
            project_id: project_hint
                .as_ref()
                .map(|project| project.project_id.as_str())
                .or(Some("planned_project")),
            project_name: project_hint
                .as_ref()
                .map(|project| project.project_name.as_str())
                .or(Some("planned project")),
            installation_profile: installation_profile_context(
                &runtime_home,
                &installation_profile,
            ),
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    if let Some(conflict) = host_plan.conflicts.first() {
        return Err(ConnectionCommandError::runtime(conflict.message.clone()));
    }
    if parsed.dry_run {
        return render_connection_plan_output(ConnectionPlanOutput {
            format: connection_output_format(&parsed),
            action: "connection_add",
            status: AgentResultStatus::DryRun,
            runtime_home: &runtime_home,
            connection_id: &connection_internal_id,
            host_kind,
            intent,
            host_scope,
            mode,
            enabled: true,
            repo_root: Some(&repo_root),
            plan: &host_plan,
            projects_remaining: None,
            user_actions: host_plan.user_actions.clone(),
        });
    }

    initialize_runtime_home(
        &runtime_home,
        AGENT_RUNTIME_HOME_ID,
        metadata_json_base()?.as_str(),
    )?;
    let project = ensure_project_for_repo(
        &runtime_home,
        RepoProjectRegistration {
            project_name: None,
            project_alias: None,
            repo_root: repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: metadata_json_base()?,
        },
    )?;
    let existing = connection_for_host_target(
        &runtime_home,
        host_kind,
        intent,
        host_scope,
        &target_hint,
        &server_name,
    )?;
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind,
            connection_intent: intent,
            connection_id: &connection_internal_id,
            repo_root: Some(&project.repo_root),
            project_id: Some(&project.project_id),
            project_name: Some(&project.project_name),
            installation_profile: installation_profile_context(
                &runtime_home,
                &installation_profile,
            ),
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    if let Some(conflict) = host_plan.conflicts.first() {
        return Err(ConnectionCommandError::runtime(conflict.message.clone()));
    }
    let mcp_command = PathBuf::from(&host_plan.entry.command);
    let metadata_json = connection_metadata_json(&host_plan, &mcp_command, &runtime_home)?;
    let mut connection = ensure_agent_connection(
        &runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: connection_internal_id.clone(),
            host_kind: host_kind.as_str().to_owned(),
            intent: intent.as_str().to_owned(),
            host_scope: host_scope.as_str().to_owned(),
            server_name: host_plan.server_name.clone(),
            config_target: host_target_text(&host_plan.target),
            mode: mode.to_owned(),
            enabled: true,
            managed_fingerprint: host_plan.fingerprint.clone(),
            last_verification_status: existing
                .as_ref()
                .map(|record| record.last_verification_status.clone())
                .unwrap_or_else(|| VERIFIED_STATUS_NOT_VERIFIED.to_owned()),
            last_verification_report_json: existing
                .as_ref()
                .map(|record| record.last_verification_report_json.clone())
                .unwrap_or_else(|| "{}".to_owned()),
            last_user_actions_json: user_actions_json(&host_plan.user_actions)?,
            metadata_json,
        },
    )?;
    enforce_single_project_scope(&runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &runtime_home,
        ConnectionProjectRegistration {
            connection_internal_id: connection.connection_internal_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;
    apply_host_plan(host_kind, &host_plan, process)?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&project.repo_root));
    let verification = verify_connection(
        &runtime_home,
        &connection,
        &host_plan,
        &launch,
        Some(&project.project_id),
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
        action: "connected",
        status: verification.status,
        runtime_home: &runtime_home,
        guard_state: guard_state_for_connection(
            &runtime_home,
            &connection.connection_internal_id,
            &projects,
        )?,
        connection: &connection,
        projects: &projects,
        affected_repo_root: Some(&project.repo_root),
        verification: Some(&verification),
        current_host: None,
        plan: Some(&host_plan),
        user_actions: verification.host.user_actions.clone(),
    })
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
    let host_plan = existing_host_plan(&connection, &runtime_home, process)?;
    let current_host = current_status_host_diagnostic(
        &runtime_home,
        &connection,
        Some(&host_plan),
        &projects,
        process,
    )?;
    let user_actions = connection_status_actions(&connection, current_host.as_ref());
    let status = status_with_current_actions(
        status_from_store(&connection.last_verification_status),
        &user_actions,
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

fn status_with_current_actions(
    status: AgentResultStatus,
    actions: &[UserAction],
) -> AgentResultStatus {
    if status == AgentResultStatus::Complete && !actions.is_empty() {
        AgentResultStatus::ActionRequired
    } else {
        status
    }
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
    let (mut connection, _) = select_connection(&runtime_home, &selector)?;
    let host_plan = existing_host_plan(&connection, &runtime_home, process)?;
    let launch = mcp_launch_from_host_plan(&host_plan, None);
    let verification = verify_connection(
        &runtime_home,
        &connection,
        &host_plan,
        &launch,
        None,
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
    let selected_project = projects
        .iter()
        .find(|project| project.project.repo_root == selector.repo_root)
        .ok_or_else(|| ConnectionCommandError::runtime("selected repository is not connected"))?;
    let remaining_count = projects.len().saturating_sub(1);
    let host_plan = if remaining_count == 0 {
        Some(existing_host_plan(&connection, &runtime_home, process)?)
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

fn host_scope_for_intent(
    host_kind: HostKind,
    intent: ConnectionIntent,
) -> Result<HostScope, ConnectionCommandError> {
    if !supports_connection_intent(host_kind, intent) {
        return Err(ConnectionCommandError::usage(
            unsupported_connection_intent_message(host_kind, intent),
        ));
    }
    match (host_kind, intent) {
        (HostKind::Codex, ConnectionIntent::Personal) => Ok(HostScope::User),
        (HostKind::Codex, ConnectionIntent::Shared) => Ok(HostScope::Project),
        (HostKind::ClaudeCode, ConnectionIntent::Personal) => Ok(HostScope::Local),
        (HostKind::ClaudeCode, ConnectionIntent::Shared) => Ok(HostScope::Project),
        (HostKind::ClaudeCode, ConnectionIntent::Global) => Ok(HostScope::User),
        (HostKind::Generic, _) => Err(ConnectionCommandError::usage(
            USER_MANAGED_CONFIGURATION_GUIDANCE,
        )),
        (HostKind::Codex, ConnectionIntent::Global) => unreachable!("validated above"),
    }
}

fn unsupported_connection_intent_message(host_kind: HostKind, intent: ConnectionIntent) -> String {
    let supported = format_supported_connection_intents(host_kind);
    if host_kind == HostKind::Generic {
        return format!("UNSUPPORTED_HOST: {USER_MANAGED_CONFIGURATION_GUIDANCE}; supported connection intents: {supported}");
    }
    format!(
        "UNSUPPORTED_HOST_INTENT: {} does not support {}; supported connection intents: {}",
        public_host_label(host_kind),
        connection_intent_selector_text(intent),
        supported
    )
}

fn connection_intent_selector_text(intent: ConnectionIntent) -> &'static str {
    match intent {
        ConnectionIntent::Personal => "personal",
        ConnectionIntent::Shared => "--shared",
        ConnectionIntent::Global => "--global",
    }
}

fn resolve_connection_host(
    explicit: Option<HostKind>,
    process: &impl ConnectionProcess,
) -> Result<HostKind, ConnectionCommandError> {
    if let Some(host_kind) = explicit {
        return Ok(host_kind);
    }
    let mut available = Vec::new();
    if let Ok(detection) = CodexAdapter::new(codex_environment(process)).detect() {
        if detection.available {
            available.push(detection.host_kind);
        }
    }
    if let Ok(detection) = ClaudeCodeAdapter::new(ProductionCommandRunner).detect() {
        if detection.available {
            available.push(detection.host_kind);
        }
    }
    available.sort_by_key(|host| host.as_str());
    available.dedup();
    match available.as_slice() {
        [host_kind] => Ok(*host_kind),
        [] => Err(ConnectionCommandError::usage(
            "HOST_NOT_DETECTED: host could not be identified; choose `codex` or `claude-code`",
        )),
        _ => Err(ConnectionCommandError::usage(
            "HOST_AMBIGUOUS: host is ambiguous; choose `codex` or `claude-code`",
        )),
    }
}

fn connection_selector(
    parsed: &ParsedConnectionOptions,
    current_dir: &Path,
    process: &impl ConnectionProcess,
) -> Result<ConnectionSelector, ConnectionCommandError> {
    let host_kind = resolve_connection_host(parsed.host_kind, process)?;
    let intent = if parsed.shared || parsed.global {
        Some(connection_intent_from_flags(parsed)?)
    } else {
        None
    };
    let host_scope = intent
        .map(|intent| host_scope_for_intent(host_kind, intent))
        .transpose()?;
    let repo_root = resolve_connection_repo_root(current_dir, parsed.repo.as_deref())?;
    Ok(ConnectionSelector {
        host_kind,
        intent,
        host_scope,
        repo_root,
    })
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

fn resolve_connection_repo_root(
    current_dir: &Path,
    selected_path: Option<&Path>,
) -> Result<PathBuf, ConnectionCommandError> {
    let selected = selected_path.unwrap_or(current_dir);
    let absolute = absolute_path(current_dir, selected.to_path_buf());
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
    let mut cursor = if metadata.is_file() {
        canonical
            .parent()
            .ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "repository path has no parent directory: {}",
                    canonical.display()
                ))
            })?
            .to_path_buf()
    } else {
        canonical
    };

    loop {
        let git_path = cursor.join(".git");
        match git_path.try_exists() {
            Ok(true) => return Ok(cursor),
            Ok(false) => {}
            Err(error) => {
                return Err(ConnectionCommandError::runtime(format!(
                    "failed to inspect Git repository marker {}: {error}",
                    git_path.display()
                )));
            }
        }
        if !cursor.pop() {
            break;
        }
    }

    Err(ConnectionCommandError::runtime(format!(
        "no Git repository root found from {}; run `volicord project use PATH` from inside a Git repository or pass --repo PATH",
        absolute.display()
    )))
}

fn connection_for_host_target(
    runtime_home: &Path,
    host_kind: HostKind,
    intent: ConnectionIntent,
    host_scope: HostScope,
    config_target: &str,
    server_name: &str,
) -> Result<Option<AgentConnectionRecord>, ConnectionCommandError> {
    let matches = list_agent_connections(runtime_home)?
        .into_iter()
        .filter(|connection| {
            connection.host_kind == host_kind.as_str()
                && connection.intent == intent.as_str()
                && connection.host_scope == host_scope.as_str()
                && connection.config_target == config_target
                && connection.server_name == server_name
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [connection] => Ok(Some(connection.clone())),
        connections => Err(ConnectionCommandError::runtime(ambiguous_target_message(
            connections,
        ))),
    }
}

fn select_connection(
    runtime_home: &Path,
    selector: &ConnectionSelector,
) -> Result<(AgentConnectionRecord, Vec<ConnectionProjectRecord>), ConnectionCommandError> {
    if project_record_by_repo_root(runtime_home, &selector.repo_root)?.is_none() {
        return Err(ConnectionCommandError::runtime(format!(
            "PROJECT_NOT_REGISTERED: repository {} is not registered; run `{}` first",
            selector.repo_root.display(),
            selector_repair_command(selector)
        )));
    }
    let mut matches = Vec::new();
    let mut same_host_connections = Vec::new();
    for connection in list_agent_connections(runtime_home)? {
        if connection.host_kind != selector.host_kind.as_str() {
            continue;
        }
        if selector
            .intent
            .is_some_and(|intent| connection.intent != intent.as_str())
        {
            continue;
        }
        if selector
            .host_scope
            .is_some_and(|scope| connection.host_scope != scope.as_str())
        {
            continue;
        }
        let projects = list_connection_projects(runtime_home, &connection.connection_internal_id)?;
        same_host_connections.push((connection.clone(), projects.clone()));
        if projects
            .iter()
            .any(|project| project.project.repo_root == selector.repo_root)
        {
            matches.push((connection, projects));
        }
    }
    match matches.len() {
        0 if same_host_connections.is_empty() => Err(ConnectionCommandError::runtime(format!(
            "CONNECTION_NOT_FOUND: no Agent Connection matches host {}, intent {}, and repository {}; run `{}`",
            public_host_label(selector.host_kind),
            selector_intent_text(selector),
            selector.repo_root.display(),
            selector_repair_command(selector)
        ))),
        0 => Err(ConnectionCommandError::runtime(format!(
            "CONNECTION_ALLOWLIST_MISMATCH: repository {} is not in the selected Agent Connection project allowlist; run `{}`",
            selector.repo_root.display(),
            selector_repair_command(selector)
        ))),
        1 => Ok(matches.remove(0)),
        _ => Err(ConnectionCommandError::runtime(ambiguous_selector_message(
            selector, &matches,
        ))),
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

fn selector_intent_text(selector: &ConnectionSelector) -> &'static str {
    selector
        .intent
        .map(|intent| intent.as_str())
        .unwrap_or("any")
}

fn selector_repair_command(selector: &ConnectionSelector) -> String {
    match selector.intent {
        Some(intent @ (ConnectionIntent::Personal | ConnectionIntent::Global)) => format!(
            "volicord connection add {}{} --repo {}",
            public_host_label(selector.host_kind),
            intent_flag_suffix(intent),
            selector.repo_root.display()
        ),
        Some(ConnectionIntent::Shared) | None => format!(
            "volicord init --host {} --repo {}",
            public_host_label(selector.host_kind),
            selector.repo_root.display()
        ),
    }
}

fn ambiguous_target_message(connections: &[AgentConnectionRecord]) -> String {
    let mut message = String::from("host target matches multiple Agent Connections; choices:\n");
    for connection in connections {
        message.push_str(&format!(
            "- host: {}; intent: {}; target: {}; mode: {}\n",
            public_host_name_text(&connection.host_kind),
            connection.intent,
            connection.config_target,
            public_mode_text(&connection.mode)
        ));
    }
    message
}

fn ambiguous_selector_message(
    selector: &ConnectionSelector,
    matches: &[(AgentConnectionRecord, Vec<ConnectionProjectRecord>)],
) -> String {
    let mut message = format!(
        "connection selector is ambiguous for host {}, intent {}, repository {}; choices:\n",
        public_host_label(selector.host_kind),
        selector_intent_text(selector),
        selector.repo_root.display()
    );
    for (connection, projects) in matches {
        message.push_str(&format!(
            "- target: {}; mode: {}; connected_repositories: {}\n",
            connection.config_target,
            public_mode_text(&connection.mode),
            display_project_roots(projects)
        ));
    }
    message.push_str("Use a more specific repository path or remove the duplicate connection.\n");
    message
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

fn verify_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    process: &impl ConnectionProcess,
) -> Result<Verification, ConnectionCommandError> {
    match host_kind {
        HostKind::Codex => {
            let mut adapter = CodexAdapter::new(codex_environment(process));
            adapter.verify(plan).map_err(Into::into)
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter.verify(plan).map_err(Into::into)
        }
        HostKind::Generic => {
            let mut adapter = GenericAdapter;
            adapter.verify(plan).map_err(Into::into)
        }
    }
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
                    server_name: &connection.server_name,
                    config_target: Path::new(&connection.config_target),
                    mcp_command: &mcp_command,
                    runtime_home: runtime_home_for_entry.as_deref(),
                    managed_fingerprint: &connection.managed_fingerprint,
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

fn verify_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    launch: &McpLaunch,
    project_id: Option<&str>,
    process: &mut impl ConnectionProcess,
) -> Result<VerificationReport, ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let mut host = verify_host_plan(host_kind, host_plan, process)?;
    let projects = list_connection_projects(runtime_home, &connection.connection_internal_id)?;
    host = attach_current_host_runtime_diagnostics(
        runtime_home,
        connection,
        host_plan,
        &projects,
        host,
    );
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
    let status = aggregate_verification_status(&host, &preflight, &handshake.step);
    Ok(VerificationReport {
        status,
        host,
        preflight,
        handshake: handshake.step,
        tools: handshake.tools,
    })
}

fn current_status_host_diagnostic(
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
    if stored_host_managed_config(connection).as_deref() == Some("match") {
        host = host
            .with_managed_config(ManagedConfigStatus::Match)
            .with_mcp_handshake_allowed(true);
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

fn stored_host_managed_config(connection: &AgentConnectionRecord) -> Option<String> {
    json_object_text(&connection.last_verification_report_json)
        .get("host")
        .and_then(|host| host.get("managed_config"))
        .and_then(Value::as_str)
        .map(str::to_owned)
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
    if host_runtime_action_applies(&host, &runtime) {
        let kind = if command.mode == HostMcpCommandLaunchMode::PathResolved
            && command.risk.as_deref() == Some("host_path_unconfirmed")
        {
            UserActionKind::HostMcpCommandPathUnconfirmed
        } else {
            UserActionKind::HostRuntimeNotObserved
        };
        push_unique_action(
            &mut actions,
            UserAction::new(kind, host_runtime_action_message(kind)),
        );
    }
    host.with_host_runtime(runtime)
        .with_host_mcp_command(command)
        .with_user_actions(actions)
}

fn host_runtime_action_applies(host: &Verification, runtime: &HostRuntimeDiagnostic) -> bool {
    runtime.status == HostRuntimeObservationStatus::NotObserved
        && host.managed_config.as_str() == "match"
        && host.mcp_handshake_allowed
        && !host.user_actions.iter().any(|action| {
            matches!(
                action.kind,
                UserActionKind::HostTrustRequired | UserActionKind::ProjectApprovalRequired
            )
        })
}

fn host_runtime_action_message(kind: UserActionKind) -> &'static str {
    match kind {
        UserActionKind::HostMcpCommandPathUnconfirmed => {
            "Make `volicord` available on the PATH seen by the Codex host process, or configure the MCP command so the host can launch it; restart, reload, resume, or start a new Codex session in this repository; confirm Volicord tools are exposed in the active Codex session"
        }
        UserActionKind::HostRuntimeNotObserved => {
            "Restart, reload, resume, or start a new Codex session in this repository; confirm Volicord tools are exposed in the active Codex session"
        }
        _ => "Complete the required host follow-up",
    }
}

fn push_unique_action(actions: &mut Vec<UserAction>, action: UserAction) {
    if !actions.iter().any(|existing| existing.kind == action.kind) {
        actions.push(action);
    }
}

fn connection_status_actions(
    connection: &AgentConnectionRecord,
    current_host: Option<&Verification>,
) -> Vec<UserAction> {
    let mut actions = current_host
        .map(|host| host.user_actions.clone())
        .unwrap_or_else(|| stored_user_actions(connection));
    for action in stored_user_actions(connection)
        .into_iter()
        .filter(|action| action.kind == UserActionKind::ReloadRequired)
    {
        push_unique_action(&mut actions, action);
    }
    actions
}

fn host_runtime_observation(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> HostRuntimeDiagnostic {
    if projects.is_empty() {
        return HostRuntimeDiagnostic {
            status: HostRuntimeObservationStatus::Unknown,
            details: "No connected project was available for Codex host runtime observation"
                .to_owned(),
            last_observed_at: None,
        };
    }
    let mut last_observed_at = None;
    for project in projects {
        match latest_watch_baseline_for_connection(
            runtime_home,
            &project.project_id,
            &connection.connection_internal_id,
        ) {
            Ok(Some(baseline)) => {
                last_observed_at = max_optional_text(last_observed_at, Some(baseline.created_at));
            }
            Ok(None) => {}
            Err(error) => {
                return HostRuntimeDiagnostic {
                    status: HostRuntimeObservationStatus::Unknown,
                    details: format!(
                        "Codex host runtime observation could not be read from session-watch state: {error}"
                    ),
                    last_observed_at: None,
                };
            }
        }
    }
    if last_observed_at.is_some() {
        HostRuntimeDiagnostic {
            status: HostRuntimeObservationStatus::Observed,
            details:
                "Volicord has observed a project-bound Codex host MCP session for this connection"
                    .to_owned(),
            last_observed_at,
        }
    } else {
        HostRuntimeDiagnostic {
            status: HostRuntimeObservationStatus::NotObserved,
            details: "Volicord has not observed a Codex host process start the Volicord MCP server for this connection".to_owned(),
            last_observed_at: None,
        }
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
    preflight: &VerificationStep,
    handshake: &VerificationStep,
) -> AgentResultStatus {
    if preflight.status == StepStatus::Failed || handshake.status == StepStatus::Failed {
        return AgentResultStatus::Failed;
    }
    match host.status {
        VerificationStatus::Complete
            if handshake.status == StepStatus::Passed && host.user_actions.is_empty() =>
        {
            AgentResultStatus::Complete
        }
        VerificationStatus::Complete | VerificationStatus::ActionRequired
            if handshake.status == StepStatus::Passed =>
        {
            AgentResultStatus::ActionRequired
        }
        VerificationStatus::NotVerified => AgentResultStatus::NotVerified,
        _ => AgentResultStatus::Failed,
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
enum ManagedJsonProjection {
    ClaudeCodeSettingsHooks,
    ClaudeCodeMcpEntry,
}

impl ManagedJsonProjection {
    fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCodeSettingsHooks => "claude_code_settings_hooks",
            Self::ClaudeCodeMcpEntry => "claude_code_mcp_entry",
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepoFileChangeStatus {
    Created,
    Updated,
    PlannedCreate,
    PlannedUpdate,
}

impl RepoFileChangeStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::PlannedCreate => "planned_create",
            Self::PlannedUpdate => "planned_update",
        }
    }

    fn text_verb(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::PlannedCreate => "would create",
            Self::PlannedUpdate => "would update",
        }
    }

    fn is_actual(self) -> bool {
        matches!(self, Self::Created | Self::Updated)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoFileChange {
    status: RepoFileChangeStatus,
    path: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookWrapperResolutionStatus {
    Ok,
    RelativePathUnsafe,
    WrapperMissing,
    WrapperNotExecutable,
    DispatchMissing,
    PlaceholderUnsupported,
    AbsolutePathStale,
    PolicyHashMismatch,
    HostOutputMismatch,
    AuthorityMismatch,
    MetadataMissing,
}

impl HookWrapperResolutionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::RelativePathUnsafe => "relative_path_unsafe",
            Self::WrapperMissing => "wrapper_missing",
            Self::WrapperNotExecutable => "wrapper_not_executable",
            Self::DispatchMissing => "dispatch_missing",
            Self::PlaceholderUnsupported => "placeholder_unsupported",
            Self::AbsolutePathStale => "absolute_path_stale",
            Self::PolicyHashMismatch => "policy_hash_mismatch",
            Self::HostOutputMismatch => "host_output_mismatch",
            Self::AuthorityMismatch => "authority_mismatch",
            Self::MetadataMissing => "metadata_missing",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "ok" => Some(Self::Ok),
            "relative_path_unsafe" => Some(Self::RelativePathUnsafe),
            "wrapper_missing" => Some(Self::WrapperMissing),
            "wrapper_not_executable" => Some(Self::WrapperNotExecutable),
            "dispatch_missing" => Some(Self::DispatchMissing),
            "placeholder_unsupported" => Some(Self::PlaceholderUnsupported),
            "absolute_path_stale" => Some(Self::AbsolutePathStale),
            "policy_hash_mismatch" => Some(Self::PolicyHashMismatch),
            "host_output_mismatch" => Some(Self::HostOutputMismatch),
            "authority_mismatch" => Some(Self::AuthorityMismatch),
            "metadata_missing" => Some(Self::MetadataMissing),
            _ => None,
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

struct InitOutput<'a> {
    format: OutputFormat,
    status: AgentResultStatus,
    host_kind: HostKind,
    init_mode: InitMode,
    runtime_home: &'a Path,
    repo_root: &'a Path,
    connection_id: &'a str,
    project_id: Option<&'a str>,
    host_plan: &'a HostPlan,
    verification: Option<&'a VerificationReport>,
    integration: &'a GuardIntegrationPlan,
    guard_installation: Option<&'a GuardInstallationRecord>,
    profile_action: &'a str,
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
    let policy_hash = policy_hash(&policy)?;
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

fn hook_wrapper_exec_command(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("exec "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn hook_wrapper_comment_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
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
fn script_is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o100 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn script_is_executable(_path: &Path) -> bool {
    true
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

fn managed_json_projection_from_actual(
    actual: &Value,
    desired: &Value,
    projection: ManagedJsonProjection,
) -> Result<Option<Value>, ConnectionCommandError> {
    match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => {
            claude_settings_hooks_projection_from_actual(actual, desired)
        }
        ManagedJsonProjection::ClaudeCodeMcpEntry => {
            claude_mcp_projection_from_actual(actual, desired)
        }
    }
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

fn claude_mcp_projection_from_actual(
    actual: &Value,
    desired: &Value,
) -> Result<Option<Value>, ConnectionCommandError> {
    let actual_servers = actual
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ConnectionCommandError::runtime("Claude Code .mcp.json mcpServers must be an object")
        })?;
    let desired_servers = desired
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| ConnectionCommandError::runtime("managed MCP projection is invalid"))?;
    let mut projection_servers = serde_json::Map::new();
    for name in desired_servers.keys() {
        let Some(entry) = actual_servers.get(name) else {
            return Ok(None);
        };
        projection_servers.insert(name.clone(), entry.clone());
    }
    Ok(Some(json!({ "mcpServers": projection_servers })))
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

fn claude_settings_hooks_projection_from_actual(
    actual: &Value,
    desired: &Value,
) -> Result<Option<Value>, ConnectionCommandError> {
    let actual_hooks = actual
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ConnectionCommandError::runtime("Claude Code settings hooks must be an object")
        })?;
    let desired_hooks = desired
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ConnectionCommandError::runtime("managed Claude Code hook projection is invalid")
        })?;
    let mut projected_hooks = serde_json::Map::new();
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
        let desired_group = desired_groups.first().ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "managed Claude Code hook projection has no {event_name} group"
            ))
        })?;
        let Some(actual_groups) = actual_hooks.get(event_name).and_then(Value::as_array) else {
            return Ok(None);
        };
        let matches = actual_groups
            .iter()
            .filter(|group| **group == *desired_group)
            .count();
        if matches != 1 {
            return Ok(None);
        }
        projected_hooks.insert(
            event_name.to_owned(),
            Value::Array(vec![desired_group.clone()]),
        );
    }
    Ok(Some(json!({ "hooks": projected_hooks })))
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

fn is_volicord_codex_hook_config(value: &Value) -> bool {
    let Some(root) = value.as_object() else {
        return false;
    };
    if root.keys().any(|key| key != "hooks") {
        return false;
    }
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    let Some(contract) = contract_for(HostKind::Codex) else {
        return false;
    };
    if hooks.len() != REQUIRED_GUARD_PHASES.len() {
        return false;
    }
    REQUIRED_GUARD_PHASES.iter().all(|phase| {
        let Some(event) = hook_event_for_phase(contract, *phase) else {
            return false;
        };
        let Some(groups) = hooks.get(event.event_name).and_then(Value::as_array) else {
            return false;
        };
        groups.len() == 1
            && groups
                .first()
                .is_some_and(|group| is_volicord_codex_hook_group(*phase, group))
    })
}

fn is_volicord_codex_hook_group(phase: HostLifecyclePhase, group: &Value) -> bool {
    let Some(group) = group.as_object() else {
        return false;
    };
    let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
        return false;
    };
    handlers.len() == 1
        && handlers
            .first()
            .is_some_and(|handler| is_volicord_codex_hook_handler(phase, handler))
}

fn is_volicord_codex_hook_handler(phase: HostLifecyclePhase, handler: &Value) -> bool {
    let Some(object) = handler.as_object() else {
        return false;
    };
    object.get("type").and_then(Value::as_str) == Some("command")
        && object
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| {
                let direct_guard = command
                    .contains(&format!("volicord _hook {}", phase.command_name()))
                    && command.contains("--connection")
                    && command.contains("--guard-installation")
                    && command.contains("--host codex")
                    && command.contains("--host-output codex");
                let wrapper = command.contains(&format!(
                    ".codex/hooks/volicord-{}.sh",
                    phase.command_name()
                )) || (command.contains(CODEX_DISPATCH_WRAPPER)
                    && command.contains(phase.command_name()));
                direct_guard || wrapper
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

fn required_guard_phase_names() -> Vec<&'static str> {
    REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| phase.capability_name())
        .collect()
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

fn generated_files_json(files: &[GeneratedFilePlan]) -> Value {
    Value::Array(
        files
            .iter()
            .map(|file| {
                let mut value = json!({
                    "kind": file.kind.as_str(),
                    "path": path_text(&file.path),
                    "status": file.status.as_str(),
                    "content_hash": sha256_text(&file.content),
                });
                let object = value
                    .as_object_mut()
                    .expect("generated file JSON should be an object");
                match file.write_kind {
                    GeneratedFileWriteKind::Block {
                        start_marker,
                        end_marker,
                        ..
                    } => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_block".to_owned()),
                        );
                        object.insert(
                            "managed_marker_start".to_owned(),
                            Value::String(start_marker.to_owned()),
                        );
                        object.insert(
                            "managed_marker_end".to_owned(),
                            Value::String(end_marker.to_owned()),
                        );
                    }
                    GeneratedFileWriteKind::Json | GeneratedFileWriteKind::ExactJson => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_json".to_owned()),
                        );
                    }
                    GeneratedFileWriteKind::JsonProjection { projection } => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_json_projection".to_owned()),
                        );
                        object.insert(
                            "managed_projection".to_owned(),
                            Value::String(projection.as_str().to_owned()),
                        );
                        object.insert(
                            "managed_projection_json".to_owned(),
                            Value::String(file.content.clone()),
                        );
                    }
                    GeneratedFileWriteKind::Script => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_script".to_owned()),
                        );
                        object.insert(
                            "managed_marker".to_owned(),
                            Value::String(HOOK_WRAPPER_MARKER.to_owned()),
                        );
                        object.insert(
                            "executable_required".to_owned(),
                            Value::Bool(script_executable_required()),
                        );
                        if file.kind == HostIntegrationFileKind::HostHookDispatch {
                            object.insert(
                                "managed_script_role".to_owned(),
                                Value::String("codex_dispatch".to_owned()),
                            );
                        } else if let Some(command) = hook_wrapper_exec_command(&file.content) {
                            object.insert(
                                "managed_script_command".to_owned(),
                                Value::String(command.to_owned()),
                            );
                        }
                        for key in [
                            "host_kind",
                            "phase",
                            "connection_id",
                            "guard_installation_id",
                            "policy_hash",
                            "host_output",
                        ] {
                            if let Some(value) = hook_wrapper_comment_value(&file.content, key) {
                                object.insert(key.to_owned(), Value::String(value.to_owned()));
                            }
                        }
                    }
                }
                value
            })
            .collect(),
    )
}

fn host_hook_commands_json(commands: &[HostHookCommand]) -> Value {
    Value::Array(
        commands
            .iter()
            .map(|command| {
                let (command_text, args) = match &command.generated_command_shape {
                    HostHookCommandShape::ShellCommandString(command) => {
                        (command.clone(), Value::Null)
                    }
                    HostHookCommandShape::Exec { command, args } => (
                        command.clone(),
                        Value::Array(args.iter().cloned().map(Value::String).collect()),
                    ),
                };
                json!({
                    "host_kind": command.host_kind.as_str(),
                    "phase": command.phase.capability_name(),
                    "policy_key": command.phase.policy_key(),
                    "command_shape": command.command_shape_name(),
                    "command": command_text,
                    "args": args,
                    "expected_wrapper_path": path_text(&command.expected_wrapper_path),
                    "expected_phase_wrapper_path": path_text(&command.expected_phase_wrapper_path),
                    "root_resolution_basis": command.root_resolution_basis.as_str(),
                    "hook_command_path_basis": command.hook_command_path_basis.as_str(),
                    "cwd_independent": command.cwd_independent,
                    "subdirectory_safe": command.subdirectory_safe,
                    "wrapper_resolution_status": command.wrapper_resolution_status.as_str(),
                    "verification": {
                        "basis_verified_by": &command.verification.basis_verified_by,
                        "host_contract_source": &command.verification.host_contract_source,
                    },
                })
            })
            .collect(),
    )
}

fn hook_root_resolution_json(commands: &[HostHookCommand]) -> Value {
    if commands.is_empty() {
        return Value::Null;
    }
    let mut bases = commands
        .iter()
        .map(|command| command.root_resolution_basis.as_str())
        .collect::<Vec<_>>();
    bases.sort_unstable();
    bases.dedup();
    let cwd_independent = commands.iter().all(|command| command.cwd_independent);
    let subdirectory_safe = commands.iter().all(|command| command.subdirectory_safe);
    let basis = if bases.len() == 1 {
        bases[0].to_owned()
    } else {
        "mixed".to_owned()
    };
    json!({
        "basis": basis,
        "all_cwd_independent": cwd_independent,
        "all_subdirectory_safe": subdirectory_safe,
        "overall_status": if cwd_independent && subdirectory_safe { "ok" } else { "relative_path_unsafe" },
        "phases": commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command.phase.capability_name(),
                    "root_resolution_basis": command.root_resolution_basis.as_str(),
                    "hook_command_path_basis": command.hook_command_path_basis.as_str(),
                    "cwd_independent": command.cwd_independent,
                    "subdirectory_safe": command.subdirectory_safe,
                    "wrapper_resolution_status": command.wrapper_resolution_status.as_str(),
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn hook_path_safety_json(commands: &[HostHookCommand]) -> Value {
    if commands.is_empty() {
        return Value::Null;
    }
    let all_cwd_independent = commands.iter().all(|command| command.cwd_independent);
    let all_subdirectory_safe = commands.iter().all(|command| command.subdirectory_safe);
    let all_ok = all_cwd_independent
        && all_subdirectory_safe
        && commands
            .iter()
            .all(|command| command.wrapper_resolution_status == HookWrapperResolutionStatus::Ok);
    json!({
        "overall_status": if all_ok { "ok" } else { "relative_path_unsafe" },
        "all_cwd_independent": all_cwd_independent,
        "all_subdirectory_safe": all_subdirectory_safe,
        "commands": commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command.phase.capability_name(),
                    "hook_command_path_basis": command.hook_command_path_basis.as_str(),
                    "cwd_independent": command.cwd_independent,
                    "subdirectory_safe": command.subdirectory_safe,
                    "wrapper_resolution_status": command.wrapper_resolution_status.as_str(),
                })
            })
            .collect::<Vec<_>>(),
    })
}

#[cfg(unix)]
fn script_executable_required() -> bool {
    true
}

#[cfg(not(unix))]
fn script_executable_required() -> bool {
    false
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

fn policy_hash(policy: &Value) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(policy)
        .map(|text| sha256_text(&text))
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
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

fn combine_optional_file_states(left: &str, right: &str) -> String {
    if file_state_rank(right) > file_state_rank(left) {
        right.to_owned()
    } else {
        left.to_owned()
    }
}

fn file_state_rank(value: &str) -> u8 {
    match value {
        "broken" => 8,
        "missing" => 7,
        "stale" => 6,
        "updated" | "created" => 5,
        "planned_update" | "planned_create" => 4,
        "unchanged" | "installed" => 3,
        "disabled" => 2,
        "unsupported_by_host" | "not_applicable" => 1,
        _ => 0,
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

struct ConnectionOutput<'a> {
    format: OutputFormat,
    action: &'a str,
    status: AgentResultStatus,
    runtime_home: &'a Path,
    guard_state: GuardOperationalState,
    connection: &'a AgentConnectionRecord,
    projects: &'a [ConnectionProjectRecord],
    affected_repo_root: Option<&'a Path>,
    verification: Option<&'a VerificationReport>,
    current_host: Option<Verification>,
    plan: Option<&'a HostPlan>,
    user_actions: Vec<UserAction>,
}

struct ConnectionPlanOutput<'a> {
    format: OutputFormat,
    action: &'a str,
    status: AgentResultStatus,
    runtime_home: &'a Path,
    connection_id: &'a str,
    host_kind: HostKind,
    intent: ConnectionIntent,
    host_scope: HostScope,
    mode: &'a str,
    enabled: bool,
    repo_root: Option<&'a Path>,
    plan: &'a HostPlan,
    projects_remaining: Option<usize>,
    user_actions: Vec<UserAction>,
}

enum ConnectionRemovePlan<'a> {
    Host(&'a HostPlan),
    MembershipOnly,
}

fn render_connection_output(data: ConnectionOutput<'_>) -> Result<String, ConnectionCommandError> {
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
    let mcp_config_state =
        connection_mcp_config_state(data.connection, data.verification, data.plan);
    let primary_next_action = primary_connection_action(
        &data.user_actions,
        data.verification,
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
            let mut value = json!({
                "action": data.action,
                "status": data.status.as_str(),
                "disclosure": detective_observation_disclosure_json(),
                "runtime_home": path_text(data.runtime_home),
                "states": connection_states_json(
                    data.status.as_str(),
                    project_registration_state(data.projects),
                    mcp_config_state.as_str(),
                    &data.guard_state,
                    has_reload_action(&data.user_actions),
                ),
                "connection": connection_json(data.connection, &project_ids, Some(&data.user_actions)),
                "target": target,
                "planned_change": planned_change,
                "checks": checks_json(
                    data.connection,
                    data.verification,
                    data.current_host.as_ref(),
                    &data.guard_state,
                ),
                "actions": actions_json_values(&data.user_actions),
                "primary_next_action": primary_next_action.as_ref().map(|action| action.to_json()),
                "host_hook": data.guard_state.to_json(),
                "verification": data.verification.map(verification_json),
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
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn render_compact_connection_text(
    data: &ConnectionOutput<'_>,
    mcp_config_state: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) -> Result<String, ConnectionCommandError> {
    let host_kind = parse_host_kind(&data.connection.host_kind)?;
    let host = public_host_display_name(host_kind);
    if data.action == "removed" {
        return Ok(render_compact_remove_text(data, host));
    }
    let title = compact_connection_title(data.action, host);
    let mut output = format!("{title}\n\nStatus:\n");
    match data.action {
        "verified" => {
            output.push_str(&format!(
                "  Verification: {}\n  Connection: {}\n  Mode: {}\n",
                compact_agent_status_text(data.status),
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode)
            ));
        }
        "status" | "mode_updated" => {
            output.push_str(&format!(
                "  Connection: {}\n  Mode: {}\n  Last verification: {}\n",
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode),
                compact_agent_status_text(data.status)
            ));
        }
        "connected" => {
            output.push_str(&format!(
                "  Connection: {}\n  Verification: {}\n  Mode: {}\n",
                enabled_text(data.connection.enabled),
                compact_agent_status_text(data.status),
                public_mode_text(&data.connection.mode)
            ));
        }
        _ => {
            output.push_str(&format!(
                "  Connection: {}\n  Mode: {}\n  Last verification: {}\n",
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode),
                compact_agent_status_text(data.status)
            ));
        }
    }

    output.push_str(&format!(
        "\nProfile:\n  {}\n\n",
        data.guard_state.selected_profile()
    ));
    if let Some(repo_root) = data.affected_repo_root {
        append_compact_repository(&mut output, repo_root);
    } else {
        append_compact_repositories(&mut output, data.projects);
    }
    if data.action == "connected" {
        let repo_root = data.affected_repo_root.or_else(|| {
            data.projects
                .first()
                .map(|project| project.project.repo_root.as_path())
        });
        append_compact_host_configuration(&mut output, data.plan, repo_root, data.status);
    }
    output.push_str("\nChecks:\n");
    for (label, value) in compact_connection_checks(data, mcp_config_state, primary_next_action) {
        output.push_str(&format!("  {label}: {value}\n"));
    }
    output.push_str("\nNext:\n");
    append_compact_next_steps(&mut output, data, host, primary_next_action);
    output.push_str(&format!(
        "\nLimits:\n{}\n\nDiagnostics:\n  Run:\n    {}\n",
        connection_limits_text(data.guard_state.selected_profile()),
        connection_diagnostics_command(data.connection, data.projects)
    ));
    Ok(output)
}

fn compact_connection_title(action: &str, host: &str) -> String {
    match action {
        "connected" => format!("Agent Connection configured for {host}"),
        "verified" => format!("Agent Connection checked for {host}"),
        "status" => format!("Agent Connection status for {host}"),
        "mode_updated" => format!("Agent Connection mode updated for {host}"),
        other => format!("Agent Connection {other} for {host}"),
    }
}

fn append_compact_repository(output: &mut String, repo_root: &Path) {
    output.push_str(&format!("Repository:\n  {}\n", repo_root.display()));
}

fn append_compact_host_configuration(
    output: &mut String,
    plan: Option<&HostPlan>,
    repo_root: Option<&Path>,
    status: AgentResultStatus,
) {
    let Some(plan) = plan else {
        return;
    };
    output.push('\n');
    if let Some(repo_root) = repo_root {
        if let Some(path) = repo_relative_host_target_path(plan, repo_root) {
            output.push_str("Repo file changes:\n");
            if let Some(status) = repo_file_change_from_host_plan(plan.change, status) {
                output.push_str(&format!("  {} {}\n", status.text_verb(), path));
            } else {
                output.push_str("  none\n");
            }
            return;
        }
    }
    output.push_str(&format!(
        "Host configuration:\n  Target: {}\n  Change: {}\n",
        host_target_text(&plan.target),
        planned_change_text(plan.change)
    ));
}

fn render_compact_remove_text(data: &ConnectionOutput<'_>, host: &str) -> String {
    let remaining = data.projects.len();
    let mut output = format!(
        "Agent Connection removed for {host}\n\nStatus:\n  Connection: removed from selected repository\n  Mode: {}\n  Remaining repositories: {}\n\n",
        public_mode_text(&data.connection.mode),
        remaining
    );
    if let Some(repo_root) = data.affected_repo_root {
        append_compact_repository(&mut output, repo_root);
    }
    if !data.projects.is_empty() {
        output.push_str("\nRemaining repositories:\n");
        for project in data.projects {
            output.push_str(&format!("  {}\n", project.project.repo_root.display()));
        }
    }
    output.push_str("\nRemoved:\n  Selected repository membership\n");
    if data.plan.is_some() && remaining == 0 {
        output.push_str(
            "  Matching managed host configuration\n  Running host processes may keep cached configuration until they reload.\n",
        );
    } else {
        output.push_str("  Host configuration kept for remaining connected repositories\n");
    }
    output.push_str("\nNext:\n");
    if data.plan.is_some() && remaining == 0 {
        output.push_str(&format!(
            "  1. Restart or reload {host} if a running host still shows cached Volicord tools.\n"
        ));
    } else {
        output.push_str("  none\n");
    }
    output.push_str(&format!(
        "\nDiagnostics:\n  Run:\n    {}\n",
        connection_diagnostics_command(data.connection, data.projects)
    ));
    output
}

fn append_compact_repositories(output: &mut String, projects: &[ConnectionProjectRecord]) {
    if projects.len() == 1 {
        output.push_str(&format!(
            "Repository:\n  {}\n",
            projects[0].project.repo_root.display()
        ));
        return;
    }
    output.push_str("Repositories:\n");
    if projects.is_empty() {
        output.push_str("  none\n");
    } else {
        for project in projects {
            output.push_str(&format!("  {}\n", project.project.repo_root.display()));
        }
    }
}

fn compact_connection_checks(
    data: &ConnectionOutput<'_>,
    mcp_config_state: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) -> Vec<(&'static str, String)> {
    if let Some(verification) = data.verification {
        let mut checks = vec![("MCP configuration", mcp_config_state.to_owned())];
        append_host_trust_compact_check(&mut checks, &verification.host);
        checks.extend([
            (
                "MCP preflight",
                verification.preflight.status.as_str().to_owned(),
            ),
            (
                "MCP handshake",
                verification.handshake.status.as_str().to_owned(),
            ),
        ]);
        append_host_runtime_compact_checks(&mut checks, &verification.host);
        checks.push((
            "Host follow-up",
            host_follow_up_text(data.status, primary_next_action).to_owned(),
        ));
        return checks;
    }
    let mut checks = vec![
        (
            "Stored connection",
            format!(
                "{}, mode {}, last verification {}",
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode),
                compact_agent_status_text(data.status)
            ),
        ),
        ("Current MCP configuration", mcp_config_state.to_owned()),
    ];
    if let Some(host) = &data.current_host {
        append_host_trust_compact_check(&mut checks, host);
    }
    checks.extend([
        (
            "Last MCP preflight",
            stored_verification_step_status(data.connection, "preflight"),
        ),
        (
            "Last MCP handshake",
            stored_verification_step_status(data.connection, "mcp_handshake"),
        ),
    ]);
    if let Some(host) = &data.current_host {
        append_host_runtime_compact_checks(&mut checks, host);
    }
    checks.push((
        "Host follow-up",
        host_follow_up_text(data.status, primary_next_action).to_owned(),
    ));
    checks
}

fn append_host_trust_compact_check(checks: &mut Vec<(&'static str, String)>, host: &Verification) {
    if let Some(trust) = &host.project_trust {
        checks.push(("Codex project trust", project_trust_text(trust.status)));
    }
}

fn append_host_runtime_compact_checks(
    checks: &mut Vec<(&'static str, String)>,
    host: &Verification,
) {
    if let Some(runtime) = &host.host_runtime {
        checks.push(("Codex host runtime", host_runtime_text(runtime.status)));
    }
    if let Some(command) = &host.host_mcp_command {
        checks.push(("Host MCP command", host_mcp_command_text(command)));
    }
}

fn project_trust_text(status: ProjectTrustStatus) -> String {
    status.as_str().replace('_', " ")
}

fn host_runtime_text(status: HostRuntimeObservationStatus) -> String {
    status.as_str().replace('_', " ")
}

fn host_mcp_command_text(command: &HostMcpCommandDiagnostic) -> String {
    match command.mode {
        HostMcpCommandLaunchMode::AbsolutePath => command
            .command
            .as_deref()
            .map(|command| format!("uses absolute path {command}"))
            .unwrap_or_else(|| "uses an absolute path".to_owned()),
        HostMcpCommandLaunchMode::PathResolved => command
            .command
            .as_deref()
            .map(|command| format!("uses {command} from the Codex host PATH"))
            .unwrap_or_else(|| "uses a command from the Codex host PATH".to_owned()),
        HostMcpCommandLaunchMode::RemoteExecutor => {
            "uses a remote or executor-backed launch environment".to_owned()
        }
        HostMcpCommandLaunchMode::Unknown => "launch mode unknown".to_owned(),
        HostMcpCommandLaunchMode::Malformed => "configuration malformed".to_owned(),
    }
}

fn append_compact_next_steps(
    output: &mut String,
    data: &ConnectionOutput<'_>,
    host: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) {
    let Some(action) = primary_next_action else {
        output.push_str("  none\n");
        return;
    };
    let command = action
        .command
        .clone()
        .or_else(|| connection_verify_command(Some(data.connection), data.projects));
    let mut index = 1;
    match action.id.as_str() {
        "reload_required" => {
            push_numbered_text(
                output,
                &mut index,
                format!("Open, restart, or reload {host} in this repository."),
            );
            if init_actions_include_trust_or_approval(&data.user_actions) {
                push_numbered_text(
                    output,
                    &mut index,
                    format!("Trust or approve the project configuration if {host} asks."),
                );
            }
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "host_trust_required" | "project_approval_required" => {
            push_numbered_text(
                output,
                &mut index,
                format!("Trust or approve the project configuration if {host} asks."),
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "host_mcp_command_path_unconfirmed" => {
            push_numbered_text(
                output,
                &mut index,
                "Make `volicord` available on the PATH seen by the Codex host process, or configure the MCP command so the host can launch it.",
            );
            push_numbered_text(
                output,
                &mut index,
                "Restart, reload, resume, or start a new Codex session in this repository.",
            );
            push_numbered_text(
                output,
                &mut index,
                "Confirm that Volicord tools are exposed in the active Codex session.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "host_runtime_not_observed" => {
            push_numbered_text(
                output,
                &mut index,
                "Restart, reload, resume, or start a new Codex session in this repository.",
            );
            push_numbered_text(
                output,
                &mut index,
                "Confirm that Volicord tools are exposed in the active Codex session.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "mcp_config_missing" => {
            push_numbered_text(
                output,
                &mut index,
                "Reinstall the missing MCP configuration.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "mcp_config_changed" => {
            push_numbered_text(output, &mut index, "Review the changed MCP configuration.");
            push_optional_numbered_command(
                output,
                &mut index,
                "If Volicord should manage it, run",
                command.as_deref(),
            );
        }
        "mcp_config_malformed" => {
            push_numbered_text(
                output,
                &mut index,
                "Repair the malformed MCP configuration.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_files_missing" => {
            push_numbered_text(
                output,
                &mut index,
                "Reinstall missing detective host-hook files.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_files_stale" => {
            push_numbered_text(
                output,
                &mut index,
                "Refresh stale detective host-hook files.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_files_broken" => {
            push_numbered_text(
                output,
                &mut index,
                "Repair broken detective host-hook files.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_hook_path_safety" => {
            push_numbered_text(
                output,
                &mut index,
                "Regenerate cwd-independent detective host-hook commands.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_capability_degraded" => {
            push_numbered_text(
                output,
                &mut index,
                "Use --profile record if host hooks are not needed, or prepare a supported host, platform, and configuration for detective.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        _ => {
            push_numbered_text(output, &mut index, action.instruction.trim_end_matches('.'));
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
    }
}

fn push_numbered_text(output: &mut String, index: &mut usize, text: impl AsRef<str>) {
    output.push_str(&format!("  {}. {}\n", *index, text.as_ref()));
    *index += 1;
}

fn push_optional_numbered_command(
    output: &mut String,
    index: &mut usize,
    label: &str,
    command: Option<&str>,
) {
    if let Some(command) = command {
        output.push_str(&format!("  {}. {label}:\n     {command}\n", *index));
        *index += 1;
    }
}

fn compact_agent_status_text(status: AgentResultStatus) -> &'static str {
    match status {
        AgentResultStatus::Complete => "complete",
        AgentResultStatus::ActionRequired => "action required",
        AgentResultStatus::Failed => "failed",
        AgentResultStatus::NotVerified => "not verified",
        AgentResultStatus::DryRun => "dry run",
    }
}

fn host_follow_up_text(
    status: AgentResultStatus,
    primary_next_action: Option<&PrimaryNextAction>,
) -> &'static str {
    if primary_next_action.is_some() {
        return "action required";
    }
    match status {
        AgentResultStatus::Complete => "ready",
        AgentResultStatus::ActionRequired => "action required",
        AgentResultStatus::Failed => "failed",
        AgentResultStatus::NotVerified => "not observed",
        AgentResultStatus::DryRun => "skipped",
    }
}

fn enabled_text(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn stored_verification_step_status(connection: &AgentConnectionRecord, step: &str) -> String {
    json_object_text(&connection.last_verification_report_json)
        .get(step)
        .and_then(|step| step.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("not observed")
        .replace('_', " ")
}

fn connection_limits_text(profile: &str) -> &'static str {
    match profile {
        "detective" => init_limits_text(InitMode::Detective),
        _ => init_limits_text(InitMode::Record),
    }
}

fn connection_status_diagnostics_command(
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> Option<String> {
    let project = projects.first()?;
    let intent = parse_connection_intent(&connection.intent).ok()?;
    Some(format!(
        "volicord connection status {}{} --repo {} --json",
        public_host_name_text(&connection.host_kind),
        intent_flag_suffix(intent),
        project.project.repo_root.display()
    ))
}

fn connection_diagnostics_command(
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> String {
    connection_status_diagnostics_command(connection, projects)
        .unwrap_or_else(|| "volicord connection list --json".to_owned())
}

fn render_connection_plan_output(
    data: ConnectionPlanOutput<'_>,
) -> Result<String, ConnectionCommandError> {
    let target = host_target_text(&data.plan.target);
    let planned_change = planned_change_text(data.plan.change);
    let guard_state = GuardOperationalState::not_configured();
    let primary_next_action =
        primary_connection_action(&data.user_actions, None, &guard_state, None, &[]);
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
                "disclosure": detective_observation_disclosure_json(),
                "runtime_home": path_text(data.runtime_home),
                "states": connection_states_json(
                    data.status.as_str(),
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
                    "verification_status": data.status.as_str(),
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

fn render_compact_plan_text(data: &ConnectionPlanOutput<'_>) -> String {
    let host = public_host_display_name(data.host_kind);
    let mut output = format!(
        "Agent Connection plan for {host}\n\nStatus:\n  Plan: dry run\n  Mode: {}\n  Intent: {}\n",
        public_mode_text(data.mode),
        data.intent.as_str()
    );
    if let Some(repo_root) = data.repo_root {
        output.push('\n');
        append_compact_repository(&mut output, repo_root);
    }
    output.push_str("\nPlanned changes:\n");
    append_compact_plan_changes(&mut output, data);
    output.push_str("\nNext:\n");
    append_compact_plan_next_steps(&mut output, data, host);
    output.push_str(&format!(
        "\nDiagnostics:\n  Run:\n    {}\n",
        connection_plan_diagnostics_command(data)
    ));
    output
}

fn append_compact_plan_changes(output: &mut String, data: &ConnectionPlanOutput<'_>) {
    if data.action == "remove" {
        output.push_str("  remove selected repository membership\n");
    }
    if let Some(repo_root) = data.repo_root {
        if let Some(path) = repo_relative_host_target_path(data.plan, repo_root) {
            match data.plan.change {
                PlannedChange::Create | PlannedChange::Update => {
                    if let Some(status) =
                        repo_file_change_from_host_plan(data.plan.change, data.status)
                    {
                        output.push_str(&format!("  {} {}\n", status.text_verb(), path));
                    }
                }
                PlannedChange::Remove => {
                    output.push_str(&format!("  would remove {path}\n"));
                }
                PlannedChange::Noop => {
                    output.push_str("  no host configuration file change\n");
                }
                PlannedChange::ExternalCommand => {
                    output.push_str(&format!(
                        "  would run external host configuration command for {}\n",
                        host_target_text(&data.plan.target)
                    ));
                }
            }
        } else {
            output.push_str(&format!(
                "  host configuration {}: {}\n",
                planned_change_text(data.plan.change),
                host_target_text(&data.plan.target)
            ));
        }
    } else {
        output.push_str(&format!(
            "  host configuration {}: {}\n",
            planned_change_text(data.plan.change),
            host_target_text(&data.plan.target)
        ));
    }
    if let Some(remaining) = data.projects_remaining {
        if remaining == 0 {
            output.push_str(
                "  remove matching managed host configuration\n  running host processes may keep cached configuration until they reload\n",
            );
        } else {
            output.push_str(&format!(
                "  keep host configuration for {} {}\n",
                remaining,
                connected_repository_phrase(remaining)
            ));
        }
    }
}

fn append_compact_plan_next_steps(
    output: &mut String,
    data: &ConnectionPlanOutput<'_>,
    host: &str,
) {
    let mut index = 1;
    if let Some(command) = connection_plan_apply_command(data) {
        push_optional_numbered_command(output, &mut index, "Run", Some(&command));
    }
    if data.action == "connection_add" {
        push_numbered_text(
            output,
            &mut index,
            format!("After applying, open, restart, or reload {host} in this repository."),
        );
        if init_actions_include_trust_or_approval(&data.user_actions) {
            push_numbered_text(
                output,
                &mut index,
                format!("Trust or approve the project configuration if {host} asks."),
            );
        }
        if let Some(repo_root) = data.repo_root {
            let command = connection_plan_verify_command(data.host_kind, data.intent, repo_root);
            push_optional_numbered_command(
                output,
                &mut index,
                "After applying, run",
                Some(&command),
            );
        }
    } else if data.action == "remove" && data.projects_remaining == Some(0) {
        push_numbered_text(
            output,
            &mut index,
            format!(
                "After applying, restart or reload {host} if it still shows cached Volicord tools."
            ),
        );
    }
    if index == 1 {
        output.push_str("  none\n");
    }
}

fn connection_plan_apply_command(data: &ConnectionPlanOutput<'_>) -> Option<String> {
    let repo_root = data.repo_root?;
    match data.action {
        "connection_add" => Some(connection_add_command(
            data.host_kind,
            data.intent,
            data.mode,
            repo_root,
            false,
            false,
        )),
        "remove" => Some(connection_remove_command(
            data.host_kind,
            data.intent,
            repo_root,
            false,
            false,
        )),
        _ => None,
    }
}

fn connection_plan_diagnostics_command(data: &ConnectionPlanOutput<'_>) -> String {
    let Some(repo_root) = data.repo_root else {
        return "volicord connection list --json".to_owned();
    };
    match data.action {
        "connection_add" => connection_add_command(
            data.host_kind,
            data.intent,
            data.mode,
            repo_root,
            true,
            true,
        ),
        "remove" => connection_remove_command(data.host_kind, data.intent, repo_root, true, true),
        _ => "volicord connection list --json".to_owned(),
    }
}

fn connection_plan_verify_command(
    host_kind: HostKind,
    intent: ConnectionIntent,
    repo_root: &Path,
) -> String {
    format!(
        "volicord connection verify {}{} --repo {}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        repo_root.display()
    )
}

fn connection_add_command(
    host_kind: HostKind,
    intent: ConnectionIntent,
    mode: &str,
    repo_root: &Path,
    dry_run: bool,
    json: bool,
) -> String {
    let read_only_flag = if mode == CONNECTION_MODE_READ_ONLY {
        " --read-only"
    } else {
        ""
    };
    format!(
        "volicord connection add {}{}{} --repo {}{}{}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        read_only_flag,
        repo_root.display(),
        if dry_run { " --dry-run" } else { "" },
        if json { " --json" } else { "" }
    )
}

fn connection_remove_command(
    host_kind: HostKind,
    intent: ConnectionIntent,
    repo_root: &Path,
    dry_run: bool,
    json: bool,
) -> String {
    format!(
        "volicord connection remove {}{} --repo {}{}{}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        repo_root.display(),
        if dry_run { " --dry-run" } else { "" },
        if json { " --json" } else { "" }
    )
}

fn connected_repository_phrase(count: usize) -> &'static str {
    if count == 1 {
        "remaining connected repository"
    } else {
        "remaining connected repositories"
    }
}

fn render_init_output(data: InitOutput<'_>) -> Result<String, ConnectionCommandError> {
    let target = host_target_text(&data.host_plan.target);
    let planned_change = planned_change_text(data.host_plan.change);
    let actions = if data.status == AgentResultStatus::DryRun {
        data.host_plan.user_actions.clone()
    } else {
        data.verification
            .map(|verification| {
                init_first_run_user_actions(
                    &verification.host.user_actions,
                    data.host_kind,
                    data.init_mode,
                )
            })
            .unwrap_or_else(|| {
                init_first_run_user_actions(
                    &data.host_plan.user_actions,
                    data.host_kind,
                    data.init_mode,
                )
            })
    };
    let guard_status = data
        .guard_installation
        .map(|guard| guard.installation_status.as_str())
        .unwrap_or(GuardInstallationStatus::Configured.as_str());
    let guard_state = if data.guard_installation.is_some() {
        GuardOperationalState::init(guard_status, data.init_mode, data.integration)
    } else {
        GuardOperationalState::planned(data.init_mode, data.integration)
    };
    let mcp_config_state = init_mcp_config_state(data.verification, Some(data.host_plan));
    let project_state = if data.project_id.is_some() {
        "registered"
    } else {
        "planned"
    };
    let mut primary_next_action =
        primary_connection_action(&actions, data.verification, &guard_state, None, &[]);
    if let Some(action) = primary_next_action.as_mut() {
        attach_init_verify_command(action, data.host_kind, data.repo_root);
    }
    let repo_file_changes = init_repo_file_changes(&data);
    match data.format {
        OutputFormat::Text => Ok(render_init_text_output(&data, &actions, &repo_file_changes)),
        OutputFormat::Json => {
            let value = json!({
                "action": "init",
                "status": data.status.as_str(),
                "disclosure": detective_observation_disclosure_json(),
                "states": connection_states_json(
                    data.status.as_str(),
                    project_state,
                    mcp_config_state.as_str(),
                    &guard_state,
                    has_reload_action(&actions),
                ),
                "host": public_host_label(data.host_kind),
                "selected_profile": data.init_mode.profile_value(),
                "control_surface": guard_state.control_surface_json(),
                "runtime_home": path_text(data.runtime_home),
                "repo_root": path_text(data.repo_root),
                "profile": {
                    "status": data.profile_action,
                },
                "connection": {
                    "connection_id": data.connection_id,
                    "host_kind": data.host_kind.as_str(),
                    "connection_intent": ConnectionIntent::Shared.as_str(),
                    "host_scope": HostScope::Project.as_str(),
                    "mode": CONNECTION_MODE_WORKFLOW,
                    "project_id": data.project_id,
                    "config_target": target,
                },
                "mcp": {
                    "command": &data.host_plan.entry.command,
                    "args": &data.host_plan.entry.args,
                    "env": &data.host_plan.entry.env,
                    "config_target": target,
                },
                "planned_change": planned_change,
                "repo_file_changes": repo_file_changes_json(&repo_file_changes),
                "changed_repo_files": changed_repo_files_json(&repo_file_changes),
                "generated_files": generated_files_json(&data.integration.generated_files),
                "host_hook_commands": host_hook_commands_json(&data.integration.host_hook_commands),
                "hook_root_resolution": hook_root_resolution_json(&data.integration.host_hook_commands),
                "guard_installation": {
                    "guard_installation_id": &data.integration.guard_installation_id,
                    "installation_status": guard_status,
                    "policy_hash": &data.integration.policy_hash,
                    "recorded": data.guard_installation.is_some(),
                },
                "host_hook": guard_state.to_json(),
                "checks": init_checks_json(data.verification, guard_status, &guard_state),
                "actions": actions_json_values(&actions),
                "primary_next_action": primary_next_action.map(|action| action.to_json()),
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn render_init_text_output(
    data: &InitOutput<'_>,
    actions: &[UserAction],
    repo_file_changes: &[RepoFileChange],
) -> String {
    let file_section_label = if data.status == AgentResultStatus::DryRun {
        "Planned repo file changes"
    } else {
        "Repo file changes"
    };
    let mut output = format!(
        "{}\n\nProfile:\n  {}\n\nRepository:\n  {}\n\n{}:\n",
        init_text_title(data.status, data.host_kind),
        data.init_mode.profile_value(),
        data.repo_root.display(),
        file_section_label,
    );
    if repo_file_changes.is_empty() {
        output.push_str("  none\n");
    } else {
        for change in repo_file_changes {
            output.push_str(&format!(
                "  {} {}\n",
                change.status.text_verb(),
                change.path
            ));
        }
    }
    output.push_str(&format!(
        "\nStored local Volicord state:\n  {}\n\nNext:\n",
        data.runtime_home.display()
    ));
    for (index, step) in init_next_steps(data, actions).iter().enumerate() {
        match step {
            InitNextStep::Text(text) => {
                output.push_str(&format!("  {}. {}\n", index + 1, text));
            }
            InitNextStep::Command { label, command } => {
                output.push_str(&format!("  {}. {}:\n     {}\n", index + 1, label, command));
            }
        }
    }
    output.push_str(&format!(
        "\nLimits:\n{}\n\nDiagnostics:\n  Run:\n    {}\n",
        init_limits_text(data.init_mode),
        init_diagnostics_command(data),
    ));
    output
}

fn init_text_title(status: AgentResultStatus, host_kind: HostKind) -> String {
    let host = public_host_display_name(host_kind);
    match status {
        AgentResultStatus::Complete | AgentResultStatus::ActionRequired => {
            format!("Volicord initialized for {host}")
        }
        AgentResultStatus::DryRun => format!("Volicord init plan for {host}"),
        AgentResultStatus::Failed => format!("Volicord init failed for {host}"),
        AgentResultStatus::NotVerified => format!("Volicord init not verified for {host}"),
    }
}

enum InitNextStep {
    Text(String),
    Command {
        label: &'static str,
        command: String,
    },
}

fn init_next_steps(data: &InitOutput<'_>, actions: &[UserAction]) -> Vec<InitNextStep> {
    let host = public_host_display_name(data.host_kind);
    let verify_command = init_verify_command(data.host_kind, data.repo_root);
    if data.status == AgentResultStatus::DryRun {
        let mut steps = vec![
            InitNextStep::Text(
                "Run the same init command without --dry-run to apply the planned repo file changes."
                    .to_owned(),
            ),
            InitNextStep::Text(format!(
                "After applying, open, restart, or reload {host} in this repository."
            )),
        ];
        if init_actions_include_trust_or_approval(actions) {
            steps.push(InitNextStep::Text(format!(
                "Trust or approve the project configuration if {host} asks."
            )));
        }
        steps.push(InitNextStep::Command {
            label: "After applying, run",
            command: verify_command,
        });
        return steps;
    }
    if data.status == AgentResultStatus::Failed {
        return vec![
            InitNextStep::Command {
                label: "Review detailed diagnostics",
                command: init_diagnostics_command(data),
            },
            InitNextStep::Text(format!(
                "Fix the reported issue, then rerun init for {host}."
            )),
        ];
    }
    let mut steps = vec![InitNextStep::Text(format!(
        "Open, restart, or reload {host} in this repository."
    ))];
    if init_actions_include_trust_or_approval(actions) {
        steps.push(InitNextStep::Text(format!(
            "Trust or approve the project configuration if {host} asks."
        )));
    }
    steps.push(InitNextStep::Command {
        label: "Run",
        command: verify_command,
    });
    steps
}

fn init_actions_include_trust_or_approval(actions: &[UserAction]) -> bool {
    actions.iter().any(|action| {
        matches!(
            action.kind,
            UserActionKind::HostTrustRequired | UserActionKind::ProjectApprovalRequired
        )
    })
}

fn init_verify_command(host_kind: HostKind, repo_root: &Path) -> String {
    format!(
        "volicord connection verify {} --shared --repo {}",
        public_host_label(host_kind),
        repo_root.display()
    )
}

fn init_status_command(host_kind: HostKind, repo_root: &Path) -> String {
    format!(
        "volicord connection status {} --shared --repo {} --json",
        public_host_label(host_kind),
        repo_root.display()
    )
}

fn init_diagnostics_command(data: &InitOutput<'_>) -> String {
    if data.status == AgentResultStatus::DryRun {
        return format!(
            "volicord init --host {} --repo {} --profile {} --dry-run --json",
            public_host_label(data.host_kind),
            data.repo_root.display(),
            data.init_mode.profile_value()
        );
    }
    init_status_command(data.host_kind, data.repo_root)
}

fn init_limits_text(init_mode: InitMode) -> &'static str {
    match init_mode {
        InitMode::Record => {
            "  The record profile supports cooperative Volicord workflow recording through MCP.\n  It does not provide OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion."
        }
        InitMode::Detective => {
            "  The detective profile adds cooperative host observation where supported.\n  It does not provide OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion."
        }
    }
}

fn init_repo_file_changes(data: &InitOutput<'_>) -> Vec<RepoFileChange> {
    let mut changes = BTreeMap::new();
    if let Some(status) = repo_file_change_from_host_plan(data.host_plan.change, data.status) {
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
    changes
        .into_iter()
        .map(|(path, status)| RepoFileChange { status, path })
        .collect()
}

fn repo_file_change_from_host_plan(
    change: PlannedChange,
    status: AgentResultStatus,
) -> Option<RepoFileChangeStatus> {
    let dry_run = status == AgentResultStatus::DryRun;
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
        _ => existing,
    }
}

fn repo_file_changes_json(changes: &[RepoFileChange]) -> Value {
    Value::Array(
        changes
            .iter()
            .map(|change| {
                json!({
                    "status": change.status.as_str(),
                    "path": change.path,
                })
            })
            .collect(),
    )
}

fn changed_repo_files_json(changes: &[RepoFileChange]) -> Value {
    Value::Array(
        changes
            .iter()
            .filter(|change| change.status.is_actual())
            .map(|change| {
                json!({
                    "status": change.status.as_str(),
                    "path": change.path,
                })
            })
            .collect(),
    )
}

fn init_checks_json(
    verification: Option<&VerificationReport>,
    guard_status: &str,
    guard_state: &GuardOperationalState,
) -> Value {
    if let Some(report) = verification {
        let mut checks = vec![
            json!({
                "id": "host",
                "status": report.host.status.as_str(),
                "summary": report.host.details,
            }),
            json!({
                "id": "mcp_preflight",
                "status": report.preflight.status.as_str(),
                "summary": report.preflight.details,
            }),
            json!({
                "id": "mcp_handshake",
                "status": report.handshake.status.as_str(),
                "summary": report.handshake.details,
            }),
            json!({
                "id": "guard_installation",
                "status": guard_status,
                "summary": "detective installation status was recorded",
            }),
        ];
        checks.extend(guard_checks_json_values(guard_state));
        Value::Array(checks)
    } else {
        let mut checks = vec![json!({
            "id": "init_plan",
            "status": "passed",
            "summary": "init plan was built without writing files or Runtime Home records"
        })];
        checks.extend(guard_checks_json_values(guard_state));
        Value::Array(checks)
    }
}

fn guard_checks_json_values(guard_state: &GuardOperationalState) -> Vec<Value> {
    let detective_hooks_applicable = guard_state.detective_hooks_applicable();
    let files_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_files_installed",
            "status": "skipped",
            "summary": "detective host-hook files are not applicable for the record profile",
        })
    } else {
        match guard_state.files_state.as_str() {
            "installed" => json!({
                "id": "guard_files_installed",
                "status": "passed",
                "summary": "detective host-hook files are installed",
            }),
            "missing" => json!({
                "id": "guard_files_installed",
                "status": "failed",
                "summary": "detective host-hook files are missing",
                "details": guard_file_details_json(guard_state),
            }),
            "stale" => json!({
                "id": "guard_files_installed",
                "status": "failed",
                "summary": "detective host-hook files are stale",
                "details": guard_file_details_json(guard_state),
            }),
            "broken" => json!({
                "id": "guard_files_installed",
                "status": "failed",
                "summary": "detective host-hook files are broken",
                "details": guard_file_details_json(guard_state),
            }),
            "disabled" => json!({
                "id": "guard_files_installed",
                "status": "skipped",
                "summary": "host hook files are disabled for record profile",
            }),
            other => json!({
                "id": "guard_files_installed",
                "status": "skipped",
                "summary": format!("detective host-hook files are {other}"),
            }),
        }
    };
    let reload_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_host_reload_required",
            "status": "skipped",
            "summary": "detective host reload is not applicable for the record profile",
        })
    } else if guard_state.installation_state == "reload_required" {
        json!({
            "id": "guard_host_reload_required",
            "status": "failed",
            "summary": "host reload is required before detective host hooks are active",
        })
    } else {
        json!({
            "id": "guard_host_reload_required",
            "status": "passed",
            "summary": "host reload is not currently required by detective installation state",
        })
    };
    let hook_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_hook_observed",
            "status": "skipped",
            "summary": "detective host-hook observation is not applicable for the record profile",
        })
    } else {
        match guard_state.hook_observed_state.as_str() {
            "observed" => json!({
                "id": "guard_hook_observed",
                "status": "passed",
                "summary": "detective host hook has been observed",
                "details": {
                    "last_observed_at": &guard_state.last_observed_at,
                    "last_guard_event_at": &guard_state.last_guard_event_at,
                },
            }),
            "not_observed" => json!({
                "id": "guard_hook_observed",
                "status": "failed",
                "summary": "detective host hook has not been observed",
                "details": {
                    "last_observed_at": Value::Null,
                    "last_guard_event_at": &guard_state.last_guard_event_at,
                },
            }),
            other => json!({
                "id": "guard_hook_observed",
                "status": "skipped",
                "summary": format!("detective host-hook observation is {other}"),
            }),
        }
    };
    let status_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_status_active",
            "status": "skipped",
            "summary": "detective signal active status is not applicable for the record profile",
        })
    } else if guard_state.effective_state == "active" {
        json!({
            "id": "guard_status_active",
            "status": "passed",
            "summary": "effective detective signal status is active",
        })
    } else {
        json!({
            "id": "guard_status_active",
            "status": "failed",
            "summary": format!("effective detective signal status is {}", guard_state.effective_state),
            "details": {
                "installation_status": &guard_state.installation_state,
                "configuration_health": &guard_state.configuration_state,
                "observation_health": &guard_state.observation_state,
                "effective_health": &guard_state.effective_state,
                "missing_required_hooks": &guard_state.missing_required_hooks,
                "unresolved_blockers": &guard_state.unresolved_blockers,
            },
        })
    };
    let capability_check = if !detective_hooks_applicable {
        json!({
            "id": "guard_required_hooks_supported",
            "status": "skipped",
            "summary": "detective host-hook capabilities are not applicable for the record profile",
        })
    } else if guard_state.missing_required_hooks.is_empty() {
        json!({
            "id": "guard_required_hooks_supported",
            "status": "passed",
            "summary": "required detective host-hook capabilities are supported",
        })
    } else {
        json!({
            "id": "guard_required_hooks_supported",
            "status": "failed",
            "summary": "required detective host-hook capabilities are missing",
            "details": {
                "missing_required_hooks": &guard_state.missing_required_hooks,
            },
        })
    };
    let prompt_capture_check = if !detective_hooks_applicable {
        json!({
            "id": "prompt_capture_available",
            "status": "skipped",
            "summary": "prompt capture is not applicable for the record profile",
        })
    } else {
        match guard_state.prompt_capture_state.as_str() {
            "active" | "observed" | "configured" => json!({
                "id": "prompt_capture_available",
                "status": "passed",
                "summary": format!("prompt capture is {}", guard_state.prompt_capture_state),
            }),
            "reload_required" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "prompt capture needs host reload",
            }),
            "unsupported_by_host" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "host does not support prompt capture",
            }),
            "not_configured" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "prompt capture is not configured",
            }),
            "degraded" => json!({
                "id": "prompt_capture_available",
                "status": "failed",
                "summary": "prompt capture is degraded",
            }),
            other => json!({
                "id": "prompt_capture_available",
                "status": "skipped",
                "summary": format!("prompt capture is {other}"),
            }),
        }
    };
    vec![
        files_check,
        reload_check,
        hook_check,
        capability_check,
        status_check,
        prompt_capture_check,
    ]
}

fn guard_file_details_json(guard_state: &GuardOperationalState) -> Value {
    json!({
        "missing_files": &guard_state.missing_files,
        "stale_files": &guard_state.stale_files,
        "broken_files": &guard_state.broken_files,
        "missing_required_hooks": &guard_state.missing_required_hooks,
        "hook_path_safety": &guard_state.hook_path_safety_state,
        "hook_path_safety_details": &guard_state.hook_path_safety_details,
    })
}

fn render_connections_output(
    format: OutputFormat,
    rows: &[(AgentConnectionRecord, Vec<ConnectionProjectRecord>)],
) -> Result<String, ConnectionCommandError> {
    match format {
        OutputFormat::Text => {
            let mut output = String::from(
                "host\tintent\tmode\tenabled\tconnected_repositories\tverification_status\ttarget\n",
            );
            for (connection, projects) in rows {
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    public_host_name_text(&connection.host_kind),
                    connection.intent,
                    public_mode_text(&connection.mode),
                    connection.enabled,
                    display_project_roots(projects),
                    connection.last_verification_status,
                    connection.config_target
                ));
            }
            Ok(output)
        }
        OutputFormat::Json => {
            let values = rows
                .iter()
                .map(|(connection, projects)| {
                    let project_ids = projects
                        .iter()
                        .map(|project| project.project_id.clone())
                        .collect::<Vec<_>>();
                    let mut value = connection_json(connection, &project_ids, None);
                    if let Some(object) = value.as_object_mut() {
                        object.insert(
                            "connected_repositories".to_owned(),
                            Value::Array(
                                projects
                                    .iter()
                                    .map(|project| {
                                        Value::String(path_text(&project.project.repo_root))
                                    })
                                    .collect(),
                            ),
                        );
                    }
                    value
                })
                .collect::<Vec<_>>();
            serde_json::to_string_pretty(&json!({
                "status": "complete",
                "disclosure": detective_observation_disclosure_json(),
                "connections": values,
                "checks": [],
                "actions": [],
            }))
            .map(|text| format!("{text}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn render_connection_remove_dry_run_output(
    format: OutputFormat,
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    selected_project: &ConnectionProjectRecord,
    plan: ConnectionRemovePlan<'_>,
    remaining_count: usize,
) -> Result<String, ConnectionCommandError> {
    match plan {
        ConnectionRemovePlan::Host(host_plan) => {
            render_connection_plan_output(ConnectionPlanOutput {
                format,
                action: "remove",
                status: AgentResultStatus::DryRun,
                runtime_home,
                connection_id: &connection.connection_internal_id,
                host_kind: parse_host_kind(&connection.host_kind)?,
                intent: parse_connection_intent(&connection.intent)?,
                host_scope: parse_host_scope(&connection.host_scope)?,
                mode: &connection.mode,
                enabled: connection.enabled,
                repo_root: Some(&selected_project.project.repo_root),
                plan: host_plan,
                projects_remaining: Some(remaining_count),
                user_actions: Vec::new(),
            })
        }
        ConnectionRemovePlan::MembershipOnly => match format {
            OutputFormat::Text => render_compact_membership_remove_plan_text(
                connection,
                selected_project,
                remaining_count,
            ),
            OutputFormat::Json => {
                let project_ids = projects
                    .iter()
                    .map(|project| project.project_id.clone())
                    .collect::<Vec<_>>();
                serde_json::to_string_pretty(&json!({
                    "action": "remove",
                    "status": AgentResultStatus::DryRun.as_str(),
                    "disclosure": detective_observation_disclosure_json(),
                    "runtime_home": path_text(runtime_home),
                    "states": {
                        "runtime_home": "ready",
                        "connection": AgentResultStatus::DryRun.as_str(),
                        "project_registration": project_registration_state(projects),
                        "mcp_config": "membership",
                        "selected_profile": "not_checked",
                        "guard_installation": "not_checked",
                        "guard_files": "not_checked",
                        "guard_hook_observed": "not_checked",
                        "last_guard_event_at": Value::Null,
                        "prompt_capture": "not_checked",
                        "host_reload_required": false,
                        "guard_blockers": [],
                    },
                    "connection": connection_json(connection, &project_ids, None),
                    "target": connection.config_target,
                    "planned_change": "membership",
                    "remaining_connected_projects": remaining_count,
                    "checks": [{
                        "id": "connection_membership",
                        "status": "passed",
                        "summary": "selected repository membership can be removed"
                    }],
                    "actions": [],
                    "primary_next_action": Value::Null,
                }))
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
            }
        },
    }
}

fn render_compact_membership_remove_plan_text(
    connection: &AgentConnectionRecord,
    selected_project: &ConnectionProjectRecord,
    remaining_count: usize,
) -> Result<String, ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let intent = parse_connection_intent(&connection.intent)?;
    let host = public_host_display_name(host_kind);
    let repo_root = &selected_project.project.repo_root;
    let apply_command = connection_remove_command(host_kind, intent, repo_root, false, false);
    let diagnostics_command = connection_remove_command(host_kind, intent, repo_root, true, true);
    Ok(format!(
        "Agent Connection plan for {host}\n\nStatus:\n  Plan: dry run\n  Mode: {}\n  Intent: {}\n\nRepository:\n  {}\n\nPlanned changes:\n  remove selected repository membership\n  keep host configuration for {} {}\n\nNext:\n  1. Run:\n     {}\n\nDiagnostics:\n  Run:\n    {}\n",
        public_mode_text(&connection.mode),
        connection.intent,
        repo_root.display(),
        remaining_count,
        connected_repository_phrase(remaining_count),
        apply_command,
        diagnostics_command
    ))
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

fn display_project_roots(projects: &[ConnectionProjectRecord]) -> String {
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

fn connection_states_json(
    connection_state: &str,
    project_registration: &str,
    mcp_config: &str,
    guard_state: &GuardOperationalState,
    host_reload_required: bool,
) -> Value {
    let guard_files_state = if guard_state.detective_hooks_applicable() {
        guard_state.files_state.as_str()
    } else {
        "disabled"
    };
    let mut states = json!({
        "runtime_home": "ready",
        "connection": connection_state,
        "project_registration": project_registration,
        "mcp_config": mcp_config,
        "selected_profile": guard_state.selected_profile(),
        "control_surface": guard_state.control_surface_json(),
        "generated_config_verified": guard_state.generated_config_verified,
        "native_host_output_adapter_verified": guard_state.native_host_output_adapter_verified,
        "cooperative_pre_tool_warning_available": guard_state.cooperative_pre_tool_warning_available(),
        "cooperative_pre_tool_denial_available": guard_state.cooperative_pre_tool_denial_available(),
        "post_tool_correlation_available": guard_state.post_tool_correlation_available(),
        "bash_shell_mutation_coverage": guard_state.bash_shell_mutation_coverage,
        "direct_file_write_matcher_coverage": guard_state.direct_file_write_matcher_coverage,
        "bypass_detection_active": guard_state.bypass_detection_active(),
        "prompt_capture_available": guard_state.prompt_capture_available(),
        "local_web_consent_available": false,
        "guard_installation": &guard_state.installation_state,
        "guard_configuration": &guard_state.configuration_state,
        "guard_observation": &guard_state.observation_state,
        "guard_effective": &guard_state.effective_state,
        "guard_files": guard_files_state,
        "agents_managed_block": &guard_state.agents_block_state,
        "volicord_policy_file": &guard_state.policy_file_state,
        "rule_instruction_config": &guard_state.rule_instruction_state,
        "hook_config": &guard_state.hook_config_state,
        "required_hook_phases": guard_state.required_hook_phases_state(),
        "missing_required_hooks": &guard_state.missing_required_hooks,
        "guard_hook_observed": &guard_state.hook_observed_state,
        "guard_observed": guard_state.guard_observed(),
        "last_guard_observed_at": &guard_state.last_observed_at,
        "last_guard_event_at": &guard_state.last_guard_event_at,
        "prompt_capture": &guard_state.prompt_capture_state,
        "guard_blockers": &guard_state.unresolved_blockers,
        "host_reload_required": host_reload_required,
    });
    if let Some(object) = states.as_object_mut() {
        object.insert(
            "hook_path_safety".to_owned(),
            Value::String(guard_state.hook_path_safety_state.clone()),
        );
        object.insert(
            "hook_commands_cwd_independent".to_owned(),
            Value::Bool(guard_state.hook_commands_cwd_independent),
        );
        object.insert(
            "hook_commands_subdirectory_safe".to_owned(),
            Value::Bool(guard_state.hook_commands_subdirectory_safe),
        );
    }
    states
}

fn connection_mcp_config_state(
    connection: &AgentConnectionRecord,
    verification: Option<&VerificationReport>,
    plan: Option<&HostPlan>,
) -> String {
    if let Some(verification) = verification {
        return verification.host.managed_config.as_str().to_owned();
    }
    if let Some(plan) = plan {
        return planned_change_text(plan.change).to_owned();
    }
    json_object_text(&connection.last_verification_report_json)
        .get("host")
        .and_then(|host| host.get("managed_config"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned()
}

fn init_mcp_config_state(
    verification: Option<&VerificationReport>,
    plan: Option<&HostPlan>,
) -> String {
    if let Some(verification) = verification {
        return verification.host.managed_config.as_str().to_owned();
    }
    plan.map(|plan| format!("planned_{}", planned_change_text(plan.change)))
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
    guard_state: &GuardOperationalState,
    connection: Option<&AgentConnectionRecord>,
    projects: &[ConnectionProjectRecord],
) -> Option<PrimaryNextAction> {
    if let Some(verification) = verification {
        if verification.host.host_executable.as_str() == "unavailable" {
            return Some(PrimaryNextAction::new(
                "path_binary_not_found",
                verification
                    .host
                    .diagnostic
                    .clone()
                    .unwrap_or_else(|| verification.host.details.clone()),
            ));
        }
        match verification.host.managed_config.as_str() {
            "missing" => {
                return Some(connection_repair_action(
                    "mcp_config_missing",
                    "Reinstall missing MCP configuration.",
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
    if verification.is_none() {
        if let Some(connection) = connection {
            let stored_report = json_object_text(&connection.last_verification_report_json);
            if stored_report
                .get("host")
                .and_then(|host| host.get("host_executable"))
                .and_then(Value::as_str)
                == Some("unavailable")
            {
                return Some(PrimaryNextAction::new(
                    "path_binary_not_found",
                    stored_report
                        .get("host")
                        .and_then(|host| host.get("diagnostic"))
                        .and_then(Value::as_str)
                        .or_else(|| {
                            stored_report
                                .get("host")
                                .and_then(|host| host.get("details"))
                                .and_then(Value::as_str)
                        })
                        .unwrap_or(
                            "Install or repair the host executable so it is available on PATH.",
                        ),
                ));
            }
            match connection_mcp_config_state(connection, None, None).as_str() {
                "missing" => {
                    return Some(connection_repair_action(
                        "mcp_config_missing",
                        "Reinstall missing MCP configuration.",
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
        }
    }
    if guard_state.detective_hooks_applicable() {
        if guard_state.installation_state == "files_missing" {
            return Some(connection_repair_action(
                "guard_files_missing",
                "Run init again to reinstall missing detective host-hook files.",
                connection,
                projects,
            ));
        }
        if guard_state.hook_path_safety_state != HookWrapperResolutionStatus::Ok.as_str()
            && !matches!(
                guard_state.hook_path_safety_state.as_str(),
                "not_checked" | "not_applicable"
            )
        {
            return Some(connection_repair_action(
                "guard_hook_path_safety",
                "Run init again to regenerate cwd-independent detective host-hook commands.",
                connection,
                projects,
            ));
        }
        if guard_state.installation_state == "stale" {
            return Some(connection_repair_action(
                "guard_files_stale",
                "Run init again to refresh stale detective host-hook files.",
                connection,
                projects,
            ));
        }
        if guard_state.installation_state == "broken" {
            return Some(connection_repair_action(
                "guard_files_broken",
                "Repair broken detective host-hook files, then run init again.",
                connection,
                projects,
            ));
        }
        if guard_state.installation_state == "degraded" {
            return Some(guard_degraded_action(connection, projects));
        }
    }
    if let Some(action) = actions
        .iter()
        .find(|action| action.kind == UserActionKind::ReloadRequired)
    {
        let mut primary =
            PrimaryNextAction::new(user_action_id(action.kind), action.message.clone());
        attach_connection_verify_command(&mut primary, connection, projects);
        return Some(primary);
    }
    actions.first().map(|action| {
        let mut primary =
            PrimaryNextAction::new(user_action_id(action.kind), action.message.clone());
        attach_connection_verify_command(&mut primary, connection, projects);
        primary
    })
}

fn connection_diagnostic_summary_card(
    action: &str,
    guard_state: &GuardOperationalState,
    host_display: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) -> Option<SummaryCard> {
    if !matches!(action, "status" | "verified") {
        return None;
    }
    Some(SummaryCard {
        task: "not_selected".to_owned(),
        recording: "diagnostic_observation".to_owned(),
        profile: guard_state.selected_profile().to_owned(),
        write_ticket: "not_selected".to_owned(),
        evidence: "not_selected".to_owned(),
        user_judgment: "not_selected".to_owned(),
        changes: "not_selected".to_owned(),
        close_status: "not_selected".to_owned(),
        transport: "Agent Connection".to_owned(),
        next: connection_summary_next_text(primary_next_action, host_display),
        next_action: None,
        guarantee: DIAGNOSTIC_SUMMARY_GUARANTEE.to_owned(),
    })
}

fn connection_summary_next_text(
    primary_next_action: Option<&PrimaryNextAction>,
    host_display: &str,
) -> String {
    let Some(action) = primary_next_action else {
        return "none".to_owned();
    };
    match action.id.as_str() {
        "host_mcp_command_path_unconfirmed" => format!(
            "{host_display} host runtime has not been observed; make the MCP command launchable by the {host_display} host process, restart or reload {host_display}, then rerun verification."
        ),
        "host_runtime_not_observed" => format!(
            "{host_display} host runtime has not been observed; restart or reload {host_display}, then rerun verification."
        ),
        "host_trust_required" => format!(
            "The project must be trusted before project-scoped {host_display} configuration loads; then rerun verification."
        ),
        "project_approval_required" => format!(
            "The project must be approved before project-scoped {host_display} configuration loads; then rerun verification."
        ),
        "reload_required" => format!(
            "Restart or reload {host_display} so it loads Volicord configuration, then rerun verification."
        ),
        "mcp_config_missing" => {
            "Reinstall missing MCP configuration, then rerun verification.".to_owned()
        }
        "mcp_config_changed" => {
            "Review the changed MCP configuration and repair it if Volicord should manage it, then rerun verification.".to_owned()
        }
        "mcp_config_malformed" => {
            "Repair the malformed MCP configuration, then rerun verification.".to_owned()
        }
        "guard_files_missing" => {
            "Reinstall missing detective host-hook files, then rerun verification.".to_owned()
        }
        "guard_files_stale" => {
            "Refresh stale detective host-hook files, then rerun verification.".to_owned()
        }
        "guard_files_broken" => {
            "Repair broken detective host-hook files, then rerun verification.".to_owned()
        }
        "guard_hook_path_safety" => {
            "Regenerate cwd-independent detective host-hook commands, then rerun verification."
                .to_owned()
        }
        "guard_capability_degraded" => {
            "Prepare a supported detective host configuration or use the record profile, then rerun verification.".to_owned()
        }
        _ => action.instruction.clone(),
    }
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

fn attach_init_verify_command(
    action: &mut PrimaryNextAction,
    host_kind: HostKind,
    repo_root: &Path,
) {
    if !next_action_should_verify(&action.id) {
        return;
    }
    let command = format!(
        "volicord connection verify {} --shared --repo {}",
        public_host_label(host_kind),
        repo_root.display()
    );
    set_verify_command(action, command);
}

fn next_action_should_verify(id: &str) -> bool {
    matches!(
        id,
        "host_trust_required"
            | "project_approval_required"
            | "reload_required"
            | "host_runtime_not_observed"
            | "host_mcp_command_path_unconfirmed"
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
            "Use --profile record if host hooks are not needed, or prepare a supported host, platform, and configuration for detective before rerunning init.",
        );
    };
    let Some(project) = projects.first() else {
        return PrimaryNextAction::new(
            "guard_capability_degraded",
            "Use --profile record if host hooks are not needed, or prepare a supported host, platform, and configuration for detective before rerunning init.",
        );
    };
    let host = public_host_name_text(&connection.host_kind);
    let command = format!(
        "volicord init --host {} --repo {}",
        host,
        project.project.repo_root.display()
    );
    PrimaryNextAction::new(
        "guard_capability_degraded",
        "Use --profile record if host hooks are not needed, or prepare a supported host, platform, and configuration for detective before rerunning init.",
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
            "volicord init --host {} --repo {}",
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
        "guard_files_missing" => "Reinstall missing detective host-hook files.".to_owned(),
        "guard_files_stale" => "Refresh stale detective host-hook files.".to_owned(),
        "guard_files_broken" => "Repair broken detective host-hook files.".to_owned(),
        "guard_hook_path_safety" => {
            "Regenerate cwd-independent detective host-hook commands.".to_owned()
        }
        _ => fallback.to_owned(),
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

#[derive(Debug, Default)]
struct GuardFileFindings {
    missing_files: Vec<String>,
    stale_files: Vec<String>,
    broken_files: Vec<String>,
    file_kind_states: BTreeMap<String, String>,
    guard_profiles: Vec<String>,
    managed_sources: Vec<String>,
    managed_bundle_hashes: Vec<String>,
    managed_verification_statuses: Vec<String>,
    native_host_output_adapter_verified_values: Vec<bool>,
    bash_shell_mutation_coverage_values: Vec<bool>,
    direct_file_write_matcher_coverage_values: Vec<bool>,
    missing_required_hooks: Vec<String>,
    hook_path_safety_statuses: Vec<String>,
    hook_path_safety_details: Vec<Value>,
    hook_cwd_independent_values: Vec<bool>,
    hook_subdirectory_safe_values: Vec<bool>,
    prompt_capture_configured: bool,
    prompt_capture_host_supported: bool,
    rule_file_supported: bool,
}

impl GuardFileFindings {
    fn merge(&mut self, other: GuardFileFindings) {
        self.missing_files.extend(other.missing_files);
        self.stale_files.extend(other.stale_files);
        self.broken_files.extend(other.broken_files);
        for (kind, state) in other.file_kind_states {
            self.set_kind_state_text(&kind, &state);
        }
        self.guard_profiles.extend(other.guard_profiles);
        self.managed_sources.extend(other.managed_sources);
        self.managed_bundle_hashes
            .extend(other.managed_bundle_hashes);
        self.managed_verification_statuses
            .extend(other.managed_verification_statuses);
        self.native_host_output_adapter_verified_values
            .extend(other.native_host_output_adapter_verified_values);
        self.bash_shell_mutation_coverage_values
            .extend(other.bash_shell_mutation_coverage_values);
        self.direct_file_write_matcher_coverage_values
            .extend(other.direct_file_write_matcher_coverage_values);
        self.missing_required_hooks
            .extend(other.missing_required_hooks);
        self.hook_path_safety_statuses
            .extend(other.hook_path_safety_statuses);
        self.hook_path_safety_details
            .extend(other.hook_path_safety_details);
        self.hook_cwd_independent_values
            .extend(other.hook_cwd_independent_values);
        self.hook_subdirectory_safe_values
            .extend(other.hook_subdirectory_safe_values);
        self.prompt_capture_configured |= other.prompt_capture_configured;
        self.prompt_capture_host_supported |= other.prompt_capture_host_supported;
        self.rule_file_supported |= other.rule_file_supported;
    }

    fn sort_dedup(&mut self) {
        self.missing_files.sort();
        self.missing_files.dedup();
        self.stale_files.sort();
        self.stale_files.dedup();
        self.broken_files.sort();
        self.broken_files.dedup();
        self.guard_profiles.sort();
        self.guard_profiles.dedup();
        self.managed_sources.sort();
        self.managed_sources.dedup();
        self.managed_bundle_hashes.sort();
        self.managed_bundle_hashes.dedup();
        self.managed_verification_statuses.sort();
        self.managed_verification_statuses.dedup();
        self.missing_required_hooks.sort();
        self.missing_required_hooks.dedup();
        self.hook_path_safety_statuses
            .sort_by_key(|status| hook_path_status_rank(status));
        self.hook_path_safety_statuses.dedup();
    }

    fn set_kind_state(&mut self, kind: HostIntegrationFileKind, state: &str) {
        self.set_kind_state_text(kind.as_str(), state);
    }

    fn set_kind_state_text(&mut self, kind: &str, state: &str) {
        let update = self
            .file_kind_states
            .get(kind)
            .is_none_or(|current| file_state_rank(state) > file_state_rank(current));
        if update {
            self.file_kind_states
                .insert(kind.to_owned(), state.to_owned());
        }
    }

    fn kind_state(&self, kind: HostIntegrationFileKind) -> &str {
        self.file_kind_states
            .get(kind.as_str())
            .map(String::as_str)
            .unwrap_or("not_configured")
    }

    fn record_hook_path_status(&mut self, status: HookWrapperResolutionStatus, detail: Value) {
        self.hook_path_safety_statuses
            .push(status.as_str().to_owned());
        self.hook_path_safety_details.push(detail);
        self.hook_cwd_independent_values
            .push(status == HookWrapperResolutionStatus::Ok);
        self.hook_subdirectory_safe_values
            .push(status == HookWrapperResolutionStatus::Ok);
        if !matches!(
            status,
            HookWrapperResolutionStatus::Ok
                | HookWrapperResolutionStatus::WrapperMissing
                | HookWrapperResolutionStatus::DispatchMissing
        ) {
            self.stale_files
                .push("host_hook_capability_json:hook_path_safety".to_owned());
        }
    }

    fn rule_instruction_state(&self, guard_disabled: bool) -> String {
        if guard_disabled {
            return "not_applicable".to_owned();
        }
        let state = self.kind_state(HostIntegrationFileKind::HostRuleInstruction);
        if state != "not_configured" {
            state.to_owned()
        } else if self.rule_file_supported {
            "not_configured".to_owned()
        } else {
            "unsupported_by_host".to_owned()
        }
    }

    fn hook_config_state(&self, guard_disabled: bool) -> String {
        if guard_disabled {
            return "disabled".to_owned();
        }
        let state = combine_optional_file_states(
            &combine_optional_file_states(
                self.kind_state(HostIntegrationFileKind::HostHookConfig),
                self.kind_state(HostIntegrationFileKind::HostHookDispatch),
            ),
            self.kind_state(HostIntegrationFileKind::HostHookWrapper),
        );
        if state != "not_configured" {
            state
        } else if self.missing_required_hooks.is_empty() {
            "not_recorded".to_owned()
        } else {
            "missing_required_hooks".to_owned()
        }
    }

    fn generated_config_verified(&self) -> bool {
        self.missing_files.is_empty()
            && self.stale_files.is_empty()
            && self.broken_files.is_empty()
            && self.kind_state(HostIntegrationFileKind::VolicordPolicy) == "installed"
            && self.kind_state(HostIntegrationFileKind::HostHookConfig) == "installed"
            && matches!(
                self.kind_state(HostIntegrationFileKind::HostHookDispatch),
                "not_configured" | "installed"
            )
            && self.kind_state(HostIntegrationFileKind::HostHookWrapper) == "installed"
            && self.hook_path_safety_ok()
    }

    fn hook_path_safety_state(&self) -> String {
        self.hook_path_safety_statuses
            .iter()
            .filter(|status| status.as_str() != HookWrapperResolutionStatus::Ok.as_str())
            .min_by_key(|status| hook_path_status_rank(status))
            .cloned()
            .unwrap_or_else(|| {
                if self.hook_path_safety_statuses.is_empty() {
                    "not_recorded".to_owned()
                } else {
                    HookWrapperResolutionStatus::Ok.as_str().to_owned()
                }
            })
    }

    fn hook_path_safety_ok(&self) -> bool {
        !self.hook_path_safety_statuses.is_empty()
            && self
                .hook_path_safety_statuses
                .iter()
                .all(|status| status == HookWrapperResolutionStatus::Ok.as_str())
            && all_recorded_values_true(&self.hook_cwd_independent_values)
            && all_recorded_values_true(&self.hook_subdirectory_safe_values)
    }

    fn native_host_output_adapter_verified(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.native_host_output_adapter_verified_values)
    }

    fn bash_shell_mutation_coverage(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.bash_shell_mutation_coverage_values)
    }

    fn direct_file_write_matcher_coverage(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.direct_file_write_matcher_coverage_values)
    }
}

fn hook_path_status_rank(status: &str) -> u8 {
    match status {
        "ok" => 100,
        "metadata_missing" => 0,
        "authority_mismatch" => 1,
        "policy_hash_mismatch" => 2,
        "host_output_mismatch" => 3,
        "relative_path_unsafe" => 4,
        "absolute_path_stale" => 5,
        "placeholder_unsupported" => 6,
        "dispatch_missing" => 7,
        "wrapper_missing" => 8,
        "wrapper_not_executable" => 9,
        _ => 10,
    }
}

fn more_severe_hook_wrapper_status(
    left: HookWrapperResolutionStatus,
    right: HookWrapperResolutionStatus,
) -> HookWrapperResolutionStatus {
    if hook_path_status_rank(left.as_str()) <= hook_path_status_rank(right.as_str()) {
        left
    } else {
        right
    }
}

fn all_recorded_values_true(values: &[bool]) -> bool {
    !values.is_empty() && values.iter().all(|value| *value)
}

#[derive(Debug, Clone, Copy)]
struct GuardAuthorityContext<'a> {
    host_kind: &'a str,
    project_repo_roots: &'a [PathBuf],
}

#[cfg(test)]
fn guard_file_findings(capability_json: &str) -> GuardFileFindings {
    guard_file_findings_with_context(capability_json, None)
}

fn guard_file_findings_for_installation(
    installation: &GuardInstallationRecord,
    projects: &[ConnectionProjectRecord],
) -> GuardFileFindings {
    let project_repo_roots = projects
        .iter()
        .map(|project| project.project.repo_root.clone())
        .collect::<Vec<_>>();
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        project_repo_roots: &project_repo_roots,
    };
    guard_file_findings_with_context(&installation.host_capability_json, Some(context))
}

fn guard_file_findings_with_context(
    capability_json: &str,
    context: Option<GuardAuthorityContext<'_>>,
) -> GuardFileFindings {
    let mut findings = GuardFileFindings::default();
    let Ok(value) = serde_json::from_str::<Value>(capability_json) else {
        findings
            .broken_files
            .push("host_hook_capability_json".to_owned());
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::MetadataMissing,
            json!({ "source": "host_hook_capability_json" }),
        );
        return findings;
    };
    findings.prompt_capture_configured = value
        .get("prompt_capture")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    findings.prompt_capture_host_supported = value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("user_prompt_submit_hook"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    findings.rule_file_supported = value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("rule_file_support"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(value) = nonempty_json_string(&value, "selected_profile") {
        findings.guard_profiles.push(value);
    }
    findings
        .native_host_output_adapter_verified_values
        .push(bool_json_field(
            &value,
            "native_host_output_adapter_verified",
        ));
    findings
        .bash_shell_mutation_coverage_values
        .push(bool_json_field(&value, "bash_shell_mutation_coverage"));
    findings
        .direct_file_write_matcher_coverage_values
        .push(bool_json_field(
            &value,
            "direct_file_write_matcher_coverage",
        ));
    findings.missing_required_hooks = missing_required_hooks_from_capability(&value);

    verify_recorded_hook_path_safety(&value, context, &mut findings);

    let files = value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for file in files {
        if record_profile_ignores_detective_file(&value, file) {
            continue;
        }
        verify_guard_file(file, &value, &mut findings);
    }
    findings
}

fn record_profile_ignores_detective_file(capability: &Value, file: &Value) -> bool {
    capability.get("selected_profile").and_then(Value::as_str) == Some("record")
        && file
            .get("kind")
            .and_then(Value::as_str)
            .and_then(host_integration_file_kind_from_str)
            .is_some_and(|kind| {
                matches!(
                    kind,
                    HostIntegrationFileKind::HostHookConfig
                        | HostIntegrationFileKind::HostHookDispatch
                        | HostIntegrationFileKind::HostHookWrapper
                        | HostIntegrationFileKind::HostRuleInstruction
                )
            })
}

fn nonempty_json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn bool_json_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn missing_required_hooks_from_capability(capability: &Value) -> Vec<String> {
    if capability.get("selected_profile").and_then(Value::as_str) == Some("record") {
        return Vec::new();
    }
    let configured_required_hooks = capability
        .get("required_hook_phases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let mut missing_required_hooks = capability
        .get("missing_required_hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for required_hook in required_guard_phase_names() {
        if !configured_required_hooks.contains(&required_hook) {
            missing_required_hooks.push(required_hook.to_owned());
        }
    }
    missing_required_hooks.sort();
    missing_required_hooks.dedup();
    missing_required_hooks
}

fn verify_recorded_hook_path_safety(
    capability: &Value,
    context: Option<GuardAuthorityContext<'_>>,
    findings: &mut GuardFileFindings,
) {
    let requires_path_safety = capability_requires_hook_path_safety(capability);
    let Some(commands) = capability
        .get("host_hook_commands")
        .and_then(Value::as_array)
    else {
        if requires_path_safety {
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::MetadataMissing,
                json!({ "source": "host_hook_commands" }),
            );
        }
        return;
    };
    if commands.is_empty() {
        if requires_path_safety {
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::MetadataMissing,
                json!({ "source": "host_hook_commands" }),
            );
        }
        return;
    }
    for command in commands {
        verify_recorded_hook_command_path_safety(command, context, findings);
    }
}

fn capability_requires_hook_path_safety(capability: &Value) -> bool {
    match capability.get("selected_profile").and_then(Value::as_str) {
        Some("record") => false,
        Some("detective" | "mixed") => true,
        _ => capability
            .get("required_hook_phases")
            .and_then(Value::as_array)
            .is_some_and(|phases| !phases.is_empty()),
    }
}

fn verify_recorded_hook_command_path_safety(
    command: &Value,
    context: Option<GuardAuthorityContext<'_>>,
    findings: &mut GuardFileFindings,
) {
    let host_kind = command
        .get("host_kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let phase = command
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let command_text = command
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = command
        .get("args")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let expected_wrapper_path = command
        .get("expected_wrapper_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_phase_wrapper_path = command
        .get("expected_phase_wrapper_path")
        .and_then(Value::as_str)
        .unwrap_or(expected_wrapper_path);
    let phase_command = phase_command_name_from_capability(phase).unwrap_or_default();
    let mut status = classify_hook_command_path(
        host_kind,
        phase_command,
        command_text,
        args,
        expected_wrapper_path,
        expected_phase_wrapper_path,
    );
    if command.get("cwd_independent").and_then(Value::as_bool) != Some(true)
        || command.get("subdirectory_safe").and_then(Value::as_bool) != Some(true)
    {
        status = HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if let Some(recorded_status) = command
        .get("wrapper_resolution_status")
        .and_then(Value::as_str)
        .filter(|value| *value != HookWrapperResolutionStatus::Ok.as_str())
    {
        let recorded_status = HookWrapperResolutionStatus::from_str(recorded_status)
            .unwrap_or(HookWrapperResolutionStatus::MetadataMissing);
        status = more_severe_hook_wrapper_status(status, recorded_status);
    }
    if let Some(context) = context {
        if !host_kind.is_empty() && host_kind != context.host_kind {
            status = HookWrapperResolutionStatus::AuthorityMismatch;
        }
        if !expected_phase_wrapper_path.is_empty()
            && !context.project_repo_roots.is_empty()
            && !context.project_repo_roots.iter().any(|repo_root| {
                path_starts_with_text(expected_phase_wrapper_path, &path_text(repo_root))
            })
        {
            status = HookWrapperResolutionStatus::AuthorityMismatch;
        }
    }
    verify_recorded_hook_wrapper_path(
        expected_phase_wrapper_path,
        HookWrapperResolutionStatus::WrapperMissing,
        findings,
    );
    if host_kind == HostKind::Codex.as_str() {
        verify_recorded_hook_wrapper_path(
            expected_wrapper_path,
            HookWrapperResolutionStatus::DispatchMissing,
            findings,
        );
    }
    findings.record_hook_path_status(
        status,
        json!({
            "phase": phase,
            "host_kind": host_kind,
            "command": command_text,
            "hook_command_path_basis": command.get("hook_command_path_basis").and_then(Value::as_str).unwrap_or("unknown"),
            "cwd_independent": command.get("cwd_independent").and_then(Value::as_bool).unwrap_or(false),
            "subdirectory_safe": command.get("subdirectory_safe").and_then(Value::as_bool).unwrap_or(false),
            "wrapper_resolution_status": status.as_str(),
            "expected_wrapper_path": expected_wrapper_path,
            "expected_phase_wrapper_path": expected_phase_wrapper_path,
        }),
    );
}

fn verify_recorded_hook_wrapper_path(
    path_text_value: &str,
    missing_status: HookWrapperResolutionStatus,
    findings: &mut GuardFileFindings,
) {
    if path_text_value.trim().is_empty() {
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::MetadataMissing,
            json!({ "source": "expected_wrapper_path" }),
        );
        return;
    }
    let path = Path::new(path_text_value);
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            if !script_is_executable(path) {
                findings.stale_files.push(path_text_value.to_owned());
                findings.record_hook_path_status(
                    HookWrapperResolutionStatus::WrapperNotExecutable,
                    json!({ "path": path_text_value }),
                );
            }
        }
        Ok(_) => {
            findings.broken_files.push(path_text_value.to_owned());
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::WrapperMissing,
                json!({ "path": path_text_value }),
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            findings.missing_files.push(path_text_value.to_owned());
            findings.record_hook_path_status(missing_status, json!({ "path": path_text_value }));
        }
        Err(_) => {
            findings.broken_files.push(path_text_value.to_owned());
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::WrapperMissing,
                json!({ "path": path_text_value }),
            );
        }
    }
}

fn verify_hook_config_commands_path_safety(
    host_kind: HostKind,
    config: &Value,
    capability: &Value,
    findings: &mut GuardFileFindings,
) -> bool {
    let Some(hooks) = config.get("hooks").and_then(Value::as_object) else {
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::MetadataMissing,
            json!({ "source": "hooks" }),
        );
        return false;
    };
    let mut ok = true;
    for (event_name, groups) in hooks {
        let Some(phase_name) = phase_capability_name_from_event(event_name) else {
            continue;
        };
        let Some(phase_command) = phase_command_name_from_capability(phase_name) else {
            ok = false;
            continue;
        };
        let (expected_wrapper_path, expected_phase_wrapper_path) =
            expected_hook_paths_from_capability(capability, phase_name);
        let Some(groups) = groups.as_array() else {
            ok = false;
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::MetadataMissing,
                json!({ "event": event_name }),
            );
            continue;
        };
        for group in groups {
            let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
                ok = false;
                findings.record_hook_path_status(
                    HookWrapperResolutionStatus::MetadataMissing,
                    json!({ "event": event_name, "phase": phase_name }),
                );
                continue;
            };
            for handler in handlers {
                let command = handler
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let args = handler
                    .get("args")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let status = classify_hook_command_path(
                    host_kind.as_str(),
                    phase_command,
                    command,
                    args,
                    expected_wrapper_path.as_deref().unwrap_or_default(),
                    expected_phase_wrapper_path.as_deref().unwrap_or_default(),
                );
                findings.record_hook_path_status(
                    status,
                    json!({
                        "source": "host_hook_config",
                        "event": event_name,
                        "phase": phase_name,
                        "command": command,
                        "wrapper_resolution_status": status.as_str(),
                    }),
                );
                if status != HookWrapperResolutionStatus::Ok {
                    ok = false;
                }
            }
        }
    }
    ok
}

fn expected_hook_paths_from_capability(
    capability: &Value,
    phase_name: &str,
) -> (Option<String>, Option<String>) {
    capability
        .get("host_hook_commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|command| command.get("phase").and_then(Value::as_str) == Some(phase_name))
        .map(|command| {
            (
                command
                    .get("expected_wrapper_path")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                command
                    .get("expected_phase_wrapper_path")
                    .and_then(Value::as_str)
                    .or_else(|| command.get("expected_wrapper_path").and_then(Value::as_str))
                    .map(str::to_owned),
            )
        })
        .unwrap_or((None, None))
}

fn phase_capability_name_from_event(event_name: &str) -> Option<&'static str> {
    match event_name {
        "SessionStart" => Some("session_start_hook"),
        "PreToolUse" => Some("pre_tool_hook"),
        "PostToolUse" => Some("post_tool_hook"),
        "UserPromptSubmit" => Some("user_prompt_submit_hook"),
        "Stop" => Some("stop_hook"),
        _ => None,
    }
}

fn classify_hook_command_path(
    host_kind: &str,
    phase_command: &str,
    command_text: &str,
    args: &[Value],
    expected_wrapper_path: &str,
    expected_phase_wrapper_path: &str,
) -> HookWrapperResolutionStatus {
    if phase_command.is_empty() || command_text.trim().is_empty() {
        return HookWrapperResolutionStatus::MetadataMissing;
    }
    match host_kind {
        "codex" => classify_codex_hook_command_path(
            phase_command,
            command_text,
            expected_wrapper_path,
            expected_phase_wrapper_path,
        ),
        "claude_code" => classify_claude_hook_command_path(
            phase_command,
            command_text,
            args,
            expected_phase_wrapper_path,
        ),
        _ => HookWrapperResolutionStatus::MetadataMissing,
    }
}

fn classify_codex_hook_command_path(
    phase_command: &str,
    command_text: &str,
    expected_dispatch_path: &str,
    expected_phase_wrapper_path: &str,
) -> HookWrapperResolutionStatus {
    let relative_wrapper = format!(".codex/hooks/volicord-{phase_command}.sh");
    if contains_bare_relative_hook_path(command_text, ".codex/hooks/") {
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains(CODEX_DISPATCH_WRAPPER) || command_text.contains(&relative_wrapper) {
        if command_text.contains("git rev-parse --show-toplevel")
            && command_text.contains(CODEX_DISPATCH_WRAPPER)
            && command_text.contains(phase_command)
        {
            return HookWrapperResolutionStatus::Ok;
        }
        if let Some(path) = absolute_path_ending_with(command_text, CODEX_DISPATCH_WRAPPER) {
            return if paths_equivalent_text(&path, expected_dispatch_path) {
                HookWrapperResolutionStatus::Ok
            } else {
                HookWrapperResolutionStatus::AbsolutePathStale
            };
        }
        if let Some(path) = absolute_path_ending_with(command_text, &relative_wrapper) {
            return if paths_equivalent_text(&path, expected_phase_wrapper_path) {
                HookWrapperResolutionStatus::Ok
            } else {
                HookWrapperResolutionStatus::AbsolutePathStale
            };
        }
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains(&format!("volicord _hook {phase_command}")) {
        return HookWrapperResolutionStatus::Ok;
    }
    HookWrapperResolutionStatus::MetadataMissing
}

fn classify_claude_hook_command_path(
    phase_command: &str,
    command_text: &str,
    args: &[Value],
    expected_phase_wrapper_path: &str,
) -> HookWrapperResolutionStatus {
    let relative_wrapper = format!(".claude/hooks/volicord-{phase_command}.sh");
    let placeholder_wrapper = format!("${{CLAUDE_PROJECT_DIR}}/{relative_wrapper}");
    if contains_bare_relative_hook_path(command_text, ".claude/hooks/") {
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains("${CLAUDE_PROJECT_DIR}") {
        return if command_text == placeholder_wrapper && args.is_empty() {
            HookWrapperResolutionStatus::Ok
        } else {
            HookWrapperResolutionStatus::PlaceholderUnsupported
        };
    }
    if command_text.contains(&relative_wrapper) {
        if let Some(path) = absolute_path_ending_with(command_text, &relative_wrapper) {
            return if paths_equivalent_text(&path, expected_phase_wrapper_path) {
                HookWrapperResolutionStatus::Ok
            } else {
                HookWrapperResolutionStatus::AbsolutePathStale
            };
        }
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains(&format!("volicord _hook {phase_command}")) {
        return HookWrapperResolutionStatus::Ok;
    }
    HookWrapperResolutionStatus::MetadataMissing
}

fn contains_bare_relative_hook_path(command_text: &str, prefix: &str) -> bool {
    let trimmed = command_text.trim_start_matches([' ', '\'', '"']);
    trimmed.starts_with(prefix)
        || trimmed.starts_with(&format!("./{prefix}"))
        || command_text.contains(&format!(" {prefix}"))
        || command_text.contains(&format!(" './{prefix}"))
        || command_text.contains(&format!(" \"./{prefix}"))
        || command_text.contains(&format!(" '{prefix}"))
        || command_text.contains(&format!(" \"{prefix}"))
}

fn absolute_path_ending_with(command_text: &str, suffix: &str) -> Option<String> {
    let index = command_text.find(suffix)?;
    let prefix = &command_text[..index];
    let start = prefix
        .rfind([' ', '\'', '"', '=', ';', '('])
        .map(|position| position + 1)
        .unwrap_or(0);
    let path_prefix = prefix.get(start..)?;
    if !path_prefix.starts_with('/') {
        return None;
    }
    Some(format!("{path_prefix}{suffix}"))
}

fn paths_equivalent_text(left: &str, right: &str) -> bool {
    lexical_absolute_path(left)
        .is_some_and(|left| lexical_absolute_path(right).is_some_and(|right| left == right))
}

fn path_starts_with_text(path: &str, prefix: &str) -> bool {
    let Some(path) = lexical_absolute_path(path) else {
        return false;
    };
    let Some(prefix) = lexical_absolute_path(prefix) else {
        return false;
    };
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn lexical_absolute_path(path_text_value: &str) -> Option<String> {
    let path = Path::new(path_text_value);
    if !path.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            std::path::Component::Prefix(_) => return None,
        }
    }
    Some(format!("/{}", parts.join("/")))
}

fn phase_command_name_from_capability(phase: &str) -> Option<&'static str> {
    match phase {
        "session_start_hook" | "session_start" => Some("session-start"),
        "pre_tool_hook" | "pre_tool" => Some("pre-tool"),
        "post_tool_hook" | "post_tool" => Some("post-tool"),
        "user_prompt_submit_hook" | "prompt_capture" => Some("prompt-capture"),
        "stop_hook" | "stop" => Some("stop"),
        _ => None,
    }
}

fn verify_guard_file(file: &Value, capability: &Value, findings: &mut GuardFileFindings) {
    let kind = file
        .get("kind")
        .and_then(Value::as_str)
        .and_then(host_integration_file_kind_from_str);
    let Some(path_text) = file.get("path").and_then(Value::as_str) else {
        findings
            .broken_files
            .push("host_hook_capability_json:files.path".to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let path = Path::new(path_text);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            findings.missing_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "missing");
            }
            return;
        }
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let expected_hash = file
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match file.get("ownership").and_then(Value::as_str) {
        Some("managed_block") => verify_managed_block_file(file, kind, path_text, &text, findings),
        Some("managed_json") => verify_managed_json_file(
            file,
            kind,
            capability,
            path_text,
            &text,
            expected_hash,
            findings,
        ),
        Some("managed_json_projection") => verify_managed_json_projection_file(
            file,
            kind,
            capability,
            path_text,
            &text,
            expected_hash,
            findings,
        ),
        Some("managed_script") => verify_managed_script_file(
            file,
            kind,
            capability,
            ManagedFileRead {
                path,
                path_text,
                text: &text,
                expected_hash,
            },
            findings,
        ),
        _ => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
        }
    }
}

fn verify_managed_block_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    path_text: &str,
    text: &str,
    findings: &mut GuardFileFindings,
) {
    let Some(start_marker) = file.get("managed_marker_start").and_then(Value::as_str) else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let Some(end_marker) = file.get("managed_marker_end").and_then(Value::as_str) else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    if marker_count(text, start_marker) != 1 || marker_count(text, end_marker) != 1 {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    let Some(block) = managed_block_slice(text, start_marker, end_marker) else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let expected_hash = file
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if sha256_text(block) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "stale");
        }
    } else if let Some(kind) = kind {
        findings.set_kind_state(kind, "installed");
    }
}

fn verify_managed_json_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    capability: &Value,
    path_text: &str,
    text: &str,
    expected_hash: &str,
    findings: &mut GuardFileFindings,
) {
    let mut state = "installed";
    if sha256_text(text) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if file.get("kind").and_then(Value::as_str) == Some("host_hook_config") {
        let value = match serde_json::from_str::<Value>(text) {
            Ok(value) if is_volicord_codex_hook_config(&value) => value,
            Ok(_) | Err(_) => {
                findings.broken_files.push(path_text.to_owned());
                if let Some(kind) = kind {
                    findings.set_kind_state(kind, "broken");
                }
                return;
            }
        };
        if validate_contract_config(HostKind::Codex, HostContractConfigKind::HookConfig, text)
            .is_err()
        {
            findings.stale_files.push(path_text.to_owned());
            state = "stale";
        }
        if !verify_hook_config_commands_path_safety(HostKind::Codex, &value, capability, findings) {
            findings.stale_files.push(path_text.to_owned());
            state = "stale";
        }
    }
    if file.get("kind").and_then(Value::as_str) != Some("volicord_policy") {
        if let Some(kind) = kind {
            findings.set_kind_state(kind, state);
        }
        return;
    }
    let policy = match serde_json::from_str::<Value>(text) {
        Ok(policy) => policy,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let expected_policy_hash = capability
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match policy_hash(&policy) {
        Ok(actual) if actual == expected_policy_hash => {}
        Ok(_) => {
            findings.stale_files.push(path_text.to_owned());
            state = "stale";
        }
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    }
    if policy
        .get("host_hook")
        .and_then(|guard| guard.get("commands"))
        != capability.get("commands")
    {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if let Some(kind) = kind {
        findings.set_kind_state(kind, state);
    }
}

#[derive(Clone, Copy)]
struct ManagedFileRead<'a> {
    path: &'a Path,
    path_text: &'a str,
    text: &'a str,
    expected_hash: &'a str,
}

fn verify_managed_script_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    capability: &Value,
    managed: ManagedFileRead<'_>,
    findings: &mut GuardFileFindings,
) {
    let ManagedFileRead {
        path,
        path_text,
        text,
        expected_hash,
    } = managed;
    let mut state = "installed";
    if file.get("managed_marker").and_then(Value::as_str) != Some(HOOK_WRAPPER_MARKER)
        || !text.contains(HOOK_WRAPPER_MARKER)
    {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    if kind == Some(HostIntegrationFileKind::HostHookDispatch) {
        verify_managed_dispatch_script_file(file, kind, managed, findings);
        return;
    }
    let Some(expected_command) = file
        .get("managed_script_command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    if hook_wrapper_exec_command(text) != Some(expected_command) {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    let expected_policy_hash = capability
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_host_output = file
        .get("host_output")
        .and_then(Value::as_str)
        .unwrap_or_default();
    for required in [
        "volicord _hook ",
        "--repo ",
        "--connection ",
        "--guard-installation ",
        "--host ",
        "--integration-profile ",
        "--policy-hash ",
        "--host-output ",
    ] {
        if !expected_command.contains(required) {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    }
    if !expected_policy_hash.is_empty()
        && hook_wrapper_comment_value(text, "policy_hash") != Some(expected_policy_hash)
    {
        findings.stale_files.push(path_text.to_owned());
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::PolicyHashMismatch,
            json!({ "path": path_text, "expected_policy_hash": expected_policy_hash }),
        );
        state = "stale";
    }
    if !expected_host_output.is_empty()
        && hook_wrapper_comment_value(text, "host_output") != Some(expected_host_output)
    {
        findings.stale_files.push(path_text.to_owned());
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::HostOutputMismatch,
            json!({ "path": path_text, "expected_host_output": expected_host_output }),
        );
        state = "stale";
    }
    for key in [
        "host_kind",
        "phase",
        "connection_id",
        "guard_installation_id",
    ] {
        let Some(expected) = file.get(key).and_then(Value::as_str) else {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        };
        if hook_wrapper_comment_value(text, key) != Some(expected) {
            findings.stale_files.push(path_text.to_owned());
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::AuthorityMismatch,
                json!({ "path": path_text, "field": key, "expected": expected }),
            );
            state = "stale";
        }
    }
    if sha256_text(text) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if file
        .get("executable_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !script_is_executable(path)
    {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if let Some(kind) = kind {
        findings.set_kind_state(kind, state);
    }
}

fn verify_managed_dispatch_script_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    managed: ManagedFileRead<'_>,
    findings: &mut GuardFileFindings,
) {
    let ManagedFileRead {
        path,
        path_text,
        text,
        expected_hash,
    } = managed;
    let mut state = "installed";
    if file.get("managed_script_role").and_then(Value::as_str) != Some("codex_dispatch")
        || hook_wrapper_comment_value(text, "host_kind") != Some("codex")
        || hook_wrapper_comment_value(text, "phase") != Some("dispatch")
        || hook_wrapper_comment_value(text, "script_role") != Some("codex_dispatch")
    {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    for required in [
        "git rev-parse --show-toplevel",
        "session-start|pre-tool|post-tool|prompt-capture|stop",
        ".codex/hooks/volicord-$phase.sh",
        "exec \"$wrapper\"",
    ] {
        if !text.contains(required) {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    }
    if sha256_text(text) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if file
        .get("executable_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !script_is_executable(path)
    {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if let Some(kind) = kind {
        findings.set_kind_state(kind, state);
    }
}

fn verify_managed_json_projection_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    capability: &Value,
    path_text: &str,
    text: &str,
    expected_hash: &str,
    findings: &mut GuardFileFindings,
) {
    let Some(projection) = file
        .get("managed_projection")
        .and_then(Value::as_str)
        .and_then(managed_json_projection_from_str)
    else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let actual = match serde_json::from_str::<Value>(text) {
        Ok(actual) => actual,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let expected_projection_json = file
        .get("managed_projection_json")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let desired = match serde_json::from_str::<Value>(expected_projection_json) {
        Ok(desired) => desired,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let actual_projection = match managed_json_projection_from_actual(&actual, &desired, projection)
    {
        Ok(Some(actual_projection)) => actual_projection,
        Ok(None) => {
            findings.stale_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "stale");
            }
            return;
        }
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    if actual_projection == desired && sha256_text(expected_projection_json) == expected_hash {
        if projection == ManagedJsonProjection::ClaudeCodeSettingsHooks
            && serde_json::to_string(&actual_projection)
                .ok()
                .is_none_or(|text| {
                    validate_contract_config(
                        HostKind::ClaudeCode,
                        HostContractConfigKind::ProjectSettings,
                        &text,
                    )
                    .is_err()
                })
        {
            findings.stale_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "stale");
            }
            return;
        }
        if projection == ManagedJsonProjection::ClaudeCodeSettingsHooks
            && !verify_hook_config_commands_path_safety(
                HostKind::ClaudeCode,
                &actual_projection,
                capability,
                findings,
            )
        {
            findings.stale_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "stale");
            }
            return;
        }
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "installed");
        }
    } else {
        findings.stale_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "stale");
        }
    }
}

fn managed_json_projection_from_str(value: &str) -> Option<ManagedJsonProjection> {
    match value {
        "claude_code_settings_hooks" => Some(ManagedJsonProjection::ClaudeCodeSettingsHooks),
        "claude_code_mcp_entry" => Some(ManagedJsonProjection::ClaudeCodeMcpEntry),
        _ => None,
    }
}

fn host_integration_file_kind_from_str(value: &str) -> Option<HostIntegrationFileKind> {
    match value {
        "volicord_policy" => Some(HostIntegrationFileKind::VolicordPolicy),
        "host_mcp_config" => Some(HostIntegrationFileKind::HostMcpConfig),
        "host_hook_config" => Some(HostIntegrationFileKind::HostHookConfig),
        "host_hook_dispatch" => Some(HostIntegrationFileKind::HostHookDispatch),
        "host_hook_wrapper" => Some(HostIntegrationFileKind::HostHookWrapper),
        "host_rule_instruction" => Some(HostIntegrationFileKind::HostRuleInstruction),
        "agents_managed_block" => Some(HostIntegrationFileKind::AgentsManagedBlock),
        _ => None,
    }
}

fn marker_count(text: &str, marker: &str) -> usize {
    text.match_indices(marker).count()
}

fn managed_block_slice<'a>(text: &'a str, start_marker: &str, end_marker: &str) -> Option<&'a str> {
    let start = text.find(start_marker)?;
    let end = start + text[start..].find(end_marker)? + end_marker.len();
    let end = if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    text.get(start..end)
}

fn actions_json_values(actions: &[UserAction]) -> Value {
    Value::Array(
        actions
            .iter()
            .map(|action| {
                json!({
                    "id": user_action_id(action.kind),
                    "instruction": action.message,
                })
            })
            .collect(),
    )
}

fn user_action_id(kind: UserActionKind) -> &'static str {
    match kind {
        UserActionKind::HostTrustRequired => "host_trust_required",
        UserActionKind::ProjectApprovalRequired => "project_approval_required",
        UserActionKind::ReloadRequired => "reload_required",
        UserActionKind::HostRuntimeNotObserved => "host_runtime_not_observed",
        UserActionKind::HostMcpCommandPathUnconfirmed => "host_mcp_command_path_unconfirmed",
    }
}

fn checks_json(
    connection: &AgentConnectionRecord,
    verification: Option<&VerificationReport>,
    current_host: Option<&Verification>,
    guard_state: &GuardOperationalState,
) -> Value {
    if let Some(verification) = verification {
        let mut checks = vec![json!({
            "id": "host",
            "status": verification.host.status.as_str(),
            "summary": verification.host.details,
            "details": {
                "host_state": verification.host.host_state.as_str(),
                "managed_config": verification.host.managed_config.as_str(),
                "host_executable": verification.host.host_executable.as_str(),
                "host_gate": verification.host.host_gate.as_str(),
                "host_configuration": verification.host.host_configuration.as_str(),
            }
        })];
        checks.extend(host_diagnostic_checks_json(&verification.host));
        checks.extend([
            json!({
                "id": "mcp_preflight",
                "status": verification.preflight.status.as_str(),
                "summary": verification.preflight.details,
            }),
            json!({
                "id": "mcp_handshake",
                "status": verification.handshake.status.as_str(),
                "summary": verification.handshake.details,
            }),
        ]);
        checks.extend(guard_checks_json_values(guard_state));
        return Value::Array(checks);
    }
    let mut checks = stored_checks_json(connection, current_host);
    checks.extend(guard_checks_json_values(guard_state));
    Value::Array(checks)
}

fn stored_checks_json(
    connection: &AgentConnectionRecord,
    current_host: Option<&Verification>,
) -> Vec<Value> {
    let report = json_object_text(&connection.last_verification_report_json);
    let Some(object) = report.as_object() else {
        return current_host
            .map(host_diagnostic_checks_json)
            .unwrap_or_default();
    };
    let mut checks = Vec::new();
    if let Some(host) = object.get("host").and_then(Value::as_object) {
        checks.push(json!({
            "id": "host",
            "status": host.get("status").and_then(Value::as_str).unwrap_or("not_verified"),
            "summary": host
                .get("details")
                .and_then(Value::as_str)
                .unwrap_or("stored host verification state"),
            "details": host,
        }));
    }
    if let Some(host) = current_host {
        checks.extend(host_diagnostic_checks_json(host));
    } else {
        checks.extend(stored_host_diagnostic_checks_json(object));
    }
    if let Some(preflight) = object.get("preflight").and_then(Value::as_object) {
        checks.push(json!({
            "id": "mcp_preflight",
            "status": preflight.get("status").and_then(Value::as_str).unwrap_or("skipped"),
            "summary": preflight
                .get("details")
                .and_then(Value::as_str)
                .unwrap_or("stored MCP preflight state"),
        }));
    }
    if let Some(handshake) = object.get("mcp_handshake").and_then(Value::as_object) {
        checks.push(json!({
            "id": "mcp_handshake",
            "status": handshake.get("status").and_then(Value::as_str).unwrap_or("skipped"),
            "summary": handshake
                .get("details")
                .and_then(Value::as_str)
                .unwrap_or("stored MCP handshake state"),
        }));
    }
    checks
}

fn host_diagnostic_checks_json(host: &Verification) -> Vec<Value> {
    let mut checks = Vec::new();
    if let Some(trust) = &host.project_trust {
        checks.push(json!({
            "id": "codex_project_trust",
            "status": project_trust_check_status(trust.status),
            "summary": trust.details,
            "details": trust,
        }));
    }
    if let Some(runtime) = &host.host_runtime {
        checks.push(json!({
            "id": "codex_host_runtime",
            "status": host_runtime_check_status(runtime.status),
            "summary": runtime.details,
            "details": runtime,
        }));
    }
    if let Some(command) = &host.host_mcp_command {
        checks.push(json!({
            "id": "host_mcp_command",
            "status": host_mcp_command_check_status(command),
            "summary": command.details,
            "details": command,
        }));
    }
    checks
}

fn stored_host_diagnostic_checks_json(object: &serde_json::Map<String, Value>) -> Vec<Value> {
    let host = object.get("host").and_then(Value::as_object);
    let project_trust = object
        .get("project_trust")
        .or_else(|| host.and_then(|host| host.get("project_trust")));
    let host_runtime = object
        .get("host_runtime")
        .or_else(|| host.and_then(|host| host.get("host_runtime")));
    let host_mcp_command = object
        .get("host_mcp_command")
        .or_else(|| host.and_then(|host| host.get("host_mcp_command")));
    let mut checks = Vec::new();
    if let Some(trust) = project_trust.and_then(Value::as_object) {
        let status = trust
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        checks.push(json!({
            "id": "codex_project_trust",
            "status": stored_project_trust_check_status(status),
            "summary": trust.get("details").and_then(Value::as_str).unwrap_or("stored Codex project trust state"),
            "details": trust,
        }));
    }
    if let Some(runtime) = host_runtime.and_then(Value::as_object) {
        let status = runtime
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        checks.push(json!({
            "id": "codex_host_runtime",
            "status": stored_host_runtime_check_status(status),
            "summary": runtime.get("details").and_then(Value::as_str).unwrap_or("stored Codex host runtime state"),
            "details": runtime,
        }));
    }
    if let Some(command) = host_mcp_command.and_then(Value::as_object) {
        checks.push(json!({
            "id": "host_mcp_command",
            "status": if command.get("risk").is_some_and(|risk| !risk.is_null()) {
                "action_required"
            } else {
                "passed"
            },
            "summary": command.get("details").and_then(Value::as_str).unwrap_or("stored host MCP command state"),
            "details": command,
        }));
    }
    checks
}

fn project_trust_check_status(status: ProjectTrustStatus) -> &'static str {
    match status {
        ProjectTrustStatus::Trusted => "passed",
        ProjectTrustStatus::Untrusted => "action_required",
        ProjectTrustStatus::Missing | ProjectTrustStatus::Unknown => "unknown",
        ProjectTrustStatus::Unreadable | ProjectTrustStatus::Malformed => "failed",
    }
}

fn host_runtime_check_status(status: HostRuntimeObservationStatus) -> &'static str {
    match status {
        HostRuntimeObservationStatus::Observed => "passed",
        HostRuntimeObservationStatus::NotObserved => "action_required",
        HostRuntimeObservationStatus::Unknown => "unknown",
    }
}

fn host_mcp_command_check_status(command: &HostMcpCommandDiagnostic) -> &'static str {
    if command.risk.is_some() {
        "action_required"
    } else if command.mode == HostMcpCommandLaunchMode::Malformed {
        "failed"
    } else {
        "passed"
    }
}

fn stored_project_trust_check_status(status: &str) -> &'static str {
    match status {
        "trusted" => "passed",
        "untrusted" => "action_required",
        "missing" | "unknown" => "unknown",
        "unreadable" | "malformed" => "failed",
        _ => "unknown",
    }
}

fn stored_host_runtime_check_status(status: &str) -> &'static str {
    match status {
        "observed" => "passed",
        "not_observed" => "action_required",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

fn stored_user_actions(connection: &AgentConnectionRecord) -> Vec<UserAction> {
    serde_json::from_str::<Vec<UserAction>>(&connection.last_user_actions_json).unwrap_or_default()
}

fn connection_json(
    connection: &AgentConnectionRecord,
    project_ids: &[String],
    user_actions: Option<&[UserAction]>,
) -> Value {
    let user_actions = user_actions
        .map(|actions| serde_json::to_value(actions).unwrap_or_else(|_| json!([])))
        .unwrap_or_else(|| json_array_text(&connection.last_user_actions_json));
    json!({
        "connection_id": connection.connection_internal_id,
        "host_kind": connection.host_kind,
        "connection_intent": connection.intent,
        "host_scope": connection.host_scope,
        "mode": connection.mode,
        "enabled": connection.enabled,
        "connected_projects": project_ids,
        "verification_status": connection.last_verification_status,
        "verification_report": json_object_text(&connection.last_verification_report_json),
        "user_actions": user_actions,
        "server_name": connection.server_name,
        "config_target": connection.config_target,
    })
}

fn json_object_text(text: &str) -> Value {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn json_array_text(text: &str) -> Value {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]))
}

fn verification_json(report: &VerificationReport) -> Value {
    json!({
        "status": report.status.as_str(),
        "disclosure": detective_observation_disclosure_json(),
        "project_trust": &report.host.project_trust,
        "host_runtime": &report.host.host_runtime,
        "host_mcp_command": &report.host.host_mcp_command,
        "host": {
            "status": report.host.status.as_str(),
            "host_state": report.host.host_state.as_str(),
            "managed_config": report.host.managed_config.as_str(),
            "host_executable": report.host.host_executable.as_str(),
            "host_gate": report.host.host_gate.as_str(),
            "host_configuration": report.host.host_configuration.as_str(),
            "project_trust": &report.host.project_trust,
            "host_runtime": &report.host.host_runtime,
            "host_mcp_command": &report.host.host_mcp_command,
            "mcp_handshake_allowed": report.host.mcp_handshake_allowed,
            "details": report.host.details,
            "diagnostic": report.host.diagnostic,
            "user_actions": report.host.user_actions,
        },
        "preflight": step_json(&report.preflight),
        "mcp_handshake": step_json(&report.handshake),
        "tools": report.tools,
    })
}

fn detailed_verification_report_json(
    report: &VerificationReport,
) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(&verification_json(report))
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn user_actions_json(
    actions: &[crate::host_integration::UserAction],
) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(actions)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn step_json(step: &VerificationStep) -> Value {
    json!({
        "status": step.status.as_str(),
        "details": step.details,
    })
}

fn status_from_store(value: &str) -> AgentResultStatus {
    match value {
        VERIFIED_STATUS_COMPLETE => AgentResultStatus::Complete,
        VERIFIED_STATUS_ACTION_REQUIRED => AgentResultStatus::ActionRequired,
        VERIFIED_STATUS_FAILED => AgentResultStatus::Failed,
        _ => AgentResultStatus::NotVerified,
    }
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
