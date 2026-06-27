use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fmt, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{
        add_connection_project, agent_connection_record, ensure_agent_connection,
        list_agent_connections, list_connection_projects, remove_agent_connection_if_unused,
        remove_connection_project, set_connection_enabled, update_agent_connection_verification,
        AgentConnectionRecord, AgentConnectionRegistration, ConnectionProjectRecord,
        ConnectionProjectRegistration, CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW,
        HOST_KIND_CLAUDE_CODE, HOST_KIND_CODEX, HOST_KIND_GENERIC, HOST_SCOPE_EXPORT,
        HOST_SCOPE_LOCAL, HOST_SCOPE_PROJECT, HOST_SCOPE_USER, VERIFIED_STATUS_ACTION_REQUIRED,
        VERIFIED_STATUS_COMPLETE, VERIFIED_STATUS_FAILED, VERIFIED_STATUS_NOT_VERIFIED,
    },
    bootstrap::{
        initialize_runtime_home, list_projects, project_record, register_project,
        validate_project_id, ProjectRecord, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

use crate::host_integration::{
    claude_code::{ClaudeCodeAdapter, ProductionCommandRunner},
    codex::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest, CodexPlanRequest},
    generic::{GenericAdapter, GenericPlanRequest},
    is_valid_server_name,
    verification::{Verification, VerificationStatus},
    HostAdapter, HostConfigError, HostKind, HostPlan, HostRemoveRequest, HostScope, HostTarget,
    ManagedServerEntry, PlannedChange,
};

const VOLICORD_HOME: &str = "VOLICORD_HOME";
const PATH_ENV: &str = "PATH";
const AGENT_METADATA_CREATED_BY: &str = "volicord_cli_agent_connection";
const AGENT_RUNTIME_HOME_ID: &str = "runtime_home_agent";
const DEFAULT_MCP_COMMAND: &str = "volicord-mcp";
const DEFAULT_SERVER_NAME: &str = "volicord";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const WORKFLOW_TOOL_NAMES: [&str; 9] = [
    "volicord.intake",
    "volicord.update_scope",
    "volicord.status",
    "volicord.prepare_write",
    "volicord.stage_artifact",
    "volicord.record_run",
    "volicord.request_user_judgment",
    "volicord.close_task",
    "volicord.list_projects",
];
const READ_ONLY_TOOL_NAMES: [&str; 3] = [
    "volicord.status",
    "volicord.close_task",
    "volicord.list_projects",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentCommandError {
    Usage(String),
    Runtime(String),
    FailureOutput(String),
}

impl AgentCommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for AgentCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::FailureOutput(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for AgentCommandError {}

impl From<StoreError> for AgentCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for AgentCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<HostConfigError> for AgentCommandError {
    fn from(error: HostConfigError) -> Self {
        Self::runtime(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProcessOutput {
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

pub trait AgentProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        project_id: Option<&str>,
    ) -> Result<AgentProcessOutput, String>;
    fn verify_mcp_stdio(
        &mut self,
        launch: &McpLaunch,
        runtime_home: &Path,
        connection_id: &str,
        mode: &str,
    ) -> Result<McpVerification, String>;
}

pub struct ProductionAgentProcess;

impl AgentProcess for ProductionAgentProcess {
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
    ) -> Result<AgentProcessOutput, String> {
        let mut child = Command::new(&launch.command);
        child.arg("--check").arg("--connection").arg(connection_id);
        if let Some(project_id) = project_id {
            child.arg("--project").arg(project_id);
        }
        apply_mcp_launch_context(&mut child, launch, runtime_home);
        child.stdin(Stdio::null());
        let output = child.output().map_err(|error| {
            format!(
                "failed to run {} --check --connection {}: {error}",
                launch.command.display(),
                connection_id
            )
        })?;
        Ok(AgentProcessOutput {
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

#[derive(Debug, Clone)]
struct ParsedAgentOptions {
    runtime_home: Option<PathBuf>,
    repo_root: Option<PathBuf>,
    project_id: Option<String>,
    connection_id: Option<String>,
    mode: Option<String>,
    host_kind: Option<HostKind>,
    host_scope: Option<HostScope>,
    server_name: Option<String>,
    mcp_command: Option<PathBuf>,
    export_path: Option<PathBuf>,
    export_dir: Option<PathBuf>,
    output: OutputFormat,
    dry_run: bool,
    allow_repository_write: bool,
    replace_managed: bool,
}

impl Default for ParsedAgentOptions {
    fn default() -> Self {
        Self {
            runtime_home: None,
            repo_root: None,
            project_id: None,
            connection_id: None,
            mode: None,
            host_kind: None,
            host_scope: None,
            server_name: None,
            mcp_command: None,
            export_path: None,
            export_dir: None,
            output: OutputFormat::Text,
            dry_run: false,
            allow_repository_write: false,
            replace_managed: false,
        }
    }
}

pub fn agent_usage() -> String {
    concat!(
        "volicord agent connect --host codex|claude-code|claude_code|generic --scope user|project|local|export [--project-id ID] [--repo-root PATH] [--connection-id ID] [--mode read_only|workflow] [--server-name NAME] [--mcp-command PATH] [--runtime-home PATH] [--export-path PATH|--export-dir PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]\n",
        "volicord agent list [--runtime-home PATH] [--output text|json]\n",
        "volicord agent status --connection-id ID [--runtime-home PATH] [--output text|json]\n",
        "volicord agent enable --connection-id ID [--runtime-home PATH] [--output text|json]\n",
        "volicord agent disable --connection-id ID [--runtime-home PATH] [--output text|json]\n",
        "volicord agent project add --connection-id ID --project-id ID [--repo-root PATH] [--runtime-home PATH] [--output text|json] [--dry-run]\n",
        "volicord agent project remove --connection-id ID --project-id ID [--runtime-home PATH] [--output text|json] [--dry-run]\n",
        "volicord agent verify --connection-id ID [--runtime-home PATH] [--output text|json]\n",
        "volicord agent uninstall --connection-id ID [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write]\n"
    )
    .to_owned()
}

fn agent_connect_usage() -> String {
    concat!(
        "Usage:\n",
        "  volicord agent connect --host codex|claude-code|claude_code|generic --scope user|project|local|export [--project-id ID] [--repo-root PATH] [--connection-id ID] [--mode read_only|workflow] [--server-name NAME] [--mcp-command PATH] [--runtime-home PATH] [--export-path PATH|--export-dir PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]\n",
        "\n",
        "Defaults:\n",
        "  --mode defaults to read_only. Use --mode workflow explicitly for workflow tools.\n",
        "  --server-name defaults to volicord.\n",
        "  Project and local scopes allow one selected project by default.\n",
        "  User scope may allow more projects with volicord agent project add.\n"
    )
    .to_owned()
}

pub fn run_agent_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(agent_usage());
    };

    match subcommand {
        "-h" | "--help" | "help" => {
            if args.len() == 1 {
                Ok(agent_usage())
            } else {
                Err(AgentCommandError::usage(format!(
                    "unexpected argument: {}\n\n{}",
                    args[1],
                    agent_usage()
                )))
            }
        }
        "connect" => command_connect(&args[1..], current_dir, process),
        "list" => command_list(&args[1..], current_dir, process),
        "status" => command_status(&args[1..], current_dir, process),
        "enable" => command_enable_disable(&args[1..], current_dir, process, true),
        "disable" => command_enable_disable(&args[1..], current_dir, process, false),
        "project" => command_project(&args[1..], current_dir, process),
        "verify" => command_verify(&args[1..], current_dir, process),
        "uninstall" => command_uninstall(&args[1..], current_dir, process),
        other => Err(AgentCommandError::usage(format!(
            "unknown agent command: {other}\n\n{}",
            agent_usage()
        ))),
    }
}

fn command_connect(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_connect_usage());
    }
    let parsed = parse_agent_options(args, connect_allowed_options())?;
    let host_kind = required_host_kind(&parsed)?;
    let host_scope = required_host_scope(&parsed)?;
    validate_host_scope(host_kind, host_scope)?;
    validate_repository_write_permission(&parsed, host_scope)?;
    let mode = parse_connection_mode(parsed.mode.as_deref().unwrap_or(CONNECTION_MODE_READ_ONLY))?;
    let server_name = parsed
        .server_name
        .clone()
        .unwrap_or_else(|| DEFAULT_SERVER_NAME.to_owned());
    validate_server_name(&server_name)?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let repo_root = resolve_optional_repo_root(parsed.repo_root.as_deref(), current_dir)?;
    let export_target = resolve_export_target(&parsed, current_dir, None);

    if parsed.dry_run {
        let project = resolve_selected_project_for_dry_run(&parsed, repo_root.as_deref())?;
        let target_hint = connection_target_hint(
            host_kind,
            host_scope,
            project.repo_root.as_deref(),
            &parsed,
            process,
            &server_name,
            export_target.as_deref(),
        )?;
        let connection_id = parsed.connection_id.clone().unwrap_or_else(|| {
            deterministic_connection_id(
                host_kind,
                host_scope,
                project.project_id.as_deref(),
                &target_hint,
                &server_name,
            )
        });
        return render_dry_run_output(
            parsed.output,
            DryRunRenderData {
                action: "connect",
                connection_id: &connection_id,
                host_kind,
                host_scope,
                mode: &mode,
                server_name: &server_name,
                config_target: &target_hint,
                project_id: project.project_id.as_deref(),
            },
        );
    }

    initialize_runtime_home(
        &runtime_home,
        AGENT_RUNTIME_HOME_ID,
        metadata_json_base()?.as_str(),
    )?;
    let project = resolve_or_register_project(
        &runtime_home,
        parsed.project_id.as_deref(),
        repo_root.as_deref(),
    )?;
    let export_target =
        resolve_export_target(&parsed, current_dir, parsed.connection_id.as_deref());
    let target_hint = connection_target_hint(
        host_kind,
        host_scope,
        Some(&project.repo_root),
        &parsed,
        process,
        &server_name,
        export_target.as_deref(),
    )?;
    let connection_id = parsed.connection_id.clone().unwrap_or_else(|| {
        deterministic_connection_id(
            host_kind,
            host_scope,
            Some(&project.project_id),
            &target_hint,
            &server_name,
        )
    });
    let mcp_command = resolve_mcp_command(&parsed, host_scope, current_dir, process)?;
    let existing = agent_connection_record(&runtime_home, &connection_id)?;
    let expected_fingerprint = existing
        .as_ref()
        .map(|record| record.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        HostPlanRequest {
            host_kind,
            scope: host_scope,
            connection_id: &connection_id,
            server_name: &server_name,
            repo_root: Some(&project.repo_root),
            mcp_command: &mcp_command,
            runtime_home: runtime_home_for_host_config(host_scope, &runtime_home),
            expected_fingerprint,
            export_target: export_target.as_deref(),
            export_dir: parsed.export_dir.as_deref(),
            current_dir,
        },
        process,
    )?;
    if let Some(conflict) = host_plan.conflicts.first() {
        return Err(AgentCommandError::runtime(conflict.message.clone()));
    }
    let metadata_json = connection_metadata_json(&host_plan, &mcp_command, &runtime_home)?;
    let mut connection = ensure_agent_connection(
        &runtime_home,
        AgentConnectionRegistration {
            connection_id: connection_id.clone(),
            host_kind: host_kind.as_str().to_owned(),
            host_scope: host_scope.as_str().to_owned(),
            server_name: host_plan.server_name.clone(),
            config_target: host_target_text(&host_plan.target),
            mode: mode.clone(),
            enabled: true,
            managed_fingerprint: host_plan.fingerprint.clone(),
            last_verified_status: existing
                .as_ref()
                .map(|record| record.last_verified_status.clone())
                .unwrap_or_else(|| VERIFIED_STATUS_NOT_VERIFIED.to_owned()),
            metadata_json,
        },
    )?;
    enforce_single_project_scope(&runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &runtime_home,
        ConnectionProjectRegistration {
            connection_id: connection.connection_id.clone(),
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
    connection = update_agent_connection_verification(
        &runtime_home,
        &connection.connection_id,
        verification.status.store_status(),
        &host_plan.fingerprint,
    )?;
    let projects = list_connection_projects(&runtime_home, &connection.connection_id)?;
    render_connection_output(
        parsed.output,
        "connected",
        verification.status,
        &connection,
        &projects,
        Some(&verification),
    )
}

fn command_list(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, list_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connections = list_agent_connections(&runtime_home)?;
    match parsed.output {
        OutputFormat::Text => {
            let mut output = String::from(
                "connection_id\thost_kind\thost_scope\tmode\tenabled\tconnected_projects\tverification_status\tserver_name\tconfig_target\n",
            );
            for connection in connections {
                let projects = project_ids_or_empty(&runtime_home, &connection.connection_id)?;
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    connection.connection_id,
                    connection.host_kind,
                    connection.host_scope,
                    connection.mode,
                    connection.enabled,
                    projects.join(","),
                    connection.last_verified_status,
                    connection.server_name,
                    connection.config_target
                ));
            }
            Ok(output)
        }
        OutputFormat::Json => {
            let mut values = Vec::new();
            for connection in connections {
                let projects = project_ids_or_empty(&runtime_home, &connection.connection_id)?;
                values.push(connection_json(&connection, &projects));
            }
            serde_json::to_string_pretty(&json!({ "connections": values }))
                .map(|text| format!("{text}\n"))
                .map_err(|error| AgentCommandError::runtime(error.to_string()))
        }
    }
}

fn command_status(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, status_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connection_id = required_text(parsed.connection_id.as_deref(), "connection-id")?;
    let connection = required_connection(&runtime_home, connection_id)?;
    let projects = list_connection_projects(&runtime_home, connection_id)?;
    render_connection_output(
        parsed.output,
        "status",
        status_from_store(&connection.last_verified_status),
        &connection,
        &projects,
        None,
    )
}

fn command_enable_disable(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
    enabled: bool,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, enable_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connection_id = required_text(parsed.connection_id.as_deref(), "connection-id")?;
    let connection = set_connection_enabled(&runtime_home, connection_id, enabled)?;
    let projects = list_connection_projects(&runtime_home, connection_id)?;
    render_connection_output(
        parsed.output,
        if enabled { "enabled" } else { "disabled" },
        status_from_store(&connection.last_verified_status),
        &connection,
        &projects,
        None,
    )
}

fn command_project(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(AgentCommandError::usage(agent_usage()));
    };
    match subcommand {
        "add" => command_project_add(&args[1..], current_dir, process),
        "remove" => command_project_remove(&args[1..], current_dir, process),
        "-h" | "--help" | "help" => Ok(agent_usage()),
        other => Err(AgentCommandError::usage(format!(
            "unknown agent project command: {other}\n\n{}",
            agent_usage()
        ))),
    }
}

fn command_project_add(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, project_add_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connection_id = required_text(parsed.connection_id.as_deref(), "connection-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "project-id")?;
    let connection = required_connection(&runtime_home, connection_id)?;
    let repo_root = resolve_optional_repo_root(parsed.repo_root.as_deref(), current_dir)?;
    if parsed.dry_run {
        return render_project_output(
            parsed.output,
            "project_add_dry_run",
            AgentResultStatus::DryRun,
            &connection,
            &[project_id.to_owned()],
        );
    }
    let project =
        resolve_or_register_project(&runtime_home, Some(project_id), repo_root.as_deref())?;
    enforce_single_project_scope(&runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &runtime_home,
        ConnectionProjectRegistration {
            connection_id: connection_id.to_owned(),
            project_id: project.project_id.clone(),
        },
    )?;
    let projects = list_connection_projects(&runtime_home, connection_id)?;
    render_connection_output(
        parsed.output,
        "project_added",
        status_from_store(&connection.last_verified_status),
        &connection,
        &projects,
        None,
    )
}

fn command_project_remove(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, project_remove_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connection_id = required_text(parsed.connection_id.as_deref(), "connection-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "project-id")?;
    let connection = required_connection(&runtime_home, connection_id)?;
    if parsed.dry_run {
        return render_project_output(
            parsed.output,
            "project_remove_dry_run",
            AgentResultStatus::DryRun,
            &connection,
            &[project_id.to_owned()],
        );
    }
    remove_connection_project(&runtime_home, connection_id, project_id)?;
    let projects = list_connection_projects(&runtime_home, connection_id)?;
    render_connection_output(
        parsed.output,
        "project_removed",
        status_from_store(&connection.last_verified_status),
        &connection,
        &projects,
        None,
    )
}

fn command_verify(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, verify_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connection_id = required_text(parsed.connection_id.as_deref(), "connection-id")?;
    let mut connection = required_connection(&runtime_home, connection_id)?;
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
    connection = update_agent_connection_verification(
        &runtime_home,
        &connection.connection_id,
        verification.status.store_status(),
        &host_plan.fingerprint,
    )?;
    let projects = list_connection_projects(&runtime_home, connection_id)?;
    render_connection_output(
        parsed.output,
        "verified",
        verification.status,
        &connection,
        &projects,
        Some(&verification),
    )
}

fn command_uninstall(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let parsed = parse_agent_options(args, uninstall_allowed_options())?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let connection_id = required_text(parsed.connection_id.as_deref(), "connection-id")?;
    let connection = required_connection(&runtime_home, connection_id)?;
    let host_scope = parse_host_scope(&connection.host_scope)?;
    if host_scope == HostScope::Project && !parsed.dry_run && !parsed.allow_repository_write {
        return Err(AgentCommandError::usage(
            "project-scoped Agent Connection uninstall requires --allow-repository-write",
        ));
    }
    let projects = list_connection_projects(&runtime_home, connection_id)?;
    if parsed.dry_run {
        return render_connection_output(
            parsed.output,
            "uninstall_dry_run",
            AgentResultStatus::DryRun,
            &connection,
            &projects,
            None,
        );
    }
    let host_plan = existing_host_plan(&connection, &runtime_home, process)?;
    remove_host_configuration(&host_plan, &connection, process)?;
    for project in &projects {
        remove_connection_project(&runtime_home, connection_id, &project.project_id)?;
    }
    remove_agent_connection_if_unused(&runtime_home, connection_id)?;
    render_connection_output(
        parsed.output,
        "uninstalled",
        AgentResultStatus::Complete,
        &connection,
        &[],
        None,
    )
}

fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}

fn parse_agent_options(
    args: &[String],
    allowed: &[&str],
) -> Result<ParsedAgentOptions, AgentCommandError> {
    let mut parsed = ParsedAgentOptions::default();
    let mut seen = BTreeSet::new();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(AgentCommandError::usage(agent_usage()));
        }
        if !token.starts_with("--") {
            return Err(AgentCommandError::usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if is_boolean_agent_option(without_prefix) {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(AgentCommandError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };

        if !allowed.iter().any(|allowed_name| *allowed_name == name) {
            return Err(AgentCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !seen.insert(name.clone()) {
            return Err(AgentCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        set_agent_option(&mut parsed, &name, value.as_deref())?;
        index += 1;
    }
    Ok(parsed)
}

fn connect_allowed_options() -> &'static [&'static str] {
    &[
        "host",
        "scope",
        "project-id",
        "repo-root",
        "connection-id",
        "mode",
        "server-name",
        "mcp-command",
        "runtime-home",
        "export-path",
        "export-dir",
        "output",
        "dry-run",
        "allow-repository-write",
        "replace-managed",
    ]
}

fn list_allowed_options() -> &'static [&'static str] {
    &["runtime-home", "output"]
}

fn status_allowed_options() -> &'static [&'static str] {
    &["connection-id", "runtime-home", "output"]
}

fn enable_allowed_options() -> &'static [&'static str] {
    &["connection-id", "runtime-home", "output"]
}

fn project_add_allowed_options() -> &'static [&'static str] {
    &[
        "connection-id",
        "project-id",
        "repo-root",
        "runtime-home",
        "output",
        "dry-run",
    ]
}

fn project_remove_allowed_options() -> &'static [&'static str] {
    &[
        "connection-id",
        "project-id",
        "runtime-home",
        "output",
        "dry-run",
    ]
}

fn verify_allowed_options() -> &'static [&'static str] {
    &["connection-id", "runtime-home", "output"]
}

fn uninstall_allowed_options() -> &'static [&'static str] {
    &[
        "connection-id",
        "runtime-home",
        "output",
        "dry-run",
        "allow-repository-write",
    ]
}

fn is_boolean_agent_option(name: &str) -> bool {
    matches!(
        name,
        "dry-run" | "allow-repository-write" | "replace-managed"
    )
}

fn set_agent_option(
    parsed: &mut ParsedAgentOptions,
    name: &str,
    value: Option<&str>,
) -> Result<(), AgentCommandError> {
    match name {
        "runtime-home" => parsed.runtime_home = Some(value_path(name, value)?),
        "repo-root" => parsed.repo_root = Some(value_path(name, value)?),
        "project-id" => parsed.project_id = Some(value_text(name, value)?),
        "connection-id" => parsed.connection_id = Some(value_text(name, value)?),
        "mode" => parsed.mode = Some(value_text(name, value)?),
        "host" => parsed.host_kind = Some(parse_host_kind(&value_text(name, value)?)?),
        "scope" => parsed.host_scope = Some(parse_host_scope(&value_text(name, value)?)?),
        "server-name" => parsed.server_name = Some(value_text(name, value)?),
        "mcp-command" => parsed.mcp_command = Some(value_path(name, value)?),
        "export-path" => parsed.export_path = Some(value_path(name, value)?),
        "export-dir" => parsed.export_dir = Some(value_path(name, value)?),
        "output" => {
            parsed.output = match value_text(name, value)?.as_str() {
                "text" => OutputFormat::Text,
                "json" => OutputFormat::Json,
                other => {
                    return Err(AgentCommandError::usage(format!(
                        "unknown output format: {other}"
                    )))
                }
            }
        }
        "dry-run" => parsed.dry_run = true,
        "allow-repository-write" => parsed.allow_repository_write = true,
        "replace-managed" => parsed.replace_managed = true,
        _ => {
            return Err(AgentCommandError::usage(format!(
                "unknown option: --{name}"
            )))
        }
    }
    Ok(())
}

fn value_text(name: &str, value: Option<&str>) -> Result<String, AgentCommandError> {
    let value =
        value.ok_or_else(|| AgentCommandError::usage(format!("missing value for --{name}")))?;
    if value.trim().is_empty() {
        Err(AgentCommandError::usage(format!(
            "--{name} must not be empty"
        )))
    } else {
        Ok(value.to_owned())
    }
}

fn value_path(name: &str, value: Option<&str>) -> Result<PathBuf, AgentCommandError> {
    Ok(PathBuf::from(value_text(name, value)?))
}

fn required_host_kind(parsed: &ParsedAgentOptions) -> Result<HostKind, AgentCommandError> {
    parsed
        .host_kind
        .ok_or_else(|| AgentCommandError::usage("missing required option: --host"))
}

fn required_host_scope(parsed: &ParsedAgentOptions) -> Result<HostScope, AgentCommandError> {
    parsed
        .host_scope
        .ok_or_else(|| AgentCommandError::usage("missing required option: --scope"))
}

fn required_text<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, AgentCommandError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AgentCommandError::usage(format!("missing required option: --{field}")))
}

fn parse_host_kind(value: &str) -> Result<HostKind, AgentCommandError> {
    match value {
        HOST_KIND_CODEX => Ok(HostKind::Codex),
        "claude-code" | HOST_KIND_CLAUDE_CODE => Ok(HostKind::ClaudeCode),
        HOST_KIND_GENERIC => Ok(HostKind::Generic),
        other => Err(AgentCommandError::usage(format!("unknown host: {other}"))),
    }
}

fn parse_host_scope(value: &str) -> Result<HostScope, AgentCommandError> {
    match value {
        HOST_SCOPE_USER => Ok(HostScope::User),
        HOST_SCOPE_PROJECT => Ok(HostScope::Project),
        HOST_SCOPE_LOCAL => Ok(HostScope::Local),
        HOST_SCOPE_EXPORT => Ok(HostScope::Export),
        other => Err(AgentCommandError::usage(format!("unknown scope: {other}"))),
    }
}

fn parse_connection_mode(value: &str) -> Result<String, AgentCommandError> {
    match value {
        CONNECTION_MODE_READ_ONLY | CONNECTION_MODE_WORKFLOW => Ok(value.to_owned()),
        other => Err(AgentCommandError::usage(format!(
            "unknown connection mode: {other}"
        ))),
    }
}

fn validate_host_scope(host_kind: HostKind, scope: HostScope) -> Result<(), AgentCommandError> {
    let valid = matches!(
        (host_kind, scope),
        (HostKind::Codex, HostScope::User)
            | (HostKind::Codex, HostScope::Project)
            | (HostKind::ClaudeCode, HostScope::Local)
            | (HostKind::ClaudeCode, HostScope::Project)
            | (HostKind::ClaudeCode, HostScope::User)
            | (HostKind::Generic, HostScope::Export)
    );
    if valid {
        Ok(())
    } else {
        Err(AgentCommandError::usage(
            "host and scope must match the supported Agent Connection matrix",
        ))
    }
}

fn validate_server_name(value: &str) -> Result<(), AgentCommandError> {
    if is_valid_server_name(value) {
        Ok(())
    } else {
        Err(AgentCommandError::usage(format!(
            "server name must use ASCII letters, numbers, hyphen, or underscore and start with a letter or number: {value}"
        )))
    }
}

fn validate_repository_write_permission(
    parsed: &ParsedAgentOptions,
    scope: HostScope,
) -> Result<(), AgentCommandError> {
    if scope == HostScope::Project && !parsed.dry_run && !parsed.allow_repository_write {
        return Err(AgentCommandError::usage(
            "project-scoped Agent Connection host configuration writes require --allow-repository-write",
        ));
    }
    Ok(())
}

fn resolve_agent_runtime_home(
    parsed: &ParsedAgentOptions,
    current_dir: &Path,
    process: &impl AgentProcess,
) -> Result<PathBuf, AgentCommandError> {
    if let Some(path) = &parsed.runtime_home {
        if path.is_absolute() {
            Ok(path.clone())
        } else {
            Err(AgentCommandError::usage(
                "--runtime-home must be an absolute path",
            ))
        }
    } else {
        resolve_runtime_home(|name| process.env_var(name), current_dir).map_err(Into::into)
    }
}

fn resolve_optional_repo_root(
    value: Option<&Path>,
    current_dir: &Path,
) -> Result<Option<PathBuf>, AgentCommandError> {
    value
        .map(|path| {
            canonical_existing_dir(&absolute_path(current_dir, path.to_path_buf()), "repo-root")
        })
        .transpose()
}

fn canonical_existing_dir(path: &Path, field: &'static str) -> Result<PathBuf, AgentCommandError> {
    let path = fs::canonicalize(path).map_err(|error| {
        AgentCommandError::runtime(format!("{field} is not accessible: {error}"))
    })?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(AgentCommandError::runtime(format!(
            "{field} must be a directory"
        )))
    }
}

fn resolve_or_register_project(
    runtime_home: &Path,
    project_id: Option<&str>,
    repo_root: Option<&Path>,
) -> Result<ProjectRecord, AgentCommandError> {
    match (project_id, repo_root) {
        (Some(project_id), Some(repo_root)) => {
            validate_project_id(project_id)?;
            if let Some(existing) = project_record(runtime_home, project_id)? {
                if existing.repo_root != repo_root {
                    return Err(AgentCommandError::runtime(
                        "--repo-root must match the existing project registration",
                    ));
                }
                Ok(existing)
            } else {
                register_project(
                    runtime_home,
                    ProjectRegistration {
                        project_id: project_id.to_owned(),
                        repo_root: repo_root.to_path_buf(),
                        project_home: None,
                        status: ACTIVE_PROJECT_STATUS.to_owned(),
                        metadata_json: metadata_json_base()?,
                    },
                )
                .map_err(Into::into)
            }
        }
        (Some(project_id), None) => project_record(runtime_home, project_id)?.ok_or_else(|| {
            AgentCommandError::runtime("project is not registered; provide --repo-root")
        }),
        (None, Some(repo_root)) => {
            let matches = list_projects(runtime_home)?
                .into_iter()
                .filter(|project| project.repo_root == repo_root)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [project] => Ok(project.clone()),
                [] => Err(AgentCommandError::usage(
                    "missing required option: --project-id",
                )),
                _ => Err(AgentCommandError::usage(
                    "--repo-root matches multiple projects; provide --project-id",
                )),
            }
        }
        (None, None) => Err(AgentCommandError::usage(
            "missing required option: --project-id",
        )),
    }
}

#[derive(Debug, Clone)]
struct DryRunProject {
    project_id: Option<String>,
    repo_root: Option<PathBuf>,
}

fn resolve_selected_project_for_dry_run(
    parsed: &ParsedAgentOptions,
    repo_root: Option<&Path>,
) -> Result<DryRunProject, AgentCommandError> {
    if parsed.project_id.is_none() && repo_root.is_none() {
        return Err(AgentCommandError::usage(
            "dry-run connect requires --project-id or --repo-root",
        ));
    }
    Ok(DryRunProject {
        project_id: parsed.project_id.clone(),
        repo_root: repo_root.map(Path::to_path_buf),
    })
}

fn enforce_single_project_scope(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    project_id: &str,
) -> Result<(), AgentCommandError> {
    let scope = parse_host_scope(&connection.host_scope)?;
    if !matches!(scope, HostScope::Project | HostScope::Local) {
        return Ok(());
    }
    let projects = list_connection_projects(runtime_home, &connection.connection_id)?;
    if projects
        .iter()
        .any(|project| project.project_id != project_id)
    {
        return Err(AgentCommandError::runtime(
            "project and local Agent Connections may allow only one project",
        ));
    }
    Ok(())
}

fn resolve_mcp_command(
    parsed: &ParsedAgentOptions,
    scope: HostScope,
    current_dir: &Path,
    process: &impl AgentProcess,
) -> Result<PathBuf, AgentCommandError> {
    if scope == HostScope::Project {
        if parsed.mcp_command.is_some() {
            return Err(AgentCommandError::usage(
                "project-scoped Agent Connections use volicord-mcp from PATH",
            ));
        }
        return Ok(PathBuf::from(DEFAULT_MCP_COMMAND));
    }
    if let Some(command) = &parsed.mcp_command {
        let command = absolute_path(current_dir, command.clone());
        if command.is_absolute() {
            return Ok(command);
        }
    }
    discover_mcp_command(process)
}

fn discover_mcp_command(process: &impl AgentProcess) -> Result<PathBuf, AgentCommandError> {
    let current_exe = process.current_exe().map_err(AgentCommandError::runtime)?;
    if let Some(parent) = current_exe.parent() {
        let sibling = parent.join(DEFAULT_MCP_COMMAND);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    if let Some(path) = process.env_var(PATH_ENV) {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(DEFAULT_MCP_COMMAND);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(AgentCommandError::runtime(
        "volicord-mcp was not found; provide --mcp-command",
    ))
}

fn connection_target_hint(
    host_kind: HostKind,
    scope: HostScope,
    repo_root: Option<&Path>,
    parsed: &ParsedAgentOptions,
    process: &impl AgentProcess,
    server_name: &str,
    export_target: Option<&Path>,
) -> Result<String, AgentCommandError> {
    match (host_kind, scope) {
        (HostKind::Codex, HostScope::User) => {
            let path = codex_home(process)?.join("config.toml");
            Ok(path_text(&path))
        }
        (HostKind::Codex, HostScope::Project) => {
            let repo_root = repo_root.ok_or_else(|| {
                AgentCommandError::usage("Codex project scope requires --repo-root")
            })?;
            Ok(path_text(&repo_root.join(".codex").join("config.toml")))
        }
        (HostKind::ClaudeCode, HostScope::Project) => {
            let repo_root = repo_root.ok_or_else(|| {
                AgentCommandError::usage("Claude Code project scope requires --repo-root")
            })?;
            Ok(path_text(&repo_root.join(".mcp.json")))
        }
        (HostKind::ClaudeCode, HostScope::Local) => {
            let repo_root = repo_root.ok_or_else(|| {
                AgentCommandError::usage("Claude Code local scope requires --repo-root")
            })?;
            Ok(format!("claude local {}", path_text(repo_root)))
        }
        (HostKind::ClaudeCode, HostScope::User) => Ok("claude user".to_owned()),
        (HostKind::Generic, HostScope::Export) => {
            let target = export_target
                .map(Path::to_path_buf)
                .unwrap_or_else(|| generic_default_export_target(parsed, server_name));
            Ok(path_text(&target))
        }
        _ => Err(AgentCommandError::usage(
            "host and scope must match the supported Agent Connection matrix",
        )),
    }
}

struct HostPlanRequest<'a> {
    host_kind: HostKind,
    scope: HostScope,
    connection_id: &'a str,
    server_name: &'a str,
    repo_root: Option<&'a Path>,
    mcp_command: &'a Path,
    runtime_home: Option<&'a Path>,
    expected_fingerprint: Option<&'a str>,
    export_target: Option<&'a Path>,
    export_dir: Option<&'a Path>,
    current_dir: &'a Path,
}

fn build_host_plan(
    request: HostPlanRequest<'_>,
    process: &impl AgentProcess,
) -> Result<HostPlan, AgentCommandError> {
    match request.host_kind {
        HostKind::Codex => {
            let adapter = CodexAdapter::new(codex_environment(process));
            adapter
                .plan(CodexPlanRequest {
                    scope: request.scope,
                    connection_id: request.connection_id,
                    explicit_server_name: Some(request.server_name),
                    repo_root: request.repo_root,
                    mcp_command: request.mcp_command,
                    runtime_home: request.runtime_home,
                    expected_fingerprint: request.expected_fingerprint,
                })
                .map_err(Into::into)
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter
                .plan(crate::host_integration::claude_code::ClaudePlanRequest {
                    scope: request.scope,
                    connection_id: request.connection_id,
                    explicit_server_name: Some(request.server_name),
                    repo_root: request.repo_root,
                    mcp_command: request.mcp_command,
                    runtime_home: request.runtime_home,
                    expected_fingerprint: request.expected_fingerprint,
                })
                .map_err(Into::into)
        }
        HostKind::Generic => {
            let adapter = GenericAdapter;
            let output_dir = request.export_dir.unwrap_or(request.current_dir);
            adapter
                .plan(GenericPlanRequest {
                    scope: request.scope,
                    connection_id: request.connection_id,
                    explicit_server_name: Some(request.server_name),
                    output_dir,
                    output_path: request.export_target,
                    mcp_command: request.mcp_command,
                    runtime_home: request.runtime_home,
                    expected_fingerprint: request.expected_fingerprint,
                })
                .map_err(Into::into)
        }
    }
}

fn apply_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    process: &impl AgentProcess,
) -> Result<(), AgentCommandError> {
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
    process: &impl AgentProcess,
) -> Result<Verification, AgentCommandError> {
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
    process: &impl AgentProcess,
) -> Result<(), AgentCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let request = HostRemoveRequest {
        host_kind,
        host_scope: parse_host_scope(&connection.host_scope)?,
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
    process: &impl AgentProcess,
) -> Result<HostPlan, AgentCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let host_scope = parse_host_scope(&connection.host_scope)?;
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
                    scope: host_scope,
                    connection_id: &connection.connection_id,
                    server_name: &connection.server_name,
                    config_target: Path::new(&connection.config_target),
                    mcp_command: &mcp_command,
                    runtime_home: runtime_home_for_entry.as_deref(),
                    managed_fingerprint: &connection.managed_fingerprint,
                })
                .map_err(Into::into)
        }
        _ => Ok(manual_existing_host_plan(
            connection,
            host_kind,
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
        host_scope,
        server_name: connection.server_name.clone(),
        target,
        entry: ManagedServerEntry::new(&connection.connection_id, mcp_command, runtime_home),
        change: PlannedChange::Noop,
        fingerprint: connection.managed_fingerprint.clone(),
        conflicts: Vec::new(),
        user_actions: Vec::new(),
        file_snapshot: None,
    }
}

fn verify_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    host_plan: &HostPlan,
    launch: &McpLaunch,
    project_id: Option<&str>,
    process: &mut impl AgentProcess,
) -> Result<VerificationReport, AgentCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let host = verify_host_plan(host_kind, host_plan, process)?;
    let preflight = run_connection_preflight(
        process,
        launch,
        runtime_home,
        &connection.connection_id,
        project_id,
        &connection.mode,
    );
    let handshake = if host.mcp_handshake_allowed && preflight.status == StepStatus::Passed {
        match process.verify_mcp_stdio(
            launch,
            runtime_home,
            &connection.connection_id,
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
    let status = aggregate_status(&host, &preflight, &handshake.step);
    Ok(VerificationReport {
        status,
        host,
        preflight,
        handshake: handshake.step,
        tools: handshake.tools,
    })
}

fn aggregate_status(
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
    process: &mut impl AgentProcess,
    launch: &McpLaunch,
    runtime_home: &Path,
    connection_id: &str,
    project_id: Option<&str>,
    mode: &str,
) -> VerificationStep {
    match process.run_preflight(launch, runtime_home, connection_id, project_id) {
        Ok(output) if output.success => {
            match validate_connection_preflight_report(&output.stdout, connection_id, mode) {
                Ok(()) => VerificationStep::passed("volicord-mcp preflight passed"),
                Err(message) => VerificationStep::failed(message),
            }
        }
        Ok(output) => VerificationStep::failed(format!(
            "volicord-mcp preflight failed with status {}; stderr: {}",
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

fn render_connection_output(
    format: OutputFormat,
    action: &str,
    status: AgentResultStatus,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    verification: Option<&VerificationReport>,
) -> Result<String, AgentCommandError> {
    let project_ids = projects
        .iter()
        .map(|project| project.project_id.clone())
        .collect::<Vec<_>>();
    match format {
        OutputFormat::Text => {
            let mut output = format!(
                "Agent Connection {action}\nconnection_id: {}\nhost_kind: {}\nhost_scope: {}\nmode: {}\nenabled: {}\nconnected_projects: {}\nverification_status: {}\nserver_name: {}\nconfig_target: {}\n",
                connection.connection_id,
                connection.host_kind,
                connection.host_scope,
                connection.mode,
                connection.enabled,
                display_projects(&project_ids),
                status.as_str(),
                connection.server_name,
                connection.config_target
            );
            if let Some(verification) = verification {
                output.push_str(&format!(
                    "host_verification: {}\npreflight: {}\nmcp_handshake: {}\n",
                    verification.host.status.as_str(),
                    verification.preflight.status.as_str(),
                    verification.handshake.status.as_str()
                ));
            }
            Ok(output)
        }
        OutputFormat::Json => {
            let value = json!({
                "action": action,
                "status": status.as_str(),
                "connection": connection_json(connection, &project_ids),
                "verification": verification.map(verification_json)
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| AgentCommandError::runtime(error.to_string()))
        }
    }
}

fn render_project_output(
    format: OutputFormat,
    action: &str,
    status: AgentResultStatus,
    connection: &AgentConnectionRecord,
    project_ids: &[String],
) -> Result<String, AgentCommandError> {
    match format {
        OutputFormat::Text => Ok(format!(
            "Agent Connection {action}\nconnection_id: {}\nconnected_projects: {}\nverification_status: {}\n",
            connection.connection_id,
            display_projects(project_ids),
            status.as_str()
        )),
        OutputFormat::Json => {
            let value = json!({
                "action": action,
                "status": status.as_str(),
                "connection_id": connection.connection_id,
                "connected_projects": project_ids,
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| AgentCommandError::runtime(error.to_string()))
        }
    }
}

struct DryRunRenderData<'a> {
    action: &'a str,
    connection_id: &'a str,
    host_kind: HostKind,
    host_scope: HostScope,
    mode: &'a str,
    server_name: &'a str,
    config_target: &'a str,
    project_id: Option<&'a str>,
}

fn render_dry_run_output(
    format: OutputFormat,
    data: DryRunRenderData<'_>,
) -> Result<String, AgentCommandError> {
    match format {
        OutputFormat::Text => Ok(format!(
            "Agent Connection {} dry_run\nconnection_id: {}\nhost_kind: {}\nhost_scope: {}\nmode: {}\nenabled: true\nconnected_projects: {}\nverification_status: dry_run\nserver_name: {}\nconfig_target: {}\n",
            data.action,
            data.connection_id,
            data.host_kind.as_str(),
            data.host_scope.as_str(),
            data.mode,
            data.project_id.unwrap_or(""),
            data.server_name,
            data.config_target
        )),
        OutputFormat::Json => {
            let value = json!({
                "action": data.action,
                "status": AgentResultStatus::DryRun.as_str(),
                "connection": {
                    "connection_id": data.connection_id,
                    "host_kind": data.host_kind.as_str(),
                    "host_scope": data.host_scope.as_str(),
                    "mode": data.mode,
                    "enabled": true,
                    "connected_projects": data.project_id.into_iter().collect::<Vec<_>>(),
                    "verification_status": AgentResultStatus::DryRun.as_str(),
                    "server_name": data.server_name,
                    "config_target": data.config_target
                }
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| AgentCommandError::runtime(error.to_string()))
        }
    }
}

fn connection_json(connection: &AgentConnectionRecord, project_ids: &[String]) -> Value {
    json!({
        "connection_id": connection.connection_id,
        "host_kind": connection.host_kind,
        "host_scope": connection.host_scope,
        "mode": connection.mode,
        "enabled": connection.enabled,
        "connected_projects": project_ids,
        "verification_status": connection.last_verified_status,
        "server_name": connection.server_name,
        "config_target": connection.config_target,
    })
}

fn verification_json(report: &VerificationReport) -> Value {
    json!({
        "status": report.status.as_str(),
        "host": {
            "status": report.host.status.as_str(),
            "details": report.host.details,
        },
        "preflight": step_json(&report.preflight),
        "mcp_handshake": step_json(&report.handshake),
        "tools": report.tools,
    })
}

fn step_json(step: &VerificationStep) -> Value {
    json!({
        "status": step.status.as_str(),
        "details": step.details,
    })
}

fn display_projects(projects: &[String]) -> String {
    if projects.is_empty() {
        String::new()
    } else {
        projects.join(",")
    }
}

fn project_ids_or_empty(
    runtime_home: &Path,
    connection_id: &str,
) -> Result<Vec<String>, AgentCommandError> {
    Ok(list_connection_projects(runtime_home, connection_id)?
        .into_iter()
        .map(|project| project.project_id)
        .collect())
}

fn status_from_store(value: &str) -> AgentResultStatus {
    match value {
        VERIFIED_STATUS_COMPLETE => AgentResultStatus::Complete,
        VERIFIED_STATUS_ACTION_REQUIRED => AgentResultStatus::ActionRequired,
        VERIFIED_STATUS_FAILED => AgentResultStatus::Failed,
        _ => AgentResultStatus::NotVerified,
    }
}

fn required_connection(
    runtime_home: &Path,
    connection_id: &str,
) -> Result<AgentConnectionRecord, AgentCommandError> {
    agent_connection_record(runtime_home, connection_id)?.ok_or_else(|| {
        AgentCommandError::runtime(format!("Agent Connection not found: {connection_id}"))
    })
}

fn connection_metadata_json(
    plan: &HostPlan,
    mcp_command: &Path,
    runtime_home: &Path,
) -> Result<String, AgentCommandError> {
    let mut value = json!({
        "created_by": AGENT_METADATA_CREATED_BY,
        "mcp_command": path_text(mcp_command),
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
    serde_json::to_string(&value).map_err(|error| AgentCommandError::runtime(error.to_string()))
}

fn metadata_json_base() -> Result<String, AgentCommandError> {
    serde_json::to_string(&json!({ "created_by": AGENT_METADATA_CREATED_BY }))
        .map_err(|error| AgentCommandError::runtime(error.to_string()))
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

fn resolve_export_target(
    parsed: &ParsedAgentOptions,
    current_dir: &Path,
    connection_id: Option<&str>,
) -> Option<PathBuf> {
    parsed
        .export_path
        .as_ref()
        .map(|path| absolute_path(current_dir, path.clone()))
        .or_else(|| {
            parsed.export_dir.as_ref().map(|dir| {
                let dir = absolute_path(current_dir, dir.clone());
                let stem = connection_id.unwrap_or(DEFAULT_SERVER_NAME);
                dir.join(format!("volicord-{}.mcp.json", sanitize_identifier(stem)))
            })
        })
}

fn generic_default_export_target(parsed: &ParsedAgentOptions, server_name: &str) -> PathBuf {
    parsed
        .export_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(format!(
            "volicord-{}.mcp.json",
            sanitize_identifier(server_name)
        ))
}

fn codex_environment(process: &impl AgentProcess) -> CodexEnvironment {
    CodexEnvironment {
        home: process.env_var("HOME").map(PathBuf::from),
        codex_home: process.env_var("CODEX_HOME").map(PathBuf::from),
        path: process.env_var(PATH_ENV),
    }
}

fn codex_home(process: &impl AgentProcess) -> Result<PathBuf, AgentCommandError> {
    if let Some(path) = process.env_var("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = process.env_var("HOME").ok_or_else(|| {
        AgentCommandError::runtime("Codex user configuration requires CODEX_HOME or HOME")
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
    fn connection_mode_defaults_and_validates() {
        assert_eq!(
            parse_connection_mode(CONNECTION_MODE_READ_ONLY).unwrap(),
            CONNECTION_MODE_READ_ONLY
        );
        assert_eq!(
            parse_connection_mode(CONNECTION_MODE_WORKFLOW).unwrap(),
            CONNECTION_MODE_WORKFLOW
        );
        assert!(parse_connection_mode("full").is_err());
    }
}
