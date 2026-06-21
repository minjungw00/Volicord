use std::path::Path;

use serde_json::{Map, Value};

use super::{
    config_edit::{read_json_object, remove_file_if_fresh, write_json_object_if_fresh},
    current_entry_fingerprint_from_json, export_file_name, managed_fingerprint,
    validated_server_name, HostAdapter, HostConfigError, HostConflict, HostConflictKind,
    HostDetection, HostEffect, HostKind, HostPlan, HostRemoveRequest, HostScope, HostTarget,
    ManagedServerEntry, PlannedChange,
};
use crate::host_integration::verification::{Verification, VerificationStatus};

#[derive(Debug, Clone, Default)]
pub struct GenericAdapter;

impl GenericAdapter {
    pub fn plan(&self, request: GenericPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        if request.scope != HostScope::Export {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "generic host integration supports only export scope",
            )));
        }
        if !request.mcp_command.is_absolute() {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidCommand,
                "generic export requires an absolute harness-mcp command path",
            )));
        }

        let server_name =
            validated_server_name(request.integration_id, request.explicit_server_name)?;
        let target = request
            .output_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| {
                request
                    .output_dir
                    .join(export_file_name(request.integration_id))
            });
        let entry = ManagedServerEntry::new(
            request.integration_id,
            request.mcp_command,
            request.runtime_home,
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
            host_scope: HostScope::Export,
            server_name,
            target: HostTarget::Export(target),
            entry,
            change,
            fingerprint,
            conflicts,
            user_actions: Vec::new(),
            file_snapshot: Some(snapshot),
        })
    }
}

impl HostAdapter for GenericAdapter {
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
        if plan.conflicts.is_empty() {
            Ok(Verification::new(
                VerificationStatus::NotVerified,
                "generic export does not claim direct host loading",
            ))
        } else {
            Ok(Verification::new(
                VerificationStatus::Failed,
                plan.conflicts[0].message.clone(),
            ))
        }
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
                    "generic export changed since Harness last managed it: {}",
                    request.server_name
                ),
            )));
        }
        remove_file_if_fresh(target, &snapshot)?;
        Ok(remove_effect(request, PlannedChange::Remove))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenericPlanRequest<'a> {
    pub scope: HostScope,
    pub integration_id: &'a str,
    pub explicit_server_name: Option<&'a str>,
    pub output_dir: &'a Path,
    pub output_path: Option<&'a Path>,
    pub mcp_command: &'a Path,
    pub runtime_home: Option<&'a Path>,
    pub expected_fingerprint: Option<&'a str>,
}

pub fn export_object(server_name: &str, entry: &ManagedServerEntry) -> Map<String, Value> {
    let mut servers = Map::new();
    servers.insert(server_name.to_owned(), entry.to_json_value());
    let mut root = Map::new();
    root.insert("mcpServers".to_owned(), Value::Object(servers));
    root
}

fn effect_from_plan(plan: &HostPlan) -> HostEffect {
    HostEffect {
        host_kind: plan.host_kind,
        host_scope: plan.host_scope,
        server_name: plan.server_name.clone(),
        target: plan.target.clone(),
        change: plan.change,
        fingerprint: plan.fingerprint.clone(),
    }
}

fn remove_effect(request: HostRemoveRequest, change: PlannedChange) -> HostEffect {
    HostEffect {
        host_kind: request.host_kind,
        host_scope: request.host_scope,
        server_name: request.server_name,
        target: request.target,
        change,
        fingerprint: request.expected_fingerprint,
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
    fn integration_specific_filename_and_command_shape() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-file")?;
        let adapter = GenericAdapter;

        let plan = adapter.plan(request(&dir, None, Path::new("/bin/harness-mcp")))?;

        assert_eq!(
            plan.target,
            HostTarget::Export(dir.join("harness-int_alpha.mcp.json"))
        );
        assert_eq!(plan.entry.command, "/bin/harness-mcp");
        assert_eq!(plan.entry.args, ["--integration", "int_alpha"]);
        assert_eq!(
            plan.entry.env.get("HARNESS_HOME"),
            Some(&"/runtime".to_owned())
        );
        assert!(!plan.entry.env.contains_key("HARNESS_PROJECT_ID"));
        assert!(!plan.entry.env.contains_key("HARNESS_SURFACE_ID"));
        Ok(())
    }

    #[test]
    fn unrelated_existing_file_is_a_conflict() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("generic-conflict")?;
        let target = dir.join("harness-int_alpha.mcp.json");
        fs::write(
            &target,
            "{\"mcpServers\":{\"other\":{\"command\":\"x\"}}}\n",
        )?;
        let adapter = GenericAdapter;

        let plan = adapter.plan(request(&dir, None, Path::new("/bin/harness-mcp")))?;

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
        let first = adapter.plan(request(&dir, None, Path::new("/bin/harness-mcp")))?;
        adapter.apply(&first)?;
        let second = adapter.plan(GenericPlanRequest {
            expected_fingerprint: Some(&first.fingerprint),
            mcp_command: Path::new("/usr/local/bin/harness-mcp"),
            ..request(&dir, None, Path::new("/bin/harness-mcp"))
        })?;
        assert_eq!(second.change, PlannedChange::Update);
        adapter.apply(&second)?;
        let HostTarget::Export(target) = second.target.clone() else {
            unreachable!("generic target");
        };

        let effect = adapter.remove(HostRemoveRequest {
            host_kind: HostKind::Generic,
            host_scope: HostScope::Export,
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
        let plan = adapter.plan(request(&dir, None, Path::new("/bin/harness-mcp")))?;
        adapter.apply(&plan)?;
        let HostTarget::Export(target) = plan.target.clone() else {
            unreachable!("generic target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/harness-mcp", "/tmp/manual"),
        )?;

        let error = adapter
            .remove(HostRemoveRequest {
                host_kind: HostKind::Generic,
                host_scope: HostScope::Export,
                server_name: plan.server_name,
                target: HostTarget::Export(target),
                expected_fingerprint: plan.fingerprint,
            })
            .expect_err("manual modification should block removal");

        assert!(matches!(error, HostConfigError::Conflict(_)));
        Ok(())
    }

    fn request<'a>(
        output_dir: &'a Path,
        output_path: Option<&'a Path>,
        mcp_command: &'a Path,
    ) -> GenericPlanRequest<'a> {
        GenericPlanRequest {
            scope: HostScope::Export,
            integration_id: "int_alpha",
            explicit_server_name: None,
            output_dir,
            output_path,
            mcp_command,
            runtime_home: Some(Path::new("/runtime")),
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
