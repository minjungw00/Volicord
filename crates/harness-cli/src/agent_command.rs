use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fmt, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use harness_store::{
    agent_integrations::{
        add_integration_project, agent_integration_record, clear_agent_integration_default_project,
        host_installation_record, list_host_installations_for_integration,
        list_integration_projects, register_agent_integration, register_host_installation,
        remove_host_installation, remove_integration_project,
        set_agent_integration_default_project, set_agent_integration_enabled,
        update_host_installation_verification, AgentIntegrationRecord,
        AgentIntegrationRegistration, HostInstallationRecord, HostInstallationRegistration,
        IntegrationProjectRecord, IntegrationProjectRegistration, AGENT_INTERACTION_ROLE,
        HOST_KIND_CLAUDE_CODE, HOST_KIND_CODEX, HOST_KIND_GENERIC, HOST_SCOPE_LOCAL,
        HOST_SCOPE_PROJECT, VERIFIED_STATUS_ACTION_REQUIRED, VERIFIED_STATUS_COMPLETE,
        VERIFIED_STATUS_FAILED, VERIFIED_STATUS_NOT_VERIFIED, VERIFIED_STATUS_PARTIAL_FAILURE,
    },
    bootstrap::{
        initialize_runtime_home, list_projects, list_surfaces, project_record_for_execution,
        register_project, register_surface, runtime_home_record, validate_project_id,
        ProjectRecord, ProjectRegistration, SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    inspection::{
        inspect_runtime_home, AgentIntegrationInspectionRecord, DatabaseInspection,
        HostInstallationInspectionRecord, InspectionSchemaState,
        IntegrationProjectInspectionRecord, ProjectInspectionRecord, RegistryInspectionSnapshot,
    },
    migrations::REGISTRY_SCHEMA_VERSION,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};
use harness_types::{AccessClass, SurfaceInteractionRole, BASELINE_WORKFLOW_ACCESS_CLASSES};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    host_integration::{
        claude_code::{ClaudeCodeAdapter, ProductionCommandRunner},
        codex::{CodexAdapter, CodexEnvironment},
        generic::GenericAdapter,
        verification::{
            HostConfigurationStatus, HostExecutableStatus, HostGateStatus, HostVerificationState,
            ManagedConfigStatus, Verification, VerificationStatus,
        },
        HostAdapter, HostConfigError, HostKind, HostPlan, HostRemoveRequest, HostScope, HostTarget,
        PlannedChange,
    },
    registration::{capability_profile_json, local_access_json, RegistrationMetadataError},
    repository_guidance::{
        apply_guidance_plan, apply_guidance_remove, compensate_new_guidance, guidance_status,
        plan_guidance_apply, plan_guidance_remove, GuidanceEffect, GuidancePlan, GuidanceStateKind,
        GuidanceStatus, GuidanceTarget,
    },
};

const HARNESS_HOME: &str = "HARNESS_HOME";
const PATH_ENV: &str = "PATH";
const AGENT_METADATA_JSON: &str =
    r#"{"created_by":"harness_cli_agent","setup_profile":"agent_integration_v1"}"#;
const AGENT_RUNTIME_HOME_ID: &str = "runtime_home_agent";
const AGENT_SURFACE_KIND: &str = "mcp";
const DEFAULT_MCP_COMMAND: &str = "harness-mcp";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const PUBLIC_METHOD_TOOL_NAMES: [&str; 9] = [
    "harness.intake",
    "harness.update_scope",
    "harness.status",
    "harness.prepare_write",
    "harness.stage_artifact",
    "harness.record_run",
    "harness.request_user_judgment",
    "harness.record_user_judgment",
    "harness.close_task",
];
const LIST_PROJECTS_TOOL_NAME: &str = "harness.list_projects";

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

    fn failure_output(message: impl Into<String>) -> Self {
        Self::FailureOutput(message.into())
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

impl From<RegistrationMetadataError> for AgentCommandError {
    fn from(error: RegistrationMetadataError) -> Self {
        match error {
            RegistrationMetadataError::Usage(message) => Self::Usage(message),
            RegistrationMetadataError::Runtime(message) => Self::Runtime(message),
        }
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

pub trait AgentProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        command: &Path,
        runtime_home: &Path,
        integration_id: &str,
        project_id: Option<&str>,
    ) -> Result<AgentProcessOutput, String>;
    fn verify_mcp_stdio(
        &mut self,
        command: &Path,
        runtime_home: &Path,
        integration_id: &str,
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
        command: &Path,
        runtime_home: &Path,
        integration_id: &str,
        project_id: Option<&str>,
    ) -> Result<AgentProcessOutput, String> {
        let mut child = Command::new(command);
        child
            .arg("--check")
            .arg("--integration")
            .arg(integration_id);
        if let Some(project_id) = project_id {
            child.arg("--project").arg(project_id);
        }
        child.env(HARNESS_HOME, runtime_home);
        child.stdin(Stdio::null());
        let output = child.output().map_err(|error| {
            format!(
                "failed to run {} --check --integration {}: {error}",
                command.display(),
                integration_id
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
        command: &Path,
        runtime_home: &Path,
        integration_id: &str,
    ) -> Result<McpVerification, String> {
        verify_mcp_stdio_process(command, runtime_home, integration_id, DEFAULT_TIMEOUT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuidanceSelection {
    None,
    Codex,
    ClaudeCode,
    Both,
}

impl GuidanceSelection {
    fn targets(self) -> &'static [GuidanceTarget] {
        const NONE: &[GuidanceTarget] = &[];
        const CODEX: &[GuidanceTarget] = &[GuidanceTarget::Codex];
        const CLAUDE_CODE: &[GuidanceTarget] = &[GuidanceTarget::ClaudeCode];
        const BOTH: &[GuidanceTarget] = &[GuidanceTarget::Codex, GuidanceTarget::ClaudeCode];

        match self {
            Self::None => NONE,
            Self::Codex => CODEX,
            Self::ClaudeCode => CLAUDE_CODE,
            Self::Both => BOTH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentResultStatus {
    Complete,
    ActionRequired,
    PartialFailure,
    Failed,
    DryRun,
}

impl AgentResultStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::PartialFailure => "partial_failure",
            Self::Failed => "failed",
            Self::DryRun => "dry_run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionState {
    Created,
    Reused,
    Updated,
    Removed,
    Skipped,
    Conflict,
    Planned,
}

impl ActionState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Reused => "reused",
            Self::Updated => "updated",
            Self::Removed => "removed",
            Self::Skipped => "skipped",
            Self::Conflict => "conflict",
            Self::Planned => "planned",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentAction {
    target: &'static str,
    state: ActionState,
    detail: String,
}

impl AgentAction {
    fn new(target: &'static str, state: ActionState, detail: impl Into<String>) -> Self {
        Self {
            target,
            state,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegistrySchemaPlan {
    current_version: i64,
    latest_supported_version: i64,
    migration_planned: bool,
}

#[derive(Debug, Clone)]
struct AgentRegistryPlan {
    schema: Option<RegistrySchemaPlan>,
    projects: Vec<ProjectInspectionRecord>,
    integrations: Vec<AgentIntegrationInspectionRecord>,
    integration_projects: Vec<IntegrationProjectInspectionRecord>,
    host_installations: Vec<HostInstallationInspectionRecord>,
}

#[derive(Debug, Clone)]
struct IntegrationProjectPlanRecord {
    project_id: String,
    project: ProjectRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpVerification {
    pub status: VerificationStatus,
    pub host_state: HostVerificationState,
    pub managed_config: ManagedConfigStatus,
    pub host_executable: HostExecutableStatus,
    pub host_gate: HostGateStatus,
    pub host_configuration: HostConfigurationStatus,
    pub mcp_handshake_allowed: bool,
    pub details: String,
    pub host_diagnostic: Option<String>,
    pub instructions_present: bool,
    pub tools: Vec<String>,
}

impl McpVerification {
    fn skipped(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::NotVerified,
            host_state: HostVerificationState::NotVerified,
            managed_config: ManagedConfigStatus::NotApplicable,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::NotApplicable,
            host_configuration: HostConfigurationStatus::NotApplicable,
            mcp_handshake_allowed: false,
            details: details.into(),
            host_diagnostic: None,
            instructions_present: false,
            tools: Vec::new(),
        }
    }

    fn failed(details: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Failed,
            host_state: HostVerificationState::Failed,
            managed_config: ManagedConfigStatus::Unknown,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::Unknown,
            host_configuration: HostConfigurationStatus::Unknown,
            mcp_handshake_allowed: false,
            details: details.into(),
            host_diagnostic: None,
            instructions_present: false,
            tools: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedAgentOptions {
    runtime_home: Option<PathBuf>,
    repo_root: Option<PathBuf>,
    project_id: Option<String>,
    integration_id: Option<String>,
    default_project_id: Option<String>,
    surface_id: Option<String>,
    surface_instance_id: Option<String>,
    host_kind: Option<HostKind>,
    host_scope: Option<HostScope>,
    server_name: Option<String>,
    installation_id: Option<String>,
    mcp_command: Option<PathBuf>,
    export_path: Option<PathBuf>,
    export_dir: Option<PathBuf>,
    output: OutputFormat,
    guidance: GuidanceSelection,
    dry_run: bool,
    yes: bool,
    allow_repository_write: bool,
    replace_managed: bool,
    remove_managed: bool,
    make_default: bool,
}

impl Default for ParsedAgentOptions {
    fn default() -> Self {
        Self {
            runtime_home: None,
            repo_root: None,
            project_id: None,
            integration_id: None,
            default_project_id: None,
            surface_id: None,
            surface_instance_id: None,
            host_kind: None,
            host_scope: None,
            server_name: None,
            installation_id: None,
            mcp_command: None,
            export_path: None,
            export_dir: None,
            output: OutputFormat::Text,
            guidance: GuidanceSelection::None,
            dry_run: false,
            yes: false,
            allow_repository_write: false,
            replace_managed: false,
            remove_managed: false,
            make_default: false,
        }
    }
}

pub fn agent_usage() -> String {
    "harness agent install --host codex|claude-code|claude_code|generic --scope user|project|local|export --project-id ID [--repo-root PATH] [--integration-id ID] [--default-project-id ID] [--server-name NAME] [--surface-id ID] [--surface-instance-id ID] [--mcp-command PATH] [--runtime-home PATH] [--export-path PATH|--export-dir PATH] [--guidance none|codex|claude-code|claude_code|both] [--output text|json] [--dry-run] [--yes] [--allow-repository-write] [--replace-managed]\n\
     harness agent project add --integration-id ID --project-id ID [--repo-root PATH] [--default] [--runtime-home PATH] [--output text|json] [--dry-run]\n\
     harness agent project remove --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json] [--dry-run]\n\
     harness agent status --integration-id ID [--runtime-home PATH] [--output text|json]\n\
     harness agent verify --integration-id ID [--installation-id ID] [--runtime-home PATH] [--output text|json]\n\
     harness agent uninstall --integration-id ID [--installation-id ID] [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--remove-managed]\n\
     harness agent guidance apply --integration-id ID --project-id ID --host codex|claude-code|claude_code [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]\n\
     harness agent guidance status --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json]\n\
     harness agent guidance remove --integration-id ID --project-id ID [--host codex|claude-code|claude_code] [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--remove-managed]\n"
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
        "install" => command_install(&args[1..], current_dir, process),
        "project" => command_project(&args[1..], current_dir, process),
        "status" => command_status(&args[1..], current_dir, process),
        "verify" => command_verify(&args[1..], current_dir, process),
        "uninstall" => command_uninstall(&args[1..], current_dir, process),
        "guidance" => command_guidance(&args[1..], current_dir, process),
        other => Err(AgentCommandError::usage(format!(
            "unknown agent command: {other}\n\n{}",
            agent_usage()
        ))),
    }
}

fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}

fn command_install(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, install_allowed_options())?;
    let host_kind = required_host_kind(&parsed)?;
    let host_scope = required_host_scope(&parsed)?;
    validate_host_scope(host_kind, host_scope)?;
    validate_repository_write_authorization(&parsed, host_scope)?;
    if !parsed.guidance.targets().is_empty() {
        validate_guidance_write_authorization(&parsed)?;
    }

    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let repo_root = resolve_optional_repo_root(parsed.repo_root.as_deref(), current_dir)?;
    if parsed.dry_run {
        return command_install_dry_run(
            parsed,
            host_kind,
            host_scope,
            runtime_home,
            repo_root,
            current_dir,
            process,
        );
    }
    let project_plan =
        resolve_install_project(&runtime_home, parsed.project_id.as_deref(), repo_root)?;
    let integration_id = parsed.integration_id.clone().unwrap_or_else(|| {
        deterministic_integration_id(host_kind, host_scope, &project_plan.project_id)
    });
    let existing_integration = agent_integration_record(&runtime_home, &integration_id)?;
    let surface_id = parsed
        .surface_id
        .clone()
        .or_else(|| {
            existing_integration
                .as_ref()
                .map(|record| record.surface_id.clone())
        })
        .unwrap_or_else(|| stable_identifier("agent_surface", &integration_id));
    let surface_instance_id = parsed
        .surface_instance_id
        .clone()
        .or_else(|| {
            existing_integration
                .as_ref()
                .map(|record| record.surface_instance_id.clone())
        })
        .unwrap_or_else(|| stable_identifier("agent_surface_instance", &integration_id));
    let default_project_id = parsed
        .default_project_id
        .clone()
        .or_else(|| {
            existing_integration
                .as_ref()
                .and_then(|record| record.default_project_id.clone())
        })
        .unwrap_or_else(|| project_plan.project_id.clone());
    let surface_exists = project_plan
        .existing_project
        .as_ref()
        .map(|project| {
            surface_exists_for_project(
                &runtime_home,
                &project.project_id,
                &surface_id,
                &surface_instance_id,
            )
        })
        .transpose()?
        .unwrap_or(false);
    let membership_exists =
        is_project_member(&runtime_home, &integration_id, &project_plan.project_id)?;
    let mcp_command = resolve_mcp_command(&parsed, host_scope, current_dir, process)?;
    let expected_installation = find_installation_for_target_hint(
        &runtime_home,
        &integration_id,
        host_kind,
        host_scope,
        parsed.server_name.as_deref(),
    )?;
    let host_plan = build_host_plan(
        HostPlanInputs {
            host_kind,
            host_scope,
            integration_id: &integration_id,
            server_name: parsed.server_name.as_deref(),
            repo_root: project_plan.repo_root.as_deref(),
            mcp_command: &mcp_command,
            runtime_home: runtime_home_for_host_config(host_scope, &runtime_home),
            expected_fingerprint: expected_installation
                .as_ref()
                .map(|record| record.managed_fingerprint.as_str()),
            parsed: &parsed,
            current_dir,
        },
        process,
    )?;
    if host_plan.has_conflicts() {
        return Err(AgentCommandError::runtime(
            host_plan.conflicts[0].message.clone(),
        ));
    }
    validate_project_scope_membership(
        &runtime_home,
        &integration_id,
        host_scope,
        &project_plan.project_id,
    )?;

    let installation_id = expected_installation
        .as_ref()
        .map(|record| record.installation_id.clone())
        .unwrap_or_else(|| deterministic_installation_id(&integration_id, &host_plan));
    let mut actions = Vec::new();
    actions.push(AgentAction::new(
        "runtime_home",
        if runtime_home_record(&runtime_home)?.is_some() {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        path_text(&runtime_home),
    ));
    actions.push(AgentAction::new(
        "project",
        project_plan.action,
        project_plan.project_id.clone(),
    ));
    actions.push(AgentAction::new(
        "surface",
        if surface_exists {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        format!("{surface_id}/{surface_instance_id}"),
    ));
    actions.push(AgentAction::new(
        "integration",
        if existing_integration.is_some() {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        integration_id.clone(),
    ));
    actions.push(AgentAction::new(
        "project_allowlist",
        if membership_exists {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        project_plan.project_id.clone(),
    ));
    actions.push(AgentAction::new(
        "host",
        planned_change_action(host_plan.change),
        host_target_text(&host_plan.target),
    ));
    let guidance_plans = if parsed.guidance.targets().is_empty() {
        Vec::new()
    } else {
        let repo_root = project_plan.repo_root.as_deref().ok_or_else(|| {
            AgentCommandError::runtime("repository guidance requires a Product Repository root")
        })?;
        plan_guidance_for_targets(
            repo_root,
            &integration_id,
            &project_plan.project_id,
            parsed.guidance.targets(),
        )?
    };
    for plan in &guidance_plans {
        actions.push(AgentAction::new(
            "guidance",
            planned_change_action(plan.change),
            format!("{} {}", plan.target.as_str(), path_text(&plan.path)),
        ));
    }
    initialize_runtime_home(&runtime_home, AGENT_RUNTIME_HOME_ID, AGENT_METADATA_JSON)?;
    let project = if let Some(existing) = project_plan.existing_project {
        existing
    } else {
        let repo_root = project_plan.repo_root.clone().ok_or_else(|| {
            AgentCommandError::runtime("project registration requires --repo-root")
        })?;
        register_project(
            &runtime_home,
            ProjectRegistration {
                project_id: project_plan.project_id.clone(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: AGENT_METADATA_JSON.to_owned(),
            },
        )?
    };
    ensure_agent_surface(
        &runtime_home,
        &project.project_id,
        &surface_id,
        &surface_instance_id,
    )?;
    let integration = register_agent_integration(
        &runtime_home,
        AgentIntegrationRegistration {
            integration_id: integration_id.clone(),
            interaction_role: AGENT_INTERACTION_ROLE.to_owned(),
            surface_id: surface_id.clone(),
            surface_instance_id: surface_instance_id.clone(),
            metadata_json: AGENT_METADATA_JSON.to_owned(),
        },
    )?;
    let membership_before = is_project_member(&runtime_home, &integration_id, &project.project_id)?;
    add_integration_project(
        &runtime_home,
        IntegrationProjectRegistration {
            integration_id: integration_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;
    if default_project_id != project.project_id
        && !is_project_member(&runtime_home, &integration_id, &default_project_id)?
    {
        return Err(AgentCommandError::runtime(
            "--default-project-id must name an allowed integration project",
        ));
    }
    set_agent_integration_default_project(&runtime_home, &integration_id, &default_project_id)?;

    match run_integration_preflight(process, &mcp_command, &runtime_home, &integration_id, None) {
        Ok(()) => (),
        Err(message) => {
            compensate_install_membership(
                &runtime_home,
                &integration_id,
                &project.project_id,
                membership_before,
            );
            let output = partial_install_output(
                &parsed,
                runtime_home,
                integration_id,
                host_plan,
                vec![project.project_id],
                actions,
                format!("MCP preflight failed before host configuration: {message}"),
            );
            return Err(AgentCommandError::failure_output(render_agent_output(
                &output,
            )?));
        }
    };

    let host_effect = {
        let result = apply_host_plan(host_kind, &host_plan, process);
        match result {
            Ok(effect) => effect,
            Err(error) => {
                compensate_install_membership(
                    &runtime_home,
                    &integration_id,
                    &project.project_id,
                    membership_before,
                );
                let output = partial_install_output(
                    &parsed,
                    runtime_home,
                    integration_id,
                    host_plan,
                    vec![project.project_id],
                    actions,
                    format!("host configuration apply failed: {error}"),
                );
                return Err(AgentCommandError::failure_output(render_agent_output(
                    &output,
                )?));
            }
        }
    };
    actions.push(AgentAction::new(
        "host_apply",
        planned_change_action(host_effect.change),
        host_target_text(&host_effect.target),
    ));

    let mut guidance_effects = Vec::new();
    for plan in &guidance_plans {
        match apply_guidance_plan(plan) {
            Ok(effect) => {
                actions.push(AgentAction::new(
                    "guidance_apply",
                    planned_change_action(effect.change),
                    format!("{} {}", effect.target.as_str(), path_text(&effect.path)),
                ));
                guidance_effects.push(effect);
            }
            Err(error) => {
                compensate_install_membership(
                    &runtime_home,
                    &integration_id,
                    &project.project_id,
                    membership_before,
                );
                let output = partial_install_output(
                    &parsed,
                    runtime_home,
                    integration_id,
                    host_plan,
                    vec![project.project_id],
                    actions,
                    format!("repository guidance apply failed: {error}"),
                );
                return Err(AgentCommandError::failure_output(render_agent_output(
                    &output,
                )?));
            }
        }
    }

    let host_status = verify_host_plan(host_kind, &host_plan, process)?;
    let mcp_verification = if should_run_diagnostic_mcp_handshake(&host_status) {
        match process.verify_mcp_stdio(&mcp_command, &runtime_home, &integration_id) {
            Ok(verification) => merge_mcp_verification_with_host(verification, &host_status),
            Err(message) => mcp_failure_from_host(&host_status, message),
        }
    } else {
        mcp_verification_from_host(host_status.clone())
    };
    let status = setup_status_from_verification(&mcp_verification);
    let mut warnings = Vec::new();
    if matches!(
        status,
        AgentResultStatus::PartialFailure | AgentResultStatus::Failed
    ) {
        compensate_guidance_effects(&guidance_effects, &mut warnings);
    }
    let last_verified_status = store_status_from_setup_status(status);
    let installation = register_host_installation(
        &runtime_home,
        HostInstallationRegistration {
            installation_id: installation_id.clone(),
            integration_id: integration.integration_id.clone(),
            host_kind: host_kind.as_str().to_owned(),
            host_scope: host_scope.as_str().to_owned(),
            server_name: host_plan.server_name.clone(),
            config_target: host_target_text(&host_plan.target),
            managed_fingerprint: host_plan.fingerprint.clone(),
            last_verified_status: last_verified_status.to_owned(),
            metadata_json: installation_metadata_json(
                &runtime_home,
                &mcp_command,
                project_plan.repo_root.as_deref(),
            )?,
        },
    )?;
    mark_planned_actions_created(&mut actions);
    let guidance = guidance_statuses_for_project(
        project_plan.repo_root.as_deref(),
        &integration_id,
        &project.project_id,
        guidance_targets_for_status(parsed.guidance.targets()),
    )?;

    let output = AgentOutput {
        status,
        runtime_home,
        registry_schema: None,
        integration_id,
        host_plan: Some(host_plan),
        allowed_projects: vec![project.project_id],
        installations: vec![installation],
        guidance,
        verification: mcp_verification,
        actions,
        warnings,
        action_required: host_required_actions(&host_status),
        output: parsed.output,
    };

    match output.status {
        AgentResultStatus::PartialFailure | AgentResultStatus::Failed => Err(
            AgentCommandError::failure_output(render_agent_output(&output)?),
        ),
        _ => render_agent_output(&output),
    }
}

fn command_install_dry_run(
    parsed: ParsedAgentOptions,
    host_kind: HostKind,
    host_scope: HostScope,
    runtime_home: PathBuf,
    repo_root: Option<PathBuf>,
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let registry = inspect_agent_registry_for_planning(&runtime_home)?;
    let project_plan = resolve_install_project_from_registry(
        &registry,
        &runtime_home,
        parsed.project_id.as_deref(),
        repo_root,
    )?;
    let integration_id = parsed.integration_id.clone().unwrap_or_else(|| {
        deterministic_integration_id(host_kind, host_scope, &project_plan.project_id)
    });
    let existing_integration = registry.integration(&integration_id);
    let surface_id = parsed
        .surface_id
        .clone()
        .or_else(|| existing_integration.map(|record| record.surface_id.clone()))
        .unwrap_or_else(|| stable_identifier("agent_surface", &integration_id));
    let surface_instance_id = parsed
        .surface_instance_id
        .clone()
        .or_else(|| existing_integration.map(|record| record.surface_instance_id.clone()))
        .unwrap_or_else(|| stable_identifier("agent_surface_instance", &integration_id));
    let default_project_id = parsed
        .default_project_id
        .clone()
        .or_else(|| existing_integration.and_then(|record| record.default_project_id.clone()))
        .unwrap_or_else(|| project_plan.project_id.clone());
    let surface_exists = registry.project_surface_exists(
        &project_plan.project_id,
        &surface_id,
        &surface_instance_id,
    )?;
    let membership_exists = registry.is_project_member(&integration_id, &project_plan.project_id);
    let mcp_command = resolve_mcp_command(&parsed, host_scope, current_dir, process)?;
    let expected_installation = registry
        .find_installation_for_target_hint(
            &integration_id,
            host_kind,
            host_scope,
            parsed.server_name.as_deref(),
        )
        .map(host_installation_record_from_inspection);
    let host_plan = build_host_plan(
        HostPlanInputs {
            host_kind,
            host_scope,
            integration_id: &integration_id,
            server_name: parsed.server_name.as_deref(),
            repo_root: project_plan.repo_root.as_deref(),
            mcp_command: &mcp_command,
            runtime_home: runtime_home_for_host_config(host_scope, &runtime_home),
            expected_fingerprint: expected_installation
                .as_ref()
                .map(|record| record.managed_fingerprint.as_str()),
            parsed: &parsed,
            current_dir,
        },
        process,
    )?;
    if host_plan.has_conflicts() {
        return Err(AgentCommandError::runtime(
            host_plan.conflicts[0].message.clone(),
        ));
    }
    validate_project_scope_membership_from_registry(
        &registry,
        &integration_id,
        host_scope,
        &project_plan.project_id,
    )?;

    if default_project_id != project_plan.project_id
        && !registry.is_project_member(&integration_id, &default_project_id)
    {
        return Err(AgentCommandError::runtime(
            "--default-project-id must name an allowed integration project",
        ));
    }

    let mut actions = Vec::new();
    if let Some(schema) = registry.schema {
        actions.push(AgentAction::new(
            "runtime_home",
            ActionState::Reused,
            path_text(&runtime_home),
        ));
        if schema.migration_planned {
            actions.push(AgentAction::new(
                "registry_migration",
                ActionState::Planned,
                format!(
                    "registry schema {} -> {}",
                    schema.current_version, schema.latest_supported_version
                ),
            ));
        }
    } else {
        actions.push(AgentAction::new(
            "runtime_home",
            ActionState::Planned,
            path_text(&runtime_home),
        ));
    }
    actions.push(AgentAction::new(
        "project",
        project_plan.action,
        project_plan.project_id.clone(),
    ));
    actions.push(AgentAction::new(
        "surface",
        if surface_exists {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        format!("{surface_id}/{surface_instance_id}"),
    ));
    actions.push(AgentAction::new(
        "integration",
        if existing_integration.is_some() {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        integration_id.clone(),
    ));
    actions.push(AgentAction::new(
        "project_allowlist",
        if membership_exists {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        project_plan.project_id.clone(),
    ));
    actions.push(AgentAction::new(
        "host",
        planned_change_action(host_plan.change),
        host_target_text(&host_plan.target),
    ));

    let guidance_plans = if parsed.guidance.targets().is_empty() {
        Vec::new()
    } else {
        let repo_root = project_plan.repo_root.as_deref().ok_or_else(|| {
            AgentCommandError::runtime("repository guidance requires a Product Repository root")
        })?;
        plan_guidance_for_targets(
            repo_root,
            &integration_id,
            &project_plan.project_id,
            parsed.guidance.targets(),
        )?
    };
    for plan in &guidance_plans {
        actions.push(AgentAction::new(
            "guidance",
            planned_change_action(plan.change),
            format!("{} {}", plan.target.as_str(), path_text(&plan.path)),
        ));
    }

    let output = AgentOutput {
        status: AgentResultStatus::DryRun,
        runtime_home,
        registry_schema: registry.schema,
        integration_id,
        host_plan: Some(host_plan),
        allowed_projects: vec![project_plan.project_id],
        installations: expected_installation.into_iter().collect(),
        guidance: guidance_plans
            .iter()
            .map(|plan| plan.status.clone())
            .collect(),
        verification: McpVerification::skipped(
            "dry run does not run preflight or MCP verification",
        ),
        actions,
        warnings: Vec::new(),
        action_required: Vec::new(),
        output: parsed.output,
    };
    render_agent_output(&output)
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
        "remove" => command_project_remove(&args[1..], current_dir),
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
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, project_add_allowed_options())?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "--project-id")?;
    validate_project_id(project_id)?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let repo_root = resolve_optional_repo_root(parsed.repo_root.as_deref(), current_dir)?;
    if parsed.dry_run {
        let registry = inspect_agent_registry_for_planning(&runtime_home)?;
        let _integration = registry.required_integration(integration_id)?;
        validate_add_membership_scope_from_registry(&registry, integration_id, project_id)?;
        let existing_project = registry.executable_project(project_id)?;
        if existing_project.is_none() && repo_root.is_none() {
            return Err(AgentCommandError::runtime(
                "project is not registered; pass --repo-root to register it before adding membership",
            ));
        }
        let installations = registry.host_installations_for_integration(integration_id);
        let actions = vec![
            AgentAction::new(
                "integration",
                ActionState::Reused,
                integration_id.to_owned(),
            ),
            AgentAction::new(
                "project",
                if existing_project.is_some() {
                    ActionState::Reused
                } else {
                    ActionState::Planned
                },
                project_id.to_owned(),
            ),
            AgentAction::new(
                "project_allowlist",
                if registry.is_project_member(integration_id, project_id) {
                    ActionState::Reused
                } else {
                    ActionState::Planned
                },
                project_id.to_owned(),
            ),
        ];
        let output = AgentOutput {
            status: AgentResultStatus::DryRun,
            runtime_home,
            registry_schema: registry.schema,
            integration_id: integration_id.to_owned(),
            host_plan: None,
            allowed_projects: vec![project_id.to_owned()],
            installations,
            guidance: Vec::new(),
            verification: McpVerification::skipped("dry run does not run project preflight"),
            actions,
            warnings: Vec::new(),
            action_required: Vec::new(),
            output: parsed.output,
        };
        return render_agent_output(&output);
    }
    let integration = required_integration(&runtime_home, integration_id)?;
    validate_add_membership_scope(&runtime_home, integration_id, project_id)?;
    let existing_project = project_record_for_execution(&runtime_home, project_id)?;
    if existing_project.is_none() && repo_root.is_none() {
        return Err(AgentCommandError::runtime(
            "project is not registered; pass --repo-root to register it before adding membership",
        ));
    }
    let actions = vec![
        AgentAction::new(
            "integration",
            ActionState::Reused,
            integration_id.to_owned(),
        ),
        AgentAction::new(
            "project",
            if existing_project.is_some() {
                ActionState::Reused
            } else {
                ActionState::Planned
            },
            project_id.to_owned(),
        ),
        AgentAction::new(
            "project_allowlist",
            if is_project_member(&runtime_home, integration_id, project_id)? {
                ActionState::Reused
            } else {
                ActionState::Planned
            },
            project_id.to_owned(),
        ),
    ];
    let project = if let Some(project) = existing_project {
        project
    } else {
        register_project(
            &runtime_home,
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: repo_root.expect("repo_root checked above"),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: AGENT_METADATA_JSON.to_owned(),
            },
        )?
    };
    ensure_agent_surface(
        &runtime_home,
        &project.project_id,
        &integration.surface_id,
        &integration.surface_instance_id,
    )?;
    add_integration_project(
        &runtime_home,
        IntegrationProjectRegistration {
            integration_id: integration_id.to_owned(),
            project_id: project.project_id.clone(),
        },
    )?;
    if parsed.make_default {
        set_agent_integration_default_project(&runtime_home, integration_id, &project.project_id)?;
    }

    let verification = match command_for_existing_installation(&runtime_home, integration_id)? {
        Some(command) => match run_integration_preflight(
            process,
            &command,
            &runtime_home,
            integration_id,
            Some(&project.project_id),
        ) {
            Ok(()) => McpVerification::skipped("project-specific startup preflight passed"),
            Err(message) => McpVerification::failed(message),
        },
        None => McpVerification::skipped("no Host Installation inventory contains an MCP command"),
    };

    let installations = list_host_installations_for_integration(&runtime_home, integration_id)?;
    let allowed_projects = list_integration_projects(&runtime_home, integration_id)?
        .into_iter()
        .map(|project| project.project_id)
        .collect();
    let output = AgentOutput {
        status: if verification.status == VerificationStatus::Failed {
            AgentResultStatus::PartialFailure
        } else {
            AgentResultStatus::Complete
        },
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects,
        installations,
        guidance: Vec::new(),
        verification,
        actions,
        warnings: Vec::new(),
        action_required: Vec::new(),
        output: parsed.output,
    };
    match output.status {
        AgentResultStatus::PartialFailure | AgentResultStatus::Failed => Err(
            AgentCommandError::failure_output(render_agent_output(&output)?),
        ),
        _ => render_agent_output(&output),
    }
}

fn command_project_remove(
    args: &[String],
    current_dir: &Path,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let no_process = EnvOnlyProcess;
    let parsed = parse_agent_options(args, project_remove_allowed_options())?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "--project-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, &no_process)?;
    if parsed.dry_run {
        let registry = inspect_agent_registry_for_planning(&runtime_home)?;
        let integration = registry.required_integration(integration_id)?;
        if integration.default_project_id.as_deref() == Some(project_id) {
            return Err(AgentCommandError::runtime(
                "cannot remove the integration default project; change or clear the default first",
            ));
        }
        let actions = vec![AgentAction::new(
            "project_allowlist",
            if registry.is_project_member(integration_id, project_id) {
                ActionState::Planned
            } else {
                ActionState::Skipped
            },
            project_id.to_owned(),
        )];
        let output = AgentOutput {
            status: AgentResultStatus::DryRun,
            runtime_home,
            registry_schema: registry.schema,
            integration_id: integration_id.to_owned(),
            host_plan: None,
            allowed_projects: Vec::new(),
            installations: Vec::new(),
            guidance: Vec::new(),
            verification: McpVerification::skipped("dry run does not change project membership"),
            actions,
            warnings: Vec::new(),
            action_required: Vec::new(),
            output: parsed.output,
        };
        return render_agent_output(&output);
    }
    let integration = required_integration(&runtime_home, integration_id)?;
    if integration.default_project_id.as_deref() == Some(project_id) {
        return Err(AgentCommandError::runtime(
            "cannot remove the integration default project; change or clear the default first",
        ));
    }
    let membership_exists = is_project_member(&runtime_home, integration_id, project_id)?;
    let actions = vec![AgentAction::new(
        "project_allowlist",
        if membership_exists {
            ActionState::Removed
        } else {
            ActionState::Skipped
        },
        project_id.to_owned(),
    )];
    remove_integration_project(&runtime_home, integration_id, project_id)?;
    let remaining = list_integration_projects(&runtime_home, integration_id)?;
    let mut warnings = Vec::new();
    if remaining.is_empty() {
        warnings.push(
            "integration has no allowed projects and is not executable until one is added"
                .to_owned(),
        );
    }
    let installations = list_host_installations_for_integration(&runtime_home, integration_id)?;
    let output = AgentOutput {
        status: AgentResultStatus::Complete,
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects: remaining
            .into_iter()
            .map(|record| record.project_id)
            .collect::<Vec<_>>(),
        installations,
        guidance: Vec::new(),
        verification: McpVerification::skipped(
            "project membership removed; host configuration was not rewritten",
        ),
        actions,
        warnings,
        action_required: Vec::new(),
        output: parsed.output,
    };
    render_agent_output(&output)
}

fn command_status(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, status_allowed_options())?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let _integration = required_integration(&runtime_home, integration_id)?;
    let installations = list_host_installations_for_integration(&runtime_home, integration_id)?;
    let projects = list_integration_projects(&runtime_home, integration_id)?;
    let mut warnings = Vec::new();
    for installation in &installations {
        match inspect_installation_host_state(&runtime_home, installation, current_dir, process) {
            Ok(state) => warnings.push(format!(
                "host_state {}: {state}",
                installation.installation_id
            )),
            Err(error) => warnings.push(format!(
                "host_state {}: {error}",
                installation.installation_id
            )),
        }
    }
    let guidance = guidance_statuses_for_projects(integration_id, &projects)?;
    let allowed_projects = projects
        .iter()
        .map(|project| project.project_id.clone())
        .collect();
    let output = AgentOutput {
        status: AgentResultStatus::Complete,
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects,
        installations,
        guidance,
        verification: McpVerification::skipped("status does not prove host loading"),
        actions: Vec::new(),
        warnings,
        action_required: Vec::new(),
        output: parsed.output,
    };
    render_agent_output(&output)
}

fn command_verify(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, verify_allowed_options())?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    let _integration = required_integration(&runtime_home, integration_id)?;
    let installations = selected_installations(
        &runtime_home,
        integration_id,
        parsed.installation_id.as_deref(),
    )?;
    let mut host_verifications = Vec::new();
    for installation in &installations {
        host_verifications.push(verify_installation_host_state(
            &runtime_home,
            installation,
            current_dir,
            process,
        )?);
    }
    let first_blocking_host = host_verifications
        .iter()
        .find(|verification| !should_run_diagnostic_mcp_handshake(verification))
        .cloned();
    let verification = if let Some(host_status) = first_blocking_host {
        mcp_verification_from_host(host_status)
    } else {
        let Some(command) = command_for_existing_installation(&runtime_home, integration_id)?
        else {
            return Err(AgentCommandError::runtime(
                "no Host Installation inventory contains an MCP command for verification",
            ));
        };
        run_integration_preflight(process, &command, &runtime_home, integration_id, None)
            .map_err(AgentCommandError::runtime)?;
        match process.verify_mcp_stdio(&command, &runtime_home, integration_id) {
            Ok(mcp) => merge_mcp_verification_with_host(
                mcp,
                host_verifications
                    .first()
                    .expect("selected installations are not empty"),
            ),
            Err(message) => mcp_failure_from_host(
                host_verifications
                    .first()
                    .expect("selected installations are not empty"),
                message,
            ),
        }
    };
    let status = setup_status_from_verification(&verification);
    let store_status = store_status_from_setup_status(status);
    let mut updated = Vec::new();
    for installation in installations {
        updated.push(update_host_installation_verification(
            &runtime_home,
            &installation.installation_id,
            store_status,
            &installation.managed_fingerprint,
        )?);
    }
    let allowed_projects = list_integration_projects(&runtime_home, integration_id)?
        .into_iter()
        .map(|project| project.project_id)
        .collect();
    let output = AgentOutput {
        status,
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects,
        installations: updated,
        guidance: Vec::new(),
        verification,
        actions: vec![AgentAction::new(
            "verification",
            ActionState::Updated,
            integration_id.to_owned(),
        )],
        warnings: Vec::new(),
        action_required: Vec::new(),
        output: parsed.output,
    };
    match output.status {
        AgentResultStatus::PartialFailure | AgentResultStatus::Failed => Err(
            AgentCommandError::failure_output(render_agent_output(&output)?),
        ),
        _ => render_agent_output(&output),
    }
}

fn command_uninstall(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, uninstall_allowed_options())?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    if parsed.dry_run {
        let registry = inspect_agent_registry_for_planning(&runtime_home)?;
        let _integration = registry.required_integration(integration_id)?;
        let installations = selected_installations_from_registry(
            &registry,
            integration_id,
            parsed.installation_id.as_deref(),
        )?;
        for installation in &installations {
            let scope = parse_host_scope(&installation.host_scope)?;
            validate_repository_write_authorization(&parsed, scope)?;
        }
        if parsed.remove_managed {
            validate_guidance_remove_authorization(&parsed)?;
        }
        let projects = registry.integration_project_plan_records(integration_id)?;
        let guidance = guidance_statuses_for_plan_projects(integration_id, &projects)?;
        let mut warnings = Vec::new();
        let mut actions = installations
            .iter()
            .map(|installation| {
                AgentAction::new(
                    "host",
                    ActionState::Planned,
                    installation.config_target.clone(),
                )
            })
            .collect::<Vec<_>>();
        if parsed.remove_managed {
            for project in &projects {
                for target in [GuidanceTarget::Codex, GuidanceTarget::ClaudeCode] {
                    match plan_guidance_remove(
                        &project.project.repo_root,
                        integration_id,
                        &project.project_id,
                        target,
                    ) {
                        Ok(plan) => {
                            actions.push(AgentAction::new(
                                "guidance",
                                planned_change_action(plan.change),
                                format!("{} {}", target.as_str(), path_text(&plan.path)),
                            ));
                        }
                        Err(HostConfigError::Conflict(conflict)) => {
                            actions.push(AgentAction::new(
                                "guidance",
                                ActionState::Conflict,
                                format!("{}: {}", target.as_str(), conflict.message),
                            ));
                            warnings.push(format!(
                                "residual guidance preserved for project {} {}: {}",
                                project.project_id,
                                target.as_str(),
                                conflict.message
                            ));
                        }
                        Err(error) => return Err(AgentCommandError::from(error)),
                    }
                }
            }
        }
        let output = AgentOutput {
            status: AgentResultStatus::DryRun,
            runtime_home,
            registry_schema: registry.schema,
            integration_id: integration_id.to_owned(),
            host_plan: None,
            allowed_projects: projects
                .iter()
                .map(|project| project.project_id.clone())
                .collect(),
            installations,
            guidance,
            verification: McpVerification::skipped("dry run does not remove host configuration"),
            actions,
            warnings,
            action_required: Vec::new(),
            output: parsed.output,
        };
        return render_agent_output(&output);
    }
    let _integration = required_integration(&runtime_home, integration_id)?;
    let installations = selected_installations(
        &runtime_home,
        integration_id,
        parsed.installation_id.as_deref(),
    )?;
    for installation in &installations {
        let scope = parse_host_scope(&installation.host_scope)?;
        validate_repository_write_authorization(&parsed, scope)?;
    }
    if parsed.remove_managed {
        validate_guidance_remove_authorization(&parsed)?;
    }
    let projects = list_integration_projects(&runtime_home, integration_id)?;
    let mut guidance_remove_plans = Vec::new();
    let mut warnings = Vec::new();
    let mut actions = installations
        .iter()
        .map(|installation| {
            AgentAction::new(
                "host",
                ActionState::Removed,
                installation.config_target.clone(),
            )
        })
        .collect::<Vec<_>>();
    if parsed.remove_managed {
        for project in &projects {
            for target in [GuidanceTarget::Codex, GuidanceTarget::ClaudeCode] {
                match plan_guidance_remove(
                    &project.project.repo_root,
                    integration_id,
                    &project.project_id,
                    target,
                ) {
                    Ok(plan) => {
                        actions.push(AgentAction::new(
                            "guidance",
                            planned_change_action(plan.change),
                            format!("{} {}", target.as_str(), path_text(&plan.path)),
                        ));
                        guidance_remove_plans.push(plan);
                    }
                    Err(HostConfigError::Conflict(conflict)) => {
                        actions.push(AgentAction::new(
                            "guidance",
                            ActionState::Conflict,
                            format!("{}: {}", target.as_str(), conflict.message),
                        ));
                        warnings.push(format!(
                            "residual guidance preserved for project {} {}: {}",
                            project.project_id,
                            target.as_str(),
                            conflict.message
                        ));
                    }
                    Err(error) => return Err(AgentCommandError::from(error)),
                }
            }
        }
    }
    for installation in &installations {
        remove_host_configuration(&runtime_home, installation, current_dir, process)?;
        remove_host_installation(&runtime_home, &installation.installation_id)?;
    }
    for plan in &guidance_remove_plans {
        if let Err(error) = apply_guidance_remove(plan) {
            warnings.push(format!(
                "residual guidance preserved for project {} {}: {}",
                plan.project_id,
                plan.target.as_str(),
                error
            ));
        }
    }
    let guidance = guidance_statuses_for_projects(integration_id, &projects)?;
    let remaining = list_host_installations_for_integration(&runtime_home, integration_id)?;
    if remaining.is_empty() {
        set_agent_integration_enabled(&runtime_home, integration_id, false)?;
    }
    let output = AgentOutput {
        status: if warnings
            .iter()
            .any(|warning| warning.contains("residual guidance"))
        {
            AgentResultStatus::PartialFailure
        } else {
            AgentResultStatus::Complete
        },
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects: projects
            .iter()
            .map(|project| project.project_id.clone())
            .collect(),
        installations: remaining,
        guidance,
        verification: McpVerification::skipped("managed host configuration removed"),
        actions,
        warnings,
        action_required: Vec::new(),
        output: parsed.output,
    };
    match output.status {
        AgentResultStatus::PartialFailure | AgentResultStatus::Failed => Err(
            AgentCommandError::failure_output(render_agent_output(&output)?),
        ),
        _ => render_agent_output(&output),
    }
}

fn command_guidance(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(AgentCommandError::usage(agent_usage()));
    };
    match subcommand {
        "apply" => command_guidance_apply(&args[1..], current_dir, process),
        "status" => command_guidance_status(&args[1..], current_dir, process),
        "remove" => command_guidance_remove(&args[1..], current_dir, process),
        "-h" | "--help" | "help" => Ok(agent_usage()),
        other => Err(AgentCommandError::usage(format!(
            "unknown agent guidance command: {other}\n\n{}",
            agent_usage()
        ))),
    }
}

fn command_guidance_apply(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, guidance_apply_allowed_options())?;
    validate_guidance_write_authorization(&parsed)?;
    let host_kind = required_host_kind(&parsed)?;
    let target = guidance_target_from_host_kind(host_kind)?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "--project-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    if parsed.dry_run {
        let registry = inspect_agent_registry_for_planning(&runtime_home)?;
        let project =
            required_guidance_project_from_registry(&registry, integration_id, project_id)?;
        let plan = plan_guidance_apply(&project.repo_root, integration_id, project_id, target)?;
        let actions = vec![AgentAction::new(
            "guidance",
            planned_change_action(plan.change),
            format!("{} {}", target.as_str(), path_text(&plan.path)),
        )];
        let output = AgentOutput {
            status: AgentResultStatus::DryRun,
            runtime_home,
            registry_schema: registry.schema,
            integration_id: integration_id.to_owned(),
            host_plan: None,
            allowed_projects: vec![project_id.to_owned()],
            installations: Vec::new(),
            guidance: vec![plan.status],
            verification: McpVerification::skipped("dry run does not apply repository guidance"),
            actions,
            warnings: Vec::new(),
            action_required: Vec::new(),
            output: parsed.output,
        };
        return render_agent_output(&output);
    }
    let project = required_guidance_project(&runtime_home, integration_id, project_id)?;
    let plan = plan_guidance_apply(&project.repo_root, integration_id, project_id, target)?;
    let effect = apply_guidance_plan(&plan)?;
    let guidance = guidance_statuses_for_project(
        Some(&project.repo_root),
        integration_id,
        project_id,
        &[target],
    )?;
    let output = AgentOutput {
        status: AgentResultStatus::Complete,
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects: vec![project_id.to_owned()],
        installations: Vec::new(),
        guidance,
        verification: McpVerification::skipped("repository guidance applied"),
        actions: vec![AgentAction::new(
            "guidance_apply",
            planned_change_action(effect.change),
            format!("{} {}", target.as_str(), path_text(&effect.path)),
        )],
        warnings: Vec::new(),
        action_required: Vec::new(),
        output: parsed.output,
    };
    render_agent_output(&output)
}

fn command_guidance_status(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, guidance_status_allowed_options())?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "--project-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    if parsed.dry_run {
        let registry = inspect_agent_registry_for_planning(&runtime_home)?;
        let project =
            required_guidance_project_from_registry(&registry, integration_id, project_id)?;
        let targets = if let Some(host_kind) = parsed.host_kind {
            vec![guidance_target_from_host_kind(host_kind)?]
        } else {
            vec![GuidanceTarget::Codex, GuidanceTarget::ClaudeCode]
        };
        let plans = plan_guidance_remove_for_targets(
            &project.repo_root,
            integration_id,
            project_id,
            &targets,
        )?;
        let actions = plans
            .iter()
            .map(|plan| {
                AgentAction::new(
                    "guidance",
                    planned_change_action(plan.change),
                    format!("{} {}", plan.target.as_str(), path_text(&plan.path)),
                )
            })
            .collect::<Vec<_>>();
        let output = AgentOutput {
            status: AgentResultStatus::DryRun,
            runtime_home,
            registry_schema: registry.schema,
            integration_id: integration_id.to_owned(),
            host_plan: None,
            allowed_projects: vec![project_id.to_owned()],
            installations: Vec::new(),
            guidance: plans.iter().map(|plan| plan.status.clone()).collect(),
            verification: McpVerification::skipped("dry run does not remove repository guidance"),
            actions,
            warnings: Vec::new(),
            action_required: Vec::new(),
            output: parsed.output,
        };
        return render_agent_output(&output);
    }
    let project = required_guidance_project(&runtime_home, integration_id, project_id)?;
    let guidance = guidance_statuses_for_project(
        Some(&project.repo_root),
        integration_id,
        project_id,
        &[GuidanceTarget::Codex, GuidanceTarget::ClaudeCode],
    )?;
    let output = AgentOutput {
        status: if guidance.iter().any(|status| {
            matches!(
                status.state,
                GuidanceStateKind::Changed | GuidanceStateKind::Conflicted
            )
        }) {
            AgentResultStatus::Failed
        } else {
            AgentResultStatus::Complete
        },
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects: vec![project_id.to_owned()],
        installations: Vec::new(),
        guidance,
        verification: McpVerification::skipped("guidance status does not prove model behavior"),
        actions: Vec::new(),
        warnings: Vec::new(),
        action_required: Vec::new(),
        output: parsed.output,
    };
    match output.status {
        AgentResultStatus::Failed => Err(AgentCommandError::failure_output(render_agent_output(
            &output,
        )?)),
        _ => render_agent_output(&output),
    }
}

fn command_guidance_remove(
    args: &[String],
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    if is_help_request(args) {
        return Ok(agent_usage());
    }
    let parsed = parse_agent_options(args, guidance_remove_allowed_options())?;
    validate_guidance_remove_authorization(&parsed)?;
    let integration_id = required_text(parsed.integration_id.as_deref(), "--integration-id")?;
    let project_id = required_text(parsed.project_id.as_deref(), "--project-id")?;
    let runtime_home = resolve_agent_runtime_home(&parsed, current_dir, process)?;
    if parsed.dry_run {
        let registry = inspect_agent_registry_for_planning(&runtime_home)?;
        let project =
            required_guidance_project_from_registry(&registry, integration_id, project_id)?;
        let targets = if let Some(host_kind) = parsed.host_kind {
            vec![guidance_target_from_host_kind(host_kind)?]
        } else {
            vec![GuidanceTarget::Codex, GuidanceTarget::ClaudeCode]
        };
        let plans = plan_guidance_remove_for_targets(
            &project.repo_root,
            integration_id,
            project_id,
            &targets,
        )?;
        let actions = plans
            .iter()
            .map(|plan| {
                AgentAction::new(
                    "guidance",
                    planned_change_action(plan.change),
                    format!("{} {}", plan.target.as_str(), path_text(&plan.path)),
                )
            })
            .collect::<Vec<_>>();
        let output = AgentOutput {
            status: AgentResultStatus::DryRun,
            runtime_home,
            registry_schema: registry.schema,
            integration_id: integration_id.to_owned(),
            host_plan: None,
            allowed_projects: vec![project_id.to_owned()],
            installations: Vec::new(),
            guidance: plans.iter().map(|plan| plan.status.clone()).collect(),
            verification: McpVerification::skipped("dry run does not remove repository guidance"),
            actions,
            warnings: Vec::new(),
            action_required: Vec::new(),
            output: parsed.output,
        };
        return render_agent_output(&output);
    }
    let project = required_guidance_project(&runtime_home, integration_id, project_id)?;
    let targets = if let Some(host_kind) = parsed.host_kind {
        vec![guidance_target_from_host_kind(host_kind)?]
    } else {
        vec![GuidanceTarget::Codex, GuidanceTarget::ClaudeCode]
    };
    let plans =
        plan_guidance_remove_for_targets(&project.repo_root, integration_id, project_id, &targets)?;
    let mut effects = Vec::new();
    for plan in &plans {
        effects.push(apply_guidance_remove(plan)?);
    }
    let guidance = guidance_statuses_for_project(
        Some(&project.repo_root),
        integration_id,
        project_id,
        &targets,
    )?;
    let output = AgentOutput {
        status: AgentResultStatus::Complete,
        runtime_home,
        registry_schema: None,
        integration_id: integration_id.to_owned(),
        host_plan: None,
        allowed_projects: vec![project_id.to_owned()],
        installations: Vec::new(),
        guidance,
        verification: McpVerification::skipped("repository guidance removed"),
        actions: effects
            .iter()
            .map(|effect| {
                AgentAction::new(
                    "guidance_remove",
                    planned_change_action(effect.change),
                    format!("{} {}", effect.target.as_str(), path_text(&effect.path)),
                )
            })
            .collect(),
        warnings: Vec::new(),
        action_required: Vec::new(),
        output: parsed.output,
    };
    render_agent_output(&output)
}

fn required_guidance_project(
    runtime_home: &Path,
    integration_id: &str,
    project_id: &str,
) -> Result<ProjectRecord, AgentCommandError> {
    let _integration = required_integration(runtime_home, integration_id)?;
    validate_project_id(project_id)?;
    if !is_project_member(runtime_home, integration_id, project_id)? {
        return Err(AgentCommandError::runtime(
            "project is not allowed for this Agent Integration Profile",
        ));
    }
    project_record_for_execution(runtime_home, project_id)?.ok_or_else(|| {
        AgentCommandError::runtime(format!("project is not executable: {project_id}"))
    })
}

fn plan_guidance_for_targets(
    repo_root: &Path,
    integration_id: &str,
    project_id: &str,
    targets: &[GuidanceTarget],
) -> Result<Vec<GuidancePlan>, AgentCommandError> {
    targets
        .iter()
        .map(|target| {
            plan_guidance_apply(repo_root, integration_id, project_id, *target)
                .map_err(AgentCommandError::from)
        })
        .collect()
}

fn plan_guidance_remove_for_targets(
    repo_root: &Path,
    integration_id: &str,
    project_id: &str,
    targets: &[GuidanceTarget],
) -> Result<Vec<GuidancePlan>, AgentCommandError> {
    targets
        .iter()
        .map(|target| {
            plan_guidance_remove(repo_root, integration_id, project_id, *target)
                .map_err(AgentCommandError::from)
        })
        .collect()
}

fn guidance_statuses_for_project(
    repo_root: Option<&Path>,
    integration_id: &str,
    project_id: &str,
    targets: &[GuidanceTarget],
) -> Result<Vec<GuidanceStatus>, AgentCommandError> {
    let Some(repo_root) = repo_root else {
        return Ok(Vec::new());
    };
    targets
        .iter()
        .map(|target| {
            guidance_status(repo_root, integration_id, project_id, *target)
                .map_err(AgentCommandError::from)
        })
        .collect()
}

fn guidance_statuses_for_projects(
    integration_id: &str,
    projects: &[IntegrationProjectRecord],
) -> Result<Vec<GuidanceStatus>, AgentCommandError> {
    let mut statuses = Vec::new();
    for project in projects {
        statuses.extend(guidance_statuses_for_project(
            Some(&project.project.repo_root),
            integration_id,
            &project.project_id,
            &[GuidanceTarget::Codex, GuidanceTarget::ClaudeCode],
        )?);
    }
    Ok(statuses)
}

fn guidance_targets_for_status(targets: &[GuidanceTarget]) -> &[GuidanceTarget] {
    targets
}

fn compensate_guidance_effects(effects: &[GuidanceEffect], warnings: &mut Vec<String>) {
    for effect in effects {
        match compensate_new_guidance(effect) {
            Ok(compensation) => {
                if let Some(residual) = compensation.residual {
                    warnings.push(format!(
                        "residual guidance preserved for project {} {}: {}",
                        effect.project_id,
                        effect.target.as_str(),
                        residual
                    ));
                } else {
                    warnings.push(format!(
                        "compensated newly-created guidance for project {} {}",
                        effect.project_id,
                        effect.target.as_str()
                    ));
                }
            }
            Err(error) => warnings.push(format!(
                "residual guidance preserved for project {} {}: {}",
                effect.project_id,
                effect.target.as_str(),
                error
            )),
        }
    }
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
        if !token.starts_with("--") {
            return Err(AgentCommandError::usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, attached_value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name, Some(value))
        } else {
            (without_prefix, None)
        };
        if !allowed.contains(&name) {
            return Err(AgentCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !is_boolean_agent_option(name) && seen.insert(name.to_owned()) {
            // first occurrence recorded
        } else if !is_boolean_agent_option(name) {
            return Err(AgentCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        if is_boolean_agent_option(name) {
            if attached_value.is_some() {
                return Err(AgentCommandError::usage(format!(
                    "--{name} does not take a value"
                )));
            }
            set_agent_boolean(&mut parsed, name);
            index += 1;
            continue;
        }
        let value = if let Some(value) = attached_value {
            value.to_owned()
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(AgentCommandError::usage(format!(
                    "missing value for --{name}"
                )));
            };
            if value.starts_with("--") {
                return Err(AgentCommandError::usage(format!(
                    "missing value for --{name}"
                )));
            }
            value.clone()
        };
        set_agent_value(&mut parsed, name, value)?;
        index += 1;
    }
    Ok(parsed)
}

fn install_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "repo-root",
        "project-id",
        "integration-id",
        "default-project-id",
        "surface-id",
        "surface-instance-id",
        "host",
        "scope",
        "server-name",
        "mcp-command",
        "export-path",
        "export-dir",
        "guidance",
        "output",
        "dry-run",
        "yes",
        "allow-repository-write",
        "replace-managed",
    ]
}

fn project_add_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "repo-root",
        "project-id",
        "integration-id",
        "default",
        "output",
        "dry-run",
    ]
}

fn project_remove_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "project-id",
        "integration-id",
        "output",
        "dry-run",
    ]
}

fn status_allowed_options() -> &'static [&'static str] {
    &["runtime-home", "integration-id", "output"]
}

fn verify_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "integration-id",
        "installation-id",
        "output",
    ]
}

fn uninstall_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "integration-id",
        "installation-id",
        "output",
        "dry-run",
        "allow-repository-write",
        "remove-managed",
    ]
}

fn guidance_apply_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "integration-id",
        "project-id",
        "host",
        "output",
        "dry-run",
        "allow-repository-write",
        "replace-managed",
    ]
}

fn guidance_status_allowed_options() -> &'static [&'static str] {
    &["runtime-home", "integration-id", "project-id", "output"]
}

fn guidance_remove_allowed_options() -> &'static [&'static str] {
    &[
        "runtime-home",
        "integration-id",
        "project-id",
        "host",
        "output",
        "dry-run",
        "allow-repository-write",
        "remove-managed",
    ]
}

fn is_boolean_agent_option(name: &str) -> bool {
    matches!(
        name,
        "dry-run"
            | "yes"
            | "allow-repository-write"
            | "replace-managed"
            | "remove-managed"
            | "default"
    )
}

fn set_agent_boolean(parsed: &mut ParsedAgentOptions, name: &str) {
    match name {
        "dry-run" => parsed.dry_run = true,
        "yes" => parsed.yes = true,
        "allow-repository-write" => parsed.allow_repository_write = true,
        "replace-managed" => parsed.replace_managed = true,
        "remove-managed" => parsed.remove_managed = true,
        "default" => parsed.make_default = true,
        _ => unreachable!("boolean option was validated"),
    }
}

fn set_agent_value(
    parsed: &mut ParsedAgentOptions,
    name: &str,
    value: String,
) -> Result<(), AgentCommandError> {
    if value.trim().is_empty() {
        return Err(AgentCommandError::usage(format!(
            "--{name} must not be empty"
        )));
    }
    match name {
        "runtime-home" => parsed.runtime_home = Some(PathBuf::from(value)),
        "repo-root" => parsed.repo_root = Some(PathBuf::from(value)),
        "project-id" => parsed.project_id = Some(value),
        "integration-id" => parsed.integration_id = Some(value),
        "default-project-id" => parsed.default_project_id = Some(value),
        "surface-id" => parsed.surface_id = Some(value),
        "surface-instance-id" => parsed.surface_instance_id = Some(value),
        "host" => parsed.host_kind = Some(parse_host_kind(&value)?),
        "scope" => parsed.host_scope = Some(parse_host_scope(&value)?),
        "server-name" => parsed.server_name = Some(value),
        "installation-id" => parsed.installation_id = Some(value),
        "mcp-command" => parsed.mcp_command = Some(PathBuf::from(value)),
        "export-path" => parsed.export_path = Some(PathBuf::from(value)),
        "export-dir" => parsed.export_dir = Some(PathBuf::from(value)),
        "guidance" => parsed.guidance = parse_guidance_selection(&value)?,
        "output" => {
            parsed.output = match value.as_str() {
                "text" => OutputFormat::Text,
                "json" => OutputFormat::Json,
                other => {
                    return Err(AgentCommandError::usage(format!(
                        "unsupported output format: {other}"
                    )));
                }
            }
        }
        _ => unreachable!("value option was validated"),
    }
    Ok(())
}

fn parse_host_kind(value: &str) -> Result<HostKind, AgentCommandError> {
    match value {
        "codex" => Ok(HostKind::Codex),
        "claude-code" | "claude_code" => Ok(HostKind::ClaudeCode),
        "generic" => Ok(HostKind::Generic),
        other => Err(AgentCommandError::usage(format!(
            "unsupported host: {other}"
        ))),
    }
}

fn parse_guidance_selection(value: &str) -> Result<GuidanceSelection, AgentCommandError> {
    match value {
        "none" => Ok(GuidanceSelection::None),
        "codex" => Ok(GuidanceSelection::Codex),
        "claude-code" | "claude_code" => Ok(GuidanceSelection::ClaudeCode),
        "both" => Ok(GuidanceSelection::Both),
        other => Err(AgentCommandError::usage(format!(
            "unsupported guidance target: {other}"
        ))),
    }
}

fn guidance_target_from_host_kind(
    host_kind: HostKind,
) -> Result<GuidanceTarget, AgentCommandError> {
    match host_kind {
        HostKind::Codex => Ok(GuidanceTarget::Codex),
        HostKind::ClaudeCode => Ok(GuidanceTarget::ClaudeCode),
        HostKind::Generic => Err(AgentCommandError::usage(
            "repository guidance supports only codex and claude_code hosts",
        )),
    }
}

fn parse_host_scope(value: &str) -> Result<HostScope, AgentCommandError> {
    match value {
        "user" => Ok(HostScope::User),
        "project" => Ok(HostScope::Project),
        "local" => Ok(HostScope::Local),
        "export" => Ok(HostScope::Export),
        other => Err(AgentCommandError::usage(format!(
            "unsupported scope: {other}"
        ))),
    }
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
    option: &'static str,
) -> Result<&'a str, AgentCommandError> {
    value.ok_or_else(|| AgentCommandError::usage(format!("missing required option: {option}")))
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
            "host and scope must match the supported matrix",
        ))
    }
}

fn validate_repository_write_authorization(
    parsed: &ParsedAgentOptions,
    scope: HostScope,
) -> Result<(), AgentCommandError> {
    if scope == HostScope::Project && !parsed.dry_run && !parsed.allow_repository_write {
        return Err(AgentCommandError::usage(
            "--allow-repository-write is required for project-scoped host configuration writes",
        ));
    }
    Ok(())
}

fn validate_guidance_write_authorization(
    parsed: &ParsedAgentOptions,
) -> Result<(), AgentCommandError> {
    if !parsed.dry_run && !parsed.allow_repository_write {
        return Err(AgentCommandError::usage(
            "--allow-repository-write is required for repository guidance writes",
        ));
    }
    Ok(())
}

fn validate_guidance_remove_authorization(
    parsed: &ParsedAgentOptions,
) -> Result<(), AgentCommandError> {
    if !parsed.remove_managed {
        return Err(AgentCommandError::usage(
            "--remove-managed is required for repository guidance removal",
        ));
    }
    if !parsed.dry_run && !parsed.allow_repository_write {
        return Err(AgentCommandError::usage(
            "--allow-repository-write is required for repository guidance removal",
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
        if !path.is_absolute() {
            return Err(AgentCommandError::usage(
                "--runtime-home must be an absolute path",
            ));
        }
    }
    let resolved = resolve_runtime_home(
        |name| {
            if name == HARNESS_HOME {
                parsed
                    .runtime_home
                    .as_ref()
                    .map(|path| path.as_os_str().to_os_string())
                    .or_else(|| process.env_var(name))
            } else {
                process.env_var(name)
            }
        },
        current_dir,
    )?;
    if resolved.is_absolute() {
        Ok(resolved)
    } else {
        Ok(current_dir.join(resolved))
    }
}

fn resolve_optional_repo_root(
    repo_root: Option<&Path>,
    current_dir: &Path,
) -> Result<Option<PathBuf>, AgentCommandError> {
    repo_root
        .map(|path| {
            canonical_existing_dir(&absolute_path(current_dir, path.to_path_buf()), "repo-root")
        })
        .transpose()
}

fn canonical_existing_dir(path: &Path, field: &'static str) -> Result<PathBuf, AgentCommandError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        AgentCommandError::runtime(format!("{field} is not accessible: {error}"))
    })?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(AgentCommandError::runtime(format!(
            "{field} must be a directory"
        )))
    }
}

#[derive(Debug, Clone)]
struct InstallProjectPlan {
    project_id: String,
    repo_root: Option<PathBuf>,
    existing_project: Option<ProjectRecord>,
    action: ActionState,
}

fn resolve_install_project(
    runtime_home: &Path,
    project_id: Option<&str>,
    repo_root: Option<PathBuf>,
) -> Result<InstallProjectPlan, AgentCommandError> {
    let projects = list_projects(runtime_home)?;
    let selected = match project_id {
        Some(project_id) => {
            validate_project_id(project_id)?;
            let existing = project_record_for_execution(runtime_home, project_id)?;
            if let (Some(existing), Some(repo_root)) = (&existing, &repo_root) {
                if !project_repo_matches(existing, repo_root) {
                    return Err(AgentCommandError::runtime(
                        "project is registered to another repo_root",
                    ));
                }
            }
            let repo_root =
                repo_root.or_else(|| existing.as_ref().map(|project| project.repo_root.clone()));
            if existing.is_none() && repo_root.is_none() {
                return Err(AgentCommandError::usage(
                    "--repo-root is required when --project-id is not already registered",
                ));
            }
            (project_id.to_owned(), repo_root, existing)
        }
        None => {
            let repo_root = repo_root.ok_or_else(|| {
                AgentCommandError::usage(
                    "--project-id or --repo-root is required; omitted --project-id resolves only an existing unique registration",
                )
            })?;
            let matches = projects
                .iter()
                .filter(|project| project_repo_matches(project, &repo_root))
                .cloned()
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [project] => (
                    project.project_id.clone(),
                    Some(project.repo_root.clone()),
                    Some(
                        project_record_for_execution(runtime_home, &project.project_id)?
                            .ok_or_else(|| {
                                AgentCommandError::runtime("matched project is not executable")
                            })?,
                    ),
                ),
                [] => {
                    return Err(AgentCommandError::usage(
                        "--project-id is required when --repo-root has no existing unique registration",
                    ));
                }
                _ => {
                    return Err(AgentCommandError::runtime(
                        "multiple projects are registered for repo_root; pass --project-id",
                    ));
                }
            }
        }
    };
    let (project_id, repo_root, existing_project) = selected;
    Ok(InstallProjectPlan {
        action: if existing_project.is_some() {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        project_id,
        repo_root,
        existing_project,
    })
}

impl AgentRegistryPlan {
    fn from_snapshot(snapshot: RegistryInspectionSnapshot) -> Self {
        Self {
            schema: Some(registry_schema_plan(&snapshot.schema)),
            projects: snapshot.projects,
            integrations: snapshot.agent_integrations,
            integration_projects: snapshot.integration_projects,
            host_installations: snapshot.host_installations,
        }
    }

    fn integration(&self, integration_id: &str) -> Option<&AgentIntegrationInspectionRecord> {
        self.integrations
            .iter()
            .find(|record| record.integration_id == integration_id)
    }

    fn required_integration(
        &self,
        integration_id: &str,
    ) -> Result<AgentIntegrationRecord, AgentCommandError> {
        self.integration(integration_id)
            .map(agent_integration_record_from_inspection)
            .ok_or_else(|| {
                AgentCommandError::runtime(format!(
                    "Agent Integration Profile not found: {integration_id}"
                ))
            })
    }

    fn project(&self, project_id: &str) -> Option<&ProjectInspectionRecord> {
        self.projects
            .iter()
            .find(|record| record.project_id == project_id)
    }

    fn executable_project(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectRecord>, AgentCommandError> {
        let Some(project) = self.project(project_id) else {
            return Ok(None);
        };
        executable_project_record_from_inspection(project).map(Some)
    }

    fn is_project_member(&self, integration_id: &str, project_id: &str) -> bool {
        self.integration_projects.iter().any(|record| {
            record.integration_id == integration_id && record.project_id == project_id
        })
    }

    fn host_installations_for_integration(
        &self,
        integration_id: &str,
    ) -> Vec<HostInstallationRecord> {
        self.host_installations
            .iter()
            .filter(|record| record.integration_id == integration_id)
            .map(host_installation_record_from_inspection)
            .collect()
    }

    fn find_installation_for_target_hint(
        &self,
        integration_id: &str,
        host_kind: HostKind,
        host_scope: HostScope,
        server_name: Option<&str>,
    ) -> Option<&HostInstallationInspectionRecord> {
        self.host_installations.iter().find(|record| {
            record.integration_id == integration_id
                && record.host_kind == host_kind.as_str()
                && record.host_scope == host_scope.as_str()
                && server_name
                    .map(|name| record.server_name == name)
                    .unwrap_or(true)
        })
    }

    fn integration_project_plan_records(
        &self,
        integration_id: &str,
    ) -> Result<Vec<IntegrationProjectPlanRecord>, AgentCommandError> {
        let mut records = Vec::new();
        for membership in self
            .integration_projects
            .iter()
            .filter(|record| record.integration_id == integration_id)
        {
            let project = self
                .executable_project(&membership.project_id)?
                .ok_or_else(|| {
                    AgentCommandError::runtime(format!(
                        "project is not executable: {}",
                        membership.project_id
                    ))
                })?;
            records.push(IntegrationProjectPlanRecord {
                project_id: membership.project_id.clone(),
                project,
            });
        }
        Ok(records)
    }

    fn project_surface_exists(
        &self,
        project_id: &str,
        surface_id: &str,
        surface_instance_id: &str,
    ) -> Result<bool, AgentCommandError> {
        let Some(project) = self.project(project_id) else {
            return Ok(false);
        };
        match &project.project_state {
            DatabaseInspection::Missing { .. } => Ok(false),
            DatabaseInspection::Present(snapshot) => Ok(snapshot.surfaces.iter().any(|surface| {
                surface.surface_id == surface_id
                    && surface.surface_instance_id == surface_instance_id
            })),
            DatabaseInspection::Unsupported {
                detected_version,
                latest_supported_version,
                detail,
                ..
            } => Err(AgentCommandError::runtime(format!(
                "project state schema version {detected_version} is not supported (latest supported {latest_supported_version}): {detail}"
            ))),
            DatabaseInspection::Malformed { detail, .. }
            | DatabaseInspection::Unreadable { detail, .. } => Err(AgentCommandError::runtime(
                format!("project state inspection failed: {detail}"),
            )),
        }
    }
}

fn inspect_agent_registry_for_planning(
    runtime_home: &Path,
) -> Result<AgentRegistryPlan, AgentCommandError> {
    match inspect_runtime_home(runtime_home).registry {
        DatabaseInspection::Missing { .. } => Ok(AgentRegistryPlan {
            schema: None,
            projects: Vec::new(),
            integrations: Vec::new(),
            integration_projects: Vec::new(),
            host_installations: Vec::new(),
        }),
        DatabaseInspection::Present(snapshot) => Ok(AgentRegistryPlan::from_snapshot(snapshot)),
        DatabaseInspection::Unsupported {
            detected_version,
            latest_supported_version,
            detail,
            ..
        } => Err(AgentCommandError::runtime(format!(
            "registry schema version {detected_version} is not supported (latest supported {latest_supported_version}): {detail}"
        ))),
        DatabaseInspection::Malformed { detail, .. }
        | DatabaseInspection::Unreadable { detail, .. } => Err(AgentCommandError::runtime(
            format!("registry inspection failed: {detail}"),
        )),
    }
}

fn registry_schema_plan(schema: &InspectionSchemaState) -> RegistrySchemaPlan {
    match schema {
        InspectionSchemaState::Current { version } => RegistrySchemaPlan {
            current_version: *version,
            latest_supported_version: REGISTRY_SCHEMA_VERSION,
            migration_planned: false,
        },
        InspectionSchemaState::MigrationRequired {
            detected_version,
            latest_supported_version,
        } => RegistrySchemaPlan {
            current_version: *detected_version,
            latest_supported_version: *latest_supported_version,
            migration_planned: true,
        },
    }
}

fn resolve_install_project_from_registry(
    registry: &AgentRegistryPlan,
    _runtime_home: &Path,
    project_id: Option<&str>,
    repo_root: Option<PathBuf>,
) -> Result<InstallProjectPlan, AgentCommandError> {
    let selected = match project_id {
        Some(project_id) => {
            validate_project_id(project_id)?;
            let existing = registry.executable_project(project_id)?;
            if let (Some(existing), Some(repo_root)) = (&existing, &repo_root) {
                if !project_repo_matches(existing, repo_root) {
                    return Err(AgentCommandError::runtime(
                        "project is registered to another repo_root",
                    ));
                }
            }
            let repo_root =
                repo_root.or_else(|| existing.as_ref().map(|project| project.repo_root.clone()));
            if existing.is_none() && repo_root.is_none() {
                return Err(AgentCommandError::usage(
                    "--repo-root is required when --project-id is not already registered",
                ));
            }
            (project_id.to_owned(), repo_root, existing)
        }
        None => {
            let repo_root = repo_root.ok_or_else(|| {
                AgentCommandError::usage(
                    "--project-id or --repo-root is required; omitted --project-id resolves only an existing unique registration",
                )
            })?;
            let matches = registry
                .projects
                .iter()
                .filter(|project| {
                    project_repo_matches(&project_record_from_inspection(project), &repo_root)
                })
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [project] => (
                    project.project_id.clone(),
                    Some(project_record_from_inspection(project).repo_root),
                    Some(
                        registry
                            .executable_project(&project.project_id)?
                            .ok_or_else(|| {
                                AgentCommandError::runtime("matched project is not executable")
                            })?,
                    ),
                ),
                [] => {
                    return Err(AgentCommandError::usage(
                        "--project-id is required when --repo-root has no existing unique registration",
                    ));
                }
                _ => {
                    return Err(AgentCommandError::runtime(
                        "multiple projects are registered for repo_root; pass --project-id",
                    ));
                }
            }
        }
    };
    let (project_id, repo_root, existing_project) = selected;
    Ok(InstallProjectPlan {
        action: if existing_project.is_some() {
            ActionState::Reused
        } else {
            ActionState::Planned
        },
        project_id,
        repo_root,
        existing_project,
    })
}

fn executable_project_record_from_inspection(
    project: &ProjectInspectionRecord,
) -> Result<ProjectRecord, AgentCommandError> {
    match &project.project_state {
        DatabaseInspection::Present(_) => Ok(project_record_from_inspection(project)),
        DatabaseInspection::Missing { path } => Err(AgentCommandError::runtime(format!(
            "project is not executable: missing project state database {}",
            path.display()
        ))),
        DatabaseInspection::Unsupported {
            detected_version,
            latest_supported_version,
            detail,
            ..
        } => Err(AgentCommandError::runtime(format!(
            "project state schema version {detected_version} is not supported (latest supported {latest_supported_version}): {detail}"
        ))),
        DatabaseInspection::Malformed { detail, .. }
        | DatabaseInspection::Unreadable { detail, .. } => Err(AgentCommandError::runtime(
            format!("project is not executable: {detail}"),
        )),
    }
}

fn project_record_from_inspection(project: &ProjectInspectionRecord) -> ProjectRecord {
    ProjectRecord {
        project_id: project.project_id.clone(),
        runtime_home_id: project.runtime_home_id.clone(),
        repo_root: project.repo_root.clone(),
        project_home: project.project_home.clone(),
        state_db_path: project.state_db_path.clone(),
        status: project.status.clone(),
        metadata_json: project.metadata_json.clone(),
    }
}

fn agent_integration_record_from_inspection(
    integration: &AgentIntegrationInspectionRecord,
) -> AgentIntegrationRecord {
    AgentIntegrationRecord {
        integration_id: integration.integration_id.clone(),
        interaction_role: integration.interaction_role.clone(),
        surface_id: integration.surface_id.clone(),
        surface_instance_id: integration.surface_instance_id.clone(),
        default_project_id: integration.default_project_id.clone(),
        enabled: integration.enabled,
        created_at: integration.created_at.clone(),
        updated_at: integration.updated_at.clone(),
        metadata_json: integration.metadata_json.clone(),
    }
}

fn host_installation_record_from_inspection(
    installation: &HostInstallationInspectionRecord,
) -> HostInstallationRecord {
    HostInstallationRecord {
        installation_id: installation.installation_id.clone(),
        integration_id: installation.integration_id.clone(),
        host_kind: installation.host_kind.clone(),
        host_scope: installation.host_scope.clone(),
        server_name: installation.server_name.clone(),
        config_target: installation.config_target.clone(),
        managed_fingerprint: installation.managed_fingerprint.clone(),
        last_verified_status: installation.last_verified_status.clone(),
        created_at: installation.created_at.clone(),
        updated_at: installation.updated_at.clone(),
        metadata_json: installation.metadata_json.clone(),
    }
}

fn selected_installations_from_registry(
    registry: &AgentRegistryPlan,
    integration_id: &str,
    installation_id: Option<&str>,
) -> Result<Vec<HostInstallationRecord>, AgentCommandError> {
    if let Some(installation_id) = installation_id {
        let record = registry
            .host_installations
            .iter()
            .find(|record| record.installation_id == installation_id)
            .ok_or_else(|| {
                AgentCommandError::runtime(format!(
                    "Host Installation not found: {installation_id}"
                ))
            })?;
        if record.integration_id != integration_id {
            return Err(AgentCommandError::runtime(
                "installation_id belongs to another integration",
            ));
        }
        Ok(vec![host_installation_record_from_inspection(record)])
    } else {
        let records = registry.host_installations_for_integration(integration_id);
        if records.is_empty() {
            Err(AgentCommandError::runtime(
                "no Host Installation records are registered for this integration",
            ))
        } else {
            Ok(records)
        }
    }
}

fn validate_project_scope_membership_from_registry(
    registry: &AgentRegistryPlan,
    integration_id: &str,
    scope: HostScope,
    project_id: &str,
) -> Result<(), AgentCommandError> {
    if !matches!(scope, HostScope::Project | HostScope::Local) {
        return Ok(());
    }
    if registry
        .integration_projects
        .iter()
        .any(|project| project.integration_id == integration_id && project.project_id != project_id)
    {
        return Err(AgentCommandError::runtime(
            "project and local scoped integrations may allow only their associated Product Repository",
        ));
    }
    Ok(())
}

fn validate_add_membership_scope_from_registry(
    registry: &AgentRegistryPlan,
    integration_id: &str,
    project_id: &str,
) -> Result<(), AgentCommandError> {
    if registry
        .host_installations
        .iter()
        .filter(|installation| installation.integration_id == integration_id)
        .any(|installation| {
            matches!(
                installation.host_scope.as_str(),
                HOST_SCOPE_PROJECT | HOST_SCOPE_LOCAL
            )
        })
        && registry.integration_projects.iter().any(|project| {
            project.integration_id == integration_id && project.project_id != project_id
        })
    {
        return Err(AgentCommandError::runtime(
            "project and local scoped integrations cannot add a second project",
        ));
    }
    Ok(())
}

fn required_guidance_project_from_registry(
    registry: &AgentRegistryPlan,
    integration_id: &str,
    project_id: &str,
) -> Result<ProjectRecord, AgentCommandError> {
    let _integration = registry.required_integration(integration_id)?;
    validate_project_id(project_id)?;
    if !registry.is_project_member(integration_id, project_id) {
        return Err(AgentCommandError::runtime(
            "project is not allowed for this Agent Integration Profile",
        ));
    }
    registry.executable_project(project_id)?.ok_or_else(|| {
        AgentCommandError::runtime(format!("project is not executable: {project_id}"))
    })
}

fn guidance_statuses_for_plan_projects(
    integration_id: &str,
    projects: &[IntegrationProjectPlanRecord],
) -> Result<Vec<GuidanceStatus>, AgentCommandError> {
    let mut statuses = Vec::new();
    for project in projects {
        statuses.extend(guidance_statuses_for_project(
            Some(&project.project.repo_root),
            integration_id,
            &project.project_id,
            &[GuidanceTarget::Codex, GuidanceTarget::ClaudeCode],
        )?);
    }
    Ok(statuses)
}

fn resolve_mcp_command(
    parsed: &ParsedAgentOptions,
    scope: HostScope,
    current_dir: &Path,
    process: &impl AgentProcess,
) -> Result<PathBuf, AgentCommandError> {
    if scope == HostScope::Project {
        if let Some(command) = &parsed.mcp_command {
            if command == Path::new(DEFAULT_MCP_COMMAND) {
                return Ok(command.clone());
            }
            return Ok(absolute_path(current_dir, command.clone()));
        }
        return Ok(PathBuf::from(DEFAULT_MCP_COMMAND));
    }
    if let Some(command) = &parsed.mcp_command {
        let command = absolute_path(current_dir, command.clone());
        if command.is_absolute() && command.exists() {
            return canonical_file(&command, "mcp-command");
        }
        return Err(AgentCommandError::runtime(
            "mcp-command must be an existing absolute executable path for this scope",
        ));
    }
    discover_mcp_command(current_dir, process)
}

fn discover_mcp_command(
    current_dir: &Path,
    process: &impl AgentProcess,
) -> Result<PathBuf, AgentCommandError> {
    let current_exe = absolute_path(
        current_dir,
        process.current_exe().map_err(AgentCommandError::runtime)?,
    );
    if let Some(parent) = current_exe.parent() {
        let candidate = parent.join(DEFAULT_MCP_COMMAND);
        if candidate.is_file() {
            return canonical_file(&candidate, "harness-mcp sibling");
        }
    }
    if let Some(path_env) = process.env_var(PATH_ENV) {
        for directory in std::env::split_paths(&path_env) {
            let candidate = absolute_path(current_dir, directory).join(DEFAULT_MCP_COMMAND);
            if candidate.is_file() {
                return canonical_file(&candidate, "harness-mcp PATH entry");
            }
        }
    }
    Err(AgentCommandError::runtime(
        "could not discover harness-mcp executable; pass --mcp-command",
    ))
}

fn canonical_file(path: &Path, label: &str) -> Result<PathBuf, AgentCommandError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        AgentCommandError::runtime(format!("{label} is not accessible: {error}"))
    })?;
    if canonical.is_file() {
        Ok(canonical)
    } else {
        Err(AgentCommandError::runtime(format!(
            "{label} must be a file: {}",
            canonical.display()
        )))
    }
}

struct HostPlanInputs<'a> {
    host_kind: HostKind,
    host_scope: HostScope,
    integration_id: &'a str,
    server_name: Option<&'a str>,
    repo_root: Option<&'a Path>,
    mcp_command: &'a Path,
    runtime_home: Option<&'a Path>,
    expected_fingerprint: Option<&'a str>,
    parsed: &'a ParsedAgentOptions,
    current_dir: &'a Path,
}

fn build_host_plan(
    inputs: HostPlanInputs<'_>,
    process: &mut impl AgentProcess,
) -> Result<HostPlan, AgentCommandError> {
    match inputs.host_kind {
        HostKind::Codex => {
            let adapter = CodexAdapter::new(CodexEnvironment {
                home: process.env_var("HOME").map(PathBuf::from),
                codex_home: process.env_var("CODEX_HOME").map(PathBuf::from),
                path: process.env_var(PATH_ENV),
            });
            Ok(
                adapter.plan(crate::host_integration::codex::CodexPlanRequest {
                    scope: inputs.host_scope,
                    integration_id: inputs.integration_id,
                    explicit_server_name: inputs.server_name,
                    repo_root: inputs.repo_root,
                    mcp_command: inputs.mcp_command,
                    runtime_home: inputs.runtime_home,
                    expected_fingerprint: inputs.expected_fingerprint,
                })?,
            )
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            Ok(
                adapter.plan(crate::host_integration::claude_code::ClaudePlanRequest {
                    scope: inputs.host_scope,
                    integration_id: inputs.integration_id,
                    explicit_server_name: inputs.server_name,
                    repo_root: inputs.repo_root,
                    mcp_command: inputs.mcp_command,
                    runtime_home: inputs.runtime_home,
                    expected_fingerprint: inputs.expected_fingerprint,
                })?,
            )
        }
        HostKind::Generic => {
            let adapter = GenericAdapter;
            let output_dir = inputs
                .parsed
                .export_dir
                .as_ref()
                .map(|path| absolute_path(inputs.current_dir, path.clone()))
                .unwrap_or_else(|| inputs.current_dir.to_path_buf());
            let output_path = inputs
                .parsed
                .export_path
                .as_ref()
                .map(|path| absolute_path(inputs.current_dir, path.clone()));
            Ok(
                adapter.plan(crate::host_integration::generic::GenericPlanRequest {
                    scope: inputs.host_scope,
                    integration_id: inputs.integration_id,
                    explicit_server_name: inputs.server_name,
                    output_dir: &output_dir,
                    output_path: output_path.as_deref(),
                    mcp_command: inputs.mcp_command,
                    runtime_home: inputs.runtime_home,
                    expected_fingerprint: inputs.expected_fingerprint,
                })?,
            )
        }
    }
}

fn apply_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    _process: &mut impl AgentProcess,
) -> Result<crate::host_integration::HostEffect, HostConfigError> {
    match host_kind {
        HostKind::Codex => {
            let mut adapter = CodexAdapter::new(CodexEnvironment::default());
            adapter.apply(plan)
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter.apply(plan)
        }
        HostKind::Generic => {
            let mut adapter = GenericAdapter;
            adapter.apply(plan)
        }
    }
}

fn verify_host_plan(
    host_kind: HostKind,
    plan: &HostPlan,
    _process: &mut impl AgentProcess,
) -> Result<crate::host_integration::verification::Verification, HostConfigError> {
    match host_kind {
        HostKind::Codex => {
            let mut adapter = CodexAdapter::new(CodexEnvironment::default());
            adapter.verify(plan)
        }
        HostKind::ClaudeCode => {
            let mut adapter = ClaudeCodeAdapter::new(ProductionCommandRunner);
            adapter.verify(plan)
        }
        HostKind::Generic => {
            let mut adapter = GenericAdapter;
            adapter.verify(plan)
        }
    }
}

fn ensure_agent_surface(
    runtime_home: &Path,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> Result<(), AgentCommandError> {
    let expected_access = BASELINE_WORKFLOW_ACCESS_CLASSES.to_vec();
    for surface in list_surfaces(runtime_home, project_id)? {
        if surface.surface_id == surface_id && surface.surface_instance_id == surface_instance_id {
            if surface.surface_kind != AGENT_SURFACE_KIND
                || surface.interaction_role != SurfaceInteractionRole::Agent.as_str()
                || !surface_access_matches(&surface.local_access_json, &expected_access)
            {
                return Err(AgentCommandError::runtime(
                    "existing integration surface is incompatible",
                ));
            }
            return Ok(());
        }
    }
    register_surface(
        runtime_home,
        SurfaceRegistration {
            project_id: project_id.to_owned(),
            surface_id: surface_id.to_owned(),
            surface_instance_id: surface_instance_id.to_owned(),
            surface_kind: AGENT_SURFACE_KIND.to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Harness Agent MCP".to_owned()),
            capability_profile_json: capability_profile_json(&expected_access, None)?,
            local_access_json: local_access_json(&expected_access)?,
            metadata_json: AGENT_METADATA_JSON.to_owned(),
        },
    )?;
    Ok(())
}

fn surface_exists_for_project(
    runtime_home: &Path,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> Result<bool, AgentCommandError> {
    Ok(list_surfaces(runtime_home, project_id)?
        .iter()
        .any(|surface| {
            surface.surface_id == surface_id && surface.surface_instance_id == surface_instance_id
        }))
}

fn surface_access_matches(text: &str, expected: &[AccessClass]) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    let Some(items) = value
        .get("authorized_access_classes")
        .and_then(Value::as_array)
    else {
        return false;
    };
    let actual = items
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    expected
        .iter()
        .all(|access| actual.contains(access.as_str()))
}

fn mark_planned_actions_created(actions: &mut [AgentAction]) {
    for action in actions {
        if action.state == ActionState::Planned {
            action.state = ActionState::Created;
        }
    }
}

fn validate_project_scope_membership(
    runtime_home: &Path,
    integration_id: &str,
    scope: HostScope,
    project_id: &str,
) -> Result<(), AgentCommandError> {
    if !matches!(scope, HostScope::Project | HostScope::Local) {
        return Ok(());
    }
    let projects = list_integration_projects(runtime_home, integration_id).unwrap_or_default();
    if projects
        .iter()
        .any(|project| project.project_id != project_id)
    {
        return Err(AgentCommandError::runtime(
            "project and local scoped integrations may allow only their associated Product Repository",
        ));
    }
    Ok(())
}

fn validate_add_membership_scope(
    runtime_home: &Path,
    integration_id: &str,
    project_id: &str,
) -> Result<(), AgentCommandError> {
    let installations = list_host_installations_for_integration(runtime_home, integration_id)?;
    if installations.iter().any(|installation| {
        matches!(
            installation.host_scope.as_str(),
            HOST_SCOPE_PROJECT | HOST_SCOPE_LOCAL
        )
    }) {
        let existing = list_integration_projects(runtime_home, integration_id)?;
        if existing
            .iter()
            .any(|project| project.project_id != project_id)
        {
            return Err(AgentCommandError::runtime(
                "project and local scoped integrations cannot add a second project",
            ));
        }
    }
    Ok(())
}

fn run_integration_preflight(
    process: &mut impl AgentProcess,
    command: &Path,
    runtime_home: &Path,
    integration_id: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    let output = process.run_preflight(command, runtime_home, integration_id, project_id)?;
    if !output.success {
        return Err(format!(
            "process exited {}; stderr: {}",
            status_text(output.status_code),
            compact_stream(&output.stderr)
        ));
    }
    validate_integration_preflight_report(runtime_home, integration_id, &output.stdout)
}

fn validate_integration_preflight_report(
    runtime_home: &Path,
    integration_id: &str,
    stdout: &str,
) -> Result<(), String> {
    let report = parse_colon_report(stdout)?;
    expect_report_field(&report, "configuration", "valid")?;
    expect_report_field(&report, "transport", "stdio")?;
    expect_report_field(&report, "runtime_home", &path_text(runtime_home))?;
    expect_report_field(&report, "integration_id", integration_id)?;
    expect_report_field(&report, "interaction_role", AGENT_INTERACTION_ROLE)?;
    expect_report_field(&report, "verification_scope", "startup_check_only")?;
    Ok(())
}

fn parse_colon_report(stdout: &str) -> Result<BTreeMap<String, String>, String> {
    let mut report = BTreeMap::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("malformed report line: {line}"));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err("malformed report line with empty key".to_owned());
        }
        if report
            .insert(key.to_owned(), value.trim_start().to_owned())
            .is_some()
        {
            return Err(format!("duplicate report field: {key}"));
        }
    }
    Ok(report)
}

fn expect_report_field(
    report: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    match report.get(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!("expected {key}: {expected}, got {actual}")),
        None => Err(format!("missing report field: {key}")),
    }
}

fn verify_mcp_stdio_process(
    command: &Path,
    runtime_home: &Path,
    integration_id: &str,
    timeout: Duration,
) -> Result<McpVerification, String> {
    let mut child = Command::new(command)
        .arg("--integration")
        .arg(integration_id)
        .env(HARNESS_HOME, runtime_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "failed to launch MCP command {}: {error}",
                command.display()
            )
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open MCP stdin".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to open MCP stdout".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to open MCP stderr".to_owned())?;

    let (line_tx, line_rx) = mpsc::channel::<Result<String, String>>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let _ = line_tx.send(line.map_err(|error| error.to_string()));
        }
    });
    let stderr_handle = thread::spawn(move || {
        let mut stderr = stderr;
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let deadline = Instant::now() + timeout;
    write_json_line(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "harness-agent-verifier",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
    )?;
    let initialize = read_json_response(&line_rx, deadline)?;
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
    let tools = read_json_response(&line_rx, deadline)?;
    let tool_names = validate_tools_response(&tools)?;
    drop(stdin);
    terminate_child(&mut child, deadline)?;
    let stderr = stderr_handle.join().unwrap_or_default();
    if !stderr.trim().is_empty() {
        return Ok(McpVerification {
            status: VerificationStatus::Complete,
            host_state: HostVerificationState::NotVerified,
            managed_config: ManagedConfigStatus::NotApplicable,
            host_executable: HostExecutableStatus::NotChecked,
            host_gate: HostGateStatus::NotApplicable,
            host_configuration: HostConfigurationStatus::NotApplicable,
            mcp_handshake_allowed: false,
            details: format!(
                "MCP initialize and tools/list succeeded; stderr: {}",
                compact_stream(&stderr)
            ),
            host_diagnostic: None,
            instructions_present: true,
            tools: tool_names,
        });
    }
    Ok(McpVerification {
        status: VerificationStatus::Complete,
        host_state: HostVerificationState::NotVerified,
        managed_config: ManagedConfigStatus::NotApplicable,
        host_executable: HostExecutableStatus::NotChecked,
        host_gate: HostGateStatus::NotApplicable,
        host_configuration: HostConfigurationStatus::NotApplicable,
        mcp_handshake_allowed: false,
        details: "MCP initialize and tools/list succeeded".to_owned(),
        host_diagnostic: None,
        instructions_present: true,
        tools: tool_names,
    })
}

fn write_json_line(writer: &mut impl Write, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, &value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn read_json_response(
    rx: &mpsc::Receiver<Result<String, String>>,
    deadline: Instant,
) -> Result<Value, String> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| "MCP verification timed out".to_owned())?;
    let line = rx
        .recv_timeout(remaining)
        .map_err(|_| "MCP verification timed out waiting for response".to_owned())?
        .map_err(|error| format!("failed reading MCP stdout: {error}"))?;
    serde_json::from_str::<Value>(&line)
        .map_err(|error| format!("MCP response was not valid JSON: {error}; line: {line}"))
}

fn validate_initialize_response(value: &Value) -> Result<(), String> {
    if value.get("error").is_some() {
        return Err(format!("initialize returned error: {value}"));
    }
    let result = value
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| "initialize response is missing result object".to_owned())?;
    match result.get("instructions").and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Ok(()),
        _ => Err("initialize response is missing server instructions".to_owned()),
    }
}

fn validate_tools_response(value: &Value) -> Result<Vec<String>, String> {
    if value.get("error").is_some() {
        return Err(format!("tools/list returned error: {value}"));
    }
    let tools = value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .ok_or_else(|| "tools/list response is missing result.tools array".to_owned())?;
    let names = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for required in PUBLIC_METHOD_TOOL_NAMES {
        if !names.iter().any(|name| name == required) {
            return Err(format!(
                "tools/list is missing required Core tool: {required}"
            ));
        }
    }
    if !names.iter().any(|name| name == LIST_PROJECTS_TOOL_NAME) {
        return Err(format!(
            "tools/list is missing required utility tool: {LIST_PROJECTS_TOOL_NAME}"
        ));
    }
    Ok(names)
}

fn terminate_child(child: &mut Child, deadline: Instant) -> Result<(), String> {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("MCP process did not exit before timeout".to_owned());
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => return Err(format!("failed waiting for MCP process: {error}")),
        }
    }
}

#[derive(Debug)]
struct AgentOutput {
    status: AgentResultStatus,
    runtime_home: PathBuf,
    registry_schema: Option<RegistrySchemaPlan>,
    integration_id: String,
    host_plan: Option<HostPlan>,
    allowed_projects: Vec<String>,
    installations: Vec<HostInstallationRecord>,
    guidance: Vec<GuidanceStatus>,
    verification: McpVerification,
    actions: Vec<AgentAction>,
    warnings: Vec<String>,
    action_required: Vec<String>,
    output: OutputFormat,
}

fn render_agent_output(output: &AgentOutput) -> Result<String, AgentCommandError> {
    match output.output {
        OutputFormat::Text => render_agent_text(output),
        OutputFormat::Json => render_agent_json(output),
    }
}

fn render_agent_text(output: &AgentOutput) -> Result<String, AgentCommandError> {
    let mut text = String::new();
    text.push_str(&format!("status: {}\n", output.status.as_str()));
    text.push_str(&format!(
        "runtime_home: {}\n",
        output.runtime_home.display()
    ));
    if let Some(schema) = output.registry_schema {
        text.push_str(&format!(
            "registry_schema_version: {}\n",
            schema.current_version
        ));
        if schema.migration_planned {
            text.push_str(&format!(
                "registry_migration: planned ({} -> {})\n",
                schema.current_version, schema.latest_supported_version
            ));
        } else {
            text.push_str("registry_migration: not_required\n");
        }
    }
    text.push_str(&format!("integration_id: {}\n", output.integration_id));
    if let Some(plan) = &output.host_plan {
        text.push_str(&format!("host_kind: {}\n", plan.host_kind.as_str()));
        text.push_str(&format!("host_scope: {}\n", plan.host_scope.as_str()));
        text.push_str(&format!("server_name: {}\n", plan.server_name));
        text.push_str(&format!(
            "host_target: {}\n",
            host_target_text(&plan.target)
        ));
    }
    if !output.allowed_projects.is_empty() {
        text.push_str("allowed_projects:\n");
        for project in &output.allowed_projects {
            text.push_str(&format!("  {project}\n"));
        }
    }
    if !output.installations.is_empty() {
        text.push_str("installations:\n");
        for installation in &output.installations {
            text.push_str(&format!(
                "  {}: {} {} {} {}\n",
                installation.installation_id,
                installation.host_kind,
                installation.host_scope,
                installation.server_name,
                installation.last_verified_status
            ));
        }
    }
    if !output.guidance.is_empty() {
        text.push_str("guidance:\n");
        for item in &output.guidance {
            text.push_str(&format!(
                "  {} {}: {} ({})\n",
                item.project_id,
                item.target.as_str(),
                item.state.as_str(),
                item.path.display()
            ));
            if !item.detail.is_empty() {
                text.push_str(&format!("    detail: {}\n", item.detail));
            }
            if let Some(content) = &item.planned_content {
                text.push_str("    planned_content:\n");
                for line in content.lines() {
                    text.push_str(&format!("      {line}\n"));
                }
            }
        }
    }
    text.push_str(&format!(
        "verification: {}\n",
        output.verification.status.as_str()
    ));
    text.push_str(&format!(
        "verification_detail: {}\n",
        output.verification.details
    ));
    text.push_str(&format!(
        "host_state: {}\n",
        output.verification.host_state.as_str()
    ));
    text.push_str(&format!(
        "managed_config: {}\n",
        output.verification.managed_config.as_str()
    ));
    text.push_str(&format!(
        "host_executable: {}\n",
        output.verification.host_executable.as_str()
    ));
    text.push_str(&format!(
        "host_gate: {}\n",
        output.verification.host_gate.as_str()
    ));
    text.push_str(&format!(
        "host_configuration: {}\n",
        output.verification.host_configuration.as_str()
    ));
    text.push_str(&format!(
        "mcp_handshake_diagnostic: {}\n",
        output.verification.mcp_handshake_allowed
    ));
    if let Some(diagnostic) = &output.verification.host_diagnostic {
        if !diagnostic.is_empty() {
            text.push_str(&format!("host_diagnostic: {diagnostic}\n"));
        }
    }
    if !output.action_required.is_empty() {
        text.push_str("action_required:\n");
        for action in &output.action_required {
            text.push_str(&format!("  {action}\n"));
        }
    }
    if !output.actions.is_empty() {
        text.push_str("actions:\n");
        for action in &output.actions {
            text.push_str(&format!(
                "  {}: {} ({})\n",
                action.target,
                action.state.as_str(),
                action.detail
            ));
        }
    }
    if !output.warnings.is_empty() {
        text.push_str("warnings:\n");
        for warning in &output.warnings {
            text.push_str(&format!("  {warning}\n"));
        }
    }
    Ok(text)
}

fn render_agent_json(output: &AgentOutput) -> Result<String, AgentCommandError> {
    let registry_schema_version = output.registry_schema.map(|schema| schema.current_version);
    let registry_latest_supported_schema_version = output
        .registry_schema
        .map(|schema| schema.latest_supported_version)
        .unwrap_or(REGISTRY_SCHEMA_VERSION);
    let registry_migration_planned = output
        .registry_schema
        .map(|schema| schema.migration_planned)
        .unwrap_or(false);
    let host = output.host_plan.as_ref().map(|plan| {
        json!({
            "host_kind": plan.host_kind.as_str(),
            "host_scope": plan.host_scope.as_str(),
            "server_name": plan.server_name,
            "target": host_target_text(&plan.target),
            "planned_change": planned_change_text(plan.change),
            "fingerprint": plan.fingerprint,
        })
    });
    let installations = output
        .installations
        .iter()
        .map(|installation| {
            json!({
                "installation_id": installation.installation_id,
                "integration_id": installation.integration_id,
                "host_kind": installation.host_kind,
                "host_scope": installation.host_scope,
                "server_name": installation.server_name,
                "config_target": installation.config_target,
                "managed_fingerprint": installation.managed_fingerprint,
                "last_verified_status": installation.last_verified_status,
            })
        })
        .collect::<Vec<_>>();
    let actions = output
        .actions
        .iter()
        .map(|action| {
            json!({
                "target": action.target,
                "action": action.state.as_str(),
                "detail": action.detail,
            })
        })
        .collect::<Vec<_>>();
    let guidance_items = output
        .guidance
        .iter()
        .map(|item| {
            json!({
                "target": item.target.as_str(),
                "integration_id": &item.integration_id,
                "project_id": &item.project_id,
                "path": path_text(&item.path),
                "state": item.state.as_str(),
                "fingerprint": &item.fingerprint,
                "detail": &item.detail,
                "planned_content": &item.planned_content,
            })
        })
        .collect::<Vec<_>>();
    let value = json!({
        "status": output.status.as_str(),
        "runtime": {
            "runtime_home": path_text(&output.runtime_home),
            "registry_schema_version": registry_schema_version,
            "registry_latest_supported_schema_version": registry_latest_supported_schema_version,
            "registry_migration_planned": registry_migration_planned,
        },
        "project": {
            "allowed_project_ids": output.allowed_projects,
        },
        "integration": {
            "integration_id": output.integration_id,
        },
        "allowed_projects": output.allowed_projects,
        "installations": installations,
        "guidance": {
            "status": guidance_summary_status(&output.guidance),
            "items": guidance_items,
        },
        "host": host,
        "verification": {
            "status": output.verification.status.as_str(),
            "details": output.verification.details,
            "host_state": output.verification.host_state.as_str(),
            "managed_config": output.verification.managed_config.as_str(),
            "host_executable": output.verification.host_executable.as_str(),
            "host_gate": output.verification.host_gate.as_str(),
            "host_configuration": output.verification.host_configuration.as_str(),
            "mcp_handshake_diagnostic": output.verification.mcp_handshake_allowed,
            "host_diagnostic": &output.verification.host_diagnostic,
            "instructions_present": output.verification.instructions_present,
            "tools": output.verification.tools,
        },
        "actions": actions,
        "effects": actions,
        "action_required": output.action_required,
        "warnings": output.warnings,
    });
    let mut text = serde_json::to_string_pretty(&value)
        .map_err(|error| AgentCommandError::runtime(format!("failed to render JSON: {error}")))?;
    text.push('\n');
    Ok(text)
}

fn guidance_summary_status(guidance: &[GuidanceStatus]) -> &'static str {
    let mut states = guidance
        .iter()
        .map(|status| status.state)
        .collect::<BTreeSet<_>>();
    if states.is_empty() {
        return "not_managed";
    }
    if states.remove(&GuidanceStateKind::Conflicted) {
        return "conflicted";
    }
    if states.remove(&GuidanceStateKind::Changed) {
        return "changed";
    }
    if states.len() == 1 {
        return states
            .iter()
            .next()
            .map(|state| state.as_str())
            .unwrap_or("not_managed");
    }
    "mixed"
}

fn setup_status_from_verification(verification: &McpVerification) -> AgentResultStatus {
    match verification.status {
        VerificationStatus::Complete => AgentResultStatus::Complete,
        VerificationStatus::ActionRequired | VerificationStatus::NotVerified => {
            AgentResultStatus::ActionRequired
        }
        VerificationStatus::Missing
        | VerificationStatus::Changed
        | VerificationStatus::Rejected
        | VerificationStatus::Unavailable
        | VerificationStatus::Unknown
        | VerificationStatus::Failed => AgentResultStatus::PartialFailure,
    }
}

fn should_run_diagnostic_mcp_handshake(verification: &Verification) -> bool {
    verification.host_state == HostVerificationState::ConfiguredReady
        && verification.mcp_handshake_allowed
}

fn mcp_verification_from_host(verification: Verification) -> McpVerification {
    McpVerification {
        status: verification.status,
        host_state: verification.host_state,
        managed_config: verification.managed_config,
        host_executable: verification.host_executable,
        host_gate: verification.host_gate,
        host_configuration: verification.host_configuration,
        mcp_handshake_allowed: verification.mcp_handshake_allowed,
        details: verification.details,
        host_diagnostic: verification.diagnostic,
        instructions_present: false,
        tools: Vec::new(),
    }
}

fn merge_mcp_verification_with_host(
    mut mcp: McpVerification,
    host: &Verification,
) -> McpVerification {
    mcp.host_state = host.host_state;
    mcp.managed_config = host.managed_config;
    mcp.host_executable = host.host_executable;
    mcp.host_gate = host.host_gate;
    mcp.host_configuration = host.host_configuration;
    mcp.mcp_handshake_allowed = host.mcp_handshake_allowed;
    mcp.host_diagnostic = host.diagnostic.clone();
    mcp
}

fn mcp_failure_from_host(host: &Verification, details: String) -> McpVerification {
    let mut verification = McpVerification::failed(details);
    verification.host_state = host.host_state;
    verification.managed_config = host.managed_config;
    verification.host_executable = host.host_executable;
    verification.host_gate = host.host_gate;
    verification.host_configuration = host.host_configuration;
    verification.mcp_handshake_allowed = host.mcp_handshake_allowed;
    verification.host_diagnostic = host.diagnostic.clone();
    verification
}

fn store_status_from_setup_status(status: AgentResultStatus) -> &'static str {
    match status {
        AgentResultStatus::Complete => VERIFIED_STATUS_COMPLETE,
        AgentResultStatus::ActionRequired => VERIFIED_STATUS_ACTION_REQUIRED,
        AgentResultStatus::PartialFailure => VERIFIED_STATUS_PARTIAL_FAILURE,
        AgentResultStatus::Failed => VERIFIED_STATUS_FAILED,
        AgentResultStatus::DryRun => VERIFIED_STATUS_NOT_VERIFIED,
    }
}

fn partial_install_output(
    parsed: &ParsedAgentOptions,
    runtime_home: PathBuf,
    integration_id: String,
    host_plan: HostPlan,
    allowed_projects: Vec<String>,
    actions: Vec<AgentAction>,
    message: String,
) -> AgentOutput {
    AgentOutput {
        status: AgentResultStatus::PartialFailure,
        runtime_home,
        registry_schema: None,
        integration_id,
        host_plan: Some(host_plan),
        allowed_projects,
        installations: Vec::new(),
        guidance: Vec::new(),
        verification: McpVerification::failed(message),
        actions,
        warnings: vec![
            "durable registry changes may remain; rerun install after fixing the error".to_owned(),
        ],
        action_required: Vec::new(),
        output: parsed.output,
    }
}

fn compensate_install_membership(
    runtime_home: &Path,
    integration_id: &str,
    project_id: &str,
    membership_before: bool,
) {
    if !membership_before {
        let _ = clear_agent_integration_default_project(runtime_home, integration_id);
        let _ = remove_integration_project(runtime_home, integration_id, project_id);
    }
}

fn remove_host_configuration(
    runtime_home: &Path,
    installation: &HostInstallationRecord,
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<(), AgentCommandError> {
    let host_kind = parse_host_kind(&installation.host_kind)?;
    let host_scope = parse_host_scope(&installation.host_scope)?;
    let target = host_target_from_record(installation)?;
    let request = HostRemoveRequest {
        host_kind,
        host_scope,
        server_name: installation.server_name.clone(),
        target,
        expected_fingerprint: installation.managed_fingerprint.clone(),
    };
    match host_kind {
        HostKind::Codex => {
            let mut adapter = CodexAdapter::new(CodexEnvironment {
                home: process.env_var("HOME").map(PathBuf::from),
                codex_home: process.env_var("CODEX_HOME").map(PathBuf::from),
                path: process.env_var(PATH_ENV),
            });
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
    let _ = current_dir;
    let _ = runtime_home;
    Ok(())
}

fn inspect_installation_host_state(
    runtime_home: &Path,
    installation: &HostInstallationRecord,
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<String, AgentCommandError> {
    let verification =
        verify_installation_host_state(runtime_home, installation, current_dir, process)?;
    if verification.details.is_empty() {
        Ok(verification.host_state.as_str().to_owned())
    } else {
        Ok(format!(
            "{}: {}",
            verification.host_state.as_str(),
            verification.details
        ))
    }
}

fn verify_installation_host_state(
    runtime_home: &Path,
    installation: &HostInstallationRecord,
    current_dir: &Path,
    process: &mut impl AgentProcess,
) -> Result<Verification, AgentCommandError> {
    let host_kind = parse_host_kind(&installation.host_kind)?;
    let host_scope = parse_host_scope(&installation.host_scope)?;
    let metadata = parse_metadata(&installation.metadata_json);
    let mcp_command = metadata
        .get("mcp_command")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MCP_COMMAND));
    let repo_root = metadata.get("repo_root").map(PathBuf::from);
    let parsed = ParsedAgentOptions::default();
    let plan = build_host_plan(
        HostPlanInputs {
            host_kind,
            host_scope,
            integration_id: &installation.integration_id,
            server_name: Some(&installation.server_name),
            repo_root: repo_root.as_deref(),
            mcp_command: &mcp_command,
            runtime_home: runtime_home_for_host_config(host_scope, runtime_home),
            expected_fingerprint: Some(&installation.managed_fingerprint),
            parsed: &parsed,
            current_dir,
        },
        process,
    );
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => return Ok(Verification::failed(error.to_string())),
    };
    Ok(verify_host_plan(host_kind, &plan, process)?)
}

fn selected_installations(
    runtime_home: &Path,
    integration_id: &str,
    installation_id: Option<&str>,
) -> Result<Vec<HostInstallationRecord>, AgentCommandError> {
    if let Some(installation_id) = installation_id {
        let record = host_installation_record(runtime_home, installation_id)?.ok_or_else(|| {
            AgentCommandError::runtime(format!("Host Installation not found: {installation_id}"))
        })?;
        if record.integration_id != integration_id {
            return Err(AgentCommandError::runtime(
                "installation_id belongs to another integration",
            ));
        }
        Ok(vec![record])
    } else {
        let records = list_host_installations_for_integration(runtime_home, integration_id)?;
        if records.is_empty() {
            Err(AgentCommandError::runtime(
                "no Host Installation records are registered for this integration",
            ))
        } else {
            Ok(records)
        }
    }
}

fn command_for_existing_installation(
    runtime_home: &Path,
    integration_id: &str,
) -> Result<Option<PathBuf>, AgentCommandError> {
    let installations = list_host_installations_for_integration(runtime_home, integration_id)?;
    Ok(installations
        .iter()
        .filter_map(|installation| {
            parse_metadata(&installation.metadata_json)
                .get("mcp_command")
                .map(PathBuf::from)
        })
        .next())
}

fn required_integration(
    runtime_home: &Path,
    integration_id: &str,
) -> Result<AgentIntegrationRecord, AgentCommandError> {
    agent_integration_record(runtime_home, integration_id)?.ok_or_else(|| {
        AgentCommandError::runtime(format!(
            "Agent Integration Profile not found: {integration_id}"
        ))
    })
}

fn find_installation_for_target_hint(
    runtime_home: &Path,
    integration_id: &str,
    host_kind: HostKind,
    host_scope: HostScope,
    server_name: Option<&str>,
) -> Result<Option<HostInstallationRecord>, AgentCommandError> {
    let records =
        list_host_installations_for_integration(runtime_home, integration_id).unwrap_or_default();
    Ok(records.into_iter().find(|record| {
        record.host_kind == host_kind.as_str()
            && record.host_scope == host_scope.as_str()
            && server_name
                .map(|name| record.server_name == name)
                .unwrap_or(true)
    }))
}

fn is_project_member(
    runtime_home: &Path,
    integration_id: &str,
    project_id: &str,
) -> Result<bool, AgentCommandError> {
    Ok(list_integration_projects(runtime_home, integration_id)
        .unwrap_or_default()
        .iter()
        .any(|record| record.project_id == project_id))
}

fn project_repo_matches(project: &ProjectRecord, repo_root: &Path) -> bool {
    project.repo_root == repo_root
        || fs::canonicalize(&project.repo_root)
            .map(|path| path == repo_root)
            .unwrap_or(false)
}

fn host_required_actions(
    verification: &crate::host_integration::verification::Verification,
) -> Vec<String> {
    if verification.status == VerificationStatus::ActionRequired {
        vec![verification.details.clone()]
    } else {
        Vec::new()
    }
}

fn planned_change_action(change: PlannedChange) -> ActionState {
    match change {
        PlannedChange::Create => ActionState::Planned,
        PlannedChange::Update => ActionState::Updated,
        PlannedChange::Remove => ActionState::Removed,
        PlannedChange::Noop => ActionState::Reused,
        PlannedChange::ExternalCommand => ActionState::Planned,
    }
}

fn planned_change_text(change: PlannedChange) -> &'static str {
    match change {
        PlannedChange::Create => "created",
        PlannedChange::Update => "updated",
        PlannedChange::Remove => "removed",
        PlannedChange::Noop => "reused",
        PlannedChange::ExternalCommand => "planned",
    }
}

fn host_target_text(target: &HostTarget) -> String {
    match target {
        HostTarget::File(path) | HostTarget::Export(path) => path_text(path),
        HostTarget::ExternalCli { program, cwd } => cwd
            .as_ref()
            .map(|cwd| format!("{program} in {}", cwd.display()))
            .unwrap_or_else(|| program.clone()),
    }
}

fn host_target_from_record(
    record: &HostInstallationRecord,
) -> Result<HostTarget, AgentCommandError> {
    match record.host_kind.as_str() {
        HOST_KIND_CODEX => Ok(HostTarget::File(PathBuf::from(&record.config_target))),
        HOST_KIND_CLAUDE_CODE if record.host_scope == HOST_SCOPE_PROJECT => {
            Ok(HostTarget::File(PathBuf::from(&record.config_target)))
        }
        HOST_KIND_CLAUDE_CODE => {
            let cwd = parse_metadata(&record.metadata_json)
                .get("repo_root")
                .map(PathBuf::from);
            Ok(HostTarget::ExternalCli {
                program: "claude".to_owned(),
                cwd,
            })
        }
        HOST_KIND_GENERIC => Ok(HostTarget::Export(PathBuf::from(&record.config_target))),
        _ => Err(AgentCommandError::runtime("unknown host kind in inventory")),
    }
}

fn runtime_home_for_host_config(scope: HostScope, runtime_home: &Path) -> Option<&Path> {
    if scope == HostScope::Project {
        None
    } else {
        Some(runtime_home)
    }
}

fn deterministic_integration_id(host_kind: HostKind, scope: HostScope, project_id: &str) -> String {
    stable_identifier(
        "agent",
        &format!("{}:{}:{project_id}", host_kind.as_str(), scope.as_str()),
    )
}

fn deterministic_installation_id(integration_id: &str, plan: &HostPlan) -> String {
    stable_identifier(
        "install",
        &format!(
            "{}:{}:{}:{}",
            integration_id,
            plan.host_kind.as_str(),
            plan.host_scope.as_str(),
            plan.server_name
        ),
    )
}

fn stable_identifier(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut suffix = String::new();
    for byte in digest.iter().take(8) {
        suffix.push_str(&format!("{byte:02x}"));
    }
    format!("{prefix}_{suffix}")
}

fn installation_metadata_json(
    runtime_home: &Path,
    mcp_command: &Path,
    repo_root: Option<&Path>,
) -> Result<String, AgentCommandError> {
    let mut value = json!({
        "created_by": "harness_cli_agent",
        "runtime_home": path_text(runtime_home),
        "mcp_command": path_text(mcp_command),
    });
    if let Some(repo_root) = repo_root {
        value["repo_root"] = Value::String(path_text(repo_root));
    }
    serde_json::to_string(&value)
        .map_err(|error| AgentCommandError::runtime(format!("failed to encode metadata: {error}")))
}

fn parse_metadata(text: &str) -> BTreeMap<String, String> {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .map(|object| {
            object
                .into_iter()
                .filter_map(|(key, value)| value.as_str().map(|value| (key, value.to_owned())))
                .collect()
        })
        .unwrap_or_default()
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
        .map(|code| format!("with status {code}"))
        .unwrap_or_else(|| "without an exit status".to_owned())
}

fn compact_stream(text: &str) -> String {
    text.trim().replace('\n', " | ")
}

struct EnvOnlyProcess;

impl AgentProcess for EnvOnlyProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe().map_err(|error| error.to_string())
    }

    fn run_preflight(
        &mut self,
        _command: &Path,
        _runtime_home: &Path,
        _integration_id: &str,
        _project_id: Option<&str>,
    ) -> Result<AgentProcessOutput, String> {
        Err("preflight is not available in this command path".to_owned())
    }

    fn verify_mcp_stdio(
        &mut self,
        _command: &Path,
        _runtime_home: &Path,
        _integration_id: &str,
    ) -> Result<McpVerification, String> {
        Err("MCP verification is not available in this command path".to_owned())
    }
}
