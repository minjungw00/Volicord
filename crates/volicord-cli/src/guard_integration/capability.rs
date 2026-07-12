use std::{path::Path, time::SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use volicord_store::guards::{
    upsert_guard_installation, GuardInstallationRecord, GuardInstallationUpsert,
};
use volicord_types::{GuardInstallationStatus, IntegrationProfile};

use crate::{
    guard_integration::{
        audit::{hook_wrapper_comment_value, hook_wrapper_exec_command, sha256_text},
        files::{
            FilePlanStatus, GeneratedFilePlan, GeneratedFileWriteKind, ManagedFileRetirementPlan,
            RetirementPlanStatus, VOLICORD_POLICY_FILE,
        },
        policy::{
            guard_has_prompt_capture_commands, lifecycle_phase_names, required_guard_phase_names,
        },
        GuardIntegrationError, GuardIntegrationPlan, HookWrapperResolutionStatus, HostHookCommand,
        HostHookCommandShape, HOOK_WRAPPER_MARKER,
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
        installed_at: (profile != IntegrationProfile::Record).then_some(now.clone()),
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
            "observation_status": if profile == IntegrationProfile::Record {
                "disabled"
            } else {
                "not_observed"
            },
        }))
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?,
    })
}

pub(crate) fn host_hook_capability_json(
    plan: &GuardIntegrationPlan,
) -> Result<String, GuardIntegrationError> {
    let capabilities = serde_json::to_value(plan.capabilities)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    serde_json::to_string(&json!({
        "schema": "volicord-host-hook-capability-v1",
        "policy_hash": plan.policy_hash,
        "selected_profile": plan.guard_profile,
        "connection_intent": plan.connection_intent,
        "native_host_output_adapter": plan.native_host_output_adapter,
        "native_host_output_adapter_verified": plan.native_host_output_adapter_verified,
        "bash_shell_mutation_coverage": plan.bash_shell_mutation_coverage,
        "direct_file_write_matcher_coverage": plan.direct_file_write_matcher_coverage,
        "host_capabilities": capabilities,
        "required_hook_phases": required_guard_phase_names(),
        "missing_required_hooks": lifecycle_phase_names(&plan.missing_required_hooks),
        "prompt_capture": plan.capabilities.user_prompt_submit_hook
            && guard_has_prompt_capture_commands(&plan.policy),
        "files": generated_files_json(&plan.generated_files),
        "host_hook_commands": host_hook_commands_json(&plan.host_hook_commands),
        "hook_root_resolution": hook_root_resolution_json(&plan.host_hook_commands),
        "hook_path_safety": hook_path_safety_json(&plan.host_hook_commands),
        "commands": plan.policy["host_hook"]["commands"].clone(),
    }))
    .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

pub(crate) fn initial_guard_installation_status(
    profile: IntegrationProfile,
    host_plan: &HostPlan,
    integration: &GuardIntegrationPlan,
) -> GuardInstallationStatus {
    if profile == IntegrationProfile::Record {
        GuardInstallationStatus::Configured
    } else if !integration.missing_required_hooks.is_empty() {
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
                    GeneratedFileWriteKind::JsonProjection { projection } => {
                        object.insert(
                            "ownership".to_owned(),
                            Value::String("managed_json_projection".to_owned()),
                        );
                        object.insert(
                            "managed_projection".to_owned(),
                            Value::String(projection.as_str().to_owned()),
                        );
                        object.insert(
                            "managed_projection_json".to_owned(),
                            Value::String(file.content.clone()),
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
                        object.insert(
                            "executable_required".to_owned(),
                            Value::Bool(script_executable_required()),
                        );
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
                    HostHookCommandShape::ShellCommandString(command) => {
                        (command.clone(), Value::Null)
                    }
                    HostHookCommandShape::Exec { command, args } => (
                        command.clone(),
                        Value::Array(args.iter().cloned().map(Value::String).collect()),
                    ),
                };
                json!({
                    "host_kind": command.host_kind.as_str(),
                    "phase": command.phase.capability_name(),
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

pub(crate) fn hook_path_safety_json(commands: &[HostHookCommand]) -> Value {
    if commands.is_empty() {
        return Value::Null;
    }
    let all_cwd_independent = commands.iter().all(|command| command.cwd_independent);
    let all_subdirectory_safe = commands.iter().all(|command| command.subdirectory_safe);
    let all_ok = all_cwd_independent
        && all_subdirectory_safe
        && commands
            .iter()
            .all(|command| command.wrapper_resolution_status == HookWrapperResolutionStatus::Ok);
    json!({
        "overall_status": if all_ok { "ok" } else { "relative_path_unsafe" },
        "all_cwd_independent": all_cwd_independent,
        "all_subdirectory_safe": all_subdirectory_safe,
        "commands": commands
            .iter()
            .map(|command| {
                json!({
                    "phase": command.phase.capability_name(),
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

#[cfg(unix)]
fn script_executable_required() -> bool {
    true
}

#[cfg(not(unix))]
fn script_executable_required() -> bool {
    false
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
