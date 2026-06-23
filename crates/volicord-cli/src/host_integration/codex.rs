use std::{
    cell::RefCell,
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use toml_edit::{value, Array, DocumentMut, Item, Table};

use super::{
    claude_code::{CommandInvocation, CommandRunner, ProductionCommandRunner},
    config_edit::{read_text_snapshot, write_if_fresh, FileSnapshot},
    managed_fingerprint, unmanaged_fingerprint, validated_server_name, HostAdapter,
    HostConfigError, HostConflict, HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan,
    HostRemoveRequest, HostScope, HostTarget, ManagedServerEntry, PlannedChange, UserAction,
    UserActionKind,
};
use crate::host_integration::verification::{
    HostConfigurationStatus, HostExecutableStatus, HostGateStatus, ManagedConfigStatus,
    Verification,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexEnvironment {
    pub home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub path: Option<OsString>,
}

#[derive(Debug, Clone)]
pub struct CodexAdapter<R = ProductionCommandRunner> {
    env: CodexEnvironment,
    runner: RefCell<R>,
}

impl CodexAdapter<ProductionCommandRunner> {
    pub fn new(env: CodexEnvironment) -> Self {
        Self::with_runner(env, ProductionCommandRunner)
    }
}

impl<R: CommandRunner> CodexAdapter<R> {
    pub fn with_runner(env: CodexEnvironment, runner: R) -> Self {
        Self {
            env,
            runner: RefCell::new(runner),
        }
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
        add_project_trust_action(request.scope, &mut user_actions);

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

    pub fn plan_existing(
        &self,
        request: CodexExistingPlanRequest<'_>,
    ) -> Result<HostPlan, HostConfigError> {
        if !matches!(request.scope, HostScope::User | HostScope::Project) {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex supports only user and project host scopes",
            )));
        }
        validate_mcp_command(request.scope, request.mcp_command)?;
        if request.scope == HostScope::Project && request.runtime_home.is_some() {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidCommand,
                "Codex project-scoped configuration must not embed a personal HARNESS_HOME",
            )));
        }

        let server_name = validated_server_name(request.integration_id, Some(request.server_name))?;
        let entry = ManagedServerEntry::new(
            request.integration_id,
            request.mcp_command,
            request.runtime_home,
        );
        let mut user_actions = Vec::new();
        add_project_trust_action(request.scope, &mut user_actions);

        Ok(HostPlan {
            host_kind: HostKind::Codex,
            host_scope: request.scope,
            server_name,
            target: HostTarget::File(request.config_target.to_path_buf()),
            entry,
            change: PlannedChange::Noop,
            fingerprint: request.managed_fingerprint.to_owned(),
            conflicts: Vec::new(),
            user_actions,
            file_snapshot: None,
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

    fn executable_availability(&self, config_target: &Path) -> CodexExecutableAvailability {
        let Some(executable) = find_executable_in_path("codex", self.env.path.as_ref()) else {
            return CodexExecutableAvailability::unavailable(
                format!(
                    "Codex executable `codex` was not found on PATH; install Codex or make it available before using this Host Installation; configuration target: {}",
                    config_target.display()
                ),
                "Codex executable `codex` was not found on PATH",
            );
        };
        let invocation = CommandInvocation {
            program: executable.display().to_string(),
            args: vec!["--version".to_owned()],
            cwd: None,
        };
        match self.runner.borrow_mut().run(&invocation) {
            Ok(output) if output.success => CodexExecutableAvailability::available(format!(
                "Codex executable availability check succeeded with `codex --version`; executable: {}; configuration target: {}",
                executable.display(),
                config_target.display()
            )),
            Ok(output) => CodexExecutableAvailability::unavailable(
                format!(
                    "Codex executable failed its availability check `codex --version` with status {}; install or repair Codex before using this Host Installation; configuration target: {}",
                    status_text(output.status_code),
                    config_target.display()
                ),
                format!(
                    "Codex executable availability check failed with status {}",
                    status_text(output.status_code)
                ),
            ),
            Err(error) => CodexExecutableAvailability::unavailable(
                format!(
                    "Codex executable could not be launched for availability check `codex --version`: {error}; install Codex or make it executable before using this Host Installation; configuration target: {}",
                    config_target.display()
                ),
                format!("Codex executable availability check could not launch: {error}"),
            ),
        }
    }
}

impl<R: CommandRunner> HostAdapter for CodexAdapter<R> {
    fn detect(&self) -> Result<HostDetection, HostConfigError> {
        let path = self.codex_home()?.join("config.toml");
        let availability = self.executable_availability(&path);
        Ok(HostDetection {
            host_kind: HostKind::Codex,
            available: availability.is_available(),
            details: availability.details,
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
            return Ok(Verification::changed(conflict.message.clone()));
        }
        let config_target = match &plan.target {
            HostTarget::File(target) => target.as_path(),
            _ => Path::new("unknown Codex configuration target"),
        };
        let executable = self.executable_availability(config_target);
        let managed = verify_codex_entry(plan)?;
        if managed != ManagedConfigStatus::Match {
            let mut verification = verification_from_managed_status(
                managed,
                format!(
                    "Codex managed MCP server entry is {} for {}",
                    managed.as_str(),
                    plan.server_name
                ),
            )
            .with_host_executable(executable.status);
            if let Some(diagnostic) = executable.diagnostic {
                verification = verification.with_diagnostic(diagnostic);
            }
            return Ok(verification);
        }
        if !executable.is_available() {
            return Ok(verification_from_executable_unavailable(
                executable,
                plan.host_scope == HostScope::Project,
            ));
        }
        if plan.host_scope == HostScope::Project {
            return Ok(Verification::action_required(
                "Codex project trust was not confirmed by this structural configuration check",
            )
            .with_host_executable(HostExecutableStatus::Available)
            .with_host_gate(HostGateStatus::ActionRequired)
            .with_mcp_handshake_allowed(true));
        }
        Ok(Verification::configured_ready(
            "Codex managed configuration is present, Codex executable is available, and no separate project trust gate applies",
        )
        .with_host_executable(HostExecutableStatus::Available)
        .with_mcp_handshake_allowed(true))
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

#[derive(Debug, Clone, Copy)]
pub struct CodexExistingPlanRequest<'a> {
    pub scope: HostScope,
    pub integration_id: &'a str,
    pub server_name: &'a str,
    pub config_target: &'a Path,
    pub mcp_command: &'a Path,
    pub runtime_home: Option<&'a Path>,
    pub managed_fingerprint: &'a str,
}

fn add_project_trust_action(scope: HostScope, user_actions: &mut Vec<UserAction>) {
    if scope == HostScope::Project {
        user_actions.push(UserAction::new(
            UserActionKind::HostTrustRequired,
            "Codex project configuration is loaded only after the project is trusted",
        ));
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexExecutableAvailability {
    status: HostExecutableStatus,
    details: String,
    diagnostic: Option<String>,
}

impl CodexExecutableAvailability {
    fn available(details: String) -> Self {
        Self {
            status: HostExecutableStatus::Available,
            details,
            diagnostic: None,
        }
    }

    fn unavailable(details: String, diagnostic: impl Into<String>) -> Self {
        Self {
            status: HostExecutableStatus::Unavailable,
            details,
            diagnostic: Some(diagnostic.into()),
        }
    }

    fn is_available(&self) -> bool {
        self.status == HostExecutableStatus::Available
    }
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "without exit status".to_owned())
}

fn verification_from_executable_unavailable(
    executable: CodexExecutableAvailability,
    project_trust_unconfirmed: bool,
) -> Verification {
    let mut details = executable.details;
    if project_trust_unconfirmed {
        details.push_str(
            "; Codex project trust was not confirmed by this structural configuration check",
        );
    }
    let mut verification = Verification::action_required(details)
        .with_host_executable(HostExecutableStatus::Unavailable)
        .with_host_gate(HostGateStatus::ActionRequired)
        .with_host_configuration(HostConfigurationStatus::Discovered)
        .with_mcp_handshake_allowed(false);
    if let Some(diagnostic) = executable.diagnostic {
        verification = verification.with_diagnostic(diagnostic);
    }
    verification
}

fn verify_codex_entry(plan: &HostPlan) -> Result<ManagedConfigStatus, HostConfigError> {
    let HostTarget::File(target) = &plan.target else {
        return Ok(ManagedConfigStatus::Unknown);
    };
    let (_, text) = read_text_snapshot(target)?;
    let Some(text) = text else {
        return Ok(ManagedConfigStatus::Missing);
    };
    let document = match parse_document(Some(&text), target) {
        Ok(document) => document,
        Err(error) => {
            return match error {
                HostConfigError::Malformed(_) => Ok(ManagedConfigStatus::Malformed),
                other => Err(other),
            };
        }
    };
    let Some(item) = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .and_then(|servers| servers.get(&plan.server_name))
    else {
        return Ok(ManagedConfigStatus::Missing);
    };
    match codex_entry_fingerprint(plan.host_scope, &plan.server_name, item) {
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
            .with_host_configuration(
                crate::host_integration::verification::HostConfigurationStatus::Malformed,
            ),
        ManagedConfigStatus::Match => Verification::configured_ready(details),
        ManagedConfigStatus::NotApplicable | ManagedConfigStatus::Unknown => {
            Verification::unknown(details)
        }
    }
}

fn find_executable_in_path(program: &str, path: Option<&OsString>) -> Option<PathBuf> {
    let path = path.cloned().or_else(|| std::env::var_os("PATH"))?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
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
        collections::VecDeque,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::host_integration::claude_code::{CommandInvocation, CommandOutput};

    use super::*;

    #[test]
    fn user_config_path_defaults_to_home_codex() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-home-default")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.clone()),
            codex_home: None,
            path: None,
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
            path: None,
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
    fn existing_plan_uses_stored_target_without_ambient_discovery(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-existing-target")?;
        let stored_target = dir.join("stored").join("config.toml");
        let ambient_codex_home = dir.join("ambient");
        fs::create_dir_all(&ambient_codex_home)?;
        fs::write(
            ambient_codex_home.join("config.toml"),
            "[mcp_servers.harness-existing]\ncommand = \"ambient\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.join("home")),
            codex_home: Some(ambient_codex_home),
            path: None,
        });

        let plan = adapter.plan_existing(existing_request(
            HostScope::User,
            &stored_target,
            Path::new("/bin/harness-mcp"),
            Some(Path::new("/runtime")),
        ))?;

        assert_eq!(plan.target, HostTarget::File(stored_target));
        assert_eq!(plan.change, PlannedChange::Noop);
        assert_eq!(plan.fingerprint, "stored-fingerprint");
        Ok(())
    }

    #[test]
    fn existing_plan_verification_reports_stored_missing_without_ambient_fallback(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-existing-missing")?;
        let stored_target = dir.join("stored").join("config.toml");
        let ambient_codex_home = dir.join("ambient");
        fs::create_dir_all(&ambient_codex_home)?;
        fs::write(
            ambient_codex_home.join("config.toml"),
            "[mcp_servers.harness-existing]\ncommand = \"ambient\"\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(ambient_codex_home),
            path: Some(dir.join("empty-path").into_os_string()),
        });
        let plan = adapter.plan_existing(existing_request(
            HostScope::User,
            &stored_target,
            Path::new("/bin/harness-mcp"),
            Some(Path::new("/runtime")),
        ))?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "missing");
        assert_eq!(verification.managed_config, ManagedConfigStatus::Missing);
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
            path: None,
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
            path: None,
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
            path: None,
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
            path: None,
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
            path: None,
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

    #[cfg(unix)]
    #[test]
    fn detect_requires_executable_on_path() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-detect")?;
        let codex_home = dir.join("codex");
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: Some(dir.join("empty").into_os_string()),
        });

        let detection = adapter.detect()?;

        assert!(!detection.available);
        assert!(detection.details.contains("not found on PATH"));
        Ok(())
    }

    #[test]
    fn detect_reports_available_executable() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-detect-available")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![Ok(ok_output())]),
        );

        let detection = adapter.detect()?;

        assert!(detection.available);
        assert!(detection.details.contains("codex --version"));
        Ok(())
    }

    #[test]
    fn verify_requires_available_executable_for_user_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify-no-executable")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: Some(dir.join("empty").into_os_string()),
        });
        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert!(!verification.mcp_handshake_allowed);
        assert!(verification.details.contains("install Codex"));
        Ok(())
    }

    #[test]
    fn verify_reports_failed_executable_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify-version-fails")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let mut adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![Ok(failed_output(42))]),
        );
        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert!(verification.details.contains("status 42"));
        assert!(verification
            .diagnostic
            .as_deref()
            .unwrap_or_default()
            .contains("status 42"));
        Ok(())
    }

    #[test]
    fn verify_reports_launch_failure() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify-launch-fails")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let mut adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![Err("permission denied".to_owned())]),
        );
        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert!(verification.details.contains("could not be launched"));
        Ok(())
    }

    #[test]
    fn detect_and_verify_use_consistent_executable_status() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = temp_dir("codex-detect-verify-consistent")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: Some(dir.join("empty").into_os_string()),
        });
        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        adapter.apply(&plan)?;

        let detection = adapter.detect()?;
        let verification = adapter.verify(&plan)?;

        assert!(!detection.available);
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert_eq!(verification.status.as_str(), "action_required");
        Ok(())
    }

    #[test]
    fn missing_executable_diagnostic_does_not_expose_path_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-diagnostic-path")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(dir.join("codex")),
            path: Some(OsString::from("/tmp/SECRET_PATH_TOKEN")),
        });

        let detection = adapter.detect()?;

        assert!(!detection.available);
        assert!(!detection.details.contains("SECRET_PATH_TOKEN"));
        Ok(())
    }

    #[test]
    fn verify_distinguishes_missing_changed_and_project_trust(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let mut adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![
                Ok(ok_output()),
                Ok(ok_output()),
                Ok(ok_output()),
                Ok(ok_output()),
            ]),
        );
        let plan = adapter.plan(request(
            HostScope::User,
            None,
            Path::new("/bin/harness-mcp"),
        ))?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "missing");
        adapter.apply(&plan)?;
        assert_eq!(
            adapter.verify(&plan)?.host_state.as_str(),
            "configured_ready"
        );
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("codex target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/harness-mcp", "/tmp/manual"),
        )?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "changed");

        let repo = temp_dir("codex-project-verify")?;
        let project = adapter.plan(CodexPlanRequest {
            scope: HostScope::Project,
            integration_id: "int_alpha",
            explicit_server_name: None,
            repo_root: Some(&repo),
            mcp_command: Path::new("harness-mcp"),
            runtime_home: None,
            expected_fingerprint: None,
        })?;
        adapter.apply(&project)?;
        let verification = adapter.verify(&project)?;
        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Available
        );
        assert!(verification.mcp_handshake_allowed);
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

    fn existing_request<'a>(
        scope: HostScope,
        config_target: &'a Path,
        mcp_command: &'a Path,
        runtime_home: Option<&'a Path>,
    ) -> CodexExistingPlanRequest<'a> {
        CodexExistingPlanRequest {
            scope,
            integration_id: "int_alpha",
            server_name: "harness-existing",
            config_target,
            mcp_command,
            runtime_home,
            managed_fingerprint: "stored-fingerprint",
        }
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn write_fake_codex_file(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        fs::write(dir.join("codex"), "fake codex")?;
        Ok(())
    }

    fn ok_output() -> CommandOutput {
        CommandOutput {
            success: true,
            status_code: Some(0),
            stdout: "codex 1.2.3\n".to_owned(),
            stderr: String::new(),
        }
    }

    fn failed_output(status_code: i32) -> CommandOutput {
        CommandOutput {
            success: false,
            status_code: Some(status_code),
            stdout: String::new(),
            stderr: "version failed".to_owned(),
        }
    }

    #[derive(Debug)]
    struct FakeRunner {
        outputs: VecDeque<Result<CommandOutput, String>>,
        calls: Vec<CommandInvocation>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<Result<CommandOutput, String>>) -> Self {
            Self {
                outputs: outputs.into(),
                calls: Vec::new(),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
            self.calls.push(invocation.clone());
            self.outputs
                .pop_front()
                .unwrap_or_else(|| Err("missing fake command output".to_owned()))
        }
    }
}
