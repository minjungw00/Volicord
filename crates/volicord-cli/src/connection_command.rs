use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fmt, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime},
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
    StoreError,
};
use volicord_types::{GuardInstallationStatus, GuardMode};

use crate::host_integration::{
    claude_code::{ClaudeCodeAdapter, ProductionCommandRunner},
    codex::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest},
    export_file_name, format_supported_connection_intents,
    generic::{GenericAdapter, GenericExportRequest},
    supports_connection_intent,
    verification::{Verification, VerificationStatus},
    ConnectionIntent, HostAdapter, HostConfigError, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, ManagedServerEntry,
    PlannedChange, ProjectContext, UserAction, UserActionKind,
};
use crate::{
    managed_block::{self, ManagedBlockError, ManagedBlockWrite},
    registration::ADMIN_METADATA_JSON,
    setup_command::{is_executable_file, path_text as setup_path_text, runtime_home_id_for_path},
};

const VOLICORD_HOME: &str = "VOLICORD_HOME";
const PATH_ENV: &str = "PATH";
const AGENT_METADATA_CREATED_BY: &str = "volicord_cli_agent_connection";
const AGENT_RUNTIME_HOME_ID: &str = "runtime_home_agent";
const INIT_METADATA_CREATED_BY: &str = "volicord_cli_init";
const DEFAULT_MCP_COMMAND: &str = "volicord";
const DEFAULT_SERVER_NAME: &str = "volicord";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const INSTALLATION_ID: &str = "default";
const VOLICORD_POLICY_SCHEMA: &str = "volicord-policy-v1";
const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
const CLAUDE_RULE_FILE: &str = ".claude/rules/volicord.md";
const AGENTS_FILE: &str = "AGENTS.md";
const GUIDANCE_START_MARKER: &str = "<!-- BEGIN VOLICORD MANAGED GUIDANCE v1 -->";
const GUIDANCE_END_MARKER: &str = "<!-- END VOLICORD MANAGED GUIDANCE v1 -->";

const WORKFLOW_TOOL_NAMES: [&str; 10] = [
    "volicord.intake",
    "volicord.update_scope",
    "volicord.status",
    "volicord.prepare_write",
    "volicord.stage_artifact",
    "volicord.record_run",
    "volicord.request_user_judgment",
    "volicord.check_close",
    "volicord.close_task",
    "volicord.list_projects",
];
const READ_ONLY_TOOL_NAMES: [&str; 3] = [
    "volicord.status",
    "volicord.check_close",
    "volicord.list_projects",
];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProcessOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpLaunch {
    command: PathBuf,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<PathBuf>,
}

pub trait ConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        project_id: Option<&str>,
    ) -> Result<ConnectionProcessOutput, String>;
    fn verify_mcp_stdio(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, String>;
}

pub struct ProductionConnectionProcess;

impl ConnectionProcess for ProductionConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }

    fn run_preflight(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        project_id: Option<&str>,
    ) -> Result<ConnectionProcessOutput, String> {
        let mut child = Command::new(&launch.command);
        child
            .arg("mcp")
            .arg("--check")
            .arg("--connection")
            .arg(connection_id);
        if let Some(project_id) = project_id {
            child.arg("--project").arg(project_id);
        }
        apply_mcp_launch_context(&mut child, launch, runtime_home);
        child.stdin(Stdio::null());
        let output = child.output().map_err(|error| {
            format!(
                "failed to run {} mcp --check --connection {}: {error}",
                launch.command.display(),
                connection_id
            )
        })?;
        Ok(ConnectionProcessOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn verify_mcp_stdio(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, String> {
        verify_mcp_stdio_process(launch, runtime_home, connection_id, mode, DEFAULT_TIMEOUT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
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
pub struct McpVerification {
    step: VerificationStep,
    tools: Vec<String>,
}

impl McpVerification {
    fn passed(tools: Vec<String>) -> Self {
        Self {
            step: VerificationStep::passed(format!("tools/list returned {} tools", tools.len())),
            tools,
        }
    }

    fn failed(details: impl Into<String>) -> Self {
        Self {
            step: VerificationStep::failed(details),
            tools: Vec::new(),
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

pub fn init_usage() -> String {
    "volicord init --host codex|claude-code --repo PATH [--mode mcp-only|guarded|managed] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]\n"
        .to_owned()
}

pub fn connect_usage() -> String {
    "volicord connect [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]\n"
        .to_owned()
}

pub fn connections_usage() -> String {
    "volicord connections [--repo PATH] [--json]\n".to_owned()
}

pub fn connection_usage() -> String {
    format!(
        "{}{}{}{}",
        connection_status_usage(),
        connection_verify_usage(),
        connection_mode_usage(),
        connection_remove_usage()
    )
}

fn connection_status_usage() -> String {
    "volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]\n".to_owned()
}

fn connection_verify_usage() -> String {
    "volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]\n".to_owned()
}

fn connection_mode_usage() -> String {
    "volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]\n"
        .to_owned()
}

fn connection_remove_usage() -> String {
    "volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]\n"
        .to_owned()
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
    let repo_root = resolve_connection_repo_root(current_dir, Some(repo))?;
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
            export_target: None,
            export_dir: None,
            current_dir,
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
            export_target: None,
            export_dir: None,
            current_dir,
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
    let integration_plan = apply_guard_integration(integration_plan)?;
    let installation_status = initial_guard_installation_status(&host_plan, &integration_plan);
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
    let user_actions = init_user_actions(&verification.host.user_actions, host_kind);
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedConnectionOptions {
    host_kind: Option<HostKind>,
    repo: Option<PathBuf>,
    shared: bool,
    global: bool,
    read_only: bool,
    dry_run: bool,
    json: bool,
    positionals: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum InitMode {
    McpOnly,
    #[default]
    Guarded,
    Managed,
}

impl InitMode {
    fn cli_value(self) -> &'static str {
        match self {
            Self::McpOnly => "mcp-only",
            Self::Guarded => "guarded",
            Self::Managed => "managed",
        }
    }

    fn guard_value(self) -> &'static str {
        match self {
            Self::McpOnly => GuardMode::McpOnly.as_str(),
            Self::Guarded => GuardMode::Guarded.as_str(),
            Self::Managed => GuardMode::Managed.as_str(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedInitOptions {
    host_kind: Option<HostKind>,
    repo: Option<PathBuf>,
    runtime_home: Option<PathBuf>,
    mcp_command: Option<PathBuf>,
    mode: InitMode,
    dry_run: bool,
    json: bool,
}

#[derive(Debug, Clone)]
struct ConnectionSelector {
    host_kind: HostKind,
    intent: ConnectionIntent,
    host_scope: HostScope,
    repo_root: PathBuf,
}

pub fn run_connect_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl ConnectionProcess,
) -> Result<String, ConnectionCommandError> {
    if is_help_request(args) {
        return Ok(connect_usage());
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
            export_target: None,
            export_dir: None,
            current_dir,
        },
        process,
    )?;
    if let Some(conflict) = host_plan.conflicts.first() {
        return Err(ConnectionCommandError::runtime(conflict.message.clone()));
    }
    if parsed.dry_run {
        return render_simplified_plan_output(SimplifiedPlanOutput {
            format: connection_output_format(&parsed),
            action: "connect",
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
            export_target: None,
            export_dir: None,
            current_dir,
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
    render_simplified_connection_output(SimplifiedConnectionOutput {
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
        verification: Some(&verification),
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
        return Ok(connections_usage());
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
    render_simplified_connections_output(connection_output_format(&parsed), &rows)
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
    render_simplified_connection_output(SimplifiedConnectionOutput {
        format: connection_output_format(&parsed),
        action: "status",
        status: status_from_store(&connection.last_verification_status),
        runtime_home: &runtime_home,
        guard_state: guard_state_for_connection(
            &runtime_home,
            &connection.connection_internal_id,
            &projects,
        )?,
        user_actions: stored_user_actions(&connection),
        connection: &connection,
        projects: &projects,
        verification: None,
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
    render_simplified_connection_output(SimplifiedConnectionOutput {
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
        verification: Some(&verification),
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
    render_simplified_connection_output(SimplifiedConnectionOutput {
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
        verification: None,
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
            .map(SimplifiedRemovePlan::Host)
            .unwrap_or(SimplifiedRemovePlan::MembershipOnly);
        return render_simplified_remove_dry_run(
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
    render_simplified_connection_output(SimplifiedConnectionOutput {
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
        verification: None,
        plan: host_plan.as_ref(),
    })
}

fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}

fn parse_connection_options(
    args: &[String],
    allowed: &[&str],
    max_positionals: usize,
) -> Result<ParsedConnectionOptions, ConnectionCommandError> {
    let mut parsed = ParsedConnectionOptions::default();
    let mut seen = BTreeSet::new();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(ConnectionCommandError::usage(connection_usage()));
        }
        if !token.starts_with("--") {
            parsed.positionals.push(token.clone());
            index += 1;
            continue;
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if is_boolean_connection_option(without_prefix) {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ConnectionCommandError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };

        if !allowed.iter().any(|allowed_name| *allowed_name == name) {
            return Err(ConnectionCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !seen.insert(name.clone()) {
            return Err(ConnectionCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        set_connection_option(&mut parsed, &name, value.as_deref())?;
        index += 1;
    }

    if parsed.positionals.len() > max_positionals {
        return Err(ConnectionCommandError::usage(format!(
            "unexpected argument: {}",
            parsed.positionals[max_positionals]
        )));
    }
    if max_positionals == 1 {
        if let Some(host) = parsed.positionals.first() {
            parsed.host_kind = Some(parse_public_host_kind(host)?);
        }
    }
    if parsed.shared && parsed.global {
        return Err(ConnectionCommandError::usage(
            "--shared and --global are mutually exclusive",
        ));
    }
    Ok(parsed)
}

fn parse_init_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedInitOptions, ConnectionCommandError> {
    let mut parsed = ParsedInitOptions {
        mode: InitMode::Guarded,
        ..ParsedInitOptions::default()
    };
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if matches!(token.as_str(), "-h" | "--help" | "help") {
            return Err(ConnectionCommandError::usage(init_usage()));
        }
        if !token.starts_with("--") {
            return Err(ConnectionCommandError::usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if matches!(without_prefix, "dry-run" | "json") {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ConnectionCommandError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };
        if !matches!(
            name.as_str(),
            "host" | "repo" | "mode" | "home" | "mcp-command" | "dry-run" | "json"
        ) {
            return Err(ConnectionCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !seen.insert(name.clone()) {
            return Err(ConnectionCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        match name.as_str() {
            "host" => {
                parsed.host_kind = Some(parse_public_host_kind(&value_text(
                    &name,
                    value.as_deref(),
                )?)?)
            }
            "repo" => {
                parsed.repo = Some(absolute_path(
                    current_dir,
                    value_path(&name, value.as_deref())?,
                ))
            }
            "mode" => parsed.mode = parse_init_mode(&value_text(&name, value.as_deref())?)?,
            "home" => {
                parsed.runtime_home = Some(absolute_path(
                    current_dir,
                    value_path(&name, value.as_deref())?,
                ));
            }
            "mcp-command" => {
                parsed.mcp_command = Some(absolute_path(
                    current_dir,
                    value_path(&name, value.as_deref())?,
                ));
            }
            "dry-run" => {
                reject_boolean_value(&name, value.as_deref())?;
                parsed.dry_run = true;
            }
            "json" => {
                reject_boolean_value(&name, value.as_deref())?;
                parsed.json = true;
            }
            _ => unreachable!("validated option name"),
        }
        index += 1;
    }
    Ok(parsed)
}

fn parse_init_mode(value: &str) -> Result<InitMode, ConnectionCommandError> {
    match value {
        "mcp-only" | "mcp_only" => Ok(InitMode::McpOnly),
        "guarded" => Ok(InitMode::Guarded),
        "managed" => Ok(InitMode::Managed),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown init mode: {other}; use mcp-only, guarded, or managed"
        ))),
    }
}

fn init_output_format(parsed: &ParsedInitOptions) -> OutputFormat {
    if parsed.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

fn is_boolean_connection_option(name: &str) -> bool {
    matches!(name, "shared" | "global" | "read-only" | "dry-run" | "json")
}

fn set_connection_option(
    parsed: &mut ParsedConnectionOptions,
    name: &str,
    value: Option<&str>,
) -> Result<(), ConnectionCommandError> {
    match name {
        "repo" => parsed.repo = Some(value_path(name, value)?),
        "shared" => {
            reject_boolean_value(name, value)?;
            parsed.shared = true;
        }
        "global" => {
            reject_boolean_value(name, value)?;
            parsed.global = true;
        }
        "read-only" => {
            reject_boolean_value(name, value)?;
            parsed.read_only = true;
        }
        "dry-run" => {
            reject_boolean_value(name, value)?;
            parsed.dry_run = true;
        }
        "json" => {
            reject_boolean_value(name, value)?;
            parsed.json = true;
        }
        _ => {
            return Err(ConnectionCommandError::usage(format!(
                "unknown option: --{name}"
            )))
        }
    }
    Ok(())
}

fn reject_boolean_value(name: &str, value: Option<&str>) -> Result<(), ConnectionCommandError> {
    if value.is_some() {
        Err(ConnectionCommandError::usage(format!(
            "--{name} does not accept a value"
        )))
    } else {
        Ok(())
    }
}

fn connection_output_format(parsed: &ParsedConnectionOptions) -> OutputFormat {
    if parsed.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
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
            "generic MCP export is not a host connection; use the export command",
        )),
        (HostKind::Codex, ConnectionIntent::Global) => unreachable!("validated above"),
    }
}

fn unsupported_connection_intent_message(host_kind: HostKind, intent: ConnectionIntent) -> String {
    let supported = format_supported_connection_intents(host_kind);
    if host_kind == HostKind::Generic {
        return format!(
            "UNSUPPORTED_HOST: generic MCP export is not a host connection; use `volicord export mcp-config`; supported connection intents: {supported}"
        );
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
    let intent = connection_intent_from_flags(parsed)?;
    let host_scope = host_scope_for_intent(host_kind, intent)?;
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

fn parse_public_host_kind(value: &str) -> Result<HostKind, ConnectionCommandError> {
    match value {
        HOST_KIND_CODEX => Ok(HostKind::Codex),
        "claude-code" | HOST_KIND_CLAUDE_CODE => Ok(HostKind::ClaudeCode),
        other => Err(ConnectionCommandError::usage(format!(
            "UNSUPPORTED_HOST: unknown host: {other}; choose `codex` or `claude-code`"
        ))),
    }
}

fn parse_user_connection_mode(value: &str) -> Result<String, ConnectionCommandError> {
    match value {
        "workflow" => Ok(CONNECTION_MODE_WORKFLOW.to_owned()),
        "read-only" => Ok(CONNECTION_MODE_READ_ONLY.to_owned()),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown connection mode: {other}; use `workflow` or `read-only`"
        ))),
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
            "PROJECT_NOT_REGISTERED: repository {} is not registered; run `volicord connect {}{} --repo {}` first",
            selector.repo_root.display(),
            public_host_label(selector.host_kind),
            intent_flag_suffix(selector.intent),
            selector.repo_root.display()
        )));
    }
    let mut matches = Vec::new();
    let mut same_host_connections = Vec::new();
    for connection in list_agent_connections(runtime_home)? {
        if connection.host_kind != selector.host_kind.as_str()
            || connection.intent != selector.intent.as_str()
            || connection.host_scope != selector.host_scope.as_str()
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
            "CONNECTION_NOT_FOUND: no Agent Connection matches host {}, intent {}, and repository {}; run `volicord connect {}{} --repo {}`",
            public_host_label(selector.host_kind),
            selector.intent.as_str(),
            selector.repo_root.display(),
            public_host_label(selector.host_kind),
            intent_flag_suffix(selector.intent),
            selector.repo_root.display()
        ))),
        0 => Err(ConnectionCommandError::runtime(format!(
            "CONNECTION_ALLOWLIST_MISMATCH: repository {} is not in the selected Agent Connection project allowlist; run `volicord connect {}{} --repo {}`",
            selector.repo_root.display(),
            public_host_label(selector.host_kind),
            intent_flag_suffix(selector.intent),
            selector.repo_root.display()
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

fn intent_flag_suffix(intent: ConnectionIntent) -> &'static str {
    match intent {
        ConnectionIntent::Personal => "",
        ConnectionIntent::Shared => " --shared",
        ConnectionIntent::Global => " --global",
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
        selector.intent.as_str(),
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

fn value_text(name: &str, value: Option<&str>) -> Result<String, ConnectionCommandError> {
    let value = value
        .ok_or_else(|| ConnectionCommandError::usage(format!("missing value for --{name}")))?;
    if value.trim().is_empty() {
        Err(ConnectionCommandError::usage(format!(
            "--{name} must not be empty"
        )))
    } else {
        Ok(value.to_owned())
    }
}

fn value_path(name: &str, value: Option<&str>) -> Result<PathBuf, ConnectionCommandError> {
    Ok(PathBuf::from(value_text(name, value)?))
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
            "SETUP_REQUIRED: installation profile is missing for Runtime Home {}; run `volicord setup` before connection workflows",
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
            "generic MCP export is not a host connection; use the export command",
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
    export_target: Option<&'a Path>,
    export_dir: Option<&'a Path>,
    current_dir: &'a Path,
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
        HostKind::Generic => {
            let adapter = GenericAdapter;
            let output_dir = request.export_dir.unwrap_or(request.current_dir);
            let target_path = request
                .export_target
                .map(Path::to_path_buf)
                .unwrap_or_else(|| output_dir.join(export_file_name(request.connection_id)));
            adapter
                .plan_export(GenericExportRequest {
                    connection_id: request.connection_id,
                    installation_profile: request.installation_profile,
                    mode: request.mode,
                    target_path: &target_path,
                    expected_fingerprint: request.expected_fingerprint,
                })
                .map_err(Into::into)
        }
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
            "generic export must be loaded, trusted, or approved in the target host by the user",
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
    let host = verify_host_plan(host_kind, host_plan, process)?;
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

fn aggregate_verification_status(
    host: &Verification,
    preflight: &VerificationStep,
    handshake: &VerificationStep,
) -> AgentResultStatus {
    if preflight.status == StepStatus::Failed || handshake.status == StepStatus::Failed {
        return AgentResultStatus::Failed;
    }
    match host.status {
        VerificationStatus::Complete if handshake.status == StepStatus::Passed => {
            AgentResultStatus::Complete
        }
        VerificationStatus::ActionRequired if handshake.status == StepStatus::Passed => {
            AgentResultStatus::ActionRequired
        }
        VerificationStatus::NotVerified => AgentResultStatus::NotVerified,
        _ => AgentResultStatus::Failed,
    }
}

fn run_connection_preflight(
    process: &mut impl ConnectionProcess,
    launch: &McpLaunch,
    runtime_home: &Path,
    connection_id: &str,
    project_id: Option<&str>,
    mode: &str,
) -> VerificationStep {
    match process.run_preflight(launch, runtime_home, connection_id, project_id) {
        Ok(output) if output.success => {
            match validate_connection_preflight_report(&output.stdout, connection_id, mode) {
                Ok(()) => VerificationStep::passed("volicord mcp preflight passed"),
                Err(message) => VerificationStep::failed(message),
            }
        }
        Ok(output) => VerificationStep::failed(format!(
            "volicord mcp preflight failed with status {}; stderr: {}",
            status_text(output.status_code),
            compact_stream(&output.stderr)
        )),
        Err(message) => VerificationStep::failed(message),
    }
}

fn validate_connection_preflight_report(
    stdout: &str,
    connection_id: &str,
    mode: &str,
) -> Result<(), String> {
    let report = parse_colon_report(stdout)?;
    expect_report_field(&report, "configuration", "valid")?;
    expect_report_field(&report, "transport", "stdio")?;
    expect_report_field(&report, "connection_id", connection_id)?;
    expect_report_field(&report, "mode", mode)?;
    expect_report_field(&report, "enabled", "true")?;
    Ok(())
}

fn parse_colon_report(stdout: &str) -> Result<BTreeMap<String, String>, String> {
    let mut report = BTreeMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once(':') {
            report.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    if report.is_empty() {
        Err("preflight did not return a key-value report".to_owned())
    } else {
        Ok(report)
    }
}

fn expect_report_field(
    report: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    match report.get(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!(
            "preflight field {key} was {actual}, expected {expected}"
        )),
        None => Err(format!("preflight field {key} was missing")),
    }
}

fn mcp_launch_from_host_plan(plan: &HostPlan, repo_root: Option<&Path>) -> McpLaunch {
    let cwd = match plan.host_scope {
        HostScope::Project | HostScope::Local => repo_root.map(Path::to_path_buf),
        HostScope::User | HostScope::Export => None,
    };
    McpLaunch {
        command: PathBuf::from(&plan.entry.command),
        args: plan.entry.args.clone(),
        env: plan.entry.env.clone(),
        cwd,
    }
}

fn apply_mcp_launch_context(command: &mut Command, launch: &McpLaunch, runtime_home: &Path) {
    command.env(VOLICORD_HOME, runtime_home);
    for (key, value) in &launch.env {
        command.env(key, value);
    }
    if let Some(cwd) = &launch.cwd {
        command.current_dir(cwd);
    }
}

fn verify_mcp_stdio_process(
    launch: &McpLaunch,
    runtime_home: &Path,
    connection_id: &str,
    mode: &str,
    timeout: Duration,
) -> Result<McpVerification, String> {
    let mut child = Command::new(&launch.command);
    child.args(&launch.args);
    apply_mcp_launch_context(&mut child, launch, runtime_home);
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn().map_err(|error| {
        format!(
            "failed to launch {} for MCP handshake with connection {}: {error}",
            launch.command.display(),
            connection_id
        )
    })?;
    let deadline = Instant::now() + timeout;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture MCP stdout".to_owned())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open MCP stdin".to_owned())?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Ok(None));
                    break;
                }
                Ok(_) => {
                    let _ = tx.send(Ok(Some(line)));
                }
                Err(error) => {
                    let _ = tx.send(Err(error.to_string()));
                    break;
                }
            }
        }
    });

    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "volicord-cli", "version": env!("CARGO_PKG_VERSION")}
            }
        }),
    )?;
    let initialize = read_json_response(&rx, deadline)?;
    validate_initialize_response(&initialize)?;
    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    )?;
    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )?;
    let tools = validate_tools_response(&read_json_response(&rx, deadline)?)?;
    validate_tools_for_mode(mode, &tools)?;
    drop(stdin);
    terminate_child(&mut child, deadline)?;
    Ok(McpVerification::passed(tools))
}

fn write_json_line(writer: &mut impl Write, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, &value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn read_json_response(
    rx: &mpsc::Receiver<Result<Option<String>, String>>,
    deadline: Instant,
) -> Result<Value, String> {
    let now = Instant::now();
    if now >= deadline {
        return Err("MCP handshake timed out".to_owned());
    }
    match rx.recv_timeout(deadline.saturating_duration_since(now)) {
        Ok(Ok(Some(line))) => serde_json::from_str::<Value>(&line)
            .map_err(|error| format!("invalid MCP JSON response: {error}; line: {line}")),
        Ok(Ok(None)) => Err("MCP process exited before response".to_owned()),
        Ok(Err(error)) => Err(format!("failed reading MCP response: {error}")),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("MCP handshake timed out".to_owned()),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("MCP response reader disconnected".to_owned())
        }
    }
}

fn validate_initialize_response(value: &Value) -> Result<(), String> {
    if value.get("error").is_some() {
        return Err(format!("MCP initialize returned error: {value}"));
    }
    let result = value
        .get("result")
        .ok_or_else(|| "MCP initialize response missing result".to_owned())?;
    if result
        .get("instructions")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("MCP initialize response missing instructions".to_owned());
    }
    Ok(())
}

fn validate_tools_response(value: &Value) -> Result<Vec<String>, String> {
    if value.get("error").is_some() {
        return Err(format!("MCP tools/list returned error: {value}"));
    }
    let tools = value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .ok_or_else(|| "MCP tools/list response missing result.tools".to_owned())?;
    let mut names = Vec::new();
    for tool in tools {
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "MCP tool entry missing name".to_owned())?;
        names.push(name.to_owned());
    }
    Ok(names)
}

fn validate_tools_for_mode(mode: &str, tools: &[String]) -> Result<(), String> {
    let expected = match mode {
        CONNECTION_MODE_READ_ONLY => READ_ONLY_TOOL_NAMES.as_slice(),
        CONNECTION_MODE_WORKFLOW => WORKFLOW_TOOL_NAMES.as_slice(),
        other => {
            return Err(format!(
                "unsupported connection mode for tool validation: {other}"
            ))
        }
    };
    for name in expected {
        if !tools.iter().any(|tool| tool == name) {
            return Err(format!("MCP tools/list missing required tool: {name}"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct GuardIntegrationPlan {
    generated_files: Vec<GeneratedFilePlan>,
    policy: Value,
    policy_hash: String,
    guard_installation_id: String,
}

#[derive(Debug, Clone)]
struct GeneratedFilePlan {
    kind: &'static str,
    path: PathBuf,
    content: String,
    status: FilePlanStatus,
    write_kind: GeneratedFileWriteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedFileWriteKind {
    ManagedBlock {
        start_marker: &'static str,
        end_marker: &'static str,
        require_existing_marker: bool,
    },
    ManagedJson,
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
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
) -> Result<GuardIntegrationPlan, ConnectionCommandError> {
    let guard_commands = guard_command_specs(
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        init_mode,
    );
    let policy = policy_json(
        host_kind,
        init_mode,
        repo_root,
        connection_id,
        guard_installation_id,
        mcp_entry,
        &guard_commands,
    );
    let mut generated_files = Vec::new();
    let agents_path = repo_root.join(AGENTS_FILE);
    generated_files.push(plan_managed_block_file(
        "agents_guidance",
        &agents_path,
        &agents_guidance_block(),
        GUIDANCE_START_MARKER,
        GUIDANCE_END_MARKER,
        false,
    )?);
    let policy_path = repo_root.join(VOLICORD_POLICY_FILE);
    generated_files.push(plan_policy_file(&policy_path, &policy)?);
    if host_kind == HostKind::ClaudeCode && init_mode != InitMode::McpOnly {
        let rule_path = repo_root.join(CLAUDE_RULE_FILE);
        generated_files.push(plan_managed_block_file(
            "claude_rule",
            &rule_path,
            &claude_rule_block(),
            GUIDANCE_START_MARKER,
            GUIDANCE_END_MARKER,
            true,
        )?);
    }
    let policy_hash = policy_hash(&policy)?;
    Ok(GuardIntegrationPlan {
        generated_files,
        policy,
        policy_hash,
        guard_installation_id: guard_installation_id.to_owned(),
    })
}

fn apply_guard_integration(
    mut plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, ConnectionCommandError> {
    for file in &mut plan.generated_files {
        file.status = match file.write_kind {
            GeneratedFileWriteKind::ManagedBlock {
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
            GeneratedFileWriteKind::ManagedJson => {
                write_managed_json_file(&file.path, &file.policy_value()?)?
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
    kind: &'static str,
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
                    kind,
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
        write_kind: GeneratedFileWriteKind::ManagedBlock {
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
        kind: "policy",
        path: path.to_path_buf(),
        content,
        status,
        write_kind: GeneratedFileWriteKind::ManagedJson,
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

fn claude_rule_block() -> String {
    format!(
        "{GUIDANCE_START_MARKER}\n# Volicord\n\nUse the repository-local `.volicord/policy.json` guard commands for session start, tool checks, prompt capture, and stop checks. Do not record user-owned judgments through the Agent Connection.\n{GUIDANCE_END_MARKER}\n"
    )
}

fn guard_command_specs(
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host_kind: HostKind,
    init_mode: InitMode,
) -> BTreeMap<String, GuardCommandSpec> {
    [
        "session-start",
        "pre-tool",
        "post-tool",
        "prompt-capture",
        "stop",
    ]
    .into_iter()
    .map(|phase| {
        let mut args = vec![
            "guard".to_owned(),
            phase.to_owned(),
            "--repo".to_owned(),
            path_text(repo_root),
            "--connection".to_owned(),
            connection_id.to_owned(),
            "--guard-installation".to_owned(),
            guard_installation_id.to_owned(),
            "--host".to_owned(),
            public_host_label(host_kind).to_owned(),
            "--guard-mode".to_owned(),
            init_mode.cli_value().to_owned(),
        ];
        args.push("--json".to_owned());
        (
            phase.replace('-', "_"),
            GuardCommandSpec {
                command: DEFAULT_MCP_COMMAND.to_owned(),
                args,
            },
        )
    })
    .collect()
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
        "mode": init_mode.cli_value(),
        "guard_mode": init_mode.guard_value(),
        "mcp": {
            "command": &mcp_entry.command,
            "args": &mcp_entry.args,
            "env": &mcp_entry.env,
        },
        "guard": {
            "enabled": init_mode != InitMode::McpOnly,
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
            host_capability_json: guard_capability_json(integration)?,
            installation_status: installation_status.as_str().to_owned(),
            installed_at: (init_mode != InitMode::McpOnly).then_some(now.clone()),
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
            }))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        },
    )
    .map_err(Into::into)
}

fn guard_capability_json(plan: &GuardIntegrationPlan) -> Result<String, ConnectionCommandError> {
    serde_json::to_string(&json!({
        "schema": "volicord-guard-capability-v1",
        "policy_hash": plan.policy_hash,
        "prompt_capture": guard_has_prompt_capture_commands(&plan.policy),
        "files": generated_files_json(&plan.generated_files),
        "commands": plan.policy["guard"]["commands"].clone(),
    }))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

fn initial_guard_installation_status(
    host_plan: &HostPlan,
    integration: &GuardIntegrationPlan,
) -> GuardInstallationStatus {
    if host_plan.change != PlannedChange::Noop
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

fn guard_has_prompt_capture_commands(policy: &Value) -> bool {
    policy
        .get("guard")
        .and_then(|guard| guard.get("commands"))
        .and_then(|commands| commands.get("prompt_capture"))
        .is_some()
}

fn generated_files_json(files: &[GeneratedFilePlan]) -> Value {
    Value::Array(
        files
            .iter()
            .map(|file| {
                json!({
                    "kind": file.kind,
                    "path": path_text(&file.path),
                    "status": file.status.as_str(),
                })
            })
            .collect(),
    )
}

fn init_user_actions(existing: &[UserAction], host_kind: HostKind) -> Vec<UserAction> {
    let mut actions = existing.to_vec();
    actions.push(UserAction::new(
        UserActionKind::ReloadRequired,
        format!(
            "Restart or reload {} so it loads the Volicord MCP and guard configuration",
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
    installation_state: String,
    files_state: String,
    hook_observed_state: String,
    last_observed_at: Option<String>,
    last_guard_event_at: Option<String>,
    prompt_capture_state: String,
    missing_files: Vec<String>,
    unresolved_blockers: Vec<String>,
}

impl GuardOperationalState {
    fn not_configured() -> Self {
        Self {
            mode_state: "not_configured".to_owned(),
            installation_state: "not_configured".to_owned(),
            files_state: "not_configured".to_owned(),
            hook_observed_state: "not_observed".to_owned(),
            last_observed_at: None,
            last_guard_event_at: None,
            prompt_capture_state: "unavailable".to_owned(),
            missing_files: Vec::new(),
            unresolved_blockers: Vec::new(),
        }
    }

    fn planned(init_mode: InitMode) -> Self {
        Self {
            mode_state: init_mode.guard_value().to_owned(),
            installation_state: "planned".to_owned(),
            files_state: if init_mode == InitMode::McpOnly {
                "disabled".to_owned()
            } else {
                "planned".to_owned()
            },
            hook_observed_state: if init_mode == InitMode::McpOnly {
                "disabled".to_owned()
            } else {
                "not_observed".to_owned()
            },
            last_observed_at: None,
            last_guard_event_at: None,
            prompt_capture_state: if init_mode == InitMode::McpOnly {
                "disabled".to_owned()
            } else {
                "planned".to_owned()
            },
            missing_files: Vec::new(),
            unresolved_blockers: Vec::new(),
        }
    }

    fn init(health: &str, init_mode: InitMode) -> Self {
        Self {
            mode_state: init_mode.guard_value().to_owned(),
            installation_state: health.to_owned(),
            files_state: if init_mode == InitMode::McpOnly {
                "disabled".to_owned()
            } else {
                "installed".to_owned()
            },
            hook_observed_state: if init_mode == InitMode::McpOnly {
                "disabled".to_owned()
            } else if health == GuardInstallationStatus::Active.as_str() {
                "observed".to_owned()
            } else {
                "not_observed".to_owned()
            },
            last_observed_at: None,
            last_guard_event_at: None,
            prompt_capture_state: if init_mode == InitMode::McpOnly {
                "disabled".to_owned()
            } else if health == GuardInstallationStatus::Active.as_str() {
                "available".to_owned()
            } else if health == GuardInstallationStatus::ReloadRequired.as_str() {
                "reload_required".to_owned()
            } else if health == GuardInstallationStatus::Configured.as_str() {
                "configured".to_owned()
            } else {
                "unavailable".to_owned()
            },
            missing_files: Vec::new(),
            unresolved_blockers: guard_blockers_for_state(
                init_mode.guard_value(),
                health,
                health == GuardInstallationStatus::Active.as_str(),
            ),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "mode": &self.mode_state,
            "installation": &self.installation_state,
            "files": &self.files_state,
            "hook_observed": &self.hook_observed_state,
            "last_observed_at": &self.last_observed_at,
            "last_guard_event_at": &self.last_guard_event_at,
            "prompt_capture": &self.prompt_capture_state,
            "missing_files": &self.missing_files,
            "unresolved_blockers": &self.unresolved_blockers,
        })
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

struct SimplifiedConnectionOutput<'a> {
    format: OutputFormat,
    action: &'a str,
    status: AgentResultStatus,
    runtime_home: &'a Path,
    guard_state: GuardOperationalState,
    connection: &'a AgentConnectionRecord,
    projects: &'a [ConnectionProjectRecord],
    verification: Option<&'a VerificationReport>,
    plan: Option<&'a HostPlan>,
    user_actions: Vec<UserAction>,
}

struct SimplifiedPlanOutput<'a> {
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

enum SimplifiedRemovePlan<'a> {
    Host(&'a HostPlan),
    MembershipOnly,
}

fn render_simplified_connection_output(
    data: SimplifiedConnectionOutput<'_>,
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
    let mcp_config_state =
        connection_mcp_config_state(data.connection, data.verification, data.plan);
    let primary_next_action = primary_connection_action(
        &data.user_actions,
        data.verification,
        &data.guard_state,
        Some(data.connection),
        data.projects,
    );
    match data.format {
        OutputFormat::Text => {
            let mut output = format!(
                "Agent Connection {}\nruntime_home_state: ready\nruntime_home: {}\nconnection_state: {}\nhost: {}\nintent: {}\nmode: {}\nenabled: {}\nproject_registration_state: {}\nconnected_repositories: {}\nmcp_config_state: {}\nmcp_config: {}\nguard_mode: {}\nguard_installation_state: {}\nguard_files_state: {}\nguard_hook_observed: {}\nlast_guard_event: {}\nprompt_capture_state: {}\nhost_reload_required: {}\nguard_blockers: {}\n",
                data.action,
                data.runtime_home.display(),
                data.status.as_str(),
                public_host_name_text(&data.connection.host_kind),
                data.connection.intent,
                public_mode_text(&data.connection.mode),
                data.connection.enabled,
                project_registration_state(data.projects),
                display_project_roots(data.projects),
                mcp_config_state,
                target
                ,
                data.guard_state.mode_state,
                data.guard_state.installation_state,
                data.guard_state.files_state,
                data.guard_state.hook_observed_state,
                optional_text(data.guard_state.last_guard_event_at.as_deref()),
                data.guard_state.prompt_capture_state,
                yes_no(has_reload_action(&data.user_actions)),
                comma_or_none(&data.guard_state.unresolved_blockers)
            );
            if let Some(planned_change) = planned_change {
                output.push_str(&format!("planned_change: {planned_change}\n"));
            }
            if let Some(verification) = data.verification {
                output.push_str(&format!(
                    "host_verification: {}\npreflight: {}\nmcp_handshake: {}\n",
                    verification.host.status.as_str(),
                    verification.preflight.status.as_str(),
                    verification.handshake.status.as_str()
                ));
            }
            append_primary_next_action_text(&mut output, primary_next_action.as_ref());
            Ok(output)
        }
        OutputFormat::Json => {
            let value = json!({
                "action": data.action,
                "status": data.status.as_str(),
                "runtime_home": path_text(data.runtime_home),
                "states": connection_states_json(
                    data.status.as_str(),
                    project_registration_state(data.projects),
                    mcp_config_state.as_str(),
                    &data.guard_state,
                    has_reload_action(&data.user_actions),
                ),
                "connection": connection_json(data.connection, &project_ids),
                "target": target,
                "planned_change": planned_change,
                "checks": checks_json(data.connection, data.verification, &data.guard_state),
                "actions": actions_json_values(&data.user_actions),
                "primary_next_action": primary_next_action.map(|action| action.to_json()),
                "guard": data.guard_state.to_json(),
                "verification": data.verification.map(verification_json),
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn render_simplified_plan_output(
    data: SimplifiedPlanOutput<'_>,
) -> Result<String, ConnectionCommandError> {
    let target = host_target_text(&data.plan.target);
    let planned_change = planned_change_text(data.plan.change);
    let guard_state = GuardOperationalState::not_configured();
    let primary_next_action =
        primary_connection_action(&data.user_actions, None, &guard_state, None, &[]);
    let project_state = data.repo_root.map(|_| "planned").unwrap_or("not_selected");
    match data.format {
        OutputFormat::Text => {
            let mut output = format!(
                "Agent Connection {} {}\nruntime_home_state: ready\nruntime_home: {}\nconnection_state: {}\nhost: {}\nintent: {}\nmode: {}\nenabled: {}\nproject_registration_state: {}\nconnected_repositories: {}\nmcp_config_state: planned_{}\nmcp_config: {}\nguard_mode: {}\nguard_installation_state: {}\nguard_files_state: {}\nguard_hook_observed: {}\nlast_guard_event: {}\nprompt_capture_state: {}\nhost_reload_required: {}\nguard_blockers: {}\nplanned_change: {}\n",
                data.action,
                data.status.as_str(),
                data.runtime_home.display(),
                data.status.as_str(),
                public_host_label(data.host_kind),
                data.intent.as_str(),
                public_mode_text(data.mode),
                data.enabled,
                project_state,
                data.repo_root
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                planned_change,
                target,
                guard_state.mode_state,
                guard_state.installation_state,
                guard_state.files_state,
                guard_state.hook_observed_state,
                optional_text(guard_state.last_guard_event_at.as_deref()),
                guard_state.prompt_capture_state,
                yes_no(has_reload_action(&data.user_actions)),
                comma_or_none(&guard_state.unresolved_blockers),
                planned_change
            );
            if let Some(remaining) = data.projects_remaining {
                output.push_str(&format!("remaining_connected_projects: {remaining}\n"));
            }
            append_primary_next_action_text(&mut output, primary_next_action.as_ref());
            Ok(output)
        }
        OutputFormat::Json => {
            let connected_repositories = data
                .repo_root
                .into_iter()
                .map(path_text)
                .collect::<Vec<_>>();
            let value = json!({
                "action": data.action,
                "status": data.status.as_str(),
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
                "guard": guard_state.to_json(),
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn render_init_output(data: InitOutput<'_>) -> Result<String, ConnectionCommandError> {
    let target = host_target_text(&data.host_plan.target);
    let planned_change = planned_change_text(data.host_plan.change);
    let actions = if data.status == AgentResultStatus::DryRun {
        data.host_plan.user_actions.clone()
    } else {
        data.verification
            .map(|verification| init_user_actions(&verification.host.user_actions, data.host_kind))
            .unwrap_or_else(|| init_user_actions(&data.host_plan.user_actions, data.host_kind))
    };
    let guard_status = data
        .guard_installation
        .map(|guard| guard.installation_status.as_str())
        .unwrap_or(GuardInstallationStatus::Configured.as_str());
    let guard_state = if data.guard_installation.is_some() {
        GuardOperationalState::init(guard_status, data.init_mode)
    } else {
        GuardOperationalState::planned(data.init_mode)
    };
    let mcp_config_state = init_mcp_config_state(data.verification, Some(data.host_plan));
    let project_state = if data.project_id.is_some() {
        "registered"
    } else {
        "planned"
    };
    let primary_next_action =
        primary_connection_action(&actions, data.verification, &guard_state, None, &[]);
    match data.format {
        OutputFormat::Text => {
            let mut output = format!(
                "Volicord init {}\nruntime_home_state: ready\nruntime_home: {}\nproject_registration_state: {}\nrepo: {}\nconnection_state: {}\nhost: {}\nmode: {}\nconnection_id: {}\nmcp_config_state: {}\nmcp_config: {}\nplanned_change: {}\nprofile: {}\nguard_mode: {}\nguard_installation_state: {}\nguard_files_state: {}\nguard_hook_observed: {}\nlast_guard_event: {}\nprompt_capture_state: {}\nhost_reload_required: {}\nguard_blockers: {}\n",
                data.status.as_str(),
                data.runtime_home.display(),
                project_state,
                data.repo_root.display(),
                data.status.as_str(),
                public_host_label(data.host_kind),
                data.init_mode.cli_value(),
                data.connection_id,
                mcp_config_state,
                target,
                planned_change,
                data.profile_action,
                guard_state.mode_state,
                guard_state.installation_state,
                guard_state.files_state,
                guard_state.hook_observed_state,
                optional_text(guard_state.last_guard_event_at.as_deref()),
                guard_state.prompt_capture_state,
                yes_no(has_reload_action(&actions)),
                comma_or_none(&guard_state.unresolved_blockers)
            );
            output.push_str(&format!(
                "generated_file_count: {}\n",
                data.integration.generated_files.len()
            ));
            append_primary_next_action_text(&mut output, primary_next_action.as_ref());
            Ok(output)
        }
        OutputFormat::Json => {
            let value = json!({
                "action": "init",
                "status": data.status.as_str(),
                "states": connection_states_json(
                    data.status.as_str(),
                    project_state,
                    mcp_config_state.as_str(),
                    &guard_state,
                    has_reload_action(&actions),
                ),
                "host": public_host_label(data.host_kind),
                "mode": data.init_mode.cli_value(),
                "guard_mode": data.init_mode.guard_value(),
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
                "generated_files": generated_files_json(&data.integration.generated_files),
                "guard_installation": {
                    "guard_installation_id": &data.integration.guard_installation_id,
                    "installation_status": guard_status,
                    "policy_hash": &data.integration.policy_hash,
                    "recorded": data.guard_installation.is_some(),
                },
                "guard": guard_state.to_json(),
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
                "summary": "guard installation status was recorded",
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
    let guard_selected = matches!(
        guard_state.mode_state.as_str(),
        "guarded" | "managed" | "mixed"
    );
    let files_check = match guard_state.files_state.as_str() {
        "installed" => json!({
            "id": "guard_files_installed",
            "status": "passed",
            "summary": "guard files are installed",
        }),
        "missing" => json!({
            "id": "guard_files_installed",
            "status": "failed",
            "summary": "guard files are missing",
            "details": { "missing_files": &guard_state.missing_files },
        }),
        "disabled" => json!({
            "id": "guard_files_installed",
            "status": "skipped",
            "summary": "guard files are disabled for mcp-only mode",
        }),
        other => json!({
            "id": "guard_files_installed",
            "status": "skipped",
            "summary": format!("guard files are {other}"),
        }),
    };
    let reload_check = if guard_state.installation_state == "reload_required" {
        json!({
            "id": "guard_host_reload_required",
            "status": "failed",
            "summary": "host reload is required before guard hooks are active",
        })
    } else if guard_selected {
        json!({
            "id": "guard_host_reload_required",
            "status": "passed",
            "summary": "host reload is not currently required by guard installation state",
        })
    } else {
        json!({
            "id": "guard_host_reload_required",
            "status": "skipped",
            "summary": "guard host reload is not applicable",
        })
    };
    let hook_check = match guard_state.hook_observed_state.as_str() {
        "observed" => json!({
            "id": "guard_hook_observed",
            "status": "passed",
            "summary": "guard hook has been observed",
            "details": {
                "last_observed_at": &guard_state.last_observed_at,
                "last_guard_event_at": &guard_state.last_guard_event_at,
            },
        }),
        "not_observed" if guard_selected => json!({
            "id": "guard_hook_observed",
            "status": "failed",
            "summary": "guard hook has not been observed",
            "details": {
                "last_observed_at": Value::Null,
                "last_guard_event_at": &guard_state.last_guard_event_at,
            },
        }),
        other => json!({
            "id": "guard_hook_observed",
            "status": "skipped",
            "summary": format!("guard hook observation is {other}"),
        }),
    };
    let status_check = if guard_state.installation_state == "active"
        && guard_state.hook_observed_state == "observed"
    {
        json!({
            "id": "guard_status_active",
            "status": "passed",
            "summary": "guard status is active and observed",
        })
    } else if guard_selected {
        json!({
            "id": "guard_status_active",
            "status": "failed",
            "summary": format!("guard status is {}", guard_state.installation_state),
            "details": {
                "installation_status": &guard_state.installation_state,
                "unresolved_blockers": &guard_state.unresolved_blockers,
            },
        })
    } else {
        json!({
            "id": "guard_status_active",
            "status": "skipped",
            "summary": "guard active status is not applicable",
        })
    };
    vec![files_check, reload_check, hook_check, status_check]
}

fn render_simplified_connections_output(
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
                    let mut value = connection_json(connection, &project_ids);
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
                "connections": values,
                "checks": [],
                "actions": [],
            }))
            .map(|text| format!("{text}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn render_simplified_remove_dry_run(
    format: OutputFormat,
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    selected_project: &ConnectionProjectRecord,
    plan: SimplifiedRemovePlan<'_>,
    remaining_count: usize,
) -> Result<String, ConnectionCommandError> {
    match plan {
        SimplifiedRemovePlan::Host(host_plan) => {
            render_simplified_plan_output(SimplifiedPlanOutput {
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
        SimplifiedRemovePlan::MembershipOnly => match format {
            OutputFormat::Text => Ok(format!(
                "Agent Connection remove dry_run\nruntime_home_state: ready\nruntime_home: {}\nconnection_state: dry_run\nhost: {}\nintent: {}\nmode: {}\nenabled: {}\nproject_registration_state: {}\nconnected_repositories: {}\nmcp_config_state: membership\nplanned_change: membership\nguard_mode: not_checked\nguard_installation_state: not_checked\nguard_files_state: not_checked\nguard_hook_observed: not_checked\nlast_guard_event: none\nprompt_capture_state: not_checked\nhost_reload_required: no\nguard_blockers: none\nremaining_connected_projects: {}\nnext_action: none\n",
                runtime_home.display(),
                public_host_name_text(&connection.host_kind),
                connection.intent,
                public_mode_text(&connection.mode),
                connection.enabled,
                project_registration_state(projects),
                display_project_roots(projects),
                remaining_count
            )),
            OutputFormat::Json => {
                let project_ids = projects
                    .iter()
                    .map(|project| project.project_id.clone())
                    .collect::<Vec<_>>();
                serde_json::to_string_pretty(&json!({
                    "action": "remove",
                    "status": AgentResultStatus::DryRun.as_str(),
                    "runtime_home": path_text(runtime_home),
                    "states": {
                        "runtime_home": "ready",
                        "connection": AgentResultStatus::DryRun.as_str(),
                        "project_registration": project_registration_state(projects),
                        "mcp_config": "membership",
                        "guard_mode": "not_checked",
                        "guard_installation": "not_checked",
                        "guard_files": "not_checked",
                        "guard_hook_observed": "not_checked",
                        "last_guard_event_at": Value::Null,
                        "prompt_capture": "not_checked",
                        "host_reload_required": false,
                        "guard_blockers": [],
                    },
                    "connection": connection_json(connection, &project_ids),
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
    json!({
        "runtime_home": "ready",
        "connection": connection_state,
        "project_registration": project_registration,
        "mcp_config": mcp_config,
        "guard_mode": &guard_state.mode_state,
        "guard_installation": &guard_state.installation_state,
        "guard_files": &guard_state.files_state,
        "guard_hook_observed": &guard_state.hook_observed_state,
        "last_guard_observed_at": &guard_state.last_observed_at,
        "last_guard_event_at": &guard_state.last_guard_event_at,
        "prompt_capture": &guard_state.prompt_capture_state,
        "guard_blockers": &guard_state.unresolved_blockers,
        "host_reload_required": host_reload_required,
    })
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
                    "Run the connection command again to reinstall missing MCP configuration.",
                    connection,
                    projects,
                ));
            }
            "changed" => {
                return Some(connection_repair_action(
                    "mcp_config_changed",
                    "Review the changed MCP configuration, then rerun the connection command if Volicord should manage it.",
                    connection,
                    projects,
                ));
            }
            "malformed" => {
                return Some(connection_repair_action(
                    "mcp_config_malformed",
                    "Repair the malformed MCP configuration, then rerun the connection command.",
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
                        "Run the connection command again to reinstall missing MCP configuration.",
                        Some(connection),
                        projects,
                    ));
                }
                "changed" => {
                    return Some(connection_repair_action(
                        "mcp_config_changed",
                        "Review the changed MCP configuration, then rerun the connection command if Volicord should manage it.",
                        Some(connection),
                        projects,
                    ));
                }
                "malformed" => {
                    return Some(connection_repair_action(
                        "mcp_config_malformed",
                        "Repair the malformed MCP configuration, then rerun the connection command.",
                        Some(connection),
                        projects,
                    ));
                }
                _ => {}
            }
        }
    }
    if guard_state.installation_state == "files_missing" {
        return Some(connection_repair_action(
            "guard_files_missing",
            "Run init again to reinstall missing guard files.",
            connection,
            projects,
        ));
    }
    if let Some(action) = actions
        .iter()
        .find(|action| action.kind == UserActionKind::ReloadRequired)
    {
        return Some(PrimaryNextAction::new(
            user_action_id(action.kind),
            action.message.clone(),
        ));
    }
    actions
        .first()
        .map(|action| PrimaryNextAction::new(user_action_id(action.kind), action.message.clone()))
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
            "volicord connect {}{} --repo {}",
            host,
            intent_flag_suffix(
                parse_connection_intent(&connection.intent).unwrap_or(ConnectionIntent::Personal)
            ),
            project.project.repo_root.display()
        )
    };
    PrimaryNextAction::new(id, fallback).with_command(command)
}

fn append_primary_next_action_text(output: &mut String, action: Option<&PrimaryNextAction>) {
    match action {
        Some(action) => output.push_str(&format!("next_action: {}\n", action.instruction)),
        None => output.push_str("next_action: none\n"),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn comma_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
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

    let mut missing_files = Vec::new();
    let mut prompt_capture_available = false;
    let prompt_capture_disabled = installations
        .iter()
        .all(|installation| installation.guard_mode == GuardMode::McpOnly.as_str());
    let mut observed = false;
    let mut last_observed_at = None;
    for installation in &installations {
        missing_files.extend(guard_missing_files(&installation.host_capability_json));
        if installation.last_seen_at.is_some() {
            observed = true;
            last_observed_at = max_optional_text(
                last_observed_at,
                installation.last_seen_at.as_deref().map(str::to_owned),
            );
        }
        if installation.guard_mode != GuardMode::McpOnly.as_str()
            && guard_has_prompt_capture(&installation.host_capability_json)
        {
            prompt_capture_available = true;
        }
    }
    missing_files.sort();
    missing_files.dedup();

    if !missing_files.is_empty() {
        return Ok(GuardOperationalState {
            mode_state: guard_mode_state(&installations),
            installation_state: "files_missing".to_owned(),
            files_state: "missing".to_owned(),
            hook_observed_state: if prompt_capture_disabled {
                "disabled".to_owned()
            } else if observed {
                "observed".to_owned()
            } else {
                "not_observed".to_owned()
            },
            last_observed_at,
            last_guard_event_at: last_guard_event_for_projects(
                runtime_home,
                connection_id,
                projects,
            )?,
            prompt_capture_state: "unavailable".to_owned(),
            missing_files,
            unresolved_blockers: vec!["guard_not_installed".to_owned()],
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
    let prompt_capture_state = if prompt_capture_available
        && installation_state == GuardInstallationStatus::Active.as_str()
        && observed
    {
        "available"
    } else if prompt_capture_disabled {
        "disabled"
    } else if installation_state == GuardInstallationStatus::ReloadRequired.as_str() {
        "reload_required"
    } else if installation_state == GuardInstallationStatus::Configured.as_str() {
        "configured"
    } else {
        "unavailable"
    };
    let mode_state = guard_mode_state(&installations);
    let hook_observed_state = if prompt_capture_disabled {
        "disabled"
    } else if observed {
        "observed"
    } else {
        "not_observed"
    };
    Ok(GuardOperationalState {
        mode_state: mode_state.clone(),
        installation_state: installation_state.to_owned(),
        files_state: if prompt_capture_disabled {
            "disabled".to_owned()
        } else {
            "installed".to_owned()
        },
        hook_observed_state: hook_observed_state.to_owned(),
        last_observed_at,
        last_guard_event_at: last_guard_event_for_projects(runtime_home, connection_id, projects)?,
        prompt_capture_state: prompt_capture_state.to_owned(),
        missing_files,
        unresolved_blockers: guard_blockers_for_state(&mode_state, installation_state, observed),
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

fn guard_blockers_for_state(
    guard_mode: &str,
    installation_state: &str,
    guard_hook_observed: bool,
) -> Vec<String> {
    if guard_mode == GuardMode::McpOnly.as_str() {
        return Vec::new();
    }
    match installation_state {
        "not_configured" | "files_missing" => vec!["guard_not_installed".to_owned()],
        "reload_required" => vec!["guard_reload_required".to_owned()],
        "configured" => vec!["guard_not_observed".to_owned()],
        "active" if !guard_hook_observed => vec!["guard_not_observed".to_owned()],
        "stale" => vec!["guard_stale".to_owned()],
        "broken" => vec!["guard_broken".to_owned()],
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

fn guard_has_prompt_capture(capability_json: &str) -> bool {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .and_then(|value| {
            value
                .get("commands")
                .and_then(|commands| commands.get("prompt_capture"))
                .cloned()
        })
        .is_some()
}

fn guard_missing_files(capability_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(capability_json) else {
        return Vec::new();
    };
    value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|file| file.get("path").and_then(Value::as_str))
        .filter(|path| !Path::new(path).exists())
        .map(str::to_owned)
        .collect()
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
    }
}

fn checks_json(
    connection: &AgentConnectionRecord,
    verification: Option<&VerificationReport>,
    guard_state: &GuardOperationalState,
) -> Value {
    if let Some(verification) = verification {
        let mut checks = vec![
            json!({
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
            }),
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
        ];
        checks.extend(guard_checks_json_values(guard_state));
        return Value::Array(checks);
    }
    let mut checks = stored_checks_json(connection);
    checks.extend(guard_checks_json_values(guard_state));
    Value::Array(checks)
}

fn stored_checks_json(connection: &AgentConnectionRecord) -> Vec<Value> {
    let report = json_object_text(&connection.last_verification_report_json);
    let Some(object) = report.as_object() else {
        return Vec::new();
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

fn stored_user_actions(connection: &AgentConnectionRecord) -> Vec<UserAction> {
    serde_json::from_str::<Vec<UserAction>>(&connection.last_user_actions_json).unwrap_or_default()
}

fn terminate_child(child: &mut Child, deadline: Instant) -> Result<(), String> {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
            Err(error) => return Err(format!("failed to wait for MCP process: {error}")),
        }
    }
}

fn connection_json(connection: &AgentConnectionRecord, project_ids: &[String]) -> Value {
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
        "user_actions": json_array_text(&connection.last_user_actions_json),
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
        "host": {
            "status": report.host.status.as_str(),
            "host_state": report.host.host_state.as_str(),
            "managed_config": report.host.managed_config.as_str(),
            "host_executable": report.host.host_executable.as_str(),
            "host_gate": report.host.host_gate.as_str(),
            "host_configuration": report.host.host_configuration.as_str(),
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

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn compact_stream(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let workflow_tools = WORKFLOW_TOOL_NAMES
            .iter()
            .map(|tool| (*tool).to_owned())
            .collect::<Vec<_>>();
        assert!(validate_tools_for_mode(CONNECTION_MODE_WORKFLOW, &workflow_tools).is_ok());

        let read_only_tools = READ_ONLY_TOOL_NAMES
            .iter()
            .map(|tool| (*tool).to_owned())
            .collect::<Vec<_>>();
        assert!(validate_tools_for_mode(CONNECTION_MODE_READ_ONLY, &read_only_tools).is_ok());
        assert!(!read_only_tools
            .iter()
            .any(|tool| tool == "volicord.close_task"));

        let stale_read_only_tools = vec![
            "volicord.status".to_owned(),
            "volicord.close_task".to_owned(),
            "volicord.list_projects".to_owned(),
        ];
        let error =
            validate_tools_for_mode(CONNECTION_MODE_READ_ONLY, &stale_read_only_tools).unwrap_err();
        assert!(error.contains("volicord.check_close"));
    }
}
