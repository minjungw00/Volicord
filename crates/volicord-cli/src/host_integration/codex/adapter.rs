use std::{
    cell::RefCell,
    ffi::OsString,
    path::{Path, PathBuf},
};

use toml_edit::Item;

use crate::host_integration::verification::{
    HostExecutableStatus, HostGateStatus, ManagedConfigStatus, ProjectTrustStatus, Verification,
};
use crate::host_integration::{
    claude_code::{CommandRunner, ProductionCommandRunner},
    config_edit::{read_text_snapshot, write_if_fresh},
    format_supported_connection_intents, validate_managed_server_entry_schema,
    validated_server_name, ConnectionIntent, HostAdapter, HostConfigError, HostConflict,
    HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, PlannedChange, ProjectContext,
    UserAction, UserActionKind, DEFAULT_MCP_COMMAND,
};

use super::{
    capabilities,
    config::{document_from_snapshot, parse_document, upsert_server_table, validate_mcp_command},
    executable::{
        codex_executable_availability, verification_from_executable_unavailable,
        CodexExecutableAvailability,
    },
    identity::{
        classify_existing_codex_entry, codex_managed_identity_fingerprint,
        codex_managed_server_entry, evaluate_codex_managed_identity,
        verification_from_managed_status,
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
        validate_mcp_command(scope, mcp_command)?;

        let server_name = validated_server_name(request.connection_id, None)?;
        let target = self.config_path(scope, request.project)?;
        let project_id = (scope == HostScope::Project)
            .then(|| request.project.map(|project| project.project_id))
            .flatten();
        let entry = codex_managed_server_entry(
            scope,
            request.connection_id,
            project_id,
            mcp_command,
            runtime_home,
        );
        validate_managed_server_entry_schema(HostKind::Codex, scope, &entry)?;
        let fingerprint = crate::host_integration::managed_fingerprint(
            HostKind::Codex,
            scope,
            &server_name,
            &entry,
        );
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
            user_actions: Vec::new(),
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
        validate_mcp_command(request.scope, request.mcp_command)?;
        if request.scope == HostScope::Project && request.runtime_home.is_some() {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidCommand,
                "Codex project-scoped configuration must not embed a personal VOLICORD_HOME",
            )));
        }

        let server_name = validated_server_name(request.connection_id, Some(request.server_name))?;
        let project_id = (request.scope == HostScope::Project)
            .then_some(request.project_id)
            .flatten();
        let entry = codex_managed_server_entry(
            request.scope,
            request.connection_id,
            project_id,
            request.mcp_command,
            request.runtime_home,
        );
        validate_managed_server_entry_schema(HostKind::Codex, request.scope, &entry)?;
        let fingerprint = crate::host_integration::managed_fingerprint(
            HostKind::Codex,
            request.scope,
            &server_name,
            &entry,
        );
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
            user_actions: Vec::new(),
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
            _ => Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                format!(
                    "Codex supports only these connection intents: {}",
                    format_supported_connection_intents(HostKind::Codex)
                ),
            ))),
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

    fn executable_availability(&self, config_target: &Path) -> CodexExecutableAvailability {
        codex_executable_availability(&self.runner, self.env.path.as_ref(), config_target)
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
        if let Some(conflict) = plan.conflicts.first() {
            return Ok(Verification::changed(conflict.message.clone())
                .merge_user_actions(&plan.user_actions));
        }
        let config_target = match &plan.target {
            HostTarget::File(target) => target.as_path(),
            _ => Path::new("unknown Codex configuration target"),
        };
        let executable = self.executable_availability(config_target);
        let managed_evaluation = evaluate_codex_managed_identity(plan)?;
        let managed = managed_evaluation.status;
        if managed != ManagedConfigStatus::Match {
            let mut verification = verification_from_managed_status(
                managed,
                format!(
                    "Codex managed MCP server entry is {} for {}",
                    managed.as_str(),
                    plan.server_name
                ),
            )
            .with_host_executable(executable.status);
            if let Some(overlay) = managed_evaluation.host_policy_overlay {
                verification = verification.with_host_policy_overlay(overlay);
            }
            if let Some(diagnostic) = executable.diagnostic {
                verification = verification.with_diagnostic(diagnostic);
            }
            return Ok(verification.merge_user_actions(&plan.user_actions));
        }
        if plan.host_scope == HostScope::Project {
            let project_trust = project_trust_for_plan(&self.env, plan);
            if !executable.is_available() {
                let mut verification = verification_from_executable_unavailable(executable);
                if let Some(overlay) = managed_evaluation.host_policy_overlay {
                    verification = verification.with_host_policy_overlay(overlay);
                }
                verification = verification.with_project_trust(project_trust);
                return Ok(verification.merge_user_actions(&plan.user_actions));
            }
            let mut verification = match project_trust.status {
                ProjectTrustStatus::Trusted => Verification::configured_ready(
                    "Codex managed configuration is present, Codex executable is available, and Codex project trust is trusted",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_gate(HostGateStatus::Ready)
                .with_mcp_handshake_allowed(true),
                ProjectTrustStatus::Untrusted => {
                    Verification::action_required(
                        "Codex managed configuration is present, Codex executable is available, and Codex project trust is untrusted",
                    )
                    .with_host_executable(HostExecutableStatus::Available)
                    .with_host_gate(HostGateStatus::ActionRequired)
                    .with_mcp_handshake_allowed(true)
                    .with_user_actions(vec![UserAction::new(
                        UserActionKind::HostTrustRequired,
                        "Codex project trust is untrusted in the Codex user configuration",
                    )])
                }
                ProjectTrustStatus::Missing
                | ProjectTrustStatus::Unknown
                | ProjectTrustStatus::Unreadable
                | ProjectTrustStatus::Malformed => Verification::configured_ready(
                    "Codex managed configuration is present and Codex executable is available; Codex project trust is not confirmed from the user configuration",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_gate(HostGateStatus::Unknown)
                .with_mcp_handshake_allowed(true),
            };
            if let Some(overlay) = managed_evaluation.host_policy_overlay {
                verification = verification.with_host_policy_overlay(overlay);
            }
            verification = verification.with_project_trust(project_trust);
            return Ok(verification.merge_user_actions(&plan.user_actions));
        }
        if !executable.is_available() {
            let mut verification = verification_from_executable_unavailable(executable);
            if let Some(overlay) = managed_evaluation.host_policy_overlay {
                verification = verification.with_host_policy_overlay(overlay);
            }
            return Ok(verification.merge_user_actions(&plan.user_actions));
        }
        let mut verification = Verification::configured_ready(
            "Codex managed configuration is present, Codex executable is available, and no separate project trust gate applies",
        )
        .with_host_executable(HostExecutableStatus::Available)
        .with_mcp_handshake_allowed(true);
        if let Some(overlay) = managed_evaluation.host_policy_overlay {
            verification = verification.with_host_policy_overlay(overlay);
        }
        Ok(verification.merge_user_actions(&plan.user_actions))
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
    pub project_id: Option<&'a str>,
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
        ConnectionIntent::Global => Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidScope,
            format!(
                "Codex does not support global connection intent; supported connection intents: {}",
                format_supported_connection_intents(HostKind::Codex)
            ),
        ))),
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
