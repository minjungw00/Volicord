use std::{error::Error, fmt};

pub(crate) mod apply;
pub(crate) mod audit;
pub(crate) mod files;
pub(crate) mod git_exclude;
pub(crate) mod hooks;
pub(crate) mod hosts;
pub(crate) mod manifest;
pub(crate) mod plan;
pub(crate) mod policy;

pub(crate) use apply::{apply_guard_integration, apply_guard_migration_protection};
pub(crate) use audit::{HookWrapperResolutionStatus, HOOK_WRAPPER_MARKER};
pub(crate) use files::{FilePlanStatus, GeneratedFilePlan};
pub(crate) use hooks::{HostHookCommand, HostHookCommandShape};
pub(crate) use manifest::{
    generated_files_json, guard_installation_upsert, hook_root_resolution_json,
    host_hook_commands_json, record_guard_installation, retired_files_json,
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
    }
}
