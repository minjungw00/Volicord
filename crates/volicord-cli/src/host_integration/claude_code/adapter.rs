use std::path::Path;

use serde_json::Value;
use volicord_mcp::RepositoryDiscoveryHost;

use crate::host_integration::verification::{
    HostConfigurationStatus, ManagedConfigStatus, Verification,
};
use crate::host_integration::{
    config_edit::{read_json_object, write_json_object_if_fresh},
    managed_fingerprint, validate_managed_server_entry_schema, validated_server_name,
    ConnectionIntent, HostAdapter, HostCapabilities, HostConfigError, HostConflict,
    HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, ManagedServerEntry,
    PlannedChange, UserAction, UserActionKind, DEFAULT_MCP_COMMAND,
};

use super::{
    capabilities,
    cli::{build_add_command, build_get_command, build_remove_command, CommandRunner},
    config::{
        classify_existing_json_entry, current_project_entry_fingerprint, remove_project_entry,
        upsert_project_entry, validate_mcp_command, verification_from_managed_status,
        verify_claude_project_entry,
    },
    parser::{
        fingerprint_from_claude_inspection, inspection_is_volicord_managed,
        parse_claude_mcp_get_output, verification_from_claude_output, ClaudeMcpState,
    },
};

#[derive(Debug, Clone)]
pub struct ClaudeCodeAdapter<R> {
    runner: R,
    claude_command: String,
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
        let entry = if scope == HostScope::Project {
            ManagedServerEntry::new_repository_discovery(RepositoryDiscoveryHost::ClaudeCode)
        } else {
            ManagedServerEntry::new_project_bound(
                request.connection_id,
                request.project.map(|project| project.project_id),
                mcp_command,
                runtime_home,
            )
        };
        validate_managed_server_entry_schema(HostKind::ClaudeCode, scope, &entry)?;
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
            host_version: None,
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
                let current = current_project_entry_fingerprint(&request.server_name, existing);
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
                if inspection.state == ClaudeMcpState::Missing {
                    return Ok(remove_effect(request, PlannedChange::Noop));
                }
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
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::host_integration::{
        managed_fingerprint,
        verification::{
            ActiveToolExposureStatus, HostApprovalState, HostConfigState, HostPolicyOverlayState,
            ManagedConfigStatus, StorageCapability,
        },
        ConnectionIntent, HostAdapter, HostConfigError, HostConflictKind, HostKind,
        HostPlanRequest, HostRemoveRequest, HostScope, HostTarget, InstallationProfile,
        ManagedServerEntry, PlannedChange, ProjectContext, UserActionKind, DEFAULT_SERVER_NAME,
    };

    use super::super::{
        cli::build_add_command,
        parser::{parse_claude_mcp_get_output, ClaudeMcpState},
        CommandInvocation, CommandOutput, CommandRunner,
    };
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
    fn project_file_managed_entry_uses_claude_mcp_identity_contract(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-project-identity")?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;
        adapter.apply(&plan)?;

        let text = fs::read_to_string(repo.join(".mcp.json"))?;
        let value: Value = serde_json::from_str(&text)?;
        let server = value["mcpServers"]["volicord"]
            .as_object()
            .expect("managed server should be an object");

        assert_eq!(
            server.get("command"),
            Some(&Value::String("volicord".to_owned()))
        );
        assert_eq!(
            server.get("args"),
            Some(&serde_json::json!([
                "mcp",
                "--stdio",
                "--discover-repository",
                "--host",
                "claude-code"
            ]))
        );
        assert_eq!(
            server.get("env"),
            Some(&serde_json::json!({
                "VOLICORD_HOME": "${VOLICORD_HOME}"
            }))
        );
        assert!(server.get("env_vars").is_none());
        assert!(!text.contains("int_alpha"));
        assert!(!text.contains("project_alpha"));
        assert!(!text.contains("/runtime"));
        assert!(!text.contains("VOLICORD_MCP_LAUNCH"));
        assert!(!text.contains("VOLICORD_MCP_HOST"));
        assert!(!text.contains("VOLICORD_MCP_CONNECTION_ID"));
        assert!(!text.contains("VOLICORD_MCP_PROJECT_ID"));
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
    fn project_file_detects_true_managed_identity_drift() -> Result<(), Box<dyn std::error::Error>>
    {
        let cases = [
            (
                "command",
                serde_json::json!({
                    "command": "/tmp/not-volicord",
                    "args": ["mcp", "--stdio", "--connection", "int_alpha", "--project", "project_alpha"]
                }),
            ),
            (
                "connection",
                serde_json::json!({
                    "command": "volicord",
                    "args": ["mcp", "--stdio", "--connection", "other", "--project", "project_alpha"]
                }),
            ),
            (
                "project",
                serde_json::json!({
                    "command": "volicord",
                    "args": ["mcp", "--stdio", "--connection", "int_alpha", "--project", "other_project"]
                }),
            ),
        ];

        for (name, server) in cases {
            let repo = temp_dir(&format!("claude-project-drift-{name}"))?;
            fs::write(
                repo.join(".mcp.json"),
                serde_json::to_string(&serde_json::json!({
                    "mcpServers": {
                        "volicord": server
                    }
                }))?,
            )?;
            let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

            let plan = adapter.plan(request(
                HostScope::Project,
                Some(&repo),
                Path::new("volicord"),
            ))?;

            assert_eq!(plan.change, PlannedChange::Noop, "{name}");
            assert_eq!(
                plan.conflicts[0].kind,
                HostConflictKind::FingerprintMismatch,
                "{name}"
            );
        }
        Ok(())
    }

    #[test]
    fn project_file_reports_unmanaged_server_name_collision(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-project-unmanaged")?;
        fs::write(
            repo.join(".mcp.json"),
            serde_json::to_string(&serde_json::json!({
                "mcpServers": {
                    "volicord": {
                        "command": "other-mcp",
                        "args": ["serve"]
                    }
                }
            }))?,
        )?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("volicord"),
        ))?;

        assert_eq!(plan.change, PlannedChange::Noop);
        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::UnmanagedNameCollision
        );
        Ok(())
    }

    #[test]
    fn stored_shared_binding_migrates_once_to_portable_discovery(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-legacy-shared-migration")?;
        let legacy = ManagedServerEntry::new_project_bound(
            "int_alpha",
            Some("project_alpha"),
            Path::new("volicord"),
            None,
        );
        let legacy_fingerprint = managed_fingerprint(
            HostKind::ClaudeCode,
            HostScope::Project,
            DEFAULT_SERVER_NAME,
            &legacy,
        );
        fs::write(
            repo.join(".mcp.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "mcpServers": {
                    "volicord": legacy.to_json_value()
                }
            }))? + "\n",
        )?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let migration = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&legacy_fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(migration.change, PlannedChange::Update);
        adapter.apply(&migration)?;

        let migrated = fs::read_to_string(repo.join(".mcp.json"))?;
        assert!(migrated.contains("--discover-repository"));
        assert!(migrated.contains("${VOLICORD_HOME}"));
        assert!(!migrated.contains("--connection"));
        assert!(!migrated.contains("int_alpha"));
        let again = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&migration.fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(again.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn stored_discovery_without_runtime_home_reference_migrates_with_v1_fingerprint(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-discovery-forwarding-migration")?;
        let mut legacy =
            ManagedServerEntry::new_repository_discovery(RepositoryDiscoveryHost::ClaudeCode);
        legacy.env.clear();
        let legacy_fingerprint = managed_fingerprint(
            HostKind::ClaudeCode,
            HostScope::Project,
            DEFAULT_SERVER_NAME,
            &legacy,
        );
        fs::write(
            repo.join(".mcp.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "mcpServers": {
                    "volicord": legacy.to_json_value()
                }
            }))? + "\n",
        )?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let migration = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&legacy_fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(migration.change, PlannedChange::Update);
        assert!(migration.conflicts.is_empty());
        adapter.apply(&migration)?;

        let migrated: Value = serde_json::from_str(&fs::read_to_string(repo.join(".mcp.json"))?)?;
        assert_eq!(
            migrated["mcpServers"]["volicord"]["env"]["VOLICORD_HOME"],
            "${VOLICORD_HOME}"
        );
        let again = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&migration.fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(again.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn project_discovery_rejects_injected_environment_shapes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            (
                "literal-home",
                serde_json::json!({"VOLICORD_HOME": "/tmp/injected"}),
                None,
            ),
            (
                "extra-env",
                serde_json::json!({
                    "VOLICORD_HOME": "${VOLICORD_HOME}",
                    "API_TOKEN": "injected"
                }),
                None,
            ),
            (
                "forwarded-env",
                serde_json::json!({"VOLICORD_HOME": "${VOLICORD_HOME}"}),
                Some(serde_json::json!(["VOLICORD_HOME"])),
            ),
        ];

        for (name, env, env_vars) in cases {
            let repo = temp_dir(&format!("claude-project-env-reject-{name}"))?;
            let mut server = serde_json::json!({
                "command": "volicord",
                "args": ["mcp", "--stdio", "--discover-repository", "--host", "claude-code"],
                "env": env,
            });
            if let Some(env_vars) = env_vars {
                server["env_vars"] = env_vars;
            }
            fs::write(
                repo.join(".mcp.json"),
                serde_json::to_string_pretty(&serde_json::json!({
                    "mcpServers": {"volicord": server}
                }))? + "\n",
            )?;
            let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

            let plan = adapter.plan(request(
                HostScope::Project,
                Some(&repo),
                Path::new("ignored"),
            ))?;

            assert_eq!(plan.change, PlannedChange::Noop, "{name}");
            assert_eq!(
                plan.conflicts[0].kind,
                HostConflictKind::UnmanagedNameCollision,
                "{name}"
            );
        }
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
    fn shared_intent_uses_path_command_and_parent_runtime_home_reference(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-project-path")?;
        let mut adapter = ClaudeCodeAdapter::new(FakeRunner::new(Vec::new()));

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("/personal/target/debug/volicord"),
        ))?;

        assert_eq!(plan.entry.command, "volicord");
        assert_eq!(
            plan.entry.args,
            [
                "mcp",
                "--stdio",
                "--discover-repository",
                "--host",
                "claude-code"
            ]
        );
        assert_eq!(
            plan.entry.env.get("VOLICORD_HOME").map(String::as_str),
            Some("${VOLICORD_HOME}")
        );
        assert!(plan.entry.env_vars.is_empty());
        assert!(!plan.entry.args.iter().any(|arg| arg == "int_alpha"));
        Ok(())
    }

    #[test]
    fn connected_claude_cli_state_does_not_confirm_active_tool_exposure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("claude-connected-contract")?;
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
        let contract = verification.common_contract();

        assert_eq!(verification.status.as_str(), "complete");
        assert!(verification.mcp_handshake_allowed);
        assert_eq!(contract.host_config, HostConfigState::Match);
        assert_eq!(contract.managed_identity, ManagedConfigStatus::Match);
        assert_eq!(contract.host_policy_overlay, HostPolicyOverlayState::Absent);
        assert_eq!(contract.host_approval, HostApprovalState::Approved);
        assert!(contract.managed_lifecycle.is_none());
        assert_eq!(
            contract.active_tool_exposure,
            ActiveToolExposureStatus::Unknown
        );
        assert_eq!(contract.storage_capability, StorageCapability::Unknown);
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
