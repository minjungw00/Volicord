use std::{collections::BTreeMap, path::Path};

use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        files::GeneratedFilePlan,
        hooks::{
            plan_codex_dispatch_wrapper_file, plan_hook_wrapper_files, GuardCommandSpec,
            HostHookCommand,
        },
        GuardIntegrationError,
    },
    host_integration::{HostKind, ManagedServerEntry, DEFAULT_SERVER_NAME},
};

pub(crate) mod claude_code;
pub(crate) mod codex;

pub(crate) fn plan_host_generated_files(
    host_kind: HostKind,
    profile: IntegrationProfile,
    repo_root: &Path,
    mcp_entry: &ManagedServerEntry,
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
    host_hook_commands: &BTreeMap<String, HostHookCommand>,
) -> Result<Vec<GeneratedFilePlan>, GuardIntegrationError> {
    let mut files = Vec::new();
    match host_kind {
        HostKind::Codex if profile == IntegrationProfile::Detective => {
            files.push(plan_codex_dispatch_wrapper_file(repo_root)?);
            files.extend(plan_hook_wrapper_files(
                repo_root,
                host_kind,
                guard_commands,
            )?);
            files.push(codex::plan_codex_hook_file(repo_root, host_hook_commands)?);
            files.push(codex::plan_codex_rule_file(repo_root, host_hook_commands)?);
        }
        HostKind::ClaudeCode => {
            files.push(claude_code::plan_claude_mcp_file(
                repo_root,
                DEFAULT_SERVER_NAME,
                mcp_entry,
            )?);
            if profile == IntegrationProfile::Detective {
                files.extend(plan_hook_wrapper_files(
                    repo_root,
                    host_kind,
                    guard_commands,
                )?);
                files.push(claude_code::plan_claude_project_settings_file(
                    repo_root,
                    host_hook_commands,
                )?);
                files.push(claude_code::plan_claude_rule_file(
                    repo_root,
                    host_hook_commands,
                )?);
            }
        }
        HostKind::Codex | HostKind::Generic => {}
    }
    Ok(files)
}
