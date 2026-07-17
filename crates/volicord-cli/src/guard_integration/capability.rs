use std::{path::Path, time::SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use volicord_store::guards::{
    upsert_guard_installation, GuardInstallationRecord, GuardInstallationUpsert,
};
use volicord_types::{GuardInstallationStatus, IntegrationProfile};

pub(crate) use volicord_types::{
    host_hook_capability_has_exact_current_shape, HOST_HOOK_CAPABILITY_SCHEMA,
};

use crate::{
    guard_integration::{
        audit::{hook_wrapper_comment_value, hook_wrapper_exec_command, sha256_text},
        files::{
            FilePlanStatus, GeneratedFilePlan, GeneratedFileWriteKind, ManagedFileRetirementPlan,
            RetirementPlanStatus, VOLICORD_POLICY_FILE,
        },
        hooks::guard_command_specs_json,
        policy::required_guard_phase_names,
        GuardIntegrationError, GuardIntegrationPlan, HostHookCommand, HostHookCommandShape,
        HOOK_WRAPPER_MARKER,
    },
    host_integration::{HostIntegrationFileKind, HostKind, HostPlan, PlannedChange},
};

const INIT_METADATA_CREATED_BY: &str = "volicord_cli_init";

pub(crate) fn record_guard_installation(
    runtime_home: &Path,
    host_kind: HostKind,
    profile: IntegrationProfile,
    installation_status: GuardInstallationStatus,
    connection_id: &str,
    project_id: &str,
    integration: &GuardIntegrationPlan,
) -> Result<GuardInstallationRecord, GuardIntegrationError> {
    let input = guard_installation_upsert(
        host_kind,
        profile,
        installation_status,
        connection_id,
        project_id,
        integration,
    )?;
    upsert_guard_installation(runtime_home, input)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

pub(crate) fn guard_installation_upsert(
    host_kind: HostKind,
    profile: IntegrationProfile,
    installation_status: GuardInstallationStatus,
    connection_id: &str,
    project_id: &str,
    integration: &GuardIntegrationPlan,
) -> Result<GuardInstallationUpsert, GuardIntegrationError> {
    let now = current_timestamp();
    Ok(GuardInstallationUpsert {
        guard_installation_id: integration.guard_installation_id.clone(),
        connection_internal_id: connection_id.to_owned(),
        project_id: Some(project_id.to_owned()),
        host_kind: host_kind.as_str().to_owned(),
        guard_mode: profile.as_str().to_owned(),
        host_capability_json: host_hook_capability_json(integration)?,
        installation_status: installation_status.as_str().to_owned(),
        installed_at: Some(now.clone()),
        last_checked_at: now,
        first_seen_at: None,
        last_seen_at: None,
        last_seen_phase: None,
        observed_host_kind: None,
        observed_policy_hash: None,
        observed_binary_version: None,
        metadata_json: serde_json::to_string(&json!({
            "created_by": INIT_METADATA_CREATED_BY,
            "policy_file": VOLICORD_POLICY_FILE,
            "selected_profile": integration.guard_profile,
            "connection_intent": integration.connection_intent,
            "required_phases": required_guard_phase_names(),
            "observation_status": "not_observed",
        }))
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?,
    })
}

pub(crate) fn host_hook_capability_json(
    plan: &GuardIntegrationPlan,
) -> Result<String, GuardIntegrationError> {
    let capabilities = serde_json::to_value(plan.capabilities)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    let capability = json!({
        "schema": HOST_HOOK_CAPABILITY_SCHEMA,
        "policy_hash": plan.policy_hash,
        "selected_profile": plan.guard_profile,
        "connection_intent": plan.connection_intent,
        "direct_file_write_matcher_coverage": plan.direct_file_write_matcher_coverage,
        "host_capabilities": capabilities,
        "files": generated_files_json(&plan.generated_files),
        "commands": guard_command_specs_json(&plan.guard_commands)?,
    });
    if !host_hook_capability_has_exact_current_shape(&capability) {
        return Err(GuardIntegrationError::runtime(
            "generated host-hook capability does not match the current exact shape",
        ));
    }
    serde_json::to_string(&capability)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

pub(crate) fn initial_guard_installation_status(
    _profile: IntegrationProfile,
    host_plan: &HostPlan,
    integration: &GuardIntegrationPlan,
) -> GuardInstallationStatus {
    if !integration.missing_required_hooks.is_empty() {
        GuardInstallationStatus::Degraded
    } else if host_plan.change != PlannedChange::Noop
        || integration.generated_files.iter().any(|file| {
            matches!(
                file.status,
                FilePlanStatus::Created | FilePlanStatus::Updated
            )
        })
        || integration.retired_files.iter().any(|file| {
            matches!(
                file.status,
                RetirementPlanStatus::Removed | RetirementPlanStatus::Updated
            )
        })
    {
        GuardInstallationStatus::ReloadRequired
    } else {
        GuardInstallationStatus::Configured
    }
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

fn current_timestamp() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs};

    use serde_json::{json, Value};
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{GuardInstallationStatus, IntegrationProfile};

    use super::{
        host_hook_capability_has_exact_current_shape, host_hook_capability_json,
        record_guard_installation,
    };
    use crate::{
        guard_integration::{
            apply_guard_integration,
            audit::{hook_wrapper_comment_value, hook_wrapper_exec_command},
            hooks::guard_command_line,
            plan::{plan_guard_integration, GuardIntegrationPlanRequest},
        },
        host_integration::{
            ConnectionIntent, HostIntegrationFileKind, HostKind, ManagedServerEntry,
            REQUIRED_GUARD_PHASES,
        },
    };

    #[test]
    fn successfully_returned_plan_generates_an_exact_current_capability(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-capability-post-hash")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedServerEntry::new_project_bound(
            fixture.connection_id(),
            Some(fixture.project_id()),
            &volicord_command,
        );
        let guard_installation_id = "guard_post_hash";
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

        let required_keys = BTreeSet::from(["post_tool", "pre_tool", "prompt_capture"]);
        let policy_commands = plan.policy["host_hook"]["commands"]
            .as_object()
            .expect("policy commands object");
        assert_eq!(
            policy_commands
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            required_keys
        );
        for phase in REQUIRED_GUARD_PHASES {
            let args = policy_commands[phase.policy_key()]["args"]
                .as_array()
                .expect("policy command args");
            assert_eq!(args.len(), 14);
            assert!(!args.iter().any(|arg| arg == "--policy-hash"));
        }

        let capability_text = host_hook_capability_json(&plan)?;
        assert_eq!(capability_text, host_hook_capability_json(&plan)?);
        let capability = serde_json::from_str::<Value>(&capability_text)?;
        assert!(host_hook_capability_has_exact_current_shape(&capability));
        let capability_files = capability["files"]
            .as_array()
            .expect("capability files array");
        let managed_scripts = capability_files
            .iter()
            .filter(|file| file["ownership"] == "managed_script")
            .collect::<Vec<_>>();
        assert_eq!(managed_scripts.len(), 4);
        assert_eq!(
            managed_scripts
                .iter()
                .filter(|file| file["kind"] == "host_hook_dispatch")
                .count(),
            1
        );
        assert_eq!(
            managed_scripts
                .iter()
                .filter(|file| file["kind"] == "host_hook_wrapper")
                .count(),
            3
        );
        assert!(managed_scripts
            .iter()
            .all(|file| file["executable_required"] == true));
        assert_eq!(
            managed_scripts
                .iter()
                .filter(|file| file["kind"] == "host_hook_wrapper")
                .map(|file| file["phase"].as_str().expect("wrapper phase"))
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["post_tool", "pre_tool", "prompt_capture"])
        );
        assert!(capability_files
            .iter()
            .filter(|file| file["ownership"] != "managed_script")
            .all(|file| file.get("executable_required").is_none()));
        let capability_commands = capability["commands"]
            .as_object()
            .expect("capability commands object");
        assert_eq!(
            capability_commands
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            required_keys
        );

        let repo_root_text = repo_root.to_str().expect("UTF-8 fixture path");
        let volicord_command_text = volicord_command.to_str().expect("UTF-8 fixture path");
        for phase in REQUIRED_GUARD_PHASES {
            let command = plan
                .guard_commands
                .get(phase.policy_key())
                .expect("typed Guard command");
            let capability_command = &capability_commands[phase.policy_key()];
            let capability_args = capability_command["args"]
                .as_array()
                .expect("capability command args");
            assert_eq!(capability_args.len(), 16);
            assert_eq!(capability_args[12], "--policy-hash");
            assert_eq!(capability_args[13], capability["policy_hash"]);
            assert_eq!(
                capability_command["command"].as_str(),
                Some(command.command.as_str())
            );
            assert!(capability_args
                .iter()
                .map(|arg| arg.as_str().expect("string command argument"))
                .eq(command.args.iter().map(String::as_str)));
            assert_eq!(command.command, volicord_command_text);
            assert!(command.args.iter().map(String::as_str).eq([
                "_hook",
                phase.command_name(),
                "--repo",
                repo_root_text,
                "--connection",
                fixture.connection_id(),
                "--guard-installation",
                guard_installation_id,
                "--host",
                "codex",
                "--integration-profile",
                "record",
                "--policy-hash",
                plan.policy_hash.as_str(),
                "--host-output",
                "codex",
            ]));

            let wrapper = plan
                .generated_files
                .iter()
                .find(|file| {
                    file.kind == HostIntegrationFileKind::HostHookWrapper
                        && hook_wrapper_comment_value(&file.content, "phase")
                            == Some(phase.policy_key())
                })
                .expect("phase wrapper");
            let expected_command_line = guard_command_line(command);
            assert_eq!(
                hook_wrapper_exec_command(&wrapper.content),
                Some(expected_command_line.as_str())
            );
            for (key, expected) in [
                ("host_kind", "codex"),
                ("connection_id", fixture.connection_id()),
                ("guard_installation_id", guard_installation_id),
                ("policy_hash", plan.policy_hash.as_str()),
                ("host_output", "codex"),
            ] {
                assert_eq!(
                    hook_wrapper_comment_value(&wrapper.content, key),
                    Some(expected)
                );
            }
        }

        let mut pre_hash_capability = capability.clone();
        pre_hash_capability["commands"]["pre_tool"]["args"]
            .as_array_mut()
            .expect("pre-tool args")
            .drain(12..14);
        assert!(!host_hook_capability_has_exact_current_shape(
            &pre_hash_capability
        ));

        let mut extra_phase_capability = capability;
        extra_phase_capability["commands"]["removed_phase"] =
            json!({"command": volicord_command_text, "args": []});
        assert!(!host_hook_capability_has_exact_current_shape(
            &extra_phase_capability
        ));
        Ok(())
    }

    #[test]
    fn persisted_capability_uses_applied_file_statuses_instead_of_preflight_statuses(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-capability-applied-statuses")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedServerEntry::new_project_bound(
            fixture.connection_id(),
            Some(fixture.project_id()),
            &volicord_command,
        );
        let plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id: "guard_applied_statuses",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;
        let preflight: Value = serde_json::from_str(&host_hook_capability_json(&plan)?)?;
        assert!(preflight["files"]
            .as_array()
            .expect("preflight files")
            .iter()
            .all(|file| file["status"] == "planned_create"));

        let applied = apply_guard_integration(plan)?;
        let installation = record_guard_installation(
            fixture.runtime_home_path(),
            HostKind::Codex,
            IntegrationProfile::Record,
            GuardInstallationStatus::Configured,
            fixture.connection_id(),
            fixture.project_id(),
            &applied,
        )?;
        let persisted: Value = serde_json::from_str(&installation.host_capability_json)?;
        assert!(host_hook_capability_has_exact_current_shape(&persisted));
        let persisted_files = persisted["files"].as_array().expect("persisted files");
        assert!(persisted_files
            .iter()
            .all(|file| matches!(file["status"].as_str(), Some("created" | "unchanged"))));
        assert!(persisted_files
            .iter()
            .any(|file| file["status"] == "created"));
        assert_ne!(persisted["files"], preflight["files"]);
        Ok(())
    }
}
