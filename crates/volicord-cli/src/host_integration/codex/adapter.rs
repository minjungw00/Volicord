use std::{
    cell::RefCell,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::host_integration::process::{CommandRunner, ProductionCommandRunner};
use crate::host_integration::verification::{ManagedConfigStatus, Verification};
use crate::host_integration::{
    config_edit::{read_text_snapshot, write_if_fresh},
    validated_server_name, ConnectionIntent, HostAdapter, HostConfigError, HostConflict,
    HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, PlannedChange, ProjectContext,
};
use toml_edit::Item;
use volicord_mcp::ManagedMcpLaunchSpec;

use super::{
    capabilities,
    config::{document_from_snapshot, parse_document, upsert_server_table},
    executable::{codex_executable_availability, CodexExecutableAvailability},
    identity::{
        classify_existing_codex_entry, codex_managed_identity_fingerprint,
        codex_managed_launch_spec, evaluate_codex_managed_identity,
    },
    trust::project_trust_for_plan,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexEnvironment {
    pub home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub path: Option<OsString>,
}

#[derive(Debug, Clone)]
pub struct CodexAdapter<R = ProductionCommandRunner> {
    env: CodexEnvironment,
    runner: RefCell<R>,
}

impl CodexAdapter<ProductionCommandRunner> {
    pub fn new(env: CodexEnvironment) -> Self {
        Self::with_runner(env, ProductionCommandRunner)
    }
}

impl<R: CommandRunner> CodexAdapter<R> {
    pub fn with_runner(env: CodexEnvironment, runner: R) -> Self {
        Self {
            env,
            runner: RefCell::new(runner),
        }
    }

    pub fn plan(&self, request: HostPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        if request.host_kind != HostKind::Codex {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex adapter cannot plan a non-Codex host request",
            )));
        }
        let scope = codex_scope_for_intent(request.connection_intent)?;
        let (mcp_command, runtime_home) =
            entry_inputs_for_scope(scope, request.installation_profile);
        let server_name = validated_server_name(request.connection_id, None)?;
        let target = self.config_path(scope, request.project)?;
        let entry =
            codex_managed_launch_spec(scope, request.connection_id, mcp_command, runtime_home)?;
        let fingerprint = entry.managed_fingerprint(&server_name);
        let (snapshot, text) = read_text_snapshot(&target)?;
        let document = parse_document(text.as_deref(), &target)?;
        if document.as_table().contains_key("mcp_servers")
            && document
                .get("mcp_servers")
                .and_then(Item::as_table)
                .is_none()
        {
            return Err(HostConfigError::Malformed(
                "Codex mcp_servers configuration must be a table".to_owned(),
            ));
        }
        let existing = document
            .get("mcp_servers")
            .and_then(Item::as_table)
            .and_then(|servers| servers.get(&server_name));
        let mut conflicts = Vec::new();
        let change = match existing {
            None => PlannedChange::Create,
            Some(item) => classify_existing_codex_entry(
                scope,
                &server_name,
                item,
                &fingerprint,
                request.expected_fingerprint,
                &mut conflicts,
            ),
        };
        Ok(HostPlan {
            host_kind: HostKind::Codex,
            connection_intent: request.connection_intent,
            host_scope: scope,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::File(target),
            entry,
            change,
            fingerprint,
            conflicts,
            actions: Vec::new(),
            file_snapshot: Some(snapshot),
        })
    }

    pub fn plan_existing(
        &self,
        request: CodexExistingPlanRequest<'_>,
    ) -> Result<HostPlan, HostConfigError> {
        if !matches!(request.scope, HostScope::User | HostScope::Project) {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex supports only user and project host scopes",
            )));
        }
        let server_name = validated_server_name(request.connection_id, Some(request.server_name))?;
        let entry = codex_managed_launch_spec(
            request.scope,
            request.connection_id,
            request.mcp_command,
            request.runtime_home,
        )?;
        let fingerprint = entry.managed_fingerprint(&server_name);
        Ok(HostPlan {
            host_kind: HostKind::Codex,
            connection_intent: request.connection_intent,
            host_scope: request.scope,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::File(request.config_target.to_path_buf()),
            entry,
            change: PlannedChange::Noop,
            fingerprint,
            conflicts: Vec::new(),
            actions: Vec::new(),
            file_snapshot: None,
        })
    }

    fn config_path(
        &self,
        scope: HostScope,
        project: Option<ProjectContext<'_>>,
    ) -> Result<PathBuf, HostConfigError> {
        match scope {
            HostScope::User => Ok(self.codex_home()?.join("config.toml")),
            HostScope::Project => {
                let project = project.ok_or_else(|| {
                    HostConfigError::Conflict(HostConflict::new(
                        HostConflictKind::InvalidScope,
                        "Codex shared connection intent requires a Product Repository root",
                    ))
                })?;
                Ok(project.repo_root.join(".codex").join("config.toml"))
            }
        }
    }

    fn codex_home(&self) -> Result<PathBuf, HostConfigError> {
        if let Some(path) = &self.env.codex_home {
            return Ok(path.clone());
        }
        let home = self.env.home.as_ref().ok_or_else(|| {
            HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Codex user configuration requires CODEX_HOME or HOME",
            ))
        })?;
        Ok(home.join(".codex"))
    }

    fn executable_availability(&self, _config_target: &Path) -> CodexExecutableAvailability {
        codex_executable_availability(&self.runner, self.env.path.as_ref())
    }
}

impl<R: CommandRunner> HostAdapter for CodexAdapter<R> {
    fn capabilities(&self) -> crate::host_integration::HostCapabilities {
        capabilities()
    }

    fn detect(&self) -> Result<HostDetection, HostConfigError> {
        let path = self.codex_home()?.join("config.toml");
        let availability = self.executable_availability(&path);
        Ok(HostDetection {
            host_kind: HostKind::Codex,
            available: availability.is_available(),
            host_version: availability.host_version,
            details: availability.details,
        })
    }

    fn apply(&mut self, plan: &HostPlan) -> Result<HostEffect, HostConfigError> {
        if plan.host_kind != HostKind::Codex {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex adapter cannot apply a non-Codex host plan",
            )));
        }
        if let Some(conflict) = plan.conflicts.first() {
            return Err(HostConfigError::Conflict(conflict.clone()));
        }
        if plan.change == PlannedChange::Noop {
            return Ok(effect_from_plan(plan));
        }
        let HostTarget::File(target) = &plan.target else {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Codex plan target must be a file",
            )));
        };
        let snapshot = plan.file_snapshot.as_ref().ok_or_else(|| {
            HostConfigError::StalePlan("Codex plan is missing its file snapshot".to_owned())
        })?;
        let mut document = document_from_snapshot(snapshot, target)?;
        upsert_server_table(&mut document, &plan.server_name, &plan.entry)?;
        write_if_fresh(target, document.to_string().as_bytes(), snapshot)?;
        Ok(effect_from_plan(plan))
    }

    fn verify(&mut self, plan: &HostPlan) -> Result<Verification, HostConfigError> {
        let config_target = match &plan.target {
            HostTarget::File(target) => target.as_path(),
            _ => Path::new("unknown Codex configuration target"),
        };
        let executable = self.executable_availability(config_target);
        let mut managed_evaluation = evaluate_codex_managed_identity(plan)?;
        if let Some(conflict) = plan.conflicts.first() {
            managed_evaluation.status = match conflict.kind {
                HostConflictKind::UnmanagedNameCollision => ManagedConfigStatus::Unmanaged,
                _ => ManagedConfigStatus::Changed,
            };
            managed_evaluation.diagnostic = Some(
                crate::host_integration::verification::ManagedConfigDiagnostic::FingerprintMismatch,
            );
            managed_evaluation.details = conflict.message.clone();
        }
        let project_trust = (plan.host_scope == HostScope::Project)
            .then(|| project_trust_for_plan(&self.env, plan));
        Ok(Verification {
            config_target: config_target.display().to_string(),
            managed_config: managed_evaluation.status,
            managed_config_diagnostic: managed_evaluation.diagnostic,
            managed_config_details: managed_evaluation.details,
            host_executable: executable.status,
            executable_path: executable.executable_path,
            host_version: executable.host_version,
            host_executable_code: executable.code,
            host_executable_details: executable.details,
            project_trust,
        })
    }

    fn remove(&mut self, request: HostRemoveRequest) -> Result<HostEffect, HostConfigError> {
        if request.host_kind != HostKind::Codex {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex adapter cannot remove a non-Codex host plan",
            )));
        }
        let HostTarget::File(target) = &request.target else {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Codex removal target must be a file",
            )));
        };
        let (snapshot, text) = read_text_snapshot(target)?;
        let mut document = parse_document(text.as_deref(), target)?;
        let Some(servers) = document.get_mut("mcp_servers").and_then(Item::as_table_mut) else {
            return Ok(remove_effect(request, PlannedChange::Noop));
        };
        let Some(existing) = servers.get(&request.server_name) else {
            return Ok(remove_effect(request, PlannedChange::Noop));
        };
        let current =
            codex_managed_identity_fingerprint(request.host_scope, &request.server_name, existing);
        if current.as_deref() != Some(request.expected_fingerprint.as_str()) {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::FingerprintMismatch,
                format!(
                    "Codex MCP server changed since Volicord last managed it: {}",
                    request.server_name
                ),
            )));
        }
        servers.remove(&request.server_name);
        write_if_fresh(target, document.to_string().as_bytes(), &snapshot)?;
        Ok(remove_effect(request, PlannedChange::Remove))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CodexExistingPlanRequest<'a> {
    pub connection_intent: ConnectionIntent,
    pub scope: HostScope,
    pub connection_id: &'a str,
    pub server_name: &'a str,
    pub config_target: &'a Path,
    pub mcp_command: &'a Path,
    pub runtime_home: Option<&'a Path>,
    pub mode: &'a str,
}

fn codex_scope_for_intent(intent: ConnectionIntent) -> Result<HostScope, HostConfigError> {
    match intent {
        ConnectionIntent::Personal => Ok(HostScope::User),
        ConnectionIntent::Shared => Ok(HostScope::Project),
    }
}

fn entry_inputs_for_scope<'a>(
    scope: HostScope,
    profile: InstallationProfile<'a>,
) -> (&'a Path, Option<&'a Path>) {
    if scope == HostScope::Project {
        (Path::new(ManagedMcpLaunchSpec::PATH_COMMAND), None)
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
        actions: plan.actions.clone(),
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
        actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::{ConnectionAction, ConnectionActionKind};

    use super::*;
    #[test]
    fn host_effect_preserves_canonical_action_kind_and_instruction() {
        let action = ConnectionAction::try_new(
            ConnectionActionKind::InspectRuntimeSession,
            "Inspect the Codex protocol failure",
        )
        .expect("canonical host action");
        let plan = HostPlan {
            host_kind: HostKind::Codex,
            connection_intent: ConnectionIntent::Personal,
            host_scope: HostScope::User,
            mode: "workflow".to_owned(),
            server_name: "volicord".to_owned(),
            target: HostTarget::File(PathBuf::from("/tmp/codex-config.toml")),
            entry: ManagedMcpLaunchSpec::personal(
                Path::new("/usr/bin/volicord"),
                Path::new("/srv/volicord/runtime"),
                "connection_1",
            )
            .expect("personal launch"),
            change: PlannedChange::Noop,
            fingerprint: "sha256:test".to_owned(),
            conflicts: Vec::new(),
            actions: vec![action.clone()],
            file_snapshot: None,
        };

        let effect = effect_from_plan(&plan);

        assert_eq!(plan.actions, vec![action.clone()]);
        assert_eq!(effect.actions, vec![action]);
        assert_eq!(
            serde_json::to_value(&effect.actions[0]).expect("action JSON"),
            serde_json::json!({
                "id": "inspect_runtime_session",
                "owner": "agent",
                "channel": "documentation",
                "prerequisites": ["host_reload"],
                "completes_checks": ["managed_capability_proof", "managed_session_health"],
                "root_finding_ids": [],
                "instruction": "Inspect the Codex protocol failure",
            })
        );
    }
}
