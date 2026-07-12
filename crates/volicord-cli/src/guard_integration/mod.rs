use std::{error::Error, fmt};

pub(crate) mod apply;
pub(crate) mod audit;
pub(crate) mod capability;
pub(crate) mod files;
pub(crate) mod git_exclude;
pub(crate) mod hooks;
pub(crate) mod hosts;
pub(crate) mod plan;
pub(crate) mod policy;

#[cfg(test)]
pub(crate) use apply::set_script_executable;
pub(crate) use apply::{apply_guard_integration, apply_guard_migration_protection};
#[cfg(all(test, unix))]
pub(crate) use audit::CODEX_DISPATCH_WRAPPER;
pub(crate) use audit::{HookWrapperResolutionStatus, ManagedJsonProjection, HOOK_WRAPPER_MARKER};
#[cfg(test)]
pub(crate) use capability::host_hook_capability_json;
pub(crate) use capability::{
    generated_files_json, hook_root_resolution_json, host_hook_commands_json,
    initial_guard_installation_status, record_guard_installation, retired_files_json,
};
pub(crate) use files::{FilePlanStatus, GeneratedFilePlan};
#[cfg(test)]
pub(crate) use files::{
    AGENTS_FILE, GUIDANCE_END_MARKER, GUIDANCE_START_MARKER, VOLICORD_POLICY_FILE,
};
#[cfg(test)]
pub(crate) use hooks::shell_word;
pub(crate) use hooks::{
    observe_hook_root_unsupported_message, HostHookCommand, HostHookCommandShape,
};
pub(crate) use plan::{plan_guard_integration, GuardIntegrationPlan, GuardIntegrationPlanRequest};
pub(crate) use policy::{guard_has_prompt_capture_commands, lifecycle_phase_names};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuardIntegrationError {
    message: String,
}

impl GuardIntegrationError {
    pub(crate) fn runtime(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GuardIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GuardIntegrationError {}

pub(crate) fn public_host_label(host_kind: crate::host_integration::HostKind) -> &'static str {
    match host_kind {
        crate::host_integration::HostKind::Codex => "codex",
        crate::host_integration::HostKind::ClaudeCode => "claude-code",
        crate::host_integration::HostKind::Generic => "generic",
    }
}
