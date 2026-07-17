use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::Value;
use volicord_store::agent_connections::agent_connection_record_read_only;
use volicord_store::guards::guard_installation;
use volicord_store::{
    bootstrap::{project_record_by_repo_root_read_only, project_record_read_only},
    core_pipeline::CoreProjectStore,
};
use volicord_types::{
    host_hook_capability_has_exact_current_shape, host_hook_capability_matches_owner_binding,
    HostHookCapabilityOwnerBinding, IntegrationProfile, ProjectId,
};

use crate::{
    guard_integration::{
        audit::policy_hash,
        capability::host_hook_capability_json,
        files::{
            plan_managed_block_file, plan_managed_file_retirement, plan_policy_file,
            GeneratedFilePlan, ManagedFileRetirementPlan, AGENTS_FILE, GUIDANCE_END_MARKER,
            GUIDANCE_START_MARKER, VOLICORD_POLICY_FILE,
        },
        git_exclude::{
            git_exclude_path, plan_git_excludes, plan_git_excludes_with_personal_protection,
        },
        hooks::{
            guard_command_specs, host_hook_command_specs, GuardCommandSpec, HostHookCommand,
            HostHookPurpose,
        },
        hosts::{plan_host_generated_files, HostGeneratedFilesRequest},
        policy::{
            policy_json, recorded_local_policy, validate_workflow_policy, LocalPolicyContext,
            RecordedLocalPolicy,
        },
        public_host_label, GuardIntegrationError,
    },
    host_integration::{
        host_capabilities, ConnectionIntent, HostCapabilities, HostIntegrationFileKind, HostKind,
        HostLifecyclePhase, ManagedServerEntry, REQUIRED_GUARD_PHASES,
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
    pub(crate) guard_commands: BTreeMap<String, GuardCommandSpec>,
    pub(crate) host_hook_commands: Vec<HostHookCommand>,
    pub(crate) policy: Value,
    pub(crate) policy_hash: String,
    pub(crate) guard_installation_id: String,
    pub(crate) guard_profile: String,
    pub(crate) connection_intent: String,
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
    let capabilities = host_capabilities(host_kind);
    let missing_required_hooks = capabilities.missing_required_hook_phases();
    if !missing_required_hooks.is_empty() {
        return Err(GuardIntegrationError::runtime(
            guard_hooks_unsupported_message(host_kind, &missing_required_hooks),
        ));
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
    let host_hook_commands = host_hook_command_specs(
        host_kind,
        repo_root,
        &REQUIRED_GUARD_PHASES,
        HostHookPurpose::Guard,
    )?;
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
        runtime_home,
        repo_root,
        commands: &guard_commands,
        host_commands: &host_hook_commands,
        phases: &REQUIRED_GUARD_PHASES,
        purpose: HostHookPurpose::Guard,
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
    validate_generated_host_hook_capability(GuardIntegrationPlan {
        repo_root: repo_root.to_path_buf(),
        prior_connection_id: prior_policy.map(|prior| prior.connection_id),
        migration_required,
        generated_files,
        retired_files,
        migration_protection,
        migration_protection_applied: false,
        guard_commands,
        host_hook_commands: host_hook_commands.into_values().collect(),
        policy,
        policy_hash,
        guard_installation_id: guard_installation_id.to_owned(),
        guard_profile: profile.as_str().to_owned(),
        connection_intent: connection_intent.as_str().to_owned(),
        direct_file_write_matcher_coverage: direct_file_write_matcher_coverage(host_kind, profile),
        capabilities,
        missing_required_hooks,
    })
}

fn validate_generated_host_hook_capability(
    plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    host_hook_capability_json(&plan)?;
    Ok(plan)
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
    validate_workflow_policy(&authority_value, None).map_err(|_| {
        GuardIntegrationError::runtime(
            "authoritative project workflow policy does not match the canonical contract; repair project policy before rerunning init",
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
            let row_public_host = installation.host_kind.as_str();
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
    let capability = serde_json::from_str::<Value>(capability_json).map_err(|_| {
        GuardIntegrationError::runtime(
            "stored host-hook capability is malformed; verify or repair the Codex Record integration",
        )
    })?;
    if !host_hook_capability_has_exact_current_shape(&capability) {
        return Err(GuardIntegrationError::runtime(
            "stored host-hook capability does not match the current exact contract; verify or repair the Codex Record integration",
        ));
    }
    if binding.is_some_and(|binding| {
        host_hook_capability_matches_owner_binding(
            &capability,
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
    }) {
        return Ok(Some(capability));
    }
    if prior_identity_matches {
        return Ok(None);
    }
    Err(GuardIntegrationError::runtime(
        "managed_integration_inventory_invalid: stored integration ownership does not match the current Codex Record connection; verify or repair it before changing ownership",
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

fn direct_file_write_matcher_coverage(host_kind: HostKind, _profile: IntegrationProfile) -> bool {
    host_kind == HostKind::Codex
}

fn guard_hooks_unsupported_message(
    host_kind: HostKind,
    missing_required_hooks: &[HostLifecyclePhase],
) -> String {
    let missing = missing_required_hooks
        .iter()
        .map(|phase| phase.capability_name())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "GUARD_HOOKS_UNSUPPORTED: {} record init requires configured host lifecycle hooks, but this adapter does not define project-local hook configuration for: {}. AGENTS.md and {VOLICORD_POLICY_FILE} are not host hook configuration.",
        public_host_label(host_kind),
        missing
    )
}

fn agents_guidance_block() -> String {
    format!(
        "{GUIDANCE_START_MARKER}\n# Volicord\n\n- Treat Volicord's recorded scope and user-owned decisions as authoritative.\n- Do not modify Product Repository files outside an active compatible write authorization.\n- Do not infer, resolve, or record user-owned judgments on the user's behalf.\n- Follow the `next_action` returned by Volicord instead of calling workflow tools speculatively.\n- Call `volicord.status` only when the current Task state is unknown or an authoritative refresh is required.\n- Do not claim completion while Volicord reports close blockers. If Volicord is unavailable, disclose that its state was not updated or verified.\n{GUIDANCE_END_MARKER}\n"
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::IntegrationProfile;

    use super::{
        plan_guard_integration, validate_generated_host_hook_capability,
        GuardIntegrationPlanRequest,
    };
    use crate::host_integration::{ConnectionIntent, HostKind, ManagedServerEntry};

    #[test]
    fn plan_finalization_rejects_a_generated_capability_with_an_invalid_exact_shape(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-plan-capability-validation")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedServerEntry::new_project_bound(
            fixture.connection_id(),
            Some(fixture.project_id()),
            &volicord_command,
        );
        let mut plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id: "guard_invalid_generated_shape",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;
        plan.guard_commands
            .get_mut("pre_tool")
            .expect("pre-tool command")
            .args[9] = "invalid_host".to_owned();

        let error = validate_generated_host_hook_capability(plan)
            .expect_err("invalid generated capability must fail plan finalization");
        assert!(error
            .to_string()
            .contains("generated host-hook capability does not match the current exact shape"));
        Ok(())
    }
}
