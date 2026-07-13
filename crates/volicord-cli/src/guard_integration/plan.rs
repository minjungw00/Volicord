use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::Value;
use volicord_store::guards::guard_installation;
use volicord_store::session_watch::{snapshot_product_repository, WatchSnapshotOptions};
use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        audit::policy_hash,
        files::{
            plan_managed_block_file, plan_managed_file_retirement, plan_policy_file,
            GeneratedFilePlan, ManagedFileRetirementPlan, AGENTS_FILE, GUIDANCE_END_MARKER,
            GUIDANCE_START_MARKER, VOLICORD_POLICY_FILE,
        },
        git_exclude::{plan_git_excludes, plan_git_excludes_with_personal_protection},
        hooks::{
            codex_hook_root_available, final_output_command_specs, guard_command_specs,
            host_hook_command_specs, HostHookCommand, HostHookPurpose,
        },
        hosts::{plan_host_generated_files, HostGeneratedFilesRequest},
        policy::{
            lifecycle_phase_names, policy_json, recorded_local_policy, LocalPolicyContext,
            RecordedLocalPolicy,
        },
        public_host_label, GuardIntegrationError,
    },
    host_integration::{
        host_capabilities, ConnectionIntent, HostCapabilities, HostIntegrationFileKind, HostKind,
        HostLifecyclePhase, ManagedServerEntry, FINAL_OUTPUT_PHASES, REQUIRED_GUARD_PHASES,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct GuardIntegrationPlan {
    pub(crate) repo_root: PathBuf,
    pub(crate) prior_connection_id: Option<String>,
    pub(crate) migration_required: bool,
    pub(crate) generated_files: Vec<GeneratedFilePlan>,
    pub(crate) retired_files: Vec<ManagedFileRetirementPlan>,
    pub(crate) migration_protection: Option<GeneratedFilePlan>,
    pub(crate) migration_protection_applied: bool,
    pub(crate) host_hook_commands: Vec<HostHookCommand>,
    pub(crate) policy: Value,
    pub(crate) policy_hash: String,
    pub(crate) guard_installation_id: String,
    pub(crate) guard_profile: String,
    pub(crate) connection_intent: String,
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

pub(crate) struct GuardIntegrationPlanRequest<'a> {
    pub(crate) host_kind: HostKind,
    pub(crate) profile: IntegrationProfile,
    pub(crate) runtime_home: &'a Path,
    pub(crate) repo_root: &'a Path,
    pub(crate) connection_id: &'a str,
    pub(crate) guard_installation_id: &'a str,
    pub(crate) mcp_entry: &'a ManagedServerEntry,
    pub(crate) connection_intent: ConnectionIntent,
}

pub(crate) fn plan_guard_integration(
    request: GuardIntegrationPlanRequest<'_>,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    let GuardIntegrationPlanRequest {
        host_kind,
        profile,
        runtime_home,
        repo_root,
        connection_id,
        guard_installation_id,
        mcp_entry,
        connection_intent,
    } = request;
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
        LocalPolicyContext {
            repo_root,
            connection_id,
            guard_installation_id,
            connection_intent,
        },
        mcp_entry,
        &policy_guard_commands,
    )?;
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
    let final_output_supported = managed_final_output_supported(host_kind, repo_root)?;
    let (generated_host_commands, host_hook_commands, generated_phases, command_purpose) =
        if profile == IntegrationProfile::Record && final_output_supported {
            (
                final_output_command_specs(
                    repo_root,
                    connection_id,
                    guard_installation_id,
                    host_kind,
                    profile,
                    &policy_hash,
                ),
                host_hook_command_specs(
                    host_kind,
                    repo_root,
                    &FINAL_OUTPUT_PHASES,
                    HostHookPurpose::FinalOutputAuthorityDisclosure,
                )?,
                FINAL_OUTPUT_PHASES.as_slice(),
                HostHookPurpose::FinalOutputAuthorityDisclosure,
            )
        } else if profile == IntegrationProfile::Detective
            && matches!(host_kind, HostKind::Codex | HostKind::ClaudeCode)
        {
            (
                guard_commands.clone(),
                host_hook_command_specs(
                    host_kind,
                    repo_root,
                    &REQUIRED_GUARD_PHASES,
                    HostHookPurpose::DetectiveGuard,
                )?,
                REQUIRED_GUARD_PHASES.as_slice(),
                HostHookPurpose::DetectiveGuard,
            )
        } else {
            (
                BTreeMap::new(),
                BTreeMap::new(),
                &[][..],
                HostHookPurpose::FinalOutputAuthorityDisclosure,
            )
        };
    let prior_policy = recorded_local_policy(repo_root)?;
    let git_exclude_plan = plan_git_excludes(repo_root, connection_intent, profile)?;
    let mut generated_files = Vec::new();
    if let Some(git_exclude_plan) = git_exclude_plan {
        generated_files.push(git_exclude_plan);
    }
    let agents_path = repo_root.join(AGENTS_FILE);
    generated_files.push(plan_managed_block_file(
        HostIntegrationFileKind::AgentsManagedBlock,
        repo_root,
        &agents_path,
        &agents_guidance_block(),
        GUIDANCE_START_MARKER,
        GUIDANCE_END_MARKER,
        false,
    )?);
    let policy_path = repo_root.join(VOLICORD_POLICY_FILE);
    generated_files.push(plan_policy_file(repo_root, &policy_path, &policy)?);
    generated_files.extend(plan_host_generated_files(HostGeneratedFilesRequest {
        host_kind,
        profile,
        connection_intent,
        repo_root,
        mcp_entry,
        commands: &generated_host_commands,
        host_commands: &host_hook_commands,
        phases: generated_phases,
        purpose: command_purpose,
    })?);
    let retired_files = plan_retired_files(
        runtime_home,
        repo_root,
        host_kind,
        profile,
        connection_intent,
        prior_policy.as_ref(),
        &generated_files,
    )?;
    let retain_personal_paths = connection_intent == ConnectionIntent::Personal
        || prior_policy.as_ref().is_some_and(|prior| {
            prior.connection_intent == ConnectionIntent::Personal
                && (prior.host != public_host_label(host_kind)
                    || prior.connection_intent != connection_intent
                    || prior.selected_profile != profile)
        });
    let migration_required = prior_policy.as_ref().is_some_and(|prior| {
        prior.host != public_host_label(host_kind)
            || prior.connection_intent != connection_intent
            || prior.selected_profile != profile
    });
    let migration_protection = plan_git_excludes_with_personal_protection(
        repo_root,
        connection_intent,
        profile,
        retain_personal_paths,
    )?;
    let managed_status = managed_status_for_profile(profile);
    Ok(GuardIntegrationPlan {
        repo_root: repo_root.to_path_buf(),
        prior_connection_id: prior_policy.map(|prior| prior.connection_id),
        migration_required,
        generated_files,
        retired_files,
        migration_protection,
        migration_protection_applied: false,
        host_hook_commands: host_hook_commands.into_values().collect(),
        policy,
        policy_hash,
        guard_installation_id: guard_installation_id.to_owned(),
        guard_profile: profile.as_str().to_owned(),
        connection_intent: connection_intent.as_str().to_owned(),
        managed_source: managed_source_for_profile(profile).to_owned(),
        managed_bundle_hash: None,
        managed_verification_status: managed_status.to_owned(),
        native_host_output_adapter: native_host_output_adapter(host_kind, final_output_supported)
            .to_owned(),
        native_host_output_adapter_verified: native_host_output_adapter_verified(
            host_kind,
            final_output_supported,
        ),
        bash_shell_mutation_coverage: bash_shell_mutation_coverage(host_kind, profile),
        direct_file_write_matcher_coverage: direct_file_write_matcher_coverage(host_kind, profile),
        capabilities,
        missing_required_hooks,
    })
}

fn plan_retired_files(
    runtime_home: &Path,
    repo_root: &Path,
    host_kind: HostKind,
    profile: IntegrationProfile,
    connection_intent: ConnectionIntent,
    prior_policy: Option<&RecordedLocalPolicy>,
    generated_files: &[GeneratedFilePlan],
) -> Result<Vec<ManagedFileRetirementPlan>, GuardIntegrationError> {
    let Some(prior) = prior_policy else {
        return Ok(Vec::new());
    };
    let installation = guard_installation(runtime_home, &prior.guard_installation_id)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let Some(installation) = installation else {
        if prior.host == public_host_label(host_kind)
            && prior.connection_intent == connection_intent
            && prior.selected_profile == profile
        {
            return Ok(Vec::new());
        }
        return Err(GuardIntegrationError::runtime(format!(
            "INTEGRATION_MIGRATION_INVENTORY_MISSING: prior managed integration {} has no ownership inventory; restore or remove it explicitly before changing the managed file set",
            prior.guard_installation_id
        )));
    };
    let capability =
        serde_json::from_str::<Value>(&installation.host_capability_json).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "prior integration ownership inventory is invalid: {error}"
            ))
        })?;
    plan_retired_files_from_capability(repo_root, &capability, generated_files)
}

fn plan_retired_files_from_capability(
    repo_root: &Path,
    capability: &Value,
    generated_files: &[GeneratedFilePlan],
) -> Result<Vec<ManagedFileRetirementPlan>, GuardIntegrationError> {
    let files = capability
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            GuardIntegrationError::runtime("prior integration ownership inventory has no files")
        })?;
    let current_paths = generated_files
        .iter()
        .map(|file| file.path.as_path())
        .collect::<std::collections::BTreeSet<_>>();
    let mut retired = Vec::new();
    for file in files {
        let kind = file.get("kind").and_then(Value::as_str).unwrap_or_default();
        if matches!(
            kind,
            "volicord_policy" | "git_info_exclude" | "agents_managed_block" | "host_mcp_config"
        ) {
            continue;
        }
        let Some(path) = file.get("path").and_then(Value::as_str).map(Path::new) else {
            return Err(GuardIntegrationError::runtime(
                "prior integration ownership inventory contains a file without a path",
            ));
        };
        if current_paths.contains(path) {
            continue;
        }
        retired.push(plan_managed_file_retirement(repo_root, file)?);
    }
    Ok(retired)
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

fn native_host_output_adapter(host_kind: HostKind, final_output_supported: bool) -> &'static str {
    if !final_output_supported {
        return "none";
    }
    match host_kind {
        HostKind::Codex => "codex",
        HostKind::ClaudeCode => "claude-code",
        _ => "none",
    }
}

fn native_host_output_adapter_verified(host_kind: HostKind, final_output_supported: bool) -> bool {
    native_host_output_adapter(host_kind, final_output_supported) != "none"
}

fn managed_final_output_supported(
    host_kind: HostKind,
    repo_root: &Path,
) -> Result<bool, GuardIntegrationError> {
    if cfg!(windows) {
        return Ok(false);
    }
    match host_kind {
        HostKind::Codex => codex_hook_root_available(repo_root),
        HostKind::ClaudeCode => Ok(true),
        HostKind::Generic => Ok(false),
    }
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
        lifecycle_phase_names(missing_required_hooks).join(", ")
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
        "{GUIDANCE_START_MARKER}\n# Volicord\n\n- Check Volicord status before planning: `volicord.status`.\n- Start a task before planning implementation: `volicord.intake`.\n- Prepare write before product-file changes: `volicord.prepare_write`.\n- Request a user action through Volicord: `volicord.request_user_action`; the user resolves it through the `User Channel`.\n- Check close before claiming completion: `volicord.check_close`.\n- If Volicord tools are unavailable, say so explicitly and do not imply Volicord state was updated.\n{GUIDANCE_END_MARKER}\n"
    )
}

#[cfg(all(test, unix))]
mod tests {
    use std::{collections::BTreeSet, fs};

    use super::*;
    use crate::guard_integration::{
        apply::apply_guard_integration,
        capability::host_hook_capability_json,
        files::{apply_managed_file_retirement, RetirementPlanStatus},
    };
    use volicord_test_support::TempRuntimeHome;

    #[test]
    fn codex_record_capability_transition_retires_git_only_final_output_files(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("codex-record-capability-transition")?;
        let repo_root = fixture.create_product_repo("product-repo")?;
        let runtime_home = fixture.path().to_path_buf();
        fs::create_dir_all(repo_root.join(".git"))?;
        let mcp_entry = ManagedServerEntry::new("conn_record", Path::new("volicord"), None);
        let request = || GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            runtime_home: &runtime_home,
            repo_root: &repo_root,
            connection_id: "conn_record",
            guard_installation_id: "guard_record",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        };

        let installed = apply_guard_integration(plan_guard_integration(request())?)?;
        assert_eq!(installed.native_host_output_adapter, "codex");
        let capability: Value = serde_json::from_str(&host_hook_capability_json(&installed)?)?;
        let hooks_path = repo_root.join(".codex/hooks.json");
        let stop_wrapper_path = repo_root.join(".codex/hooks/volicord-stop.sh");
        assert!(hooks_path.exists());
        assert!(stop_wrapper_path.exists());

        fs::rename(repo_root.join(".git"), repo_root.join(".git.removed"))?;
        let policy_path = repo_root.join(VOLICORD_POLICY_FILE);
        let policy_text = fs::read_to_string(&policy_path)?;
        fs::remove_file(&policy_path)?;
        let non_git = plan_guard_integration(request())?;
        fs::write(&policy_path, policy_text)?;
        assert_eq!(non_git.native_host_output_adapter, "none");

        let mut retired =
            plan_retired_files_from_capability(&repo_root, &capability, &non_git.generated_files)?;
        let retired_paths = retired
            .iter()
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            retired_paths,
            BTreeSet::from([hooks_path.clone(), stop_wrapper_path.clone()])
        );
        for file in &mut retired {
            file.status = apply_managed_file_retirement(file)?;
            assert_eq!(file.status, RetirementPlanStatus::Removed);
        }
        assert!(!hooks_path.exists());
        assert!(!stop_wrapper_path.exists());
        assert!(repo_root.join(AGENTS_FILE).exists());
        assert!(policy_path.exists());
        Ok(())
    }
}
