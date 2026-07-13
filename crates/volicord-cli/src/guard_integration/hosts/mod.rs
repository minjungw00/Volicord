use std::{collections::BTreeMap, path::Path};

use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        files::GeneratedFilePlan,
        hooks::{
            plan_codex_dispatch_wrapper_file, plan_hook_wrapper_files, GuardCommandSpec,
            HostHookCommand, HostHookPurpose,
        },
        GuardIntegrationError,
    },
    host_integration::{
        ConnectionIntent, HostKind, HostLifecyclePhase, ManagedServerEntry, DEFAULT_SERVER_NAME,
    },
};

pub(crate) mod claude_code;
pub(crate) mod codex;

pub(crate) struct HostGeneratedFilesRequest<'a> {
    pub(crate) host_kind: HostKind,
    pub(crate) profile: IntegrationProfile,
    pub(crate) connection_intent: ConnectionIntent,
    pub(crate) repo_root: &'a Path,
    pub(crate) mcp_entry: &'a ManagedServerEntry,
    pub(crate) commands: &'a BTreeMap<String, GuardCommandSpec>,
    pub(crate) host_commands: &'a BTreeMap<String, HostHookCommand>,
    pub(crate) phases: &'a [HostLifecyclePhase],
    pub(crate) purpose: HostHookPurpose,
}

pub(crate) fn plan_host_generated_files(
    request: HostGeneratedFilesRequest<'_>,
) -> Result<Vec<GeneratedFilePlan>, GuardIntegrationError> {
    let HostGeneratedFilesRequest {
        host_kind,
        profile,
        connection_intent,
        repo_root,
        mcp_entry,
        commands,
        host_commands,
        phases,
        purpose,
    } = request;
    let mut files = Vec::new();
    match host_kind {
        HostKind::Codex if !phases.is_empty() => {
            if profile == IntegrationProfile::Detective {
                files.push(plan_codex_dispatch_wrapper_file(repo_root)?);
            }
            files.extend(plan_hook_wrapper_files(
                repo_root, host_kind, commands, phases, purpose,
            )?);
            files.push(codex::plan_codex_hook_file(
                repo_root,
                host_commands,
                phases,
            )?);
            if profile == IntegrationProfile::Detective {
                files.push(codex::plan_codex_rule_file(repo_root, host_commands)?);
            }
        }
        HostKind::ClaudeCode => {
            if connection_intent == ConnectionIntent::Shared {
                files.push(claude_code::plan_claude_mcp_file(
                    repo_root,
                    DEFAULT_SERVER_NAME,
                    mcp_entry,
                )?);
            }
            if !phases.is_empty() {
                files.extend(plan_hook_wrapper_files(
                    repo_root, host_kind, commands, phases, purpose,
                )?);
                files.push(claude_code::plan_claude_project_settings_file(
                    repo_root,
                    host_commands,
                    connection_intent,
                    phases,
                )?);
            }
            if profile == IntegrationProfile::Detective {
                files.push(claude_code::plan_claude_rule_file(
                    repo_root,
                    host_commands,
                )?);
            }
        }
        HostKind::Codex | HostKind::Generic => {}
    }
    Ok(files)
}
