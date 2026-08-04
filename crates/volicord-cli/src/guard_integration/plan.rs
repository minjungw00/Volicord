use std::path::{Path, PathBuf};

use serde_json::Value;
use volicord_host_contract::McpServerKey;
use volicord_mcp::ManagedMcpLaunchSpec;
use volicord_store::agent_connections::agent_connection_record_read_only;
use volicord_store::guards::guard_installation;
use volicord_store::operational_sessions::connection_integration_revision;
use volicord_store::{
    bootstrap::{project_record_by_repo_root_read_only, project_record_read_only},
    core_pipeline::CoreProjectStore,
};
use volicord_types::guard_manifest::{
    guard_manifest_from_json, guard_manifest_matches_owner_binding, GuardCommandInvocationSet,
    GuardCommandSet, GuardManagedArtifact, GuardManifestOwnerBinding, PolicyHash,
};
use volicord_types::ids::ProjectId;
use volicord_types::integration_verification::IntegrationVerificationWorkflowState;
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::{GuardHookPhase, IntegrationProfile};

use crate::{
    guard_integration::{
        audit::policy_hash,
        files::{
            generated_file_plan_matches_artifact_spec, plan_managed_block_file,
            plan_managed_file_retirement, plan_policy_file, GeneratedFilePlan,
            ManagedFileRetirementPlan, GUIDANCE_END_MARKER, GUIDANCE_START_MARKER,
        },
        git_exclude::{
            git_exclude_path, plan_git_excludes, plan_git_excludes_with_personal_protection,
        },
        hooks::{guard_command_specs, host_hook_command_specs, HostHookPurpose},
        hosts::{plan_host_generated_files, HostGeneratedFilesRequest},
        policy::{
            policy_json, recorded_local_policy, validate_workflow_policy, LocalPolicyContext,
            RecordedLocalPolicy,
        },
        public_host_label, GuardIntegrationError,
    },
    host_integration::{
        guard_phase_capability_name, host_capabilities, ConnectionIntent, HostKind,
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
    pub(crate) policy_commands: GuardCommandSet,
    pub(crate) runtime_commands: GuardCommandSet,
    pub(crate) policy: Value,
    pub(crate) policy_hash: PolicyHash,
    pub(crate) guard_installation_id: String,
    pub(crate) guard_profile: String,
    pub(crate) connection_intent: String,
    pub(crate) required_hook_phases: Vec<GuardHookPhase>,
}

impl GuardIntegrationPlan {
    pub(crate) fn hook_definition_changed(&self) -> bool {
        self.generated_files.iter().any(|file| {
            file.artifact == GuardManagedArtifact::HostHookConfig
                && matches!(
                    file.status,
                    crate::guard_integration::FilePlanStatus::PlannedCreate
                        | crate::guard_integration::FilePlanStatus::PlannedUpdate
                )
        })
    }
}

pub(crate) struct GuardIntegrationPlanRequest<'a> {
    pub(crate) host_kind: HostKind,
    pub(crate) profile: IntegrationProfile,
    pub(crate) server_name: &'a str,
    pub(crate) runtime_home: &'a Path,
    pub(crate) volicord_command: &'a Path,
    pub(crate) repo_root: &'a Path,
    pub(crate) connection_id: &'a str,
    pub(crate) guard_installation_id: &'a str,
    pub(crate) mcp_entry: &'a ManagedMcpLaunchSpec,
    pub(crate) connection_intent: ConnectionIntent,
}

pub(crate) fn plan_guard_integration(
    request: GuardIntegrationPlanRequest<'_>,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    let GuardIntegrationPlanRequest {
        host_kind,
        profile,
        server_name,
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
    let server = McpServerKey::parse(server_name)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let policy_commands = guard_command_specs(
        volicord_command,
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        profile,
        None,
    )?;
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
        &policy_commands,
    )?;
    preserve_authoritative_workflow_policy(runtime_home, repo_root, &mut policy)?;
    let policy_hash = PolicyHash::parse(
        policy_hash(&policy).map_err(|error| GuardIntegrationError::runtime(error.to_string()))?,
    )
    .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let runtime_commands = guard_command_specs(
        volicord_command,
        repo_root,
        connection_id,
        guard_installation_id,
        host_kind,
        profile,
        Some(&policy_hash),
    )?;
    let host_hook_commands = host_hook_command_specs(
        host_kind,
        repo_root,
        &GuardHookPhase::REQUIRED,
        HostHookPurpose::Guard,
    )?;
    let prior_policy = recorded_local_policy(repo_root)?;
    let git_exclude_plan = plan_git_excludes(repo_root, connection_intent, profile)?;
    let mut generated_files = Vec::new();
    if let Some(git_exclude_plan) = git_exclude_plan {
        generated_files.push(git_exclude_plan);
    }
    let agents_path = GuardManagedArtifact::AgentsManagedBlock
        .expected_path(repo_root, None)
        .expect("the managed AGENTS block has a repository-owned path");
    generated_files.push(plan_managed_block_file(
        GuardManagedArtifact::AgentsManagedBlock,
        repo_root,
        &agents_path,
        &agents_guidance_block(),
        GUIDANCE_START_MARKER,
        GUIDANCE_END_MARKER,
        false,
    )?);
    let policy_path = GuardManagedArtifact::VolicordPolicy
        .expected_path(repo_root, None)
        .expect("the Guard policy has a repository-owned path");
    generated_files.push(plan_policy_file(repo_root, &policy_path, &policy)?);
    generated_files.extend(plan_host_generated_files(HostGeneratedFilesRequest {
        host_kind,
        runtime_home,
        repo_root,
        commands: &runtime_commands,
        host_commands: &host_hook_commands,
        phases: &GuardHookPhase::REQUIRED,
        purpose: HostHookPurpose::Guard,
        server: &server,
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
    validate_generated_guard_plan(GuardIntegrationPlan {
        repo_root: repo_root.to_path_buf(),
        prior_connection_id: prior_policy.map(|prior| prior.connection_id),
        migration_required,
        generated_files,
        retired_files,
        migration_protection,
        migration_protection_applied: false,
        policy_commands,
        runtime_commands,
        policy,
        policy_hash,
        guard_installation_id: guard_installation_id.to_owned(),
        guard_profile: profile.as_str().to_owned(),
        connection_intent: connection_intent.as_str().to_owned(),
        required_hook_phases: GuardHookPhase::REQUIRED.to_vec(),
    })
}

fn validate_generated_guard_plan(
    plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    if !plan
        .generated_files
        .iter()
        .all(generated_file_plan_matches_artifact_spec)
    {
        return Err(GuardIntegrationError::runtime(
            "generated Guard plan does not match the managed-artifact registry",
        ));
    }
    let policy = GuardCommandInvocationSet::from_policy_commands(&plan.policy_commands);
    let runtime =
        GuardCommandInvocationSet::from_runtime_commands(&plan.runtime_commands, &plan.policy_hash);
    let projection_matches = policy
        .as_ref()
        .ok()
        .zip(runtime.as_ref().ok())
        .is_some_and(|(policy, runtime)| policy.fields_match_except_policy_hash(runtime));
    if !projection_matches {
        return Err(GuardIntegrationError::runtime(
            "generated Guard plan does not preserve the policy/runtime command projection contract",
        ));
    }
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
    let authority_value = serde_json::to_value(&authority.policy).map_err(|_| {
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
    let owning_project = project_record_read_only(runtime_home, &installation.project_id)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?
        .filter(|project| project.project_internal_id == installation.project_internal_id);
    let owning_git_info_exclude_path = owning_project
        .as_ref()
        .map(|project| git_exclude_path(&project.repo_root))
        .transpose()?
        .flatten();
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        GuardIntegrationError::runtime(
            "stored Guard manifest is malformed; verify or repair the Codex Record integration",
        )
    })?;
    let manifest_value = serde_json::to_value(&manifest)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let binding_matches = connection
        .as_ref()
        .zip(owning_project.as_ref())
        .is_some_and(|(connection, owning_project)| {
            let Ok(revision) = connection_integration_revision(connection) else {
                return false;
            };
            prior.host == manifest.host_kind.as_str()
                && prior.repo_root == repo_root
                && owning_project.repo_root == repo_root
                && prior.connection_id == installation.connection_internal_id
                && prior.selected_profile == manifest.integration_profile
                && prior.connection_intent.as_str() == connection.intent
                && guard_manifest_matches_owner_binding(
                    &manifest_value,
                    GuardManifestOwnerBinding {
                        row_guard_installation_id: &installation.guard_installation_id,
                        row_connection_id: &installation.connection_internal_id,
                        row_project_id: &installation.project_id,
                        connection_host_kind: &connection.host_kind,
                        connection_integration_revision: revision.as_str(),
                        project_repo_root: &owning_project.repo_root,
                        project_git_info_exclude_path: owning_git_info_exclude_path.as_deref(),
                    },
                )
        });
    if !binding_matches {
        if prior_identity_matches {
            return Ok(Vec::new());
        }
        return Err(GuardIntegrationError::runtime(
            "managed_integration_inventory_invalid: stored Guard manifest ownership does not match the current Codex Record connection; verify or repair it before changing ownership",
        ));
    }
    plan_retired_files_from_manifest(repo_root, &manifest, generated_files)
}

fn plan_retired_files_from_manifest(
    repo_root: &Path,
    manifest: &volicord_types::guard_manifest::GuardManifest,
    generated_files: &[GeneratedFilePlan],
) -> Result<Vec<ManagedFileRetirementPlan>, GuardIntegrationError> {
    let current_paths = generated_files
        .iter()
        .map(|file| file.path.as_path())
        .collect::<std::collections::BTreeSet<_>>();
    let mut retired = Vec::new();
    for expectation in &manifest.managed_files {
        if matches!(
            expectation.artifact(),
            GuardManagedArtifact::VolicordPolicy
                | GuardManagedArtifact::GitInfoExclude
                | GuardManagedArtifact::AgentsManagedBlock
        ) {
            continue;
        }
        let path = expectation.path();
        if current_paths.contains(path) {
            continue;
        }
        retired.push(plan_managed_file_retirement(repo_root, expectation)?);
    }
    Ok(retired)
}

fn guard_hooks_unsupported_message(
    host_kind: HostKind,
    missing_required_hooks: &[GuardHookPhase],
) -> String {
    let missing = missing_required_hooks
        .iter()
        .map(|phase| guard_phase_capability_name(*phase))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "GUARD_HOOKS_UNSUPPORTED: {} record init requires configured host lifecycle hooks, but this adapter does not define project-local hook configuration for: {}. AGENTS.md and .volicord/policy.json are not host hook configuration.",
        public_host_label(host_kind),
        missing
    )
}

fn agents_guidance_block() -> String {
    format!(
        concat!(
            "{guidance_start_marker}\n# Volicord\n\n",
            "- Treat Volicord's recorded scope and Store-derived tagged current effective shaping authority graph as authoritative. Superseded checkpoint, request, resolution, and application history is immutable audit history, never actionable authority.\n",
            "- Do not modify Product Repository files outside an active compatible write authorization.\n",
            "- Do not infer, resolve, approve, or record user-owned trust judgments on the user's behalf.\n",
            "- For the request `Run the Volicord integration verification.`, call `{}`, then `{}`. Follow its returned `workflow` tagged state: `{}` calls its exact `{}` tool once; `{}` calls its exact `{}` status tool once; `{}` and `{}` call no verification tool. Do not use shell sleep or poll loops, make repeated status calls, or automatically restart the workflow in the same turn. Begin, probe, and status expose the same workflow state.\n",
            "- Only that first-party state-directed workflow proves current MCP and Guard correlation. Manual stdio and CLI MCP preflight are diagnostic and are not managed-host evidence.\n",
            "- If Volicord tools are not exposed, report the managed MCP connection as unavailable. Do not substitute raw stdio, hand-author Codex `_meta`, or treat resources/list or resource templates as tool proof; use read-only connection status or MCP preflight only for diagnosis.\n",
            "- `volicord connection verify` is optional active diagnostics only; it does not replace the managed-host workflow.\n",
            "- Follow the tagged workflow's `required_action`; do not call workflow tools speculatively or select progression from top-level array order.\n",
            "- Never replace the current shaping checkpoint to remove a pending or accepted-but-unapplied decision. Preserve its UserAction authority and follow the tagged recovery method.\n",
            "- Carry every current compatible applied decision explicitly through `carry_forward_application_refs` when revising a checkpoint. Never replace a checkpoint to discard applied authority.\n",
            "- Stale shaping authority grants no permission. Never carry a stale application forward or reuse its accepted resolution. For `application_authority_stale`, follow the exact `stale_authority_actions`: `retire` it or `reauthorize` it through a fresh successor gap and fresh `UserActionRequest`, preserving immutable `ShapingAuthorityReauthorization` lineage.\n",
            "- Inspect the exact User Channel resolution outcome. Resolution does not apply a shaping decision: apply only accepted, current, compatible authority through its `application_owner`, using the exact current resolution refs.\n",
            "- After rejection, deferral, or expiration, follow `decision_recovery_required` and revise shaping. Never retry resolution of a terminal or expired request. If the revised plan still needs that judgment, create a successor UserActionRequest with an independent identity; chat text cannot replace it.\n",
            "- A rejected, deferred, or expired decision grants no authority and keeps Product Repository mutation unavailable. Surface that outcome and do not hide it as success.\n",
            "- During `work/implementation`, an authority-invalidating scope, baseline, or Change Unit update is rejected before mutation. Follow the tagged `volicord.close_task` recovery to leave implementation; implementation work never returns silently to shaping.\n",
            "- Do not invent a scope decision or pass a scope-decision ref for product-only or technical-only work; follow that decision's mode-specific application owner.\n",
            "- Creating or replacing a Change Unit does not advance the Task phase. For work, call `volicord.advance_task` only when the tagged workflow requires explicit advance and never while a UserAction is pending. Do not call `volicord.prepare_write` before the Task enters implementation.\n",
            "- Advisor work uses only a non-write Change Unit. On `ready_to_finalize_advice`, finalize the current advisor result with `volicord.finalize_advice`; do not use `volicord.record_run`, `volicord.advance_task`, or `volicord.prepare_write` for advisor.\n",
            "- Create current UserAction requests before presenting user-owned choices. A chat reply is not a User Channel resolution; surface the canonical CLI inbox instruction.\n",
            "- Never hide or paraphrase a rejected mutation as success. Surface the tagged workflow and every rejection and recovery fact in `presentation.must_surface`, including the current Task phase and exact recovery method.\n",
            "- Evaluate close readiness only during close review. Close blockers do not replace tagged workflow progression.\n",
            "- Call `volicord.status` only when the current Task state is unknown or an authoritative refresh is required.\n",
            "- Do not claim completion while Volicord reports close blockers. If Volicord is unavailable, disclose that its state was not updated or verified.\n",
            "{guidance_end_marker}\n",
        ),
        AgentToolId::LIST_PROJECTS.wire_name(),
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
        IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND,
        AgentToolId::GUARD_PROBE.wire_name(),
        IntegrationVerificationWorkflowState::AWAITING_OBSERVATION_KIND,
        AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
        IntegrationVerificationWorkflowState::REPAIR_REQUIRED_KIND,
        IntegrationVerificationWorkflowState::COMPLETE_KIND,
        guidance_start_marker = GUIDANCE_START_MARKER,
        guidance_end_marker = GUIDANCE_END_MARKER,
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use volicord_host_contract::{
        CanonicalToolName, HostHookMatcherStrategy, McpServerKey, McpToolCatalog,
    };
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::guard_manifest::GuardManagedArtifact;
    use volicord_types::tool_names::AgentToolId;
    use volicord_types::values::IntegrationProfile;

    use super::{
        plan_guard_integration, validate_generated_guard_plan, GuardIntegrationPlanRequest,
    };
    use crate::{
        guard_integration::apply_guard_integration,
        host_integration::{ConnectionIntent, HostKind},
    };
    use volicord_mcp::ManagedMcpLaunchSpec;

    #[test]
    fn plan_finalization_rejects_an_invalid_policy_runtime_projection(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-plan-projection-validation")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)?;
        let mut plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            server_name: "volicord-test",
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id: "guard_invalid_generated_shape",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;
        plan.runtime_commands.pre_tool.args[9] = "invalid_host".to_owned();

        let error = validate_generated_guard_plan(plan)
            .expect_err("invalid generated command projection must fail plan finalization");
        assert!(error
            .to_string()
            .contains("does not preserve the policy/runtime command projection contract"));
        Ok(())
    }

    #[test]
    fn generated_host_guidance_preserves_the_canonical_verification_boundary(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-plan-verification-guidance")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)?;
        let plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            server_name: "volicord-test",
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id: "guard_verification_guidance",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;

        let agents = plan
            .generated_files
            .iter()
            .find(|file| file.artifact == GuardManagedArtifact::AgentsManagedBlock)
            .expect("managed AGENTS guidance");
        for required in [
            "Run the Volicord integration verification.",
            "`volicord.list_projects`",
            "`volicord.guard_probe`",
            "`awaiting_probe`",
            "`awaiting_observation`",
            "`repair_required`",
            "`complete`",
            "same workflow state",
            "raw stdio",
            "Codex `_meta`",
            "resources/list",
            "not managed-host evidence",
            "user-owned trust judgments",
            "Do not use shell sleep or poll loops",
            "automatically restart the workflow in the same turn",
            "`volicord connection verify` is optional active diagnostics only",
            "tagged workflow's `required_action`",
            "Store-derived tagged current effective shaping authority graph",
            "immutable audit history, never actionable authority",
            "Never replace the current shaping checkpoint",
            "accepted-but-unapplied decision",
            "Carry every current compatible applied decision explicitly",
            "`carry_forward_application_refs`",
            "Never replace a checkpoint to discard applied authority",
            "Stale shaping authority grants no permission",
            "Never carry a stale application forward or reuse its accepted resolution",
            "exact `stale_authority_actions`",
            "fresh successor gap and fresh `UserActionRequest`",
            "immutable `ShapingAuthorityReauthorization` lineage",
            "Inspect the exact User Channel resolution outcome",
            "apply only accepted, current, compatible authority",
            "through its `application_owner`",
            "follow `decision_recovery_required` and revise shaping",
            "Never retry resolution of a terminal or expired request",
            "successor UserActionRequest with an independent identity",
            "keeps Product Repository mutation unavailable",
            "authority-invalidating scope, baseline, or Change Unit update is rejected before mutation",
            "tagged `volicord.close_task` recovery",
            "implementation work never returns silently to shaping",
            "product-only or technical-only work",
            "Creating or replacing a Change Unit does not advance the Task phase",
            "tagged workflow requires explicit advance",
            "Advisor work uses only a non-write Change Unit",
            "`ready_to_finalize_advice`",
            "finalize the current advisor result with `volicord.finalize_advice`",
            "UserAction requests before presenting user-owned choices",
            "chat reply is not a User Channel resolution",
            "never while a UserAction is pending",
            "`volicord.prepare_write` before the Task enters implementation",
            "rejected mutation as success",
            "rejection and recovery fact in `presentation.must_surface`",
            "Close blockers do not replace tagged workflow progression",
            "close readiness only during close review",
        ] {
            assert!(agents.content.contains(required));
        }
        for control in [
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
            AgentToolId::GET_INTEGRATION_VERIFICATION,
        ] {
            assert!(agents
                .content
                .contains(&format!("`{}`", control.wire_name())));
        }
        for forbidden in ["awaiting_hook_completion", "restart_required", "codex-1."] {
            assert!(!agents.content.contains(forbidden));
        }

        let hook_config = plan
            .generated_files
            .iter()
            .find(|file| file.artifact == GuardManagedArtifact::HostHookConfig)
            .expect("managed Codex hook configuration");
        let hook_config: Value = serde_json::from_str(&hook_config.content)?;
        let matcher = hook_config
            .pointer("/hooks/PreToolUse/0/matcher")
            .and_then(Value::as_str)
            .expect("PreToolUse matcher");
        let server = McpServerKey::parse("volicord-test")?;
        let strategy = HostHookMatcherStrategy::parse_codex_guard(matcher, &server)?;
        let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL)?;
        let guard_callable = catalog
            .require(&server, AgentToolId::GUARD_PROBE)?
            .callable_name()
            .as_str();
        let status_callable = catalog
            .require(&server, AgentToolId::STATUS)?
            .callable_name()
            .as_str();
        assert!(strategy.routes(&CanonicalToolName::parse(guard_callable)?));
        assert!(strategy.routes(&CanonicalToolName::parse(status_callable)?));
        assert!(!strategy.routes(&CanonicalToolName::parse(
            "mcp__foreign__volicord_guard_probe"
        )?));

        let codex_rule = plan
            .generated_files
            .iter()
            .find(|file| file.artifact == GuardManagedArtifact::HostRuleInstruction)
            .expect("managed Codex rule guidance");
        for required in [
            "Hook review and trust remain user/host owned",
            "Run the Volicord integration verification.",
            "volicord.list_projects",
            "volicord.guard_probe",
            "awaiting_probe",
            "awaiting_observation",
            "repair_required",
            "complete",
            "raw stdio",
            "Codex _meta",
            "shell sleep or poll loops",
            "automatically restart the workflow in the same turn",
            "volicord connection verify is optional active diagnostics only",
        ] {
            assert!(codex_rule.content.contains(required));
        }
        for control in [
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
            AgentToolId::GET_INTEGRATION_VERIFICATION,
        ] {
            assert!(codex_rule.content.contains(control.wire_name()));
        }
        for forbidden in ["awaiting_hook_completion", "restart_required", "codex-1."] {
            assert!(!codex_rule.content.contains(forbidden));
        }
        Ok(())
    }

    #[test]
    fn hook_definition_change_detection_is_exact_for_create_and_replay(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-plan-hook-definition-change")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)?;
        let request = || GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            server_name: "volicord-test",
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id: "guard_hook_definition_change",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        };

        let create = plan_guard_integration(request())?;
        assert!(create.hook_definition_changed());
        apply_guard_integration(create)?;

        let replay = plan_guard_integration(request())?;
        assert!(!replay.hook_definition_changed());
        Ok(())
    }
}
