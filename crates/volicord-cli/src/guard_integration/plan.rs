use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::Value;
use volicord_store::agent_connections::agent_connection_record_read_only;
use volicord_store::guards::guard_installation;
use volicord_store::session_watch::{snapshot_product_repository, WatchSnapshotOptions};
use volicord_store::{
    bootstrap::{project_record_by_repo_root_read_only, project_record_read_only},
    core_pipeline::CoreProjectStore,
};
use volicord_types::{
    host_hook_capability_matches_owner_binding, HostHookCapabilityOwnerBinding, IntegrationProfile,
    ProjectId,
};

use crate::{
    guard_integration::{
        audit::policy_hash,
        files::{
            plan_managed_block_file, plan_managed_file_retirement, plan_policy_file,
            GeneratedFilePlan, ManagedFileRetirementPlan, AGENTS_FILE, GUIDANCE_END_MARKER,
            GUIDANCE_START_MARKER, VOLICORD_POLICY_FILE,
        },
        git_exclude::{
            git_exclude_path, plan_git_excludes, plan_git_excludes_with_personal_protection,
        },
        hooks::{
            codex_hook_root_available, final_output_command_specs, guard_command_specs,
            host_hook_command_specs, HostHookCommand, HostHookPurpose,
        },
        hosts::{plan_host_generated_files, HostGeneratedFilesRequest},
        policy::{
            lifecycle_phase_names, policy_json, recorded_local_policy, validate_policy_v2,
            LocalPolicyContext, RecordedLocalPolicy,
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
    pub(crate) final_output_authority_disclosure_implementation_available: bool,
    pub(crate) native_host_output_adapter: String,
    pub(crate) native_host_output_adapter_config_verified: bool,
    pub(crate) bash_shell_mutation_coverage: bool,
    pub(crate) direct_file_write_matcher_coverage: bool,
    pub(crate) capabilities: HostCapabilities,
    pub(crate) missing_required_hooks: Vec<HostLifecyclePhase>,
}

pub(crate) struct GuardIntegrationPlanRequest<'a> {
    pub(crate) host_kind: HostKind,
    pub(crate) profile: IntegrationProfile,
    pub(crate) runtime_home: &'a Path,
    pub(crate) volicord_command: &'a Path,
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
        volicord_command,
        repo_root,
        connection_id,
        guard_installation_id,
        mcp_entry,
        connection_intent,
    } = request;
    for (label, path) in [
        ("Runtime Home", runtime_home),
        ("installation profile volicord_command", volicord_command),
    ] {
        if !path.is_absolute() {
            return Err(GuardIntegrationError::runtime(format!(
                "MANAGED_PROCESS_BINDING_INVALID: {label} must be an absolute path before managed host wrappers are generated: {}",
                path.display()
            )));
        }
    }
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
        volicord_command,
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        profile,
        None,
    );
    let mut policy = policy_json(
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
    preserve_authoritative_workflow_policy(runtime_home, repo_root, &mut policy)?;
    let policy_hash =
        policy_hash(&policy).map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let guard_commands = guard_command_specs(
        volicord_command,
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        profile,
        Some(&policy_hash),
    );
    let final_output_implementation_available =
        managed_final_output_implementation_available(host_kind, repo_root)?;
    let (generated_host_commands, host_hook_commands, generated_phases, command_purpose) =
        if profile == IntegrationProfile::Record && final_output_implementation_available {
            (
                final_output_command_specs(
                    volicord_command,
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
        runtime_home,
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
        final_output_authority_disclosure_implementation_available:
            final_output_implementation_available,
        native_host_output_adapter: native_host_output_adapter(
            host_kind,
            final_output_implementation_available,
        )
        .to_owned(),
        native_host_output_adapter_config_verified: native_host_output_adapter_config_verified(
            host_kind,
            final_output_implementation_available,
        ),
        bash_shell_mutation_coverage: bash_shell_mutation_coverage(host_kind, profile),
        direct_file_write_matcher_coverage: direct_file_write_matcher_coverage(host_kind, profile),
        capabilities,
        missing_required_hooks,
    })
}

fn preserve_authoritative_workflow_policy(
    runtime_home: &Path,
    repo_root: &Path,
    generated_policy: &mut Value,
) -> Result<(), GuardIntegrationError> {
    let Some(project) = project_record_by_repo_root_read_only(runtime_home, repo_root)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?
    else {
        return Ok(());
    };
    let store =
        CoreProjectStore::open_read_only(runtime_home, &ProjectId::new(project.project_id.clone()))
            .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let Some(authority) = store
        .project_workflow_policy()
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?
    else {
        return Ok(());
    };
    let authority_value = serde_json::from_str::<Value>(&authority.policy_json).map_err(|_| {
        GuardIntegrationError::runtime(
            "authoritative project workflow policy is malformed; repair project policy before rerunning init",
        )
    })?;
    validate_policy_v2(&authority_value, None).map_err(|_| {
        GuardIntegrationError::runtime(
            "authoritative project workflow policy has an invalid v2 shape; repair project policy before rerunning init",
        )
    })?;
    generated_policy["workflow"] = authority_value["workflow"].clone();
    Ok(())
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
    let prior_identity_matches = prior.host == public_host_label(host_kind)
        && prior.connection_intent == connection_intent
        && prior.selected_profile == profile;
    let connection =
        agent_connection_record_read_only(runtime_home, &installation.connection_internal_id)
            .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let owning_project = match (
        installation.project_internal_id.as_deref(),
        installation.project_id.as_deref(),
    ) {
        (Some(project_internal_id), Some(project_id)) if project_internal_id == project_id => {
            project_record_read_only(runtime_home, project_internal_id)
                .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?
                .filter(|project| project.project_internal_id == project_internal_id)
        }
        _ => None,
    };
    let owning_git_info_exclude_path = owning_project
        .as_ref()
        .map(|project| git_exclude_path(&project.repo_root))
        .transpose()?
        .flatten();
    let prior_binding = connection.as_ref().zip(owning_project.as_ref()).and_then(
        |(connection, owning_project)| {
            let row_public_host = match installation.host_kind.as_str() {
                "claude_code" => "claude-code",
                value => value,
            };
            (prior.host == row_public_host
                && prior.repo_root == repo_root
                && owning_project.repo_root == repo_root
                && prior.connection_id == installation.connection_internal_id
                && prior.selected_profile.as_str() == installation.guard_mode
                && prior.connection_intent.as_str() == connection.intent)
                .then_some(PriorCapabilityBinding {
                    row_host_kind: &installation.host_kind,
                    row_guard_mode: &installation.guard_mode,
                    row_guard_installation_id: &installation.guard_installation_id,
                    connection_internal_id: &connection.connection_internal_id,
                    connection_host_kind: &connection.host_kind,
                    connection_intent: &connection.intent,
                    project_repo_root: &owning_project.repo_root,
                    project_git_info_exclude_path: owning_git_info_exclude_path.as_deref(),
                })
        },
    );
    let Some(capability) = prior_capability_for_retirement(
        &installation.host_capability_json,
        prior_identity_matches,
        prior_binding,
    )?
    else {
        return Ok(Vec::new());
    };
    plan_retired_files_from_capability(repo_root, &capability, generated_files)
}

fn prior_capability_for_retirement(
    capability_json: &str,
    prior_identity_matches: bool,
    binding: Option<PriorCapabilityBinding<'_>>,
) -> Result<Option<Value>, GuardIntegrationError> {
    let capability = serde_json::from_str::<Value>(capability_json).ok();
    if capability
        .as_ref()
        .zip(binding)
        .is_some_and(|(capability, binding)| {
            host_hook_capability_matches_owner_binding(
                capability,
                HostHookCapabilityOwnerBinding {
                    row_host_kind: binding.row_host_kind,
                    row_guard_mode: binding.row_guard_mode,
                    row_guard_installation_id: binding.row_guard_installation_id,
                    connection_internal_id: binding.connection_internal_id,
                    connection_host_kind: binding.connection_host_kind,
                    connection_intent: binding.connection_intent,
                    project_repo_root: Some(binding.project_repo_root),
                    project_git_info_exclude_path: binding.project_git_info_exclude_path,
                },
            )
        })
    {
        return Ok(capability);
    }
    if prior_identity_matches {
        return Ok(None);
    }
    Err(GuardIntegrationError::runtime(
        "INTEGRATION_MIGRATION_INVENTORY_INVALID: prior managed integration ownership inventory is not exact v2 input; rerun the same host, intent, and profile to repair it before migration",
    ))
}

#[derive(Clone, Copy)]
struct PriorCapabilityBinding<'a> {
    row_host_kind: &'a str,
    row_guard_mode: &'a str,
    row_guard_installation_id: &'a str,
    connection_internal_id: &'a str,
    connection_host_kind: &'a str,
    connection_intent: &'a str,
    project_repo_root: &'a Path,
    project_git_info_exclude_path: Option<&'a Path>,
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
        "DETECTIVE_WINDOWS_UNSUPPORTED: native Windows accepts only the record setup path for {}, while detective setup is unavailable because Windows host-hook wrappers and session watcher behavior are not implemented and tested. Use --profile record on native Windows, or run Volicord in WSL2, Linux, or macOS where every selected host-hook and watcher prerequisite is implemented and repository-tested.",
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

fn native_host_output_adapter(
    host_kind: HostKind,
    final_output_implementation_available: bool,
) -> &'static str {
    if !final_output_implementation_available {
        return "none";
    }
    match host_kind {
        HostKind::Codex => "codex",
        HostKind::ClaudeCode => "claude-code",
        _ => "none",
    }
}

fn native_host_output_adapter_config_verified(
    host_kind: HostKind,
    final_output_implementation_available: bool,
) -> bool {
    native_host_output_adapter(host_kind, final_output_implementation_available) != "none"
}

fn managed_final_output_implementation_available(
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
        "DETECTIVE_HOOKS_UNSUPPORTED: {} detective init requires configured host lifecycle hooks, but this adapter does not define project-local hook configuration for: {}. AGENTS.md and {VOLICORD_POLICY_FILE} are not host hook configuration. Use --profile record for record-only setup, or prepare a host, platform, and configuration that meet every Detective prerequisite before rerunning init.",
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
                "DETECTIVE_WATCHER_UNSUPPORTED: {} detective init requires a session watcher for the selected Product Repository, but the watcher snapshot check failed: {error}. Use --profile record for record-only setup, or prepare a host, platform, and repository configuration that meet every Detective prerequisite before rerunning init.",
                public_host_label(host_kind)
            ))
        },
    )?;
    Ok(())
}

fn agents_guidance_block() -> String {
    format!(
        "{GUIDANCE_START_MARKER}\n# Volicord\n\n- Treat Volicord's recorded scope and user-owned decisions as authoritative.\n- Do not modify Product Repository files outside an active compatible write authorization.\n- Do not infer, resolve, or record user-owned judgments on the user's behalf.\n- Follow the `next_action` returned by Volicord instead of calling workflow tools speculatively.\n- Call `volicord.status` only when the current Task state is unknown or an authoritative refresh is required.\n- Do not claim completion while Volicord reports close blockers. If Volicord is unavailable, disclose that its state was not updated or verified.\n{GUIDANCE_END_MARKER}\n"
    )
}

#[cfg(all(test, unix))]
mod tests {
    use std::{collections::BTreeSet, fs};

    use serde_json::json;

    use super::*;
    use crate::guard_integration::{
        apply::apply_guard_integration,
        audit::guard_file_findings,
        capability::host_hook_capability_json,
        files::{apply_managed_file_retirement, RetirementPlanStatus},
    };
    use volicord_store::bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };
    use volicord_store::{
        agent_connections::{
            add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
            ConnectionProjectRegistration, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX,
            HOST_SCOPE_PROJECT, VERIFIED_STATUS_NOT_VERIFIED,
        },
        guards::{upsert_guard_installation, GuardInstallationUpsert},
        sqlite::registry_db_path,
    };
    use volicord_test_support::TempRuntimeHome;

    #[test]
    fn invalid_prior_capability_is_repairable_only_without_identity_migration() {
        let cases = [
            (
                "v1",
                r#"{"schema":"volicord-host-hook-capability-v1","files":[]}"#,
            ),
            ("missing schema", r#"{"files":[]}"#),
            (
                "unknown schema",
                r#"{"schema":"volicord-host-hook-capability-v3","files":[]}"#,
            ),
            (
                "malformed v2 shape",
                r#"{"schema":"volicord-host-hook-capability-v2","files":[]}"#,
            ),
            ("non-object", "[]"),
            ("malformed JSON", "{"),
        ];

        for (name, capability) in cases {
            assert!(
                prior_capability_for_retirement(capability, true, None)
                    .unwrap_or_else(|error| panic!("{name} same-identity repair failed: {error}"))
                    .is_none(),
                "{name} must be regenerated without decoding retirement inventory"
            );
            let error = prior_capability_for_retirement(capability, false, None)
                .expect_err("migration must require exact v2 inventory");
            assert!(
                error
                    .to_string()
                    .contains("INTEGRATION_MIGRATION_INVENTORY_INVALID"),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn cross_bound_or_noncanonical_inventory_cannot_authorize_retirement(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("cross-bound-retirement-inventory")?;
        let repo_root = fixture.create_product_repo("product-repo")?;
        let other_repo_root = fixture.create_product_repo("other-product-repo")?;
        fs::create_dir_all(repo_root.join(".git"))?;
        fs::create_dir_all(other_repo_root.join(".git"))?;
        initialize_runtime_home(fixture.path(), "runtime_cross_bound", "{}")?;
        for (project_id, registered_root) in [
            ("project_current", repo_root.as_path()),
            ("project_other", other_repo_root.as_path()),
        ] {
            register_project(
                fixture.path(),
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root: registered_root.to_path_buf(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
        }
        let mcp_entry = ManagedServerEntry::new("conn_owner", Path::new("volicord"), None);
        let plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            runtime_home: fixture.path(),
            volicord_command: Path::new("/bin/volicord"),
            repo_root: &repo_root,
            connection_id: "conn_owner",
            guard_installation_id: "guard_owner",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;
        let valid: Value = serde_json::from_str(&host_hook_capability_json(&plan)?)?;
        let git_info_exclude_path = git_exclude_path(&repo_root)?;
        let binding = PriorCapabilityBinding {
            row_host_kind: "codex",
            row_guard_mode: "record",
            row_guard_installation_id: "guard_owner",
            connection_internal_id: "conn_owner",
            connection_host_kind: "codex",
            connection_intent: "shared",
            project_repo_root: &repo_root,
            project_git_info_exclude_path: git_info_exclude_path.as_deref(),
        };
        assert!(
            prior_capability_for_retirement(&valid.to_string(), false, Some(binding))?.is_some()
        );

        let other_git_info_exclude_path = git_exclude_path(&other_repo_root)?;
        let cross_project_binding = PriorCapabilityBinding {
            project_repo_root: &other_repo_root,
            project_git_info_exclude_path: other_git_info_exclude_path.as_deref(),
            ..binding
        };
        assert!(prior_capability_for_retirement(
            &valid.to_string(),
            true,
            Some(cross_project_binding),
        )?
        .is_none());
        let error =
            prior_capability_for_retirement(&valid.to_string(), false, Some(cross_project_binding))
                .expect_err("migration must reject inventory coordinated to another project root");
        assert!(error
            .to_string()
            .contains("INTEGRATION_MIGRATION_INVENTORY_INVALID"));

        let wrapper_index = valid["files"]
            .as_array()
            .expect("files")
            .iter()
            .position(|file| file["kind"] == "host_hook_wrapper")
            .expect("wrapper inventory");
        let config_index = valid["files"]
            .as_array()
            .expect("files")
            .iter()
            .position(|file| file["kind"] == "host_hook_config")
            .expect("hook config inventory");
        let mut cases = Vec::new();

        let mut capability = valid.clone();
        capability["files"][wrapper_index]["connection_id"] = json!("conn_other");
        cases.push(("connection_id", capability));

        let mut capability = valid.clone();
        capability["files"][wrapper_index]["guard_installation_id"] = json!("guard_other");
        cases.push(("guard_installation_id", capability));

        let mut capability = valid.clone();
        capability["files"][wrapper_index]["policy_hash"] = json!("other-policy");
        cases.push(("policy_hash", capability));

        let mut capability = valid.clone();
        capability["files"][wrapper_index]["host_output"] = json!("claude-code");
        cases.push(("host_output", capability));

        let mut capability = valid.clone();
        capability["files"][wrapper_index]["host_kind"] = json!("claude-code");
        cases.push(("host_kind", capability));

        let mut capability = valid.clone();
        capability["files"][wrapper_index]["phase"] = json!("pre_tool");
        cases.push(("phase", capability));

        let arbitrary_path = repo_root.join("src/main.rs").display().to_string();
        let mut capability = valid.clone();
        capability["files"][wrapper_index]["path"] = json!(arbitrary_path);
        capability["host_hook_commands"][0]["expected_wrapper_path"] =
            capability["files"][wrapper_index]["path"].clone();
        capability["host_hook_commands"][0]["expected_phase_wrapper_path"] =
            capability["files"][wrapper_index]["path"].clone();
        cases.push(("coordinated_wrapper_path", capability));

        let mut capability = valid.clone();
        capability["files"][config_index]["path"] =
            json!(repo_root.join("src/main.rs").display().to_string());
        cases.push(("hook_config_path", capability));

        for kind in ["host_hook_wrapper", "host_hook_config"] {
            let mut capability = valid.clone();
            let files = capability["files"].as_array_mut().expect("files");
            let index = files
                .iter()
                .position(|file| file["kind"] == kind)
                .expect("required inventory kind");
            files.remove(index);
            cases.push((kind, capability));
        }

        for (name, capability) in cases {
            let encoded = capability.to_string();
            assert!(
                prior_capability_for_retirement(&encoded, true, Some(binding))?.is_none(),
                "{name} must be repaired without consuming retirement inventory"
            );
            let error = prior_capability_for_retirement(&encoded, false, Some(binding))
                .expect_err("migration must reject cross-bound retirement inventory");
            assert!(
                error
                    .to_string()
                    .contains("INTEGRATION_MIGRATION_INVENTORY_INVALID"),
                "{name}: {error}"
            );
        }
        Ok(())
    }

    #[test]
    fn stored_cross_project_inventory_cannot_authorize_migration_retirement(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("stored-cross-project-retirement")?;
        let repo_a = fixture.create_product_repo("repo-a")?;
        let repo_b = fixture.create_product_repo("repo-b")?;
        fs::create_dir_all(repo_a.join(".git"))?;
        fs::create_dir_all(repo_b.join(".git"))?;
        initialize_runtime_home(fixture.path(), "runtime_stored_cross_project", "{}")?;
        for (project_id, repo_root) in [
            ("project_a", repo_a.as_path()),
            ("project_b", repo_b.as_path()),
        ] {
            register_project(
                fixture.path(),
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root: repo_root.to_path_buf(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
        }
        ensure_agent_connection(
            fixture.path(),
            AgentConnectionRegistration {
                connection_internal_id: "conn_owner".to_owned(),
                host_kind: HOST_KIND_CODEX.to_owned(),
                intent: "shared".to_owned(),
                host_scope: HOST_SCOPE_PROJECT.to_owned(),
                server_name: "volicord".to_owned(),
                config_target: "/tmp/volicord-owner.toml".to_owned(),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: "fixture-fingerprint".to_owned(),
                last_verification_status: VERIFIED_STATUS_NOT_VERIFIED.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        for project_id in ["project_a", "project_b"] {
            add_connection_project(
                fixture.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: "conn_owner".to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
        }

        let codex_entry = ManagedServerEntry::new_repository_discovery(
            volicord_mcp::RepositoryDiscoveryHost::Codex,
        );
        let installed_a =
            apply_guard_integration(plan_guard_integration(GuardIntegrationPlanRequest {
                host_kind: HostKind::Codex,
                profile: IntegrationProfile::Record,
                runtime_home: fixture.path(),
                volicord_command: Path::new("/bin/volicord"),
                repo_root: &repo_a,
                connection_id: "conn_owner",
                guard_installation_id: "guard_owner",
                mcp_entry: &codex_entry,
                connection_intent: ConnectionIntent::Shared,
            })?)?;
        upsert_guard_installation(
            fixture.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_owner".to_owned(),
                connection_internal_id: "conn_owner".to_owned(),
                project_id: Some("project_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "record".to_owned(),
                host_capability_json: host_hook_capability_json(&installed_a)?,
                installation_status: "configured".to_owned(),
                installed_at: None,
                last_checked_at: "2026-07-14T00:00:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;

        let installed_b =
            apply_guard_integration(plan_guard_integration(GuardIntegrationPlanRequest {
                host_kind: HostKind::Codex,
                profile: IntegrationProfile::Record,
                runtime_home: fixture.path(),
                volicord_command: Path::new("/bin/volicord"),
                repo_root: &repo_b,
                connection_id: "conn_owner",
                guard_installation_id: "guard_owner",
                mcp_entry: &codex_entry,
                connection_intent: ConnectionIntent::Shared,
            })?)?;
        let coordinated_repo_b_capability = host_hook_capability_json(&installed_b)?;
        let registry = rusqlite::Connection::open(registry_db_path(fixture.path()))?;
        assert_eq!(
            registry.execute(
                "UPDATE guard_installations
                    SET host_capability_json = ?2
                  WHERE guard_installation_id = ?1",
                rusqlite::params!["guard_owner", &coordinated_repo_b_capability],
            )?,
            1
        );
        drop(registry);

        let repair = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            runtime_home: fixture.path(),
            volicord_command: Path::new("/bin/volicord"),
            repo_root: &repo_b,
            connection_id: "conn_owner",
            guard_installation_id: "guard_owner",
            mcp_entry: &codex_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;
        assert!(repair.retired_files.is_empty());

        let claude_entry = ManagedServerEntry::new_repository_discovery(
            volicord_mcp::RepositoryDiscoveryHost::ClaudeCode,
        );
        let error = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::ClaudeCode,
            profile: IntegrationProfile::Record,
            runtime_home: fixture.path(),
            volicord_command: Path::new("/bin/volicord"),
            repo_root: &repo_b,
            connection_id: "conn_claude",
            guard_installation_id: "guard_claude",
            mcp_entry: &claude_entry,
            connection_intent: ConnectionIntent::Shared,
        })
        .expect_err("cross-project coordinated inventory must not authorize migration retirement");
        assert!(error
            .to_string()
            .contains("INTEGRATION_MIGRATION_INVENTORY_INVALID"));

        upsert_guard_installation(
            fixture.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_owner".to_owned(),
                connection_internal_id: "conn_owner".to_owned(),
                project_id: Some("project_b".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "record".to_owned(),
                host_capability_json: coordinated_repo_b_capability,
                installation_status: "configured".to_owned(),
                installed_at: None,
                last_checked_at: "2026-07-14T00:00:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        let policy_path = repo_b.join(VOLICORD_POLICY_FILE);
        let baseline_policy: Value = serde_json::from_str(&fs::read_to_string(&policy_path)?)?;
        for (name, field, replacement) in [
            (
                "policy_repo_root",
                "repo_root",
                repo_a.display().to_string(),
            ),
            (
                "policy_connection_id",
                "connection_id",
                "conn_other".to_owned(),
            ),
        ] {
            let mut mismatched_policy = baseline_policy.clone();
            mismatched_policy[field] = json!(replacement);
            fs::write(
                &policy_path,
                serde_json::to_string_pretty(&mismatched_policy)? + "\n",
            )?;
            let result = plan_guard_integration(GuardIntegrationPlanRequest {
                host_kind: HostKind::ClaudeCode,
                profile: IntegrationProfile::Record,
                runtime_home: fixture.path(),
                volicord_command: Path::new("/bin/volicord"),
                repo_root: &repo_b,
                connection_id: "conn_claude",
                guard_installation_id: "guard_claude",
                mcp_entry: &claude_entry,
                connection_intent: ConnectionIntent::Shared,
            });
            let error = match result {
                Ok(_) => panic!("{name} must not authorize migration retirement"),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("INTEGRATION_MIGRATION_INVENTORY_INVALID"),
                "{name}: {error}"
            );
        }
        Ok(())
    }

    #[test]
    fn managed_wrapper_plan_rejects_relative_process_bindings(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("relative-managed-process-binding")?;
        let repo_root = fixture.create_product_repo("product-repo")?;
        let runtime_home = fixture.path().to_path_buf();
        let volicord_command = runtime_home.join("bin/volicord");
        let mcp_entry = ManagedServerEntry::new("conn_record", Path::new("volicord"), None);

        for (selected_home, selected_command, expected_label) in [
            (
                Path::new("relative-runtime-home"),
                volicord_command.as_path(),
                "Runtime Home",
            ),
            (
                runtime_home.as_path(),
                Path::new("relative-volicord"),
                "installation profile volicord_command",
            ),
        ] {
            let error = plan_guard_integration(GuardIntegrationPlanRequest {
                host_kind: HostKind::Codex,
                profile: IntegrationProfile::Record,
                runtime_home: selected_home,
                volicord_command: selected_command,
                repo_root: &repo_root,
                connection_id: "conn_record",
                guard_installation_id: "guard_record",
                mcp_entry: &mcp_entry,
                connection_intent: ConnectionIntent::Shared,
            })
            .expect_err("relative managed process binding must fail closed");

            assert!(error
                .to_string()
                .contains("MANAGED_PROCESS_BINDING_INVALID"));
            assert!(error.to_string().contains(expected_label));
        }
        Ok(())
    }

    #[test]
    fn managed_wrapper_audit_accepts_quoted_absolute_profile_command(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("quoted-managed-process-binding")?;
        let repo_root = fixture.create_product_repo("product-repo")?;
        fs::create_dir_all(repo_root.join(".git"))?;
        let runtime_home = fixture.path().join("selected home's records");
        let volicord_command = fixture
            .path()
            .join("selected build")
            .join("volicord's binary");
        let mcp_entry = ManagedServerEntry::new_repository_discovery(
            volicord_mcp::RepositoryDiscoveryHost::Codex,
        );

        let installed =
            apply_guard_integration(plan_guard_integration(GuardIntegrationPlanRequest {
                host_kind: HostKind::Codex,
                profile: IntegrationProfile::Detective,
                runtime_home: &runtime_home,
                volicord_command: &volicord_command,
                repo_root: &repo_root,
                connection_id: "conn_quoted",
                guard_installation_id: "guard_quoted",
                mcp_entry: &mcp_entry,
                connection_intent: ConnectionIntent::Shared,
            })?)?;
        let capability_json = host_hook_capability_json(&installed)?;
        let findings = guard_file_findings(&capability_json);

        assert!(
            findings.stale_files.is_empty(),
            "{:#?}",
            findings.stale_files
        );
        assert!(
            findings.broken_files.is_empty(),
            "{:#?}",
            findings.broken_files
        );
        Ok(())
    }

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
            volicord_command: Path::new("/bin/volicord"),
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
