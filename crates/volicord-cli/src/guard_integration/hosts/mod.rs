use std::{collections::BTreeMap, path::Path};

use crate::{
    guard_integration::{
        files::GeneratedFilePlan,
        hooks::{
            plan_codex_dispatch_wrapper_file, plan_hook_wrapper_files, GuardCommandSpec,
            HostHookCommand, HostHookPurpose,
        },
        GuardIntegrationError,
    },
    host_integration::{HostKind, HostLifecyclePhase},
};

pub(crate) mod codex;

pub(crate) struct HostGeneratedFilesRequest<'a> {
    pub(crate) host_kind: HostKind,
    pub(crate) runtime_home: &'a Path,
    pub(crate) repo_root: &'a Path,
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
        runtime_home,
        repo_root,
        commands,
        host_commands,
        phases,
        purpose,
    } = request;
    let mut files = Vec::new();
    match host_kind {
        HostKind::Codex if !phases.is_empty() => {
            files.push(plan_codex_dispatch_wrapper_file(repo_root)?);
            files.extend(plan_hook_wrapper_files(
                repo_root,
                runtime_home,
                host_kind,
                commands,
                phases,
                purpose,
            )?);
            files.push(codex::plan_codex_hook_file(
                repo_root,
                host_commands,
                phases,
            )?);
            files.push(codex::plan_codex_rule_file(repo_root, host_commands)?);
        }
        HostKind::Codex => {}
    }
    Ok(files)
}
