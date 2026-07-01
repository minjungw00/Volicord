use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;

use super::{
    config_edit::{read_json_object, write_json_object_if_fresh},
    current_entry_fingerprint_from_json, is_volicord_managed_entry, managed_entry_from_json,
    managed_fingerprint, validated_server_name, ConnectionIntent, HostAdapter, HostConfigError,
    HostConflict, HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, ManagedServerEntry,
    PlannedChange, UserAction, UserActionKind, DEFAULT_MCP_COMMAND,
};
use crate::host_integration::{
    verification::{
        HostConfigurationStatus, HostExecutableStatus, HostGateStatus, ManagedConfigStatus,
        Verification,
    },
    HostCapabilities,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaudeMcpState {
    Connected,
    PendingApproval,
    Rejected,
    Missing,
    CommandFailed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaudeMcpInspection {
    state: ClaudeMcpState,
    scope: Option<HostScope>,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: BTreeMap<String, String>,
    diagnostic: Option<String>,
}

pub trait CommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String>;
}

#[derive(Debug, Default, Clone)]
pub struct ProductionCommandRunner;

impl CommandRunner for ProductionCommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
        let mut command = Command::new(&invocation.program);
        command.args(&invocation.args);
        if let Some(cwd) = &invocation.cwd {
            command.current_dir(cwd);
        }
        let output = command.output().map_err(|error| {
            format!(
                "failed to run {} {}: {error}",
                invocation.program,
                invocation.args.join(" ")
            )
        })?;
        Ok(CommandOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ClaudeCodeAdapter<R> {
    runner: R,
    claude_command: String,
}

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        http_mcp: false,
        session_start_hook: true,
        pre_tool_hook: true,
        post_tool_hook: true,
        user_prompt_submit_hook: true,
        stop_hook: true,
        rule_file_support: true,
        project_local_configuration: true,
    }
}

pub fn project_settings_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".claude").join("settings.json")
}

pub fn project_rule_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".claude").join("rules").join("volicord.md")
}

pub fn project_rule_block(policy_path: &str, command_lines: &[(String, String)]) -> String {
    let mut block = format!(
        "# Volicord\n\nUse the repository-local `{policy_path}` guard policy. Do not record user-owned judgments through the Agent Connection.\n\nConfigured local guard commands:\n"
    );
    for (phase, command) in command_lines {
        block.push_str(&format!("- `{phase}`: `{command}`\n"));
    }
    block
}

impl<R: CommandRunner> ClaudeCodeAdapter<R> {
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            claude_command: "claude".to_owned(),
        }
    }

    pub fn with_command(runner: R, claude_command: impl Into<String>) -> Self {
        Self {
            runner,
            claude_command: claude_command.into(),
        }
    }

    pub fn plan(&mut self, request: HostPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        if request.host_kind != HostKind::ClaudeCode {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Claude Code adapter cannot plan a non-Claude Code host request",
            )));
        }
        let scope = claude_scope_for_intent(request.connection_intent);
        let (mcp_command, runtime_home) =
            entry_inputs_for_scope(scope, request.installation_profile);
        validate_mcp_command(scope, mcp_command)?;
        let server_name = validated_server_name(request.connection_id, None)?;
        if server_name == "workspace" {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidServerName,
                "Claude Code reserves the MCP server name `workspace`",
            )));
        }
        let entry = ManagedServerEntry::new_project_bound(
            request.connection_id,
            request.project.map(|project| project.project_id),
            mcp_command,
            runtime_home,
        );
        let fingerprint = managed_fingerprint(HostKind::ClaudeCode, scope, &server_name, &entry);
        match scope {
            HostScope::Project => self.plan_project_file(request, server_name, entry, fingerprint),
            HostScope::Local | HostScope::User => {
                self.plan_external_cli(request, server_name, entry, fingerprint)
            }
            _ => unreachable!("Claude Code intent mapping validated above"),
        }
    }

    fn plan_project_file(
        &self,
        request: HostPlanRequest<'_>,
        server_name: String,
        entry: ManagedServerEntry,
        fingerprint: String,
    ) -> Result<HostPlan, HostConfigError> {
        let project = request.project.ok_or_else(|| {
            HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Claude Code shared connection intent requires a Product Repository root",
            ))
        })?;
        let target = project.repo_root.join(".mcp.json");
        let (snapshot, object) = read_json_object(&target)?;
        if object
            .get("mcpServers")
            .is_some_and(|value| !value.is_object())
        {
            return Err(HostConfigError::Malformed(
                "Claude Code .mcp.json mcpServers must be an object".to_owned(),
            ));
        }
        let existing = object
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&server_name));
        let mut conflicts = Vec::new();
        let change = match existing {
            None => PlannedChange::Create,
            Some(existing) => classify_existing_json_entry(
                HostScope::Project,
                &server_name,
                existing,
                &fingerprint,
                request.expected_fingerprint,
                &mut conflicts,
                "Claude Code project MCP server name",
            ),
        };
        Ok(HostPlan {
            host_kind: HostKind::ClaudeCode,
            connection_intent: request.connection_intent,
            host_scope: HostScope::Project,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::File(target),
            entry,
            change,
            fingerprint,
            conflicts,
            user_actions: vec![UserAction::new(
                UserActionKind::ProjectApprovalRequired,
                "Claude Code requires user approval before project-scoped .mcp.json servers load",
            )],
            file_snapshot: Some(snapshot),
        })
    }

    fn plan_external_cli(
        &mut self,
        request: HostPlanRequest<'_>,
        server_name: String,
        entry: ManagedServerEntry,
        fingerprint: String,
    ) -> Result<HostPlan, HostConfigError> {
        let scope = claude_scope_for_intent(request.connection_intent);
        let cwd = match scope {
            HostScope::Local => Some(
                request
                    .project
                    .ok_or_else(|| {
                        HostConfigError::Conflict(HostConflict::new(
                            HostConflictKind::InvalidScope,
                            "Claude Code personal connection intent requires a Product Repository root",
                        ))
                    })?
                    .repo_root
                    .to_path_buf(),
            ),
            HostScope::User => None,
            _ => unreachable!("external CLI only handles local and user scopes"),
        };
        let status = self.runner.run(&build_get_command(
            &self.claude_command,
            &server_name,
            cwd.clone(),
        ));
        let mut conflicts = Vec::new();
        let change = match status {
            Ok(output) if parse_claude_mcp_get_output(&output).state == ClaudeMcpState::Missing => {
                PlannedChange::ExternalCommand
            }
            Ok(output) if output.success => {
                let inspection = parse_claude_mcp_get_output(&output);
                if inspection.state == ClaudeMcpState::Connected {
                    let current =
                        fingerprint_from_claude_inspection(scope, &server_name, &inspection);
                    if current.as_deref() == Some(fingerprint.as_str()) {
                        PlannedChange::Noop
                    } else if current.as_deref() == request.expected_fingerprint {
                        PlannedChange::ExternalCommand
                    } else if inspection_is_volicord_managed(&inspection) {
                        conflicts.push(HostConflict::new(
                            HostConflictKind::FingerprintMismatch,
                            format!(
                                "Claude Code MCP server name is already configured by a different Volicord-managed entry: {server_name}"
                            ),
                        ));
                        PlannedChange::Noop
                    } else {
                        conflicts.push(HostConflict::new(
                            HostConflictKind::UnmanagedNameCollision,
                            format!(
                                "Claude Code MCP server name is already configured by an unrelated entry: {server_name}"
                            ),
                        ));
                        PlannedChange::Noop
                    }
                } else {
                    conflicts.push(HostConflict::new(
                        HostConflictKind::UnmanagedNameCollision,
                        format!(
                            "Claude Code MCP server name could not be safely interpreted for update: {server_name}"
                        ),
                    ));
                    PlannedChange::Noop
                }
            }
            Ok(_) | Err(_) => PlannedChange::ExternalCommand,
        };
        Ok(HostPlan {
            host_kind: HostKind::ClaudeCode,
            connection_intent: request.connection_intent,
            host_scope: scope,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::ExternalCli {
                program: self.claude_command.clone(),
                cwd,
            },
            entry,
            change,
            fingerprint,
            conflicts,
            user_actions: Vec::new(),
            file_snapshot: None,
        })
    }
}

impl<R: CommandRunner> HostAdapter for ClaudeCodeAdapter<R> {
    fn capabilities(&self) -> HostCapabilities {
        capabilities()
    }

    fn detect(&self) -> Result<HostDetection, HostConfigError> {
        Ok(HostDetection {
            host_kind: HostKind::ClaudeCode,
            available: true,
            details: format!("Claude Code command target: {}", self.claude_command),
        })
    }

    fn apply(&mut self, plan: &HostPlan) -> Result<HostEffect, HostConfigError> {
        if let Some(conflict) = plan.conflicts.first() {
            return Err(HostConfigError::Conflict(conflict.clone()));
        }
        if plan.change == PlannedChange::Noop {
            return Ok(effect_from_plan(plan));
        }
        match &plan.target {
            HostTarget::File(target) if plan.host_scope == HostScope::Project => {
                let snapshot = plan.file_snapshot.as_ref().ok_or_else(|| {
                    HostConfigError::StalePlan(
                        "Claude Code project plan is missing its file snapshot".to_owned(),
                    )
                })?;
                let (_, mut object) = read_json_object(target)?;
                upsert_project_entry(&mut object, &plan.server_name, &plan.entry)?;
                write_json_object_if_fresh(target, &object, snapshot)?;
                Ok(effect_from_plan(plan))
            }
            HostTarget::ExternalCli { cwd, .. } => {
                let invocation = build_add_command(
                    &self.claude_command,
                    plan.host_scope,
                    &plan.server_name,
                    &plan.entry,
                    cwd.clone(),
                );
                let output = self
                    .runner
                    .run(&invocation)
                    .map_err(HostConfigError::ExternalCommand)?;
                if output.success {
                    Ok(effect_from_plan(plan))
                } else {
                    Err(HostConfigError::ExternalCommand(format!(
                        "claude mcp add failed with status {}; stderr: {}",
                        output
                            .status_code
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "unknown".to_owned()),
                        output.stderr.trim()
                    )))
                }
            }
            _ => Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Claude Code plan target is not valid for its scope",
            ))),
        }
    }

    fn verify(&mut self, plan: &HostPlan) -> Result<Verification, HostConfigError> {
        if let Some(conflict) = plan.conflicts.first() {
            return Ok(Verification::changed(conflict.message.clone())
                .merge_user_actions(&plan.user_actions));
        }
        match &plan.target {
            HostTarget::File(target) if plan.host_scope == HostScope::Project => {
                let managed = verify_claude_project_entry(plan)?;
                if managed != ManagedConfigStatus::Match {
                    return Ok(verification_from_managed_status(
                        managed,
                        format!(
                            "Claude Code managed project MCP entry is {} for {}",
                            managed.as_str(),
                            plan.server_name
                        ),
                    )
                    .merge_user_actions(&plan.user_actions));
                }
                let cwd = target.parent().map(Path::to_path_buf);
                let output = self.runner.run(&build_get_command(
                    &self.claude_command,
                    &plan.server_name,
                    cwd,
                ));
                Ok(match output {
                    Ok(output) => verification_from_claude_output(plan, &output)
                        .merge_user_actions(&plan.user_actions),
                    Err(error) => Verification::unavailable(format!(
                        "Claude Code executable is unavailable for `{} mcp get {}`: {error}",
                        self.claude_command, plan.server_name
                    ))
                    .with_managed_config(ManagedConfigStatus::Match)
                    .with_host_configuration(HostConfigurationStatus::Discovered)
                    .merge_user_actions(&plan.user_actions),
                })
            }
            HostTarget::ExternalCli { cwd, .. } => {
                let output = self.runner.run(&build_get_command(
                    &self.claude_command,
                    &plan.server_name,
                    cwd.clone(),
                ));
                Ok(match output {
                    Ok(output) => verification_from_claude_output(plan, &output)
                        .merge_user_actions(&plan.user_actions),
                    Err(error) => Verification::unavailable(format!(
                        "Claude Code executable is unavailable for `{} mcp get {}`: {error}",
                        self.claude_command, plan.server_name
                    ))
                    .merge_user_actions(&plan.user_actions),
                })
            }
            _ => Ok(
                Verification::failed("Claude Code verification target is invalid")
                    .merge_user_actions(&plan.user_actions),
            ),
        }
    }

    fn remove(&mut self, request: HostRemoveRequest) -> Result<HostEffect, HostConfigError> {
        if request.host_kind != HostKind::ClaudeCode {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Claude Code adapter cannot remove a non-Claude Code host target",
            )));
        }
        match &request.target {
            HostTarget::File(target) if request.host_scope == HostScope::Project => {
                let (snapshot, mut object) = read_json_object(target)?;
                let existing = object
                    .get("mcpServers")
                    .and_then(Value::as_object)
                    .and_then(|servers| servers.get(&request.server_name));
                let Some(existing) = existing else {
                    return Ok(remove_effect(request, PlannedChange::Noop));
                };
                let current = current_entry_fingerprint_from_json(
                    HostKind::ClaudeCode,
                    HostScope::Project,
                    &request.server_name,
                    existing,
                );
                if current.as_deref() != Some(request.expected_fingerprint.as_str()) {
                    return Err(HostConfigError::Conflict(HostConflict::new(
                        HostConflictKind::FingerprintMismatch,
                        format!(
                            "Claude Code project MCP entry changed since Volicord last managed it: {}",
                            request.server_name
                        ),
                    )));
                }
                remove_project_entry(&mut object, &request.server_name)?;
                write_json_object_if_fresh(target, &object, &snapshot)?;
                Ok(remove_effect(request, PlannedChange::Remove))
            }
            HostTarget::ExternalCli { cwd, .. } => {
                let output = self
                    .runner
                    .run(&build_get_command(
                        &self.claude_command,
                        &request.server_name,
                        cwd.clone(),
                    ))
                    .map_err(HostConfigError::ExternalCommand)?;
                let inspection = parse_claude_mcp_get_output(&output);
                let current = fingerprint_from_claude_inspection(
                    request.host_scope,
                    &request.server_name,
                    &inspection,
                );
                if current.as_deref() != Some(request.expected_fingerprint.as_str()) {
                    return Err(HostConfigError::Conflict(HostConflict::new(
                        HostConflictKind::FingerprintMismatch,
                        format!(
                            "Claude Code MCP entry changed since Volicord last managed it: {}",
                            request.server_name
                        ),
                    )));
                }
                let remove = build_remove_command(
                    &self.claude_command,
                    request.host_scope,
                    &request.server_name,
                    cwd.clone(),
                );
                let output = self
                    .runner
                    .run(&remove)
                    .map_err(HostConfigError::ExternalCommand)?;
                if output.success {
                    Ok(remove_effect(request, PlannedChange::Remove))
                } else {
                    Err(HostConfigError::ExternalCommand(format!(
                        "claude mcp remove failed with status {}; stderr: {}",
                        output
                            .status_code
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "unknown".to_owned()),
                        output.stderr.trim()
                    )))
                }
            }
            _ => Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Claude Code removal target is not valid for its scope",
            ))),
        }
    }
}

fn claude_scope_for_intent(intent: ConnectionIntent) -> HostScope {
    match intent {
        ConnectionIntent::Personal => HostScope::Local,
        ConnectionIntent::Shared => HostScope::Project,
        ConnectionIntent::Global => HostScope::User,
    }
}

fn entry_inputs_for_scope<'a>(
    scope: HostScope,
    profile: InstallationProfile<'a>,
) -> (&'a Path, Option<&'a Path>) {
    if scope == HostScope::Project {
        (Path::new(DEFAULT_MCP_COMMAND), None)
    } else {
        (profile.volicord_mcp_command, Some(profile.runtime_home))
    }
}

fn classify_existing_json_entry(
    scope: HostScope,
    server_name: &str,
    value: &Value,
    desired_fingerprint: &str,
    expected_fingerprint: Option<&str>,
    conflicts: &mut Vec<HostConflict>,
    label: &str,
) -> PlannedChange {
    let Some(entry) = managed_entry_from_json(value).filter(is_volicord_managed_entry) else {
        conflicts.push(HostConflict::new(
            HostConflictKind::UnmanagedNameCollision,
            format!("{label} is already configured by an unmanaged entry: {server_name}"),
        ));
        return PlannedChange::Noop;
    };
    let current = managed_fingerprint(HostKind::ClaudeCode, scope, server_name, &entry);
    if current == desired_fingerprint {
        PlannedChange::Noop
    } else if expected_fingerprint == Some(current.as_str()) {
        PlannedChange::Update
    } else {
        conflicts.push(HostConflict::new(
            HostConflictKind::FingerprintMismatch,
            format!("{label} is already configured by a different Volicord-managed entry: {server_name}"),
        ));
        PlannedChange::Noop
    }
}

fn inspection_is_volicord_managed(inspection: &ClaudeMcpInspection) -> bool {
    let Some(command) = &inspection.command else {
        return false;
    };
    let Some(args) = &inspection.args else {
        return false;
    };
    is_volicord_managed_entry(&ManagedServerEntry {
        command: command.clone(),
        args: args.clone(),
        env: inspection.env.clone(),
    })
}

pub fn build_add_command(
    program: &str,
    scope: HostScope,
    server_name: &str,
    entry: &ManagedServerEntry,
    cwd: Option<PathBuf>,
) -> CommandInvocation {
    let mut args = vec!["mcp".to_owned(), "add".to_owned()];
    for (key, value) in &entry.env {
        args.push("--env".to_owned());
        args.push(format!("{key}={value}"));
    }
    args.extend([
        "--transport".to_owned(),
        "stdio".to_owned(),
        "--scope".to_owned(),
        scope.as_str().to_owned(),
        server_name.to_owned(),
        "--".to_owned(),
        entry.command.clone(),
    ]);
    args.extend(entry.args.clone());
    CommandInvocation {
        program: program.to_owned(),
        args,
        cwd,
    }
}

pub fn build_get_command(
    program: &str,
    server_name: &str,
    cwd: Option<PathBuf>,
) -> CommandInvocation {
    CommandInvocation {
        program: program.to_owned(),
        args: vec!["mcp".to_owned(), "get".to_owned(), server_name.to_owned()],
        cwd,
    }
}

pub fn build_remove_command(
    program: &str,
    scope: HostScope,
    server_name: &str,
    cwd: Option<PathBuf>,
) -> CommandInvocation {
    CommandInvocation {
        program: program.to_owned(),
        args: vec![
            "mcp".to_owned(),
            "remove".to_owned(),
            "--scope".to_owned(),
            scope.as_str().to_owned(),
            server_name.to_owned(),
        ],
        cwd,
    }
}

fn validate_mcp_command(scope: HostScope, command: &Path) -> Result<(), HostConfigError> {
    if scope == HostScope::Project {
        if command == Path::new(DEFAULT_MCP_COMMAND) {
            return Ok(());
        }
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Claude Code project-scoped configuration must use volicord from PATH",
        )));
    }
    if command.is_absolute() {
        Ok(())
    } else {
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Claude Code local and user scopes require an absolute volicord command path",
        )))
    }
}

fn upsert_project_entry(
    object: &mut serde_json::Map<String, Value>,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> Result<(), HostConfigError> {
    let servers = object
        .entry("mcpServers".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            HostConfigError::Malformed(
                "Claude Code .mcp.json mcpServers must be an object".to_owned(),
            )
        })?;
    servers.insert(server_name.to_owned(), entry.to_json_value());
    Ok(())
}

fn remove_project_entry(
    object: &mut serde_json::Map<String, Value>,
    server_name: &str,
) -> Result<(), HostConfigError> {
    let Some(servers) = object.get_mut("mcpServers").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    servers.remove(server_name);
    Ok(())
}

fn verify_claude_project_entry(plan: &HostPlan) -> Result<ManagedConfigStatus, HostConfigError> {
    let HostTarget::File(target) = &plan.target else {
        return Ok(ManagedConfigStatus::Unknown);
    };
    let (_, object) = match read_json_object(target) {
        Ok(result) => result,
        Err(HostConfigError::Malformed(_)) => return Ok(ManagedConfigStatus::Malformed),
        Err(error) => return Err(error),
    };
    let Some(existing) = object
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(&plan.server_name))
    else {
        return Ok(ManagedConfigStatus::Missing);
    };
    let current = current_entry_fingerprint_from_json(
        HostKind::ClaudeCode,
        HostScope::Project,
        &plan.server_name,
        existing,
    );
    match current {
        Some(fingerprint) if fingerprint == plan.fingerprint => Ok(ManagedConfigStatus::Match),
        Some(_) => Ok(ManagedConfigStatus::Changed),
        None => Ok(ManagedConfigStatus::Malformed),
    }
}

fn verification_from_claude_output(plan: &HostPlan, output: &CommandOutput) -> Verification {
    let inspection = parse_claude_mcp_get_output(output);
    match inspection.state {
        ClaudeMcpState::Connected => {
            let Some(current) =
                fingerprint_from_claude_inspection(plan.host_scope, &plan.server_name, &inspection)
            else {
                return Verification::unknown(format!(
                    "Claude Code command `claude mcp get {}` returned connected output, but command, args, env, or scope could not be parsed reliably",
                    plan.server_name
                ))
                .with_managed_config(ManagedConfigStatus::Match)
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_configuration(HostConfigurationStatus::Discovered)
                .with_diagnostic(inspection.diagnostic.unwrap_or_default());
            };
            if current == plan.fingerprint {
                Verification::configured_ready(
                    "Claude Code reports the managed MCP server is connected and matches Volicord configuration",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_gate(HostGateStatus::Ready)
                .with_mcp_handshake_allowed(true)
            } else {
                Verification::changed(
                    "Claude Code reports an MCP server with that name, but command, args, env, or scope differ from Volicord-managed configuration",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_configuration(HostConfigurationStatus::Changed)
            }
        }
        ClaudeMcpState::PendingApproval => Verification::action_required(
            "Claude Code reports the MCP server is pending project approval",
        )
        .with_host_executable(HostExecutableStatus::Available)
        .with_host_gate(HostGateStatus::ActionRequired)
        .with_mcp_handshake_allowed(true)
        .with_user_actions(vec![UserAction::new(
            UserActionKind::ProjectApprovalRequired,
            "Claude Code requires user approval before the MCP server is available",
        )]),
        ClaudeMcpState::Rejected => {
            Verification::rejected("Claude Code reports the MCP server was rejected")
        }
        ClaudeMcpState::Missing => Verification::missing(
            "Claude Code did not report a configured MCP server with that name",
        )
        .with_host_executable(HostExecutableStatus::Available),
        ClaudeMcpState::CommandFailed => Verification::failed(format!(
            "Claude Code command `claude mcp get {}` failed with status {}; host output was not echoed",
            plan.server_name,
            output
                .status_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        ))
        .with_host_executable(HostExecutableStatus::Available),
        ClaudeMcpState::Unknown => Verification::unknown(format!(
            "Claude Code command `claude mcp get {}` returned unsupported output; cannot interpret host state",
            plan.server_name
        ))
        .with_host_executable(HostExecutableStatus::Available)
        .with_diagnostic(inspection.diagnostic.unwrap_or_default()),
    }
}

fn verification_from_managed_status(status: ManagedConfigStatus, details: String) -> Verification {
    match status {
        ManagedConfigStatus::Missing => Verification::missing(details),
        ManagedConfigStatus::Changed => Verification::changed(details),
        ManagedConfigStatus::Malformed => Verification::failed(details)
            .with_managed_config(ManagedConfigStatus::Malformed)
            .with_host_configuration(HostConfigurationStatus::Malformed),
        ManagedConfigStatus::Match => Verification::configured_ready(details),
        ManagedConfigStatus::NotApplicable | ManagedConfigStatus::Unknown => {
            Verification::unknown(details)
        }
    }
}

fn fingerprint_from_claude_inspection(
    scope: HostScope,
    server_name: &str,
    inspection: &ClaudeMcpInspection,
) -> Option<String> {
    if inspection.scope.is_some_and(|actual| actual != scope) {
        return Some(managed_fingerprint(
            HostKind::ClaudeCode,
            inspection.scope.unwrap(),
            server_name,
            &ManagedServerEntry {
                command: inspection.command.clone()?,
                args: inspection.args.clone()?,
                env: inspection.env.clone(),
            },
        ));
    }
    Some(managed_fingerprint(
        HostKind::ClaudeCode,
        scope,
        server_name,
        &ManagedServerEntry {
            command: inspection.command.clone()?,
            args: inspection.args.clone()?,
            env: inspection.env.clone(),
        },
    ))
}

fn parse_claude_mcp_get_output(output: &CommandOutput) -> ClaudeMcpInspection {
    let combined = format!("{}\n{}", output.stdout, output.stderr);
    let mut state = None;
    let mut scope = None;
    let mut command = None;
    let mut args = None;
    let mut env = BTreeMap::new();
    let mut in_env = false;

    for line in combined.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_pending_marker(trimmed) {
            state = Some(ClaudeMcpState::PendingApproval);
        } else if is_rejected_marker(trimmed) {
            state = Some(ClaudeMcpState::Rejected);
        } else if is_missing_marker(trimmed) {
            state = Some(ClaudeMcpState::Missing);
        } else if is_connected_marker(trimmed) && state.is_none() {
            state = Some(ClaudeMcpState::Connected);
        }

        if let Some(value) = field_value(trimmed, "scope") {
            scope = parse_scope(value);
            in_env = false;
        } else if let Some(value) = field_value(trimmed, "command") {
            command = Some(value.to_owned());
            in_env = false;
        } else if let Some(value) = field_value(trimmed, "args") {
            args = parse_args(value);
            in_env = false;
        } else if let Some(value) = field_value(trimmed, "environment") {
            in_env = true;
            parse_env_assignment(value, &mut env);
        } else if let Some(value) = field_value(trimmed, "env") {
            in_env = true;
            parse_env_assignment(value, &mut env);
        } else if in_env {
            parse_env_assignment(trimmed, &mut env);
        }
    }

    let state = state.unwrap_or({
        if output.success {
            ClaudeMcpState::Unknown
        } else {
            ClaudeMcpState::CommandFailed
        }
    });
    ClaudeMcpInspection {
        state,
        scope,
        command,
        args,
        env,
        diagnostic: Some(host_output_summary(output)),
    }
}

fn host_output_summary(output: &CommandOutput) -> String {
    format!(
        "claude mcp get output summary: stdout_lines={}, stderr_lines={}, stderr_present={}",
        output.stdout.lines().count(),
        output.stderr.lines().count(),
        !output.stderr.trim().is_empty()
    )
}

fn field_value<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    let (actual, value) = line.split_once(':')?;
    if actual.trim().eq_ignore_ascii_case(label) {
        Some(value.trim())
    } else {
        None
    }
}

fn parse_scope(value: &str) -> Option<HostScope> {
    match value.trim().to_ascii_lowercase().as_str() {
        "local" => Some(HostScope::Local),
        "project" => Some(HostScope::Project),
        "user" => Some(HostScope::User),
        _ => None,
    }
}

fn parse_args(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Some(Vec::new());
    }
    if value.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(value).ok();
    }
    if value.contains('"') || value.contains('\'') {
        return None;
    }
    Some(value.split_whitespace().map(str::to_owned).collect())
}

fn parse_env_assignment(value: &str, env: &mut BTreeMap<String, String>) {
    let value = value.trim().trim_start_matches('-').trim();
    let Some((key, value)) = value.split_once('=') else {
        return;
    };
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return;
    }
    env.insert(key.to_owned(), value.trim().to_owned());
}

fn is_pending_marker(line: &str) -> bool {
    line == "⏸ Pending approval"
        || line == "Pending approval"
        || line == "Status: ⏸ Pending approval"
        || line.eq_ignore_ascii_case("Status: Pending approval")
}

fn is_rejected_marker(line: &str) -> bool {
    line == "✗ Rejected"
        || line == "Rejected"
        || line == "Status: ✗ Rejected"
        || line.eq_ignore_ascii_case("Status: Rejected")
}

fn is_missing_marker(line: &str) -> bool {
    line == "Server not found"
        || line == "No MCP server found"
        || line == "MCP server not found"
        || line.eq_ignore_ascii_case("Error: Server not found")
}

fn is_connected_marker(line: &str) -> bool {
    line == "✓ Connected"
        || line == "Connected"
        || line == "Status: ✓ Connected"
        || line.eq_ignore_ascii_case("Status: Connected")
}

fn effect_from_plan(plan: &HostPlan) -> HostEffect {
    HostEffect {
        host_kind: plan.host_kind,
        connection_intent: plan.connection_intent,
        host_scope: plan.host_scope,
        mode: plan.mode.clone(),
        server_name: plan.server_name.clone(),
        target: plan.target.clone(),
        change: plan.change,
        fingerprint: plan.fingerprint.clone(),
        user_actions: plan.user_actions.clone(),
    }
}

fn remove_effect(request: HostRemoveRequest, change: PlannedChange) -> HostEffect {
    HostEffect {
        host_kind: request.host_kind,
        connection_intent: request.connection_intent,
        host_scope: request.host_scope,
        mode: request.mode,
        server_name: request.server_name,
        target: request.target,
        change,
        fingerprint: request.expected_fingerprint,
        user_actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::host_integration::ProjectContext;

    use super::*;

    #[test]
    fn local_project_and_user_command_construction() {
        let entry = ManagedServerEntry::new(
            "int_alpha",
            Path::new("/bin/volicord"),
            Some(Path::new("/runtime")),
        );
        let local = build_add_command(
            "claude",
            HostScope::Local,
            "volicord",
            &entry,
            Some(PathBuf::from("/repo")),
        );
        let project = build_add_command(
            "claude",
            HostScope::Project,
            "volicord",
            &ManagedServerEntry::new("int_alpha", Path::new("volicord"), None),
            Some(PathBuf::from("/repo")),
        );
        let user = build_add_command("claude", HostScope::User, "volicord", &entry, None);

        assert_eq!(local.cwd, Some(PathBuf::from("/repo")));
        assert_eq!(project.cwd, Some(PathBuf::from("/repo")));
        assert_eq!(user.cwd, None);
        assert!(local
            .args
            .windows(2)
            .any(|pair| pair == ["--env", "VOLICORD_HOME=/runtime"]));
        let separator = local
            .args
            .iter()
            .position(|arg| arg == "--")
            .expect("separator");
        assert_eq!(
            &local.args[separator + 1..],
            [
                "/bin/volicord",
                "mcp",
                "--stdio",
                "--connection",
                "int_alpha"
            ]
        );
        let project_separator = project
            .args
            .iter()
            .position(|arg| arg == "--")
            .expect("project separator");
        assert_eq!(
            &project.args[project_separator + 1..],
            ["volicord", "mcp", "--stdio", "--connection", "int_alpha"]
        );
    }

    #[test]
    fn fake_cli_success_and_failure() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-cli")?;
        let mut adapter =
            ClaudeCodeAdapter::new(FakeRunner::new(vec![missing_output(), ok_output("added")]));
        let plan = adapter.plan(request(
            HostScope::Local,
            Some(&repo),
            Path::new("/bin/volicord"),
        ))?;
        let effect = adapter.apply(&plan)?;
        assert_eq!(effect.change, PlannedChange::ExternalCommand);
        assert_eq!(adapter.runner.calls[0].args, ["mcp", "get", "volicord"]);
        assert_eq!(adapter.runner.calls[1].args[0..2], ["mcp", "add"]);

        let mut failing = ClaudeCodeAdapter::new(FakeRunner::new(vec![
            missing_output(),
            CommandOutput {
                success: false,
                status_code: Some(1),
                stdout: String::new(),
                stderr: "boom".to_owned(),
            },
        ]));
        let plan = failing.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        assert!(matches!(
            failing.apply(&plan),
            Err(HostConfigError::ExternalCommand(_))
        ));
        Ok(())
    }

    #[test]
    fn intent_mapping_selects_claude_scopes() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-intent")?;
        let mut personal = ClaudeCodeAdapter::new(FakeRunner::new(vec![missing_output()]));
        let mut shared = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));
        let mut global = ClaudeCodeAdapter::new(FakeRunner::new(vec![missing_output()]));

        let personal_plan = personal.plan(request(
            HostScope::Local,
            Some(&repo),
            Path::new("/bin/volicord"),
        ))?;
        let shared_plan = shared.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("/bin/volicord"),
        ))?;
        let global_plan =
            global.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(personal_plan.host_scope, HostScope::Local);
        assert_eq!(shared_plan.host_scope, HostScope::Project);
        assert_eq!(global_plan.host_scope, HostScope::User);
        Ok(())
    }

    #[test]
    fn verify_distinguishes_pending_and_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-verify")?;
        let mut pending = ClaudeCodeAdapter::new(FakeRunner::new(vec![
            missing_output(),
            CommandOutput {
                success: true,
                status_code: Some(0),
                stdout: "⏸ Pending approval".to_owned(),
                stderr: String::new(),
            },
        ]));
        let plan = pending.plan(request(
            HostScope::Local,
            Some(&repo),
            Path::new("/bin/volicord"),
        ))?;
        let verification = pending.verify(&plan)?;
        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.user_actions[0].kind,
            UserActionKind::ProjectApprovalRequired
        );

        let mut rejected = ClaudeCodeAdapter::new(FakeRunner::new(vec![
            missing_output(),
            CommandOutput {
                success: true,
                status_code: Some(0),
                stdout: "✗ Rejected".to_owned(),
                stderr: String::new(),
            },
        ]));
        let plan = rejected.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        assert_eq!(rejected.verify(&plan)?.status.as_str(), "rejected");
        Ok(())
    }

    #[test]
    fn parser_distinguishes_supported_claude_mcp_outputs() {
        let connected = parse_claude_mcp_get_output(&CommandOutput {
            success: true,
            status_code: Some(0),
            stdout: "Status: ✓ Connected\nScope: local\nCommand: /bin/volicord\nArgs: [\"mcp\",\"--stdio\",\"--connection\",\"int_alpha\"]\nEnvironment:\n  VOLICORD_HOME=/runtime\n".to_owned(),
            stderr: String::new(),
        });
        assert_eq!(connected.state, ClaudeMcpState::Connected);
        assert_eq!(connected.scope, Some(HostScope::Local));
        assert_eq!(connected.command.as_deref(), Some("/bin/volicord"));
        assert_eq!(
            connected.args,
            Some(vec![
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--connection".to_owned(),
                "int_alpha".to_owned()
            ])
        );
        assert_eq!(
            connected.env.get("VOLICORD_HOME"),
            Some(&"/runtime".to_owned())
        );

        for (text, state, success) in [
            ("⏸ Pending approval", ClaudeMcpState::PendingApproval, true),
            ("✗ Rejected", ClaudeMcpState::Rejected, true),
            ("Server not found", ClaudeMcpState::Missing, false),
            ("unexpected traceback", ClaudeMcpState::CommandFailed, false),
            ("all quiet", ClaudeMcpState::Unknown, true),
        ] {
            let parsed = parse_claude_mcp_get_output(&CommandOutput {
                success,
                status_code: if success { Some(0) } else { Some(1) },
                stdout: text.to_owned(),
                stderr: String::new(),
            });
            assert_eq!(parsed.state, state, "output: {text}");
        }

        let unknown = parse_claude_mcp_get_output(&CommandOutput {
            success: true,
            status_code: Some(0),
            stdout: "SECRET_TOKEN=should-not-leak".to_owned(),
            stderr: String::new(),
        });
        assert!(!unknown
            .diagnostic
            .as_deref()
            .unwrap_or_default()
            .contains("should-not-leak"));
    }

    #[test]
    fn verify_connected_requires_reliable_command_args_env_and_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-connected")?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(vec![
            missing_output(),
            ok_output(
                "Status: ✓ Connected\nScope: local\nCommand: /bin/volicord\nArgs: mcp --stdio --connection int_alpha --project project_alpha\nEnvironment:\n  VOLICORD_HOME=/runtime\n",
            ),
        ]));
        let plan = adapter.plan(request(
            HostScope::Local,
            Some(&repo),
            Path::new("/bin/volicord"),
        ))?;
        let verification = adapter.verify(&plan)?;
        assert_eq!(verification.status.as_str(), "complete");
        assert_eq!(verification.host_state.as_str(), "configured_ready");

        let mut unknown = ClaudeCodeAdapter::new(FakeRunner::new(vec![
            missing_output(),
            ok_output("Status: ✓ Connected\nCommand: /bin/volicord\n"),
        ]));
        let plan = unknown.plan(request(
            HostScope::Local,
            Some(&repo),
            Path::new("/bin/volicord"),
        ))?;
        assert_eq!(unknown.verify(&plan)?.status.as_str(), "unknown");
        Ok(())
    }

    #[test]
    fn verify_project_file_runs_get_from_repo_root() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-project-verify")?;
        let mut adapter =
            ClaudeCodeAdapter::new(FakeRunner::new(vec![ok_output("⏸ Pending approval")]));
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(adapter.runner.calls[0].cwd, Some(repo));
        assert_eq!(adapter.runner.calls[0].args, ["mcp", "get", "volicord"]);
        Ok(())
    }

    #[test]
    fn project_file_preserves_unrelated_entries_and_is_idempotent(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-project")?;
        fs::write(
            repo.join(".mcp.json"),
            "{\"mcpServers\":{\"other\":{\"command\":\"other\"}},\"note\":true}\n",
        )?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;
        adapter.apply(&plan)?;
        let text = fs::read_to_string(repo.join(".mcp.json"))?;
        assert!(text.contains("\"other\""));
        assert!(text.contains("\"note\": true"));
        assert!(text.contains("\"volicord\""));

        let again = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;
        assert_eq!(again.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn project_file_reports_managed_fingerprint_mismatch() -> Result<(), Box<dyn std::error::Error>>
    {
        let repo = temp_dir("claude-project-mismatch")?;
        fs::write(
            repo.join(".mcp.json"),
            "{\"mcpServers\":{\"volicord\":{\"command\":\"volicord\",\"args\":[\"mcp\",\"--stdio\",\"--connection\",\"other\"]}}}\n",
        )?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::FingerprintMismatch
        );
        Ok(())
    }

    #[test]
    fn project_safe_remove_only_owned_entry() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-remove")?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;
        adapter.apply(&plan)?;
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("project target");
        };

        let effect = adapter.remove(HostRemoveRequest {
            host_kind: HostKind::ClaudeCode,
            connection_intent: plan.connection_intent,
            host_scope: HostScope::Project,
            mode: plan.mode.clone(),
            server_name: plan.server_name,
            target: HostTarget::File(target.clone()),
            expected_fingerprint: plan.fingerprint,
        })?;
        let text = fs::read_to_string(target)?;

        assert_eq!(effect.change, PlannedChange::Remove);
        assert!(!text.contains("volicord"));
        Ok(())
    }

    #[test]
    fn project_remove_refuses_manual_change() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-remove-mismatch")?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;
        adapter.apply(&plan)?;
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("project target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?
                .replace("\"command\": \"volicord\"", "\"command\": \"manual-mcp\""),
        )?;

        let error = adapter
            .remove(HostRemoveRequest {
                host_kind: HostKind::ClaudeCode,
                connection_intent: plan.connection_intent,
                host_scope: HostScope::Project,
                mode: plan.mode.clone(),
                server_name: plan.server_name,
                target: HostTarget::File(target),
                expected_fingerprint: plan.fingerprint,
            })
            .expect_err("manual change should block removal");

        assert!(matches!(error, HostConfigError::Conflict(_)));
        Ok(())
    }

    #[test]
    fn shared_intent_uses_path_command_and_no_runtime_home(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-project-path")?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("/personal/target/debug/volicord"),
        ))?;

        assert_eq!(plan.entry.command, "volicord");
        assert!(!plan.entry.env.contains_key("VOLICORD_HOME"));
        Ok(())
    }

    fn request<'a>(
        scope: HostScope,
        repo_root: Option<&'a Path>,
        mcp_command: &'a Path,
    ) -> HostPlanRequest<'a> {
        let connection_intent = match scope {
            HostScope::Local => ConnectionIntent::Personal,
            HostScope::Project => ConnectionIntent::Shared,
            HostScope::User => ConnectionIntent::Global,
            HostScope::Export => ConnectionIntent::Personal,
        };
        HostPlanRequest {
            host_kind: HostKind::ClaudeCode,
            connection_intent,
            project: repo_root.map(|repo_root| ProjectContext {
                project_id: "project_alpha",
                project_name: "Alpha",
                repo_root,
            }),
            installation_profile: InstallationProfile {
                runtime_home: Path::new("/runtime"),
                volicord_command: Path::new("/bin/volicord"),
                volicord_mcp_command: mcp_command,
                default_connection_mode: "workflow",
            },
            connection_id: "int_alpha",
            mode: "workflow",
            expected_fingerprint: None,
        }
    }

    fn missing_output() -> CommandOutput {
        CommandOutput {
            success: false,
            status_code: Some(1),
            stdout: String::new(),
            stderr: "Server not found".to_owned(),
        }
    }

    fn ok_output(text: &str) -> CommandOutput {
        CommandOutput {
            success: true,
            status_code: Some(0),
            stdout: text.to_owned(),
            stderr: String::new(),
        }
    }

    #[derive(Debug)]
    struct FakeRunner {
        outputs: VecDeque<CommandOutput>,
        calls: Vec<CommandInvocation>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<CommandOutput>) -> Self {
            Self {
                outputs: outputs.into(),
                calls: Vec::new(),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
            self.calls.push(invocation.clone());
            self.outputs
                .pop_front()
                .ok_or_else(|| "missing fake command output".to_owned())
        }
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
