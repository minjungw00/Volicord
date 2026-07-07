use std::{error::Error, fmt};

pub(crate) mod audit;
pub(crate) mod files;
pub(crate) mod hooks;
pub(crate) mod hosts;
pub(crate) mod plan;

#[cfg(test)]
pub(crate) use audit::CODEX_DISPATCH_WRAPPER;
pub(crate) use audit::{HookWrapperResolutionStatus, ManagedJsonProjection, HOOK_WRAPPER_MARKER};
pub(crate) use files::{
    managed_block_conflict, managed_json_projection_merge, plan_managed_exact_json_file,
    plan_managed_script_file, plan_policy_file, FilePlanStatus, GeneratedFilePlan,
    GeneratedFileWriteKind, VOLICORD_POLICY_FILE,
};
#[cfg(test)]
pub(crate) use files::{AGENTS_FILE, GUIDANCE_END_MARKER, GUIDANCE_START_MARKER};
#[cfg(test)]
pub(crate) use hooks::shell_word;
pub(crate) use hooks::{
    guard_has_prompt_capture_commands, lifecycle_phase_names,
    observe_hook_root_unsupported_message, HostHookCommand, HostHookCommandShape,
};
pub(crate) use plan::{plan_guard_integration, GuardIntegrationPlan};

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
