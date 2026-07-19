mod adapter;
mod config;
mod executable;
mod identity;
mod trust;

use crate::host_integration::HostCapabilities;

pub use adapter::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest};
pub(crate) use identity::managed_identity_evaluation_for_plan;
pub(crate) use trust::project_trust_diagnostic;

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        pre_tool_hook: true,
        post_tool_hook: true,
        user_prompt_submit_hook: true,
        rule_file_support: true,
        project_local_configuration: true,
    }
}
