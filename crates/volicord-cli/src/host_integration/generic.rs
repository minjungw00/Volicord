use std::path::Path;

use serde_json::{Map, Value};

use super::{
    config_edit::{read_json_object, remove_file_if_fresh, write_json_object_if_fresh},
    current_entry_fingerprint_from_json, managed_fingerprint, validated_server_name,
    ConnectionIntent, HostAdapter, HostCapabilities, HostConfigError, HostConflict,
    HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, ManagedServerEntry,
    PlannedChange,
};
use crate::host_integration::verification::{
    HostConfigurationStatus, HostExecutableStatus, HostGateStatus, ManagedConfigStatus,
    Verification,
};

#[derive(Debug, Clone, Default)]
pub struct GenericAdapter;

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        http_mcp: false,
        session_start_hook: false,
        pre_tool_hook: false,
        post_tool_hook: false,
        user_prompt_submit_hook: false,
        stop_hook: false,
        rule_file_support: false,
        project_local_configuration: false,
    }
}

impl GenericAdapter {
    pub fn plan(&self, request: HostPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        let _ = request;
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidScope,
            "generic MCP configuration is export-only and is not an ordinary host connection plan",
        )))
    }

    pub fn plan_export(
        &self,
        request: GenericExportRequest<'_>,
    ) -> Result<HostPlan, HostConfigError> {
        let server_name = validated_server_name(request.connection_id, None)?;
        let target = request.target_path.to_path_buf();
        let entry = ManagedServerEntry::new(
            request.connection_id,
            request.installation_profile.volicord_mcp_command,
            Some(request.installation_profile.runtime_home),
        );
        let fingerprint =
            managed_fingerprint(HostKind::Generic, HostScope::Export, &server_name, &entry);
        let (snapshot, object) = read_json_object(&target)?;
        if object
            .get("mcpServers")
            .is_some_and(|value| !value.is_object())
        {
            return Err(HostConfigError::Malformed(
                "generic export mcpServers must be an object".to_owned(),
            ));
        }
        let existing = object
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&server_name));
        let mut conflicts = Vec::new();
        let change = match existing {
            None if object.is_empty() => PlannedChange::Create,
            None => {
                conflicts.push(HostConflict::new(
                    HostConflictKind::UnmanagedNameCollision,
                    format!(
                        "generic export target already contains unrelated configuration: {}",
                        target.display()
                    ),
                ));
                PlannedChange::Noop
            }
            Some(existing) => {
                let current = current_entry_fingerprint_from_json(
                    HostKind::Generic,
                    HostScope::Export,
                    &server_name,
                    existing,
                );
                if current.as_deref() == Some(fingerprint.as_str()) {
                    PlannedChange::Noop
                } else if current.as_deref() == request.expected_fingerprint {
                    PlannedChange::Update
                } else {
                    conflicts.push(HostConflict::new(
                        HostConflictKind::UnmanagedNameCollision,
                        format!(
                            "generic export server name is already configured by an unrelated entry: {server_name}"
                        ),
                    ));
                    PlannedChange::Noop
                }
            }
        };

        Ok(HostPlan {
            host_kind: HostKind::Generic,
            connection_intent: ConnectionIntent::Personal,
            host_scope: HostScope::Export,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::Export(target),
            entry,
            change,
            fingerprint,
            conflicts,
            user_actions: vec![super::UserAction::new(
                super::UserActionKind::HostTrustRequired,
                "generic export must be loaded, trusted, or approved in the target host by the user",
            )],
            file_snapshot: Some(snapshot),
        })
    }
}

impl HostAdapter for GenericAdapter {
    fn capabilities(&self) -> HostCapabilities {
        capabilities()
    }

    fn detect(&self) -> Result<HostDetection, HostConfigError> {
        Ok(HostDetection {
            host_kind: HostKind::Generic,
            available: true,
            details: "generic export fallback is available".to_owned(),
        })
    }

    fn apply(&mut self, plan: &HostPlan) -> Result<HostEffect, HostConfigError> {
        if let Some(conflict) = plan.conflicts.first() {
            return Err(HostConfigError::Conflict(conflict.clone()));
        }
        if plan.change == PlannedChange::Noop {
            return Ok(effect_from_plan(plan));
        }
        let HostTarget::Export(target) = &plan.target else {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "generic plan target must be an export file",
            )));
        };
        let snapshot = plan.file_snapshot.as_ref().ok_or_else(|| {
            HostConfigError::StalePlan("generic plan is missing its file snapshot".to_owned())
        })?;
        let object = export_object(&plan.server_name, &plan.entry);
        write_json_object_if_fresh(target, &object, snapshot)?;
        Ok(effect_from_plan(plan))
    }

    fn verify(&mut self, plan: &HostPlan) -> Result<Verification, HostConfigError> {
        if let Some(conflict) = plan.conflicts.first() {
            return Ok(Verification::changed(conflict.message.clone())
                .merge_user_actions(&plan.user_actions));
        }
        let managed = verify_generic_export(plan)?;
        if managed != ManagedConfigStatus::Match {
            return Ok(verification_from_managed_status(
                managed,
                format!(
                    "generic export managed MCP entry is {} for {}",
                    managed.as_str(),
                    plan.server_name
                ),
            )
            .merge_user_actions(&plan.user_actions));
        }
        Ok(Verification::action_required(
            "generic export is valid, but external host loading remains user-managed and unverified",
        )
        .with_host_executable(HostExecutableStatus::NotRequired)
        .with_host_gate(HostGateStatus::ActionRequired)
        .with_mcp_handshake_allowed(true)
        .merge_user_actions(&plan.user_actions))
    }

    fn remove(&mut self, request: HostRemoveRequest) -> Result<HostEffect, HostConfigError> {
        if request.host_kind != HostKind::Generic {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "generic adapter cannot remove a non-generic host target",
            )));
        }
        let HostTarget::Export(target) = &request.target else {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "generic removal target must be an export file",
            )));
        };
        let (snapshot, object) = read_json_object(target)?;
        let existing = object
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&request.server_name));
        let Some(existing) = existing else {
            return Ok(remove_effect(request, PlannedChange::Noop));
        };
        let current = current_entry_fingerprint_from_json(
            HostKind::Generic,
            HostScope::Export,
            &request.server_name,
            existing,
        );
        if current.as_deref() != Some(request.expected_fingerprint.as_str()) {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::FingerprintMismatch,
                format!(
                    "generic export changed since Volicord last managed it: {}",
                    request.server_name
                ),
            )));
        }
        remove_file_if_fresh(target, &snapshot)?;
        Ok(remove_effect(request, PlannedChange::Remove))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenericExportRequest<'a> {
    pub connection_id: &'a str,
    pub installation_profile: InstallationProfile<'a>,
    pub mode: &'a str,
    pub target_path: &'a Path,
    pub expected_fingerprint: Option<&'a str>,
}

pub fn export_object(server_name: &str, entry: &ManagedServerEntry) -> Map<String, Value> {
    let mut servers = Map::new();
    servers.insert(server_name.to_owned(), entry.to_json_value());
    let mut root = Map::new();
    root.insert("mcpServers".to_owned(), Value::Object(servers));
    root
}

fn verify_generic_export(plan: &HostPlan) -> Result<ManagedConfigStatus, HostConfigError> {
    let HostTarget::Export(target) = &plan.target else {
        return Ok(ManagedConfigStatus::Unknown);
    };
    let (_, object) = match read_json_object(target) {
        Ok(result) => result,
        Err(HostConfigError::Malformed(_)) => return Ok(ManagedConfigStatus::Malformed),
        Err(error) => return Err(error),
    };
    let Some(existing) = object
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(&plan.server_name))
    else {
        return Ok(ManagedConfigStatus::Missing);
    };
    let current = current_entry_fingerprint_from_json(
        HostKind::Generic,
        HostScope::Export,
        &plan.server_name,
        existing,
    );
    match current {
        Some(fingerprint) if fingerprint == plan.fingerprint => Ok(ManagedConfigStatus::Match),
        Some(_) => Ok(ManagedConfigStatus::Changed),
        None => Ok(ManagedConfigStatus::Malformed),
    }
}

fn verification_from_managed_status(status: ManagedConfigStatus, details: String) -> Verification {
    match status {
        ManagedConfigStatus::Missing => Verification::missing(details),
        ManagedConfigStatus::Changed => Verification::changed(details),
        ManagedConfigStatus::Malformed => Verification::failed(details)
            .with_managed_config(ManagedConfigStatus::Malformed)
            .with_host_configuration(HostConfigurationStatus::Malformed),
        ManagedConfigStatus::Match => Verification::action_required(details),
        ManagedConfigStatus::NotApplicable | ManagedConfigStatus::Unknown => {
            Verification::unknown(details)
        }
    }
}

fn effect_from_plan(plan: &HostPlan) -> HostEffect {
    HostEffect {
        host_kind: plan.host_kind,
        connection_intent: plan.connection_intent,
        host_scope: plan.host_scope,
        mode: plan.mode.clone(),
        server_name: plan.server_name.clone(),
        target: plan.target.clone(),
        change: plan.change,
        fingerprint: plan.fingerprint.clone(),
        user_actions: plan.user_actions.clone(),
    }
}

fn remove_effect(request: HostRemoveRequest, change: PlannedChange) -> HostEffect {
    HostEffect {
        host_kind: request.host_kind,
        connection_intent: request.connection_intent,
        host_scope: request.host_scope,
        mode: request.mode,
        server_name: request.server_name,
        target: request.target,
        change,
        fingerprint: request.expected_fingerprint,
        user_actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn integration_specific_filename_command_and_environment_contract(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-file")?;
        let adapter = GenericAdapter;

        let target = dir.join("volicord.mcp.json");
        let plan = adapter.plan_export(request(&target, Path::new("/bin/volicord")))?;

        assert_eq!(plan.target, HostTarget::Export(target));
        assert_eq!(plan.entry.command, "/bin/volicord");
        assert_eq!(
            plan.entry.args,
            ["mcp", "--stdio", "--connection", "int_alpha"]
        );
        let expected_env =
            std::collections::BTreeMap::from([("VOLICORD_HOME".to_owned(), "/runtime".to_owned())]);
        assert_eq!(
            plan.entry.env, expected_env,
            "generic export environment should contain only the supported MCP process environment input"
        );
        Ok(())
    }

    #[test]
    fn ordinary_connection_planning_is_rejected() {
        let adapter = GenericAdapter;
        let error = adapter
            .plan(HostPlanRequest {
                host_kind: HostKind::Generic,
                connection_intent: ConnectionIntent::Personal,
                project: None,
                installation_profile: InstallationProfile {
                    runtime_home: Path::new("/runtime"),
                    volicord_command: Path::new("/bin/volicord"),
                    volicord_mcp_command: Path::new("/bin/volicord"),
                    default_connection_mode: "workflow",
                },
                connection_id: "int_alpha",
                mode: "workflow",
                expected_fingerprint: None,
            })
            .expect_err("generic planning should be export-only");

        assert!(matches!(error, HostConfigError::Conflict(_)));
    }

    #[test]
    fn unrelated_existing_file_is_a_conflict() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-conflict")?;
        let target = dir.join("volicord.mcp.json");
        fs::write(
            &target,
            "{\"mcpServers\":{\"other\":{\"command\":\"x\"}}}\n",
        )?;
        let adapter = GenericAdapter;

        let plan = adapter.plan_export(request(&target, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::UnmanagedNameCollision
        );
        assert_eq!(
            fs::read_to_string(target)?,
            "{\"mcpServers\":{\"other\":{\"command\":\"x\"}}}\n"
        );
        Ok(())
    }

    #[test]
    fn safe_owned_update_and_removal() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-owned")?;
        let mut adapter = GenericAdapter;
        let target = dir.join("volicord.mcp.json");
        let first = adapter.plan_export(request(&target, Path::new("/bin/volicord")))?;
        adapter.apply(&first)?;
        let second = adapter.plan_export(GenericExportRequest {
            expected_fingerprint: Some(&first.fingerprint),
            installation_profile: InstallationProfile {
                volicord_mcp_command: Path::new("/usr/local/bin/volicord"),
                ..request(&target, Path::new("/bin/volicord")).installation_profile
            },
            ..request(&target, Path::new("/bin/volicord"))
        })?;
        assert_eq!(second.change, PlannedChange::Update);
        adapter.apply(&second)?;
        let HostTarget::Export(target) = second.target.clone() else {
            unreachable!("generic target");
        };

        let effect = adapter.remove(HostRemoveRequest {
            host_kind: HostKind::Generic,
            connection_intent: second.connection_intent,
            host_scope: HostScope::Export,
            mode: second.mode.clone(),
            server_name: second.server_name,
            target: HostTarget::Export(target.clone()),
            expected_fingerprint: second.fingerprint,
        })?;

        assert_eq!(effect.change, PlannedChange::Remove);
        assert!(!target.exists());
        Ok(())
    }

    #[test]
    fn removal_refuses_fingerprint_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-remove-mismatch")?;
        let mut adapter = GenericAdapter;
        let target = dir.join("volicord.mcp.json");
        let plan = adapter.plan_export(request(&target, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;
        let HostTarget::Export(target) = plan.target.clone() else {
            unreachable!("generic target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/volicord", "/tmp/manual"),
        )?;

        let error = adapter
            .remove(HostRemoveRequest {
                host_kind: HostKind::Generic,
                connection_intent: plan.connection_intent,
                host_scope: HostScope::Export,
                mode: plan.mode.clone(),
                server_name: plan.server_name,
                target: HostTarget::Export(target),
                expected_fingerprint: plan.fingerprint,
            })
            .expect_err("manual modification should block removal");

        assert!(matches!(error, HostConfigError::Conflict(_)));
        Ok(())
    }

    #[test]
    fn verify_valid_export_remains_action_required_and_detects_changes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-verify")?;
        let mut adapter = GenericAdapter;
        let target = dir.join("volicord.mcp.json");
        let plan = adapter.plan_export(request(&target, Path::new("/bin/volicord")))?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "missing");
        adapter.apply(&plan)?;
        let verification = adapter.verify(&plan)?;
        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_state.as_str(),
            "configured_action_required"
        );
        assert_eq!(
            verification.user_actions[0].kind,
            crate::host_integration::UserActionKind::HostTrustRequired
        );
        assert!(verification.mcp_handshake_allowed);
        let HostTarget::Export(target) = plan.target.clone() else {
            unreachable!("generic target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/volicord", "/tmp/manual"),
        )?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "changed");
        fs::write(&target, "{")?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "failed");
        Ok(())
    }

    fn request<'a>(target_path: &'a Path, mcp_command: &'a Path) -> GenericExportRequest<'a> {
        GenericExportRequest {
            connection_id: "int_alpha",
            installation_profile: InstallationProfile {
                runtime_home: Path::new("/runtime"),
                volicord_command: Path::new("/bin/volicord"),
                volicord_mcp_command: mcp_command,
                default_connection_mode: "workflow",
            },
            mode: "workflow",
            target_path,
            expected_fingerprint: None,
        }
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
