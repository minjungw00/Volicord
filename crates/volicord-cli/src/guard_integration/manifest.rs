use std::path::Path;

use serde_json::{json, Value};
use volicord_store::agent_connections::AgentConnectionRecord;
use volicord_store::guards::{
    upsert_guard_installation, GuardInstallationRecord, GuardInstallationUpsert,
};
use volicord_store::operational_sessions::connection_integration_revision;
use volicord_types::{
    AgentConnectionId, GuardInstallationId, GuardManifest, HostKind as ManifestHostKind,
    IntegrationProfile, ManagedFileExpectation, ProjectId, GUARD_MANIFEST_SCHEMA,
};

pub(crate) use volicord_types::guard_manifest_has_exact_current_shape;

use crate::{
    guard_integration::{
        audit::{hook_wrapper_comment_value, hook_wrapper_exec_command, sha256_text},
        files::{GeneratedFilePlan, GeneratedFileWriteKind, ManagedFileRetirementPlan},
        GuardIntegrationError, GuardIntegrationPlan, HostHookCommand, HostHookCommandShape,
        HOOK_WRAPPER_MARKER,
    },
    host_integration::HostIntegrationFileKind,
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

pub(crate) fn retired_files_json(files: &[ManagedFileRetirementPlan]) -> Value {
    Value::Array(
        files
            .iter()
            .map(|file| {
                json!({
                    "kind": file.kind.as_str(),
                    "path": path_text(&file.path),
                    "status": file.status.as_str(),
                })
            })
            .collect(),
    )
}

pub(crate) fn generated_files_json(files: &[GeneratedFilePlan]) -> Value {
    Value::Array(
        files
            .iter()
            .map(|file| {
                let mut value = json!({
                    "kind": file.kind.as_str(),
                    "path": path_text(&file.path),
                    "status": file.status.as_str(),
                    "content_hash": sha256_text(&file.content),
                });
                let object = value
                    .as_object_mut()
                    .expect("generated file JSON should be an object");
                match file.write_kind {
                    GeneratedFileWriteKind::Block {
                        start_marker,
                        end_marker,
                        ..
                    } => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_block".to_owned()),
                        );
                        object.insert(
                            "managed_marker_start".to_owned(),
                            Value::String(start_marker.to_owned()),
                        );
                        object.insert(
                            "managed_marker_end".to_owned(),
                            Value::String(end_marker.to_owned()),
                        );
                    }
                    GeneratedFileWriteKind::Json | GeneratedFileWriteKind::ExactJson => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_json".to_owned()),
                        );
                    }
                    GeneratedFileWriteKind::Script => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_script".to_owned()),
                        );
                        object.insert(
                            "managed_marker".to_owned(),
                            Value::String(HOOK_WRAPPER_MARKER.to_owned()),
                        );
                        object.insert("executable_required".to_owned(), Value::Bool(true));
                        if file.kind == HostIntegrationFileKind::HostHookDispatch {
                            object.insert(
                                "managed_script_role".to_owned(),
                                Value::String("codex_dispatch".to_owned()),
                            );
                        } else if let Some(command) = hook_wrapper_exec_command(&file.content) {
                            object.insert(
                                "managed_script_command".to_owned(),
                                Value::String(command.to_owned()),
                            );
                        }
                        for key in [
                            "host_kind",
                            "phase",
                            "purpose",
                            "connection_id",
                            "guard_installation_id",
                            "policy_hash",
                            "host_output",
                        ] {
                            if let Some(value) = hook_wrapper_comment_value(&file.content, key) {
                                object.insert(key.to_owned(), Value::String(value.to_owned()));
                            }
                        }
                    }
                }
                value
            })
            .collect(),
    )
}

fn managed_file_expectations(
    files: &[GeneratedFilePlan],
) -> Result<Vec<ManagedFileExpectation>, GuardIntegrationError> {
    let Value::Array(values) = generated_files_json(files) else {
        unreachable!("generated files serialize as an array")
    };
    values
        .into_iter()
        .map(|mut value| {
            value
                .as_object_mut()
                .expect("generated file entry is an object")
                .remove("status");
            serde_json::from_value(value)
                .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
        })
        .collect()
}

pub(crate) fn host_hook_commands_json(commands: &[HostHookCommand]) -> Value {
    Value::Array(
        commands
            .iter()
            .map(|command| {
                let (command_text, args) = match &command.generated_command_shape {
                    HostHookCommandShape::ShellCommandString { command_text, .. } => {
                        (command_text.clone(), Value::Null)
                    }
                };
                json!({
                    "host_kind": command.host_kind.as_str(),
                    "phase": command.phase.capability_name(),
                    "purpose": command.purpose.as_str(),
                    "policy_key": command.phase.policy_key(),
                    "command_shape": command.command_shape_name(),
                    "command": command_text,
                    "args": args,
                    "expected_wrapper_path": path_text(&command.expected_wrapper_path),
                    "expected_phase_wrapper_path": path_text(&command.expected_phase_wrapper_path),
                    "root_resolution_basis": command.root_resolution_basis.as_str(),
                    "hook_command_path_basis": command.hook_command_path_basis.as_str(),
                    "cwd_independent": command.cwd_independent,
                    "subdirectory_safe": command.subdirectory_safe,
                    "wrapper_resolution_status": command.wrapper_resolution_status.as_str(),
                    "verification": {
                        "basis_verified_by": &command.verification.basis_verified_by,
                        "host_contract_source": &command.verification.host_contract_source,
                    },
                })
            })
            .collect(),
    )
}

pub(crate) fn hook_root_resolution_json(commands: &[HostHookCommand]) -> Value {
    if commands.is_empty() {
        return Value::Null;
    }
    let mut bases = commands
        .iter()
        .map(|command| command.root_resolution_basis.as_str())
        .collect::<Vec<_>>();
    bases.sort_unstable();
    bases.dedup();
    let cwd_independent = commands.iter().all(|command| command.cwd_independent);
    let subdirectory_safe = commands.iter().all(|command| command.subdirectory_safe);
    let basis = if bases.len() == 1 {
        bases[0].to_owned()
    } else {
        "mixed".to_owned()
    };
    json!({
        "basis": basis,
        "all_cwd_independent": cwd_independent,
        "all_subdirectory_safe": subdirectory_safe,
        "overall_status": if cwd_independent && subdirectory_safe { "ok" } else { "relative_path_unsafe" },
        "phases": commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command.phase.capability_name(),
                    "root_resolution_basis": command.root_resolution_basis.as_str(),
                    "hook_command_path_basis": command.hook_command_path_basis.as_str(),
                    "cwd_independent": command.cwd_independent,
                    "subdirectory_safe": command.subdirectory_safe,
                    "wrapper_resolution_status": command.wrapper_resolution_status.as_str(),
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs};

    use volicord_store::agent_connections::agent_connection_record_read_only;
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{guard_manifest_from_json, GuardHookPhase, IntegrationProfile};

    use super::{guard_manifest_json, record_guard_installation};
    use crate::{
        guard_integration::{
            apply_guard_integration,
            audit::{hook_wrapper_comment_value, hook_wrapper_exec_command},
            hooks::guard_command_line,
            plan::{plan_guard_integration, GuardIntegrationPlanRequest},
        },
        host_integration::{
            ConnectionIntent, HostIntegrationFileKind, HostKind, ManagedServerEntry,
        },
    };

    #[test]
    fn manifest_preserves_policy_and_runtime_command_forms_without_status_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-manifest-command-forms")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedServerEntry::new_project_bound(
            fixture.connection_id(),
            Some(fixture.project_id()),
            &volicord_command,
        );
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
                .find(|file| {
                    file.kind == HostIntegrationFileKind::HostHookWrapper
                        && hook_wrapper_comment_value(&file.content, "phase")
                            == Some(phase.as_str())
                })
                .expect("phase wrapper");
            let expected_command = guard_command_line(runtime);
            assert_eq!(
                hook_wrapper_exec_command(&wrapper.content),
                Some(expected_command.as_str())
            );
        }

        let connection = agent_connection_record_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?
        .expect("fixture connection");
        let manifest_text = guard_manifest_json(&connection, fixture.project_id(), &plan)?;
        let manifest = guard_manifest_from_json(&manifest_text)?;
        assert_eq!(manifest.runtime_commands, plan.runtime_commands);
        assert_eq!(
            manifest
                .required_hook_phases
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            GuardHookPhase::REQUIRED.into_iter().collect()
        );
        assert_eq!(
            manifest
                .managed_files
                .iter()
                .filter(|file| file.ownership == "managed_script")
                .count(),
            4
        );
        assert!(manifest
            .managed_files
            .iter()
            .filter(|file| file.ownership == "managed_script")
            .all(|file| file.executable_required == Some(true)));
        let manifest_value = serde_json::to_value(&manifest)?;
        assert!(manifest_value["managed_files"]
            .as_array()
            .expect("manifest managed files")
            .iter()
            .all(|file| file.get("status").is_none()));

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
