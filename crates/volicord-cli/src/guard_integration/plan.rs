use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};
use volicord_store::session_watch::{snapshot_product_repository, WatchSnapshotOptions};
use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        audit::policy_hash,
        files::{
            plan_managed_block_file, plan_policy_file, GeneratedFilePlan, AGENTS_FILE,
            GUIDANCE_END_MARKER, GUIDANCE_START_MARKER, VOLICORD_POLICY_FILE,
            VOLICORD_POLICY_SCHEMA,
        },
        hooks::{guard_command_specs, host_hook_command_specs, GuardCommandSpec, HostHookCommand},
        hosts::plan_host_generated_files,
        public_host_label, GuardIntegrationError,
    },
    host_integration::{
        host_capabilities, HostCapabilities, HostIntegrationFileKind, HostKind, HostLifecyclePhase,
        ManagedServerEntry,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct GuardIntegrationPlan {
    pub(crate) generated_files: Vec<GeneratedFilePlan>,
    pub(crate) host_hook_commands: Vec<HostHookCommand>,
    pub(crate) policy: Value,
    pub(crate) policy_hash: String,
    pub(crate) guard_installation_id: String,
    pub(crate) guard_profile: String,
    pub(crate) managed_source: String,
    pub(crate) managed_bundle_hash: Option<String>,
    pub(crate) managed_verification_status: String,
    pub(crate) native_host_output_adapter: String,
    pub(crate) native_host_output_adapter_verified: bool,
    pub(crate) bash_shell_mutation_coverage: bool,
    pub(crate) direct_file_write_matcher_coverage: bool,
    pub(crate) capabilities: HostCapabilities,
    pub(crate) missing_required_hooks: Vec<HostLifecyclePhase>,
}

pub(crate) fn plan_guard_integration(
    host_kind: HostKind,
    profile: IntegrationProfile,
    runtime_home: &Path,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    if profile != IntegrationProfile::Record {
        ensure_observe_profile_supported_on_platform(host_kind)?;
    }
    let capabilities = host_capabilities(host_kind);
    let missing_required_hooks = if profile == IntegrationProfile::Record {
        Vec::new()
    } else {
        capabilities.missing_required_hook_phases()
    };
    if profile != IntegrationProfile::Record && !missing_required_hooks.is_empty() {
        return Err(GuardIntegrationError::runtime(
            observe_hooks_unsupported_message(host_kind, &missing_required_hooks),
        ));
    }
    if profile != IntegrationProfile::Record {
        ensure_observe_session_watcher_supported(runtime_home, repo_root, host_kind)?;
    }
    let policy_guard_commands = guard_command_specs(
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        profile,
        None,
    );
    let policy = policy_json(
        host_kind,
        profile,
        repo_root,
        connection_id,
        guard_installation_id,
        mcp_entry,
        &policy_guard_commands,
    );
    let policy_hash =
        policy_hash(&policy).map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let guard_commands = guard_command_specs(
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        profile,
        Some(&policy_hash),
    );
    let host_hook_commands = if profile != IntegrationProfile::Record
        && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
    {
        host_hook_command_specs(host_kind, repo_root)?
    } else {
        BTreeMap::new()
    };
    let mut generated_files = Vec::new();
    let agents_path = repo_root.join(AGENTS_FILE);
    generated_files.push(plan_managed_block_file(
        HostIntegrationFileKind::AgentsManagedBlock,
        &agents_path,
        &agents_guidance_block(),
        GUIDANCE_START_MARKER,
        GUIDANCE_END_MARKER,
        false,
    )?);
    let policy_path = repo_root.join(VOLICORD_POLICY_FILE);
    generated_files.push(plan_policy_file(&policy_path, &policy)?);
    generated_files.extend(plan_host_generated_files(
        host_kind,
        profile,
        repo_root,
        mcp_entry,
        &guard_commands,
        &host_hook_commands,
    )?);
    let managed_status = managed_status_for_profile(profile);
    Ok(GuardIntegrationPlan {
        generated_files,
        host_hook_commands: host_hook_commands.into_values().collect(),
        policy,
        policy_hash,
        guard_installation_id: guard_installation_id.to_owned(),
        guard_profile: profile.as_str().to_owned(),
        managed_source: managed_source_for_profile(profile).to_owned(),
        managed_bundle_hash: None,
        managed_verification_status: managed_status.to_owned(),
        native_host_output_adapter: native_host_output_adapter(host_kind, profile).to_owned(),
        native_host_output_adapter_verified: native_host_output_adapter_verified(
            host_kind, profile,
        ),
        bash_shell_mutation_coverage: bash_shell_mutation_coverage(host_kind, profile),
        direct_file_write_matcher_coverage: direct_file_write_matcher_coverage(host_kind, profile),
        capabilities,
        missing_required_hooks,
    })
}

#[cfg(not(windows))]
fn ensure_observe_profile_supported_on_platform(
    _host_kind: HostKind,
) -> Result<(), GuardIntegrationError> {
    Ok(())
}

#[cfg(windows)]
fn ensure_observe_profile_supported_on_platform(
    host_kind: HostKind,
) -> Result<(), GuardIntegrationError> {
    Err(GuardIntegrationError::runtime(format!(
        "DETECTIVE_WINDOWS_UNSUPPORTED: native Windows supports the record profile for {}, but detective profile is not supported because Windows host-hook wrappers and session watcher behavior are not implemented and tested. Use --profile record on native Windows, or run Volicord in WSL2, Linux, or macOS where the selected host hook contract is supported.",
        public_host_label(host_kind)
    )))
}

fn managed_source_for_profile(profile: IntegrationProfile) -> &'static str {
    match profile {
        IntegrationProfile::Record => "not_applicable",
        IntegrationProfile::Detective => "host_hooks",
    }
}

fn managed_status_for_profile(profile: IntegrationProfile) -> &'static str {
    match profile {
        IntegrationProfile::Record | IntegrationProfile::Detective => "not_applicable",
    }
}

fn native_host_output_adapter(host_kind: HostKind, profile: IntegrationProfile) -> &'static str {
    match (host_kind, profile) {
        (HostKind::Codex, IntegrationProfile::Detective) => "codex",
        (HostKind::ClaudeCode, IntegrationProfile::Detective) => "claude-code",
        _ => "none",
    }
}

fn native_host_output_adapter_verified(host_kind: HostKind, profile: IntegrationProfile) -> bool {
    native_host_output_adapter(host_kind, profile) != "none"
}

fn bash_shell_mutation_coverage(host_kind: HostKind, profile: IntegrationProfile) -> bool {
    matches!(profile, IntegrationProfile::Detective)
        && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
}

fn direct_file_write_matcher_coverage(host_kind: HostKind, profile: IntegrationProfile) -> bool {
    matches!(profile, IntegrationProfile::Detective)
        && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
}

fn observe_hooks_unsupported_message(
    host_kind: HostKind,
    missing_required_hooks: &[HostLifecyclePhase],
) -> String {
    format!(
        "DETECTIVE_HOOKS_UNSUPPORTED: {} detective init requires supported host lifecycle hook configuration, but this adapter does not know verified project-local hook support for: {}. AGENTS.md and {VOLICORD_POLICY_FILE} are not host hook configuration. Use --profile record for record-only setup, or prepare a supported host, platform, and configuration for detective before rerunning init.",
        public_host_label(host_kind),
        super::lifecycle_phase_names(missing_required_hooks).join(", ")
    )
}

fn ensure_observe_session_watcher_supported(
    runtime_home: &Path,
    repo_root: &Path,
    host_kind: HostKind,
) -> Result<(), GuardIntegrationError> {
    snapshot_product_repository(runtime_home, repo_root, WatchSnapshotOptions::default()).map_err(
        |error| {
            GuardIntegrationError::runtime(format!(
                "DETECTIVE_WATCHER_UNSUPPORTED: {} detective init requires session watcher support for the selected Product Repository, but the watcher snapshot check failed: {error}. Use --profile record for record-only setup, or prepare a supported host, platform, and repository configuration for detective before rerunning init.",
                public_host_label(host_kind)
            ))
        },
    )?;
    Ok(())
}

fn agents_guidance_block() -> String {
    format!(
        "{GUIDANCE_START_MARKER}\n# Volicord\n\n- Check Volicord status before planning: `volicord.status`.\n- Start a task before planning implementation: `volicord.intake`.\n- Prepare write before product-file changes: `volicord.prepare_write`.\n- Request user judgment through Volicord: `volicord.request_user_judgment`; the user records decisions through the `User Channel`.\n- Check close before claiming completion: `volicord.check_close`.\n- If Volicord tools are unavailable, say so explicitly and do not imply Volicord state was updated.\n{GUIDANCE_END_MARKER}\n"
    )
}

fn policy_json(
    host_kind: HostKind,
    profile: IntegrationProfile,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
) -> Value {
    let commands = guard_commands
        .iter()
        .map(|(phase, spec)| {
            (
                phase.clone(),
                json!({
                    "command": &spec.command,
                    "args": &spec.args,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "schema": VOLICORD_POLICY_SCHEMA,
        "managed_by": "volicord",
        "host": public_host_label(host_kind),
        "repo_root": path_text(repo_root),
        "connection_id": connection_id,
        "guard_installation_id": guard_installation_id,
        "selected_profile": profile.as_str(),
        "mcp": {
            "command": &mcp_entry.command,
            "args": &mcp_entry.args,
            "env": &mcp_entry.env,
        },
        "host_hook": {
            "enabled": profile != IntegrationProfile::Record,
            "commands": commands,
        },
    })
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
