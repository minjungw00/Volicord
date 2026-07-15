use super::{
    HostAdapter, HostCapabilities, HostConfigError, HostConflict, HostConflictKind, HostDetection,
    HostEffect, HostKind, HostPlan, HostPlanRequest, HostRemoveRequest, PlannedChange, UserAction,
    UserActionKind,
};
use crate::host_integration::verification::{
    HostConfigurationStatus, HostExecutableStatus, HostGateStatus, ManagedConfigStatus,
    Verification,
};

#[derive(Debug, Clone, Default)]
pub struct GenericAdapter;

pub const USER_MANAGED_CONFIGURATION_GUIDANCE: &str = "generic MCP host configuration is user-managed; accepted managed connection host values are `codex` and `claude-code`; configure external hosts manually for an enabled Agent Connection and require the launched process to pass MCP startup validation";
const USER_MANAGED_CONFIGURATION_DETAILS: &str =
    "generic MCP host configuration is user-managed and unverified by Volicord";
const USER_MANAGED_CONFIGURATION_ACTION: &str = "Configure the external MCP host manually for an enabled Agent Connection and require the launched process to pass MCP startup validation; Volicord does not write generic host configuration";

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        http_mcp: false,
        session_start_hook: false,
        pre_tool_hook: false,
        post_tool_hook: false,
        user_prompt_submit_hook: false,
        stop_hook: false,
        rule_file_support: false,
        project_local_configuration: false,
    }
}

impl GenericAdapter {
    pub fn plan(&self, request: HostPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        let _ = request;
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidScope,
            USER_MANAGED_CONFIGURATION_GUIDANCE,
        )))
    }
}

impl HostAdapter for GenericAdapter {
    fn capabilities(&self) -> HostCapabilities {
        capabilities()
    }

    fn detect(&self) -> Result<HostDetection, HostConfigError> {
        Ok(HostDetection {
            host_kind: HostKind::Generic,
            available: false,
            host_version: None,
            details: USER_MANAGED_CONFIGURATION_GUIDANCE.to_owned(),
        })
    }

    fn apply(&mut self, plan: &HostPlan) -> Result<HostEffect, HostConfigError> {
        if plan.host_kind != HostKind::Generic {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "generic adapter cannot apply a non-generic host target",
            )));
        }
        Ok(effect_from_plan(plan, PlannedChange::Noop))
    }

    fn verify(&mut self, plan: &HostPlan) -> Result<Verification, HostConfigError> {
        if plan.host_kind != HostKind::Generic {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "generic adapter cannot verify a non-generic host target",
            )));
        }
        Ok(
            Verification::action_required(USER_MANAGED_CONFIGURATION_DETAILS)
                .with_managed_config(ManagedConfigStatus::NotApplicable)
                .with_host_executable(HostExecutableStatus::NotRequired)
                .with_host_gate(HostGateStatus::ActionRequired)
                .with_host_configuration(HostConfigurationStatus::NotApplicable)
                .with_mcp_handshake_allowed(true)
                .merge_user_actions(&generic_user_actions())
                .merge_user_actions(&plan.user_actions),
        )
    }

    fn remove(&mut self, request: HostRemoveRequest) -> Result<HostEffect, HostConfigError> {
        if request.host_kind != HostKind::Generic {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "generic adapter cannot remove a non-generic host target",
            )));
        }
        Ok(remove_effect(request, PlannedChange::Noop))
    }
}

fn generic_user_actions() -> Vec<UserAction> {
    vec![UserAction::new(
        UserActionKind::HostTrustRequired,
        USER_MANAGED_CONFIGURATION_ACTION,
    )]
}

fn merged_user_actions(existing: &[UserAction]) -> Vec<UserAction> {
    let mut actions = generic_user_actions();
    for action in existing {
        if !actions.contains(action) {
            actions.push(action.clone());
        }
    }
    actions
}

fn effect_from_plan(plan: &HostPlan, change: PlannedChange) -> HostEffect {
    HostEffect {
        host_kind: plan.host_kind,
        connection_intent: plan.connection_intent,
        host_scope: plan.host_scope,
        mode: plan.mode.clone(),
        server_name: plan.server_name.clone(),
        target: plan.target.clone(),
        change,
        fingerprint: plan.fingerprint.clone(),
        user_actions: merged_user_actions(&plan.user_actions),
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
        user_actions: generic_user_actions(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::super::{
        ConnectionIntent, HostRemoveRequest, HostScope, HostTarget, ManagedServerEntry,
    };
    use super::*;

    #[test]
    fn generic_host_is_user_managed_configuration() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-user-managed")?;
        let target = dir.join("external-host.json");
        let mut adapter = GenericAdapter;
        let plan = stored_generic_plan(target.clone());

        let verification = adapter.verify(&plan)?;
        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(verification.managed_config.as_str(), "not_applicable");
        assert_eq!(verification.host_configuration.as_str(), "not_applicable");
        assert_eq!(verification.host_executable.as_str(), "not_required");
        assert_eq!(verification.host_gate.as_str(), "action_required");
        assert!(verification.mcp_handshake_allowed);
        assert!(verification.details.contains("user-managed"));
        assert_eq!(
            verification.user_actions[0].message,
            USER_MANAGED_CONFIGURATION_ACTION
        );

        let applied = adapter.apply(&plan)?;
        assert_eq!(applied.change, PlannedChange::Noop);
        assert!(!target.exists());
        let effect = adapter.remove(HostRemoveRequest {
            host_kind: HostKind::Generic,
            connection_intent: plan.connection_intent,
            host_scope: HostScope::Export,
            mode: plan.mode.clone(),
            server_name: plan.server_name.clone(),
            target: HostTarget::Export(target.clone()),
            expected_fingerprint: plan.fingerprint.clone(),
        })?;
        assert_eq!(effect.change, PlannedChange::Noop);
        assert!(!target.exists());
        Ok(())
    }

    fn stored_generic_plan(target: PathBuf) -> HostPlan {
        HostPlan {
            host_kind: HostKind::Generic,
            connection_intent: ConnectionIntent::Personal,
            host_scope: HostScope::Export,
            mode: "workflow".to_owned(),
            server_name: "volicord".to_owned(),
            target: HostTarget::Export(target),
            entry: ManagedServerEntry::new(
                "int_alpha",
                Path::new("/bin/volicord"),
                Some(Path::new("/runtime")),
            ),
            change: PlannedChange::Noop,
            fingerprint: "user-managed".to_owned(),
            conflicts: Vec::new(),
            user_actions: Vec::new(),
            file_snapshot: None,
        }
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
