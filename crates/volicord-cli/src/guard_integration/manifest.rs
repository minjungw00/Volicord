use std::path::Path;

use volicord_host_contract::HostContractProfileId;
use volicord_store::agent_connections::AgentConnectionRecord;
use volicord_store::guards::{
    upsert_guard_installation, GuardInstallationRecord, GuardInstallationUpsert,
};
use volicord_store::operational_sessions::connection_integration_revision;
use volicord_types::{
    AgentConnectionId, GuardArtifactContentHash, GuardInstallationId, GuardManagedArtifact,
    GuardManifest, HostKind as ManifestHostKind, IntegrationProfile, ManagedFileExpectation,
    ProjectId, GUARD_MANIFEST_SCHEMA,
};

pub(crate) use volicord_types::guard_manifest_has_exact_current_shape;

use crate::guard_integration::{
    audit::{hook_wrapper_comment_value, hook_wrapper_exec_command, sha256_text},
    files::{generated_file_plan_matches_artifact_spec, GeneratedFilePlan, GeneratedFileWriteKind},
    GuardIntegrationError, GuardIntegrationPlan, HOOK_WRAPPER_MARKER,
};

pub(crate) fn record_guard_installation(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    project_id: &str,
    integration: &GuardIntegrationPlan,
) -> Result<GuardInstallationRecord, GuardIntegrationError> {
    let input = guard_installation_upsert(connection, project_id, integration)?;
    upsert_guard_installation(runtime_home, input)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

pub(crate) fn guard_installation_upsert(
    connection: &AgentConnectionRecord,
    project_id: &str,
    integration: &GuardIntegrationPlan,
) -> Result<GuardInstallationUpsert, GuardIntegrationError> {
    Ok(GuardInstallationUpsert {
        guard_installation_id: integration.guard_installation_id.clone(),
        connection_internal_id: connection.connection_internal_id.clone(),
        project_id: project_id.to_owned(),
        manifest_json: guard_manifest_json(connection, project_id, integration)?,
    })
}

pub(crate) fn guard_manifest_json(
    connection: &AgentConnectionRecord,
    project_id: &str,
    plan: &GuardIntegrationPlan,
) -> Result<String, GuardIntegrationError> {
    let integration_revision = connection_integration_revision(connection)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let manifest = GuardManifest {
        schema: GUARD_MANIFEST_SCHEMA.to_owned(),
        guard_installation_id: GuardInstallationId::new(plan.guard_installation_id.clone()),
        connection_id: AgentConnectionId::new(connection.connection_internal_id.clone()),
        project_id: ProjectId::new(project_id),
        host_kind: ManifestHostKind::Codex,
        integration_profile: IntegrationProfile::Record,
        host_contract_profile: HostContractProfileId::CodexHooksV1.as_str().to_owned(),
        host_contract_digest: HostContractProfileId::CodexHooksV1.contract_digest(),
        policy_hash: plan.policy_hash.clone(),
        integration_revision,
        runtime_commands: plan.runtime_commands.clone(),
        managed_files: managed_file_expectations(&plan.generated_files)?,
        required_hook_phases: plan.required_hook_phases.clone(),
    };
    let value = serde_json::to_value(&manifest)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    if !guard_manifest_has_exact_current_shape(&value) {
        return Err(GuardIntegrationError::runtime(
            "generated Guard manifest does not match the current exact shape",
        ));
    }
    serde_json::to_string(&manifest)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

fn managed_file_expectations(
    files: &[GeneratedFilePlan],
) -> Result<Vec<ManagedFileExpectation>, GuardIntegrationError> {
    files.iter().map(managed_file_expectation).collect()
}

fn managed_file_expectation(
    file: &GeneratedFilePlan,
) -> Result<ManagedFileExpectation, GuardIntegrationError> {
    if !generated_file_plan_matches_artifact_spec(file) {
        return Err(GuardIntegrationError::runtime(format!(
            "generated {} does not match its registered Guard artifact specification",
            file.artifact.kind().as_str()
        )));
    }
    let content_hash = GuardArtifactContentHash::parse(sha256_text(&file.content))
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let path = file.path.clone();
    match (file.artifact, file.write_kind) {
        (
            GuardManagedArtifact::AgentsManagedBlock,
            GeneratedFileWriteKind::Block {
                start_marker,
                end_marker,
                ..
            },
        ) => ManagedFileExpectation::managed_block(
            file.artifact,
            path,
            content_hash,
            start_marker,
            end_marker,
        )
        .map_err(|error| GuardIntegrationError::runtime(error.to_string())),
        (
            GuardManagedArtifact::GitInfoExclude,
            GeneratedFileWriteKind::Block {
                start_marker,
                end_marker,
                ..
            },
        ) => ManagedFileExpectation::managed_block(
            file.artifact,
            path,
            content_hash,
            start_marker,
            end_marker,
        )
        .map_err(|error| GuardIntegrationError::runtime(error.to_string())),
        (
            GuardManagedArtifact::HostRuleInstruction,
            GeneratedFileWriteKind::Block {
                start_marker,
                end_marker,
                ..
            },
        ) => ManagedFileExpectation::managed_block(
            file.artifact,
            path,
            content_hash,
            start_marker,
            end_marker,
        )
        .map_err(|error| GuardIntegrationError::runtime(error.to_string())),
        (GuardManagedArtifact::VolicordPolicy, GeneratedFileWriteKind::Json) => {
            ManagedFileExpectation::managed_json(file.artifact, path, content_hash)
                .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
        }
        (GuardManagedArtifact::HostHookConfig, GeneratedFileWriteKind::ExactJson) => {
            ManagedFileExpectation::managed_json(file.artifact, path, content_hash)
                .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
        }
        (GuardManagedArtifact::HostHookDispatch, GeneratedFileWriteKind::Script) => {
            let host_kind = required_wrapper_host_kind(&file.content)?;
            require_wrapper_comment(&file.content, "phase", "dispatch")?;
            require_wrapper_comment(&file.content, "script_role", "codex_dispatch")?;
            if host_kind != ManifestHostKind::Codex {
                return Err(GuardIntegrationError::runtime(
                    "generated dispatch script has a non-Codex host coordinate",
                ));
            }
            Ok(ManagedFileExpectation::codex_dispatch_script(
                path,
                content_hash,
                HOOK_WRAPPER_MARKER,
            ))
        }
        (GuardManagedArtifact::HostHookWrapper(phase), GeneratedFileWriteKind::Script) => {
            let managed_script_command = hook_wrapper_exec_command(&file.content)
                .ok_or_else(|| GuardIntegrationError::runtime("generated wrapper has no command"))?
                .to_owned();
            let host_kind = required_wrapper_host_kind(&file.content)?;
            require_wrapper_comment(&file.content, "phase", phase.as_str())?;
            require_wrapper_comment(&file.content, "purpose", "guard")?;
            let connection_id =
                AgentConnectionId::new(required_wrapper_comment(&file.content, "connection_id")?);
            let guard_installation_id = GuardInstallationId::new(required_wrapper_comment(
                &file.content,
                "guard_installation_id",
            )?);
            let policy_hash = volicord_types::PolicyHash::parse(required_wrapper_comment(
                &file.content,
                "policy_hash",
            )?)
            .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
            let host_output = required_wrapper_comment(&file.content, "host_output")?
                .parse::<ManifestHostKind>()
                .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
            if host_kind != ManifestHostKind::Codex || host_output != ManifestHostKind::Codex {
                return Err(GuardIntegrationError::runtime(
                    "generated wrapper has a non-Codex host coordinate",
                ));
            }
            Ok(ManagedFileExpectation::hook_wrapper(
                phase,
                path,
                content_hash,
                HOOK_WRAPPER_MARKER,
                managed_script_command,
                connection_id,
                guard_installation_id,
                policy_hash,
            ))
        }
        _ => Err(GuardIntegrationError::runtime(format!(
            "generated {} does not use its canonical Guard artifact semantics",
            file.artifact.kind().as_str()
        ))),
    }
}

fn required_wrapper_host_kind(content: &str) -> Result<ManifestHostKind, GuardIntegrationError> {
    required_wrapper_comment(content, "host_kind")?
        .parse::<ManifestHostKind>()
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

fn required_wrapper_comment<'a>(
    content: &'a str,
    key: &str,
) -> Result<&'a str, GuardIntegrationError> {
    hook_wrapper_comment_value(content, key).ok_or_else(|| {
        GuardIntegrationError::runtime(format!("generated wrapper is missing typed {key} metadata"))
    })
}

fn require_wrapper_comment(
    content: &str,
    key: &str,
    expected: &str,
) -> Result<(), GuardIntegrationError> {
    if required_wrapper_comment(content, key)? == expected {
        Ok(())
    } else {
        Err(GuardIntegrationError::runtime(format!(
            "generated wrapper {key} does not match its Guard artifact coordinate"
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::PathBuf};

    use volicord_host_contract::HostContractProfileId;
    use volicord_store::agent_connections::agent_connection_record_read_only;
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{
        guard_manifest_from_json, GuardCommandInvocation, GuardHookPhase, GuardManagedArtifact,
        GuardManagedOwnership, IntegrationProfile,
    };

    use super::{
        guard_manifest_has_exact_current_shape, guard_manifest_json, record_guard_installation,
    };
    use crate::{
        guard_integration::{
            apply_guard_integration,
            audit::{hook_wrapper_comment_value, hook_wrapper_exec_command},
            hooks::guard_command_line,
            plan::{plan_guard_integration, GuardIntegrationPlanRequest},
        },
        host_integration::{ConnectionIntent, HostKind},
    };
    use volicord_mcp::ManagedMcpLaunchSpec;

    #[test]
    fn manifest_preserves_policy_and_runtime_command_forms_without_status_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-manifest-command-forms")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)?;
        let guard_installation_id = "guard_manifest_command_forms";
        let plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id,
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;

        for phase in GuardHookPhase::REQUIRED {
            let policy = plan.policy_commands.get(phase);
            let runtime = plan.runtime_commands.get(phase);
            assert_eq!(policy.args.len(), 14);
            assert_eq!(runtime.args.len(), 16);
            assert!(!policy.args.iter().any(|arg| arg == "--policy-hash"));
            assert_eq!(&runtime.args[..12], &policy.args[..12]);
            assert_eq!(
                &runtime.args[12..14],
                &["--policy-hash", plan.policy_hash.as_str()]
            );
            assert_eq!(&runtime.args[14..], &policy.args[12..]);

            let wrapper = plan
                .generated_files
                .iter()
                .find(|file| file.artifact == GuardManagedArtifact::HostHookWrapper(phase))
                .expect("phase wrapper");
            let expected_command = guard_command_line(runtime);
            assert_eq!(
                hook_wrapper_exec_command(&wrapper.content),
                Some(expected_command.as_str())
            );
            let invocation = GuardCommandInvocation::from_runtime_command(runtime)?;
            assert_eq!(
                hook_wrapper_comment_value(&wrapper.content, "connection_id"),
                Some(invocation.connection_id.as_str())
            );
            assert_eq!(
                hook_wrapper_comment_value(&wrapper.content, "guard_installation_id"),
                Some(invocation.guard_installation_id.as_str())
            );
            assert_eq!(
                hook_wrapper_comment_value(&wrapper.content, "policy_hash"),
                invocation.policy_hash.as_ref().map(|hash| hash.as_str())
            );
            assert_eq!(
                hook_wrapper_comment_value(&wrapper.content, "host_output"),
                Some(invocation.host_output.as_str())
            );
        }

        let connection = agent_connection_record_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?
        .expect("fixture connection");
        let manifest_text = guard_manifest_json(&connection, fixture.project_id(), &plan)?;
        let manifest = guard_manifest_from_json(&manifest_text)?;
        assert_eq!(
            manifest.host_contract_profile,
            HostContractProfileId::CodexHooksV1.as_str()
        );
        assert_eq!(
            manifest.host_contract_digest,
            HostContractProfileId::CodexHooksV1.contract_digest()
        );
        assert_eq!(manifest.runtime_commands, plan.runtime_commands);
        assert_eq!(
            manifest
                .required_hook_phases
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            GuardHookPhase::REQUIRED.into_iter().collect()
        );
        let managed_scripts = manifest
            .managed_files
            .iter()
            .filter(|file| file.ownership() == GuardManagedOwnership::ManagedScript)
            .map(|file| (PathBuf::from(file.path()), file.artifact()))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            managed_scripts,
            BTreeSet::from([
                (
                    repo_root.join(".codex/hooks/volicord-dispatch.sh"),
                    GuardManagedArtifact::HostHookDispatch,
                ),
                (
                    repo_root.join(".codex/hooks/volicord-pre-tool.sh"),
                    GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
                ),
                (
                    repo_root.join(".codex/hooks/volicord-post-tool.sh"),
                    GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PostTool),
                ),
                (
                    repo_root.join(".codex/hooks/volicord-prompt-capture.sh"),
                    GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PromptCapture),
                ),
            ])
        );
        assert!(manifest
            .managed_files
            .iter()
            .filter(|file| file.ownership() == GuardManagedOwnership::ManagedScript)
            .all(|file| file.executable_required() == Some(true)));
        assert!(manifest
            .managed_files
            .iter()
            .filter(|file| file.ownership() != GuardManagedOwnership::ManagedScript)
            .all(|file| file.executable_required().is_none()));

        let mut manifest_value = serde_json::to_value(&manifest)?;
        assert!(manifest_value["managed_files"]
            .as_array()
            .expect("manifest managed files")
            .iter()
            .all(|file| file.get("status").is_none()));
        let script = manifest_value["managed_files"]
            .as_array_mut()
            .expect("manifest managed files")
            .iter_mut()
            .find(|file| file["ownership"] == "managed_script")
            .expect("managed script entry");
        script["executable_required"] = serde_json::Value::Bool(false);
        assert!(!guard_manifest_has_exact_current_shape(&manifest_value));

        let applied = apply_guard_integration(plan)?;
        let first = record_guard_installation(
            fixture.runtime_home_path(),
            &connection,
            fixture.project_id(),
            &applied,
        )?;
        let second = record_guard_installation(
            fixture.runtime_home_path(),
            &connection,
            fixture.project_id(),
            &applied,
        )?;
        assert_eq!(first.guard_installation_id, second.guard_installation_id);
        assert_eq!(first.created_at, second.created_at);
        assert_eq!(first.manifest_json, second.manifest_json);
        guard_manifest_from_json(&second.manifest_json)?;
        Ok(())
    }
}
