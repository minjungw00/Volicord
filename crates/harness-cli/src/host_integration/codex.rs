use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use toml_edit::{value, Array, DocumentMut, Item, Table};

use super::{
    config_edit::{read_text_snapshot, write_if_fresh, FileSnapshot},
    managed_fingerprint, unmanaged_fingerprint, validated_server_name, HostAdapter,
    HostConfigError, HostConflict, HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan,
    HostRemoveRequest, HostScope, HostTarget, ManagedServerEntry, PlannedChange, UserAction,
    UserActionKind,
};
use crate::host_integration::verification::{Verification, VerificationStatus};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexEnvironment {
    pub home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CodexAdapter {
    env: CodexEnvironment,
}

impl CodexAdapter {
    pub fn new(env: CodexEnvironment) -> Self {
        Self { env }
    }

    pub fn plan(&self, request: CodexPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        if !matches!(request.scope, HostScope::User | HostScope::Project) {
            return Ok(conflicted_plan(
                HostScope::Project,
                request.integration_id,
                request.explicit_server_name,
                request.mcp_command,
                HostConflict::new(
                    HostConflictKind::InvalidScope,
                    "Codex supports only user and project host scopes",
                ),
            ));
        }
        validate_mcp_command(request.scope, request.mcp_command)?;
        if request.scope == HostScope::Project && request.runtime_home.is_some() {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidCommand,
                "Codex project-scoped configuration must not embed a personal HARNESS_HOME",
            )));
        }

        let server_name =
            validated_server_name(request.integration_id, request.explicit_server_name)?;
        let target = self.config_path(request.scope, request.repo_root)?;
        let entry = ManagedServerEntry::new(
            request.integration_id,
            request.mcp_command,
            request.runtime_home,
        );
        let fingerprint = managed_fingerprint(HostKind::Codex, request.scope, &server_name, &entry);
        let (snapshot, text) = read_text_snapshot(&target)?;
        let document = parse_document(text.as_deref(), &target)?;
        if document.as_table().contains_key("mcp_servers")
            && document
                .get("mcp_servers")
                .and_then(Item::as_table)
                .is_none()
        {
            return Err(HostConfigError::Malformed(
                "Codex mcp_servers configuration must be a table".to_owned(),
            ));
        }
        let existing = document
            .get("mcp_servers")
            .and_then(Item::as_table)
            .and_then(|servers| servers.get(&server_name));
        let mut conflicts = Vec::new();
        let change = match existing {
            None => PlannedChange::Create,
            Some(item) => {
                let current = codex_entry_fingerprint(request.scope, &server_name, item);
                if current.as_deref() == Some(fingerprint.as_str()) {
                    PlannedChange::Noop
                } else if current.as_deref() == request.expected_fingerprint {
                    PlannedChange::Update
                } else {
                    conflicts.push(HostConflict::new(
                        HostConflictKind::UnmanagedNameCollision,
                        format!(
                            "Codex MCP server name is already configured by an unrelated entry: {server_name}"
                        ),
                    ));
                    PlannedChange::Noop
                }
            }
        };
        let mut user_actions = Vec::new();
        if request.scope == HostScope::Project {
            user_actions.push(UserAction::new(
                UserActionKind::HostTrustRequired,
                "Codex project configuration is loaded only after the project is trusted",
            ));
        }

        Ok(HostPlan {
            host_kind: HostKind::Codex,
            host_scope: request.scope,
            server_name,
            target: HostTarget::File(target),
            entry,
            change,
            fingerprint,
            conflicts,
            user_actions,
            file_snapshot: Some(snapshot),
        })
    }

    fn config_path(
        &self,
        scope: HostScope,
        repo_root: Option<&Path>,
    ) -> Result<PathBuf, HostConfigError> {
        match scope {
            HostScope::User => Ok(self.codex_home()?.join("config.toml")),
            HostScope::Project => {
                let repo_root = repo_root.ok_or_else(|| {
                    HostConfigError::Conflict(HostConflict::new(
                        HostConflictKind::InvalidScope,
                        "Codex project scope requires a Product Repository root",
                    ))
                })?;
                Ok(repo_root.join(".codex").join("config.toml"))
            }
            _ => Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex supports only user and project host scopes",
            ))),
        }
    }

    fn codex_home(&self) -> Result<PathBuf, HostConfigError> {
        if let Some(path) = &self.env.codex_home {
            return Ok(path.clone());
        }
        let home = self.env.home.as_ref().ok_or_else(|| {
            HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Codex user configuration requires CODEX_HOME or HOME",
            ))
        })?;
        Ok(home.join(".codex"))
    }
}

impl HostAdapter for CodexAdapter {
    fn detect(&self) -> Result<HostDetection, HostConfigError> {
        let path = self.codex_home()?.join("config.toml");
        Ok(HostDetection {
            host_kind: HostKind::Codex,
            available: true,
            details: format!("Codex user configuration target: {}", path.display()),
        })
    }

    fn apply(&mut self, plan: &HostPlan) -> Result<HostEffect, HostConfigError> {
        if plan.host_kind != HostKind::Codex {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex adapter cannot apply a non-Codex host plan",
            )));
        }
        if let Some(conflict) = plan.conflicts.first() {
            return Err(HostConfigError::Conflict(conflict.clone()));
        }
        if plan.change == PlannedChange::Noop {
            return Ok(effect_from_plan(plan));
        }
        let HostTarget::File(target) = &plan.target else {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Codex plan target must be a file",
            )));
        };
        let snapshot = plan.file_snapshot.as_ref().ok_or_else(|| {
            HostConfigError::StalePlan("Codex plan is missing its file snapshot".to_owned())
        })?;
        let mut document = document_from_snapshot(snapshot, target)?;
        upsert_server_table(&mut document, &plan.server_name, &plan.entry)?;
        write_if_fresh(target, document.to_string().as_bytes(), snapshot)?;
        Ok(effect_from_plan(plan))
    }

    fn verify(&mut self, plan: &HostPlan) -> Result<Verification, HostConfigError> {
        if let Some(conflict) = plan.conflicts.first() {
            return Ok(Verification::new(
                VerificationStatus::Failed,
                conflict.message.clone(),
            ));
        }
        if plan.host_scope == HostScope::Project {
            return Ok(Verification::new(
                VerificationStatus::ActionRequired,
                "Codex project trust was not confirmed by this structural configuration check",
            ));
        }
        Ok(Verification::new(
            VerificationStatus::NotVerified,
            "Codex host loading was not launched by this adapter",
        ))
    }

    fn remove(&mut self, request: HostRemoveRequest) -> Result<HostEffect, HostConfigError> {
        if request.host_kind != HostKind::Codex {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex adapter cannot remove a non-Codex host plan",
            )));
        }
        let HostTarget::File(target) = &request.target else {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::UnsafeTarget,
                "Codex removal target must be a file",
            )));
        };
        let (snapshot, text) = read_text_snapshot(target)?;
        let mut document = parse_document(text.as_deref(), target)?;
        let Some(servers) = document.get_mut("mcp_servers").and_then(Item::as_table_mut) else {
            return Ok(remove_effect(request, PlannedChange::Noop));
        };
        let Some(existing) = servers.get(&request.server_name) else {
            return Ok(remove_effect(request, PlannedChange::Noop));
        };
        let current = codex_entry_fingerprint(request.host_scope, &request.server_name, existing);
        if current.as_deref() != Some(request.expected_fingerprint.as_str()) {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::FingerprintMismatch,
                format!(
                    "Codex MCP server changed since Harness last managed it: {}",
                    request.server_name
                ),
            )));
        }
        servers.remove(&request.server_name);
        write_if_fresh(target, document.to_string().as_bytes(), &snapshot)?;
        Ok(remove_effect(request, PlannedChange::Remove))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CodexPlanRequest<'a> {
    pub scope: HostScope,
    pub integration_id: &'a str,
    pub explicit_server_name: Option<&'a str>,
    pub repo_root: Option<&'a Path>,
    pub mcp_command: &'a Path,
    pub runtime_home: Option<&'a Path>,
    pub expected_fingerprint: Option<&'a str>,
}

fn validate_mcp_command(scope: HostScope, command: &Path) -> Result<(), HostConfigError> {
    if scope == HostScope::Project {
        if command == Path::new("harness-mcp") {
            return Ok(());
        }
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Codex project-scoped configuration must use harness-mcp from PATH",
        )));
    }
    if command.is_absolute() {
        Ok(())
    } else {
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Codex user-scoped configuration requires an absolute harness-mcp command path",
        )))
    }
}

fn parse_document(text: Option<&str>, target: &Path) -> Result<DocumentMut, HostConfigError> {
    match text {
        None => Ok(DocumentMut::new()),
        Some(text) if text.trim().is_empty() => Ok(DocumentMut::new()),
        Some(text) => text.parse::<DocumentMut>().map_err(|error| {
            HostConfigError::Malformed(format!(
                "failed to parse Codex TOML configuration {}: {error}",
                target.display()
            ))
        }),
    }
}

fn document_from_snapshot(
    snapshot: &FileSnapshot,
    target: &Path,
) -> Result<DocumentMut, HostConfigError> {
    match snapshot {
        FileSnapshot::Missing => Ok(DocumentMut::new()),
        FileSnapshot::Present { bytes } => {
            let text = String::from_utf8(bytes.clone()).map_err(|error| {
                HostConfigError::Malformed(format!(
                    "Codex configuration is not UTF-8 text {}: {error}",
                    target.display()
                ))
            })?;
            parse_document(Some(&text), target)
        }
    }
}

fn upsert_server_table(
    document: &mut DocumentMut,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> Result<(), HostConfigError> {
    if !document.as_table().contains_key("mcp_servers") {
        document["mcp_servers"] = Item::Table(Table::new());
    }
    let servers = document
        .get_mut("mcp_servers")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| {
            HostConfigError::Malformed("Codex mcp_servers configuration must be a table".to_owned())
        })?;
    servers.insert(server_name, Item::Table(server_table(entry)));
    Ok(())
}

fn server_table(entry: &ManagedServerEntry) -> Table {
    let mut table = Table::new();
    table["command"] = value(entry.command.clone());
    let mut args = Array::default();
    for arg in &entry.args {
        args.push(arg.as_str());
    }
    table["args"] = value(args);
    if !entry.env.is_empty() {
        let mut env = Table::new();
        for (key, value_text) in &entry.env {
            env[key] = value(value_text.clone());
        }
        table["env"] = Item::Table(env);
    }
    table
}

fn codex_entry_fingerprint(scope: HostScope, server_name: &str, item: &Item) -> Option<String> {
    let table = item.as_table()?;
    let allowed_keys = ["command", "args", "env"];
    if table.iter().any(|(key, _)| !allowed_keys.contains(&key)) {
        return Some(unmanaged_fingerprint(
            HostKind::Codex,
            scope,
            server_name,
            &item.to_string(),
        ));
    }
    let command = table.get("command")?.as_str()?.to_owned();
    let args = table
        .get("args")
        .and_then(Item::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_else(|| Some(Vec::new()))?;
    let env = table
        .get("env")
        .and_then(Item::as_table)
        .map(|items| {
            items
                .iter()
                .map(|(key, item)| {
                    item.as_str()
                        .map(|value| (key.to_owned(), value.to_owned()))
                })
                .collect::<Option<BTreeMap<_, _>>>()
        })
        .unwrap_or_else(|| Some(BTreeMap::new()))?;
    Some(managed_fingerprint(
        HostKind::Codex,
        scope,
        server_name,
        &ManagedServerEntry { command, args, env },
    ))
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

fn conflicted_plan(
    scope: HostScope,
    integration_id: &str,
    explicit_server_name: Option<&str>,
    command: &Path,
    conflict: HostConflict,
) -> HostPlan {
    let server_name = validated_server_name(integration_id, explicit_server_name)
        .unwrap_or_else(|_| super::default_server_name(integration_id));
    let entry = ManagedServerEntry::new(integration_id, command, None);
    let fingerprint = managed_fingerprint(HostKind::Codex, scope, &server_name, &entry);
    HostPlan {
        host_kind: HostKind::Codex,
        host_scope: scope,
        server_name,
        target: HostTarget::File(PathBuf::new()),
        entry,
        change: PlannedChange::Noop,
        fingerprint,
        conflicts: vec![conflict],
        user_actions: Vec::new(),
        file_snapshot: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn user_config_path_defaults_to_home_codex() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-home-default")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.clone()),
            codex_home: None,
        });

        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;

        assert_eq!(
            plan.target,
            HostTarget::File(dir.join(".codex").join("config.toml"))
        );
        Ok(())
    }

    #[test]
    fn user_config_path_honors_codex_home() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-home-override")?;
        let codex_home = dir.join("custom-codex");
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir),
            codex_home: Some(codex_home.clone()),
        });

        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;

        assert_eq!(
            plan.target,
            HostTarget::File(codex_home.join("config.toml"))
        );
        Ok(())
    }

    #[test]
    fn project_config_path_is_repository_scoped() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project")?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());

        let plan = adapter.plan(CodexPlanRequest {
            scope: HostScope::Project,
            integration_id: "int-project",
            explicit_server_name: None,
            repo_root: Some(&repo),
            mcp_command: Path::new("harness-mcp"),
            runtime_home: None,
            expected_fingerprint: None,
        })?;

        assert_eq!(
            plan.target,
            HostTarget::File(repo.join(".codex").join("config.toml"))
        );
        assert_eq!(plan.user_actions[0].kind, UserActionKind::HostTrustRequired);
        Ok(())
    }

    #[test]
    fn insertion_preserves_comments_and_unrelated_keys() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-preserve")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        let target = codex_home.join("config.toml");
        fs::write(
            &target,
            "# keep me\nmodel = \"gpt-5.5\"\n\n[mcp_servers.other]\ncommand = \"other\"\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
        });

        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&plan)?;
        let text = fs::read_to_string(target)?;

        assert!(text.contains("# keep me"));
        assert!(text.contains("model = \"gpt-5.5\""));
        assert!(text.contains("[mcp_servers.other]"));
        assert!(text.contains("[mcp_servers.harness-int_alpha]"));
        assert!(text.contains("args = [\"--integration\", \"int_alpha\"]"));
        Ok(())
    }

    #[test]
    fn owned_table_updates_and_idempotent_reapply() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-update")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
        });
        let first = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&first)?;

        let second = adapter.plan(CodexPlanRequest {
            expected_fingerprint: Some(&first.fingerprint),
            mcp_command: Path::new("/usr/local/bin/harness-mcp"),
            ..request(HostScope::User, None, Path::new("/bin/harness-mcp"))
        })?;
        assert_eq!(second.change, PlannedChange::Update);
        adapter.apply(&second)?;

        let third = adapter.plan(CodexPlanRequest {
            mcp_command: Path::new("/usr/local/bin/harness-mcp"),
            ..request(HostScope::User, None, Path::new("/bin/harness-mcp"))
        })?;
        assert_eq!(third.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn unmanaged_name_collision_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-collision")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        fs::write(
            codex_home.join("config.toml"),
            "[mcp_servers.harness-int_alpha]\ncommand = \"other\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
        });

        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::UnmanagedNameCollision
        );
        Ok(())
    }

    #[test]
    fn malformed_toml_is_rejected_without_write() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-malformed")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        let target = codex_home.join("config.toml");
        fs::write(&target, "[mcp_servers.\n")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
        });

        let error = adapter
            .plan(request(
                HostScope::User,
                None,
                Path::new("/bin/harness-mcp"),
            ))
            .expect_err("malformed TOML should fail");

        assert!(matches!(error, HostConfigError::Malformed(_)));
        assert_eq!(fs::read_to_string(target)?, "[mcp_servers.\n");
        Ok(())
    }

    #[test]
    fn project_scope_requires_path_command_and_no_runtime_home(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project-path")?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());

        assert!(matches!(
            adapter.plan(CodexPlanRequest {
                scope: HostScope::Project,
                integration_id: "int_alpha",
                explicit_server_name: None,
                repo_root: Some(&repo),
                mcp_command: Path::new("/personal/target/debug/harness-mcp"),
                runtime_home: None,
                expected_fingerprint: None,
            }),
            Err(HostConfigError::Conflict(_))
        ));
        assert!(matches!(
            adapter.plan(CodexPlanRequest {
                scope: HostScope::Project,
                integration_id: "int_alpha",
                explicit_server_name: None,
                repo_root: Some(&repo),
                mcp_command: Path::new("harness-mcp"),
                runtime_home: Some(Path::new("/home/me/.harness")),
                expected_fingerprint: None,
            }),
            Err(HostConfigError::Conflict(_))
        ));
        Ok(())
    }

    #[test]
    fn safe_removal_requires_matching_fingerprint() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-remove")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
        });
        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&plan)?;
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("codex target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/harness-mcp", "/tmp/manual"),
        )?;

        let error = adapter
            .remove(HostRemoveRequest {
                host_kind: HostKind::Codex,
                host_scope: HostScope::User,
                server_name: plan.server_name,
                target: HostTarget::File(target),
                expected_fingerprint: plan.fingerprint,
            })
            .expect_err("manual edits should block removal");

        assert!(matches!(error, HostConfigError::Conflict(_)));
        Ok(())
    }

    fn request<'a>(
        scope: HostScope,
        repo_root: Option<&'a Path>,
        mcp_command: &'a Path,
    ) -> CodexPlanRequest<'a> {
        CodexPlanRequest {
            scope,
            integration_id: "int_alpha",
            explicit_server_name: None,
            repo_root,
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
