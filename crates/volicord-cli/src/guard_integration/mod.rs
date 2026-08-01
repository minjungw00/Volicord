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

pub(crate) use apply::apply_guard_integration;
pub(crate) use audit::HOOK_WRAPPER_MARKER;
pub(crate) use files::FilePlanStatus;
pub(crate) use manifest::{guard_installation_upsert, record_guard_installation};
pub(crate) use plan::{plan_guard_integration, GuardIntegrationPlan, GuardIntegrationPlanRequest};

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
