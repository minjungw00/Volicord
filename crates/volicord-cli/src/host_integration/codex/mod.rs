mod adapter;
mod binding;
mod config;
mod executable;
mod identity;
mod trust;

use std::path::{Path, PathBuf};

use crate::host_integration::HostCapabilities;

pub use adapter::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest};
pub(crate) use binding::{
    issue_host_verification_receipt, managed_host_evidence_for_live_process,
    CheckedInCodexReleaseCatalog, HostVerificationReceiptIssue, ManagedHostEvidence,
};
pub(crate) use identity::managed_identity_evaluation_for_plan;
pub(crate) use trust::project_trust_diagnostic;

const CODEX_TOOL_APPROVAL_OVERLAY_KIND: &str = "codex_tool_approval";

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

pub(crate) fn project_hooks_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".codex").join("hooks.json")
}

pub(crate) fn project_rule_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".codex")
        .join("rules")
        .join("volicord.rules")
}
