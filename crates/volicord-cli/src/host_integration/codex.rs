use std::{
    cell::RefCell,
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use toml_edit::{value, Array, DocumentMut, Item, Table};

use super::{
    claude_code::{CommandInvocation, CommandRunner, ProductionCommandRunner},
    config_edit::{read_text_snapshot, write_if_fresh, FileSnapshot},
    format_supported_connection_intents, is_volicord_managed_entry, managed_fingerprint,
    unmanaged_fingerprint, validated_server_name, ConnectionIntent, HostAdapter, HostConfigError,
    HostConflict, HostConflictKind, HostDetection, HostEffect, HostKind, HostPlan, HostPlanRequest,
    HostRemoveRequest, HostScope, HostTarget, InstallationProfile, ManagedServerEntry,
    PlannedChange, ProjectContext, UserAction, UserActionKind, DEFAULT_MCP_COMMAND,
};
use crate::host_integration::verification::{
    HostConfigurationStatus, HostExecutableStatus, HostGateStatus, ManagedConfigStatus,
    ProjectTrustDiagnostic, ProjectTrustStatus, Verification,
};
use crate::host_integration::HostCapabilities;

const VOLICORD_MCP_LAUNCH: &str = "VOLICORD_MCP_LAUNCH";
const VOLICORD_MCP_HOST: &str = "VOLICORD_MCP_HOST";
const VOLICORD_MCP_CONNECTION_ID: &str = "VOLICORD_MCP_CONNECTION_ID";
const VOLICORD_MCP_PROJECT_ID: &str = "VOLICORD_MCP_PROJECT_ID";
const MANAGED_HOST_LAUNCH_VALUE: &str = "managed_host";
const CODEX_HOST_VALUE: &str = "codex";

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

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        http_mcp: false,
        session_start_hook: true,
        pre_tool_hook: true,
        post_tool_hook: true,
        user_prompt_submit_hook: true,
        stop_hook: true,
        rule_file_support: true,
        project_local_configuration: true,
    }
}

pub fn project_hooks_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".codex").join("hooks.json")
}

pub fn project_rule_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".codex")
        .join("rules")
        .join("volicord.rules")
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

    pub fn plan(&self, request: HostPlanRequest<'_>) -> Result<HostPlan, HostConfigError> {
        if request.host_kind != HostKind::Codex {
            return Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                "Codex adapter cannot plan a non-Codex host request",
            )));
        }
        let scope = codex_scope_for_intent(request.connection_intent)?;
        let (mcp_command, runtime_home) =
            entry_inputs_for_scope(scope, request.installation_profile);
        validate_mcp_command(scope, mcp_command)?;

        let server_name = validated_server_name(request.connection_id, None)?;
        let target = self.config_path(scope, request.project)?;
        let project_id = (scope == HostScope::Project)
            .then(|| request.project.map(|project| project.project_id))
            .flatten();
        let entry = codex_managed_server_entry(
            request.connection_id,
            project_id,
            mcp_command,
            runtime_home,
        );
        let fingerprint = managed_fingerprint(HostKind::Codex, scope, &server_name, &entry);
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
            Some(item) => classify_existing_codex_entry(
                scope,
                &server_name,
                item,
                &fingerprint,
                request.expected_fingerprint,
                &mut conflicts,
            ),
        };
        Ok(HostPlan {
            host_kind: HostKind::Codex,
            connection_intent: request.connection_intent,
            host_scope: scope,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::File(target),
            entry,
            change,
            fingerprint,
            conflicts,
            user_actions: Vec::new(),
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
                "Codex project-scoped configuration must not embed a personal VOLICORD_HOME",
            )));
        }

        let server_name = validated_server_name(request.connection_id, Some(request.server_name))?;
        let project_id = (request.scope == HostScope::Project)
            .then_some(request.project_id)
            .flatten();
        let entry = codex_managed_server_entry(
            request.connection_id,
            project_id,
            request.mcp_command,
            request.runtime_home,
        );
        let fingerprint = managed_fingerprint(HostKind::Codex, request.scope, &server_name, &entry);
        Ok(HostPlan {
            host_kind: HostKind::Codex,
            connection_intent: request.connection_intent,
            host_scope: request.scope,
            mode: request.mode.to_owned(),
            server_name,
            target: HostTarget::File(request.config_target.to_path_buf()),
            entry,
            change: PlannedChange::Noop,
            fingerprint,
            conflicts: Vec::new(),
            user_actions: Vec::new(),
            file_snapshot: None,
        })
    }

    fn config_path(
        &self,
        scope: HostScope,
        project: Option<ProjectContext<'_>>,
    ) -> Result<PathBuf, HostConfigError> {
        match scope {
            HostScope::User => Ok(self.codex_home()?.join("config.toml")),
            HostScope::Project => {
                let project = project.ok_or_else(|| {
                    HostConfigError::Conflict(HostConflict::new(
                        HostConflictKind::InvalidScope,
                        "Codex shared connection intent requires a Product Repository root",
                    ))
                })?;
                Ok(project.repo_root.join(".codex").join("config.toml"))
            }
            _ => Err(HostConfigError::Conflict(HostConflict::new(
                HostConflictKind::InvalidScope,
                format!(
                    "Codex supports only these connection intents: {}",
                    format_supported_connection_intents(HostKind::Codex)
                ),
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
                    "Codex executable `codex` was not found on PATH; install Codex or make it available before using this Agent Connection; configuration target: {}",
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
                    "Codex executable failed its availability check `codex --version` with status {}; install or repair Codex before using this Agent Connection; configuration target: {}",
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
                    "Codex executable could not be launched for availability check `codex --version`: {error}; install Codex or make it executable before using this Agent Connection; configuration target: {}",
                    config_target.display()
                ),
                format!("Codex executable availability check could not launch: {error}"),
            ),
        }
    }
}

impl<R: CommandRunner> HostAdapter for CodexAdapter<R> {
    fn capabilities(&self) -> HostCapabilities {
        capabilities()
    }

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
            return Ok(Verification::changed(conflict.message.clone())
                .merge_user_actions(&plan.user_actions));
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
            return Ok(verification.merge_user_actions(&plan.user_actions));
        }
        if plan.host_scope == HostScope::Project {
            let project_trust = project_trust_for_plan(&self.env, plan);
            if !executable.is_available() {
                let mut verification = verification_from_executable_unavailable(executable);
                verification = verification.with_project_trust(project_trust);
                return Ok(verification.merge_user_actions(&plan.user_actions));
            }
            let mut verification = match project_trust.status {
                ProjectTrustStatus::Trusted => Verification::configured_ready(
                    "Codex managed configuration is present, Codex executable is available, and Codex project trust is trusted",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_gate(HostGateStatus::Ready)
                .with_mcp_handshake_allowed(true),
                ProjectTrustStatus::Untrusted => {
                    Verification::action_required(
                        "Codex managed configuration is present, Codex executable is available, and Codex project trust is untrusted",
                    )
                    .with_host_executable(HostExecutableStatus::Available)
                    .with_host_gate(HostGateStatus::ActionRequired)
                    .with_mcp_handshake_allowed(true)
                    .with_user_actions(vec![UserAction::new(
                        UserActionKind::HostTrustRequired,
                        "Codex project trust is untrusted in the Codex user configuration",
                    )])
                }
                ProjectTrustStatus::Missing
                | ProjectTrustStatus::Unknown
                | ProjectTrustStatus::Unreadable
                | ProjectTrustStatus::Malformed => Verification::configured_ready(
                    "Codex managed configuration is present and Codex executable is available; Codex project trust is not confirmed from the user configuration",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_gate(HostGateStatus::Unknown)
                .with_mcp_handshake_allowed(true),
            };
            verification = verification.with_project_trust(project_trust);
            return Ok(verification.merge_user_actions(&plan.user_actions));
        }
        if !executable.is_available() {
            return Ok(verification_from_executable_unavailable(executable)
                .merge_user_actions(&plan.user_actions));
        }
        Ok(Verification::configured_ready(
            "Codex managed configuration is present, Codex executable is available, and no separate project trust gate applies",
        )
        .with_host_executable(HostExecutableStatus::Available)
        .with_mcp_handshake_allowed(true)
        .merge_user_actions(&plan.user_actions))
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
                    "Codex MCP server changed since Volicord last managed it: {}",
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
pub struct CodexExistingPlanRequest<'a> {
    pub connection_intent: ConnectionIntent,
    pub scope: HostScope,
    pub connection_id: &'a str,
    pub project_id: Option<&'a str>,
    pub server_name: &'a str,
    pub config_target: &'a Path,
    pub mcp_command: &'a Path,
    pub runtime_home: Option<&'a Path>,
    pub mode: &'a str,
}

fn codex_scope_for_intent(intent: ConnectionIntent) -> Result<HostScope, HostConfigError> {
    match intent {
        ConnectionIntent::Personal => Ok(HostScope::User),
        ConnectionIntent::Shared => Ok(HostScope::Project),
        ConnectionIntent::Global => Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidScope,
            format!(
                "Codex does not support global connection intent; supported connection intents: {}",
                format_supported_connection_intents(HostKind::Codex)
            ),
        ))),
    }
}

fn codex_managed_server_entry(
    connection_id: impl Into<String>,
    project_id: Option<&str>,
    mcp_command: &Path,
    runtime_home: Option<&Path>,
) -> ManagedServerEntry {
    let connection_id = connection_id.into();
    let mut entry = ManagedServerEntry::new_project_bound(
        connection_id.clone(),
        project_id,
        mcp_command,
        runtime_home,
    );
    entry.env.insert(
        VOLICORD_MCP_LAUNCH.to_owned(),
        MANAGED_HOST_LAUNCH_VALUE.to_owned(),
    );
    entry
        .env
        .insert(VOLICORD_MCP_HOST.to_owned(), CODEX_HOST_VALUE.to_owned());
    entry
        .env
        .insert(VOLICORD_MCP_CONNECTION_ID.to_owned(), connection_id);
    if let Some(project_id) = project_id {
        entry
            .env
            .insert(VOLICORD_MCP_PROJECT_ID.to_owned(), project_id.to_owned());
    }
    entry
}

fn entry_inputs_for_scope<'a>(
    scope: HostScope,
    profile: InstallationProfile<'a>,
) -> (&'a Path, Option<&'a Path>) {
    if scope == HostScope::Project {
        (Path::new(DEFAULT_MCP_COMMAND), None)
    } else {
        (profile.volicord_mcp_command, Some(profile.runtime_home))
    }
}

fn classify_existing_codex_entry(
    scope: HostScope,
    server_name: &str,
    item: &Item,
    desired_fingerprint: &str,
    expected_fingerprint: Option<&str>,
    conflicts: &mut Vec<HostConflict>,
) -> PlannedChange {
    let Some(entry) = codex_managed_entry(item) else {
        conflicts.push(HostConflict::new(
            HostConflictKind::UnmanagedNameCollision,
            format!(
                "Codex MCP server name is already configured by an unmanaged entry: {server_name}"
            ),
        ));
        return PlannedChange::Noop;
    };
    let current = managed_fingerprint(HostKind::Codex, scope, server_name, &entry);
    if current == desired_fingerprint {
        PlannedChange::Noop
    } else if expected_fingerprint == Some(current.as_str()) {
        PlannedChange::Update
    } else {
        conflicts.push(HostConflict::new(
            HostConflictKind::FingerprintMismatch,
            format!(
                "Codex MCP server name is already configured by a different Volicord-managed entry: {server_name}"
            ),
        ));
        PlannedChange::Noop
    }
}

fn validate_mcp_command(scope: HostScope, command: &Path) -> Result<(), HostConfigError> {
    if scope == HostScope::Project {
        if command == Path::new(DEFAULT_MCP_COMMAND) {
            return Ok(());
        }
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Codex project-scoped configuration must use volicord from PATH",
        )));
    }
    if command.is_absolute() {
        Ok(())
    } else {
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Codex user-scoped configuration requires an absolute volicord command path",
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

fn codex_managed_entry(item: &Item) -> Option<ManagedServerEntry> {
    let table = item.as_table()?;
    let allowed_keys = ["command", "args", "env"];
    if table.iter().any(|(key, _)| !allowed_keys.contains(&key)) {
        return None;
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
    let entry = ManagedServerEntry { command, args, env };
    is_volicord_managed_entry(&entry).then_some(entry)
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
) -> Verification {
    let mut verification = Verification::action_required(executable.details)
        .with_host_executable(HostExecutableStatus::Unavailable)
        .with_host_gate(HostGateStatus::ActionRequired)
        .with_host_configuration(HostConfigurationStatus::Discovered)
        .with_mcp_handshake_allowed(false);
    if let Some(diagnostic) = executable.diagnostic {
        verification = verification.with_diagnostic(diagnostic);
    }
    verification
}

pub fn project_trust_diagnostic(
    env: &CodexEnvironment,
    repo_root: &Path,
) -> ProjectTrustDiagnostic {
    let Some(config_path) = codex_user_config_path(env) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Unknown,
            config_path: String::new(),
            repo_root: repo_root.display().to_string(),
            details: "CODEX_HOME was not set and HOME was unavailable, so Codex user configuration could not be located".to_owned(),
        };
    };
    let config_path_text = config_path.display().to_string();
    let repo_root_text = repo_root.display().to_string();
    let text = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Missing,
                config_path: config_path_text,
                repo_root: repo_root_text,
                details: "Codex user configuration file was not found".to_owned(),
            };
        }
        Err(error) => {
            return ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Unreadable,
                config_path: config_path_text,
                repo_root: repo_root_text,
                details: format!("Codex user configuration could not be read: {error}"),
            };
        }
    };
    let document = match parse_document(Some(&text), &config_path) {
        Ok(document) => document,
        Err(_) => {
            return ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Malformed,
                config_path: config_path_text,
                repo_root: repo_root_text,
                details: "Codex user configuration is malformed TOML".to_owned(),
            };
        }
    };
    let Some(projects) = document.get("projects").and_then(Item::as_table) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Missing,
            config_path: config_path_text,
            repo_root: repo_root_text,
            details: "Codex user configuration has no matching projects table entry".to_owned(),
        };
    };
    let Some((project_path, project_item)) = matching_project_entry(projects, repo_root) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Missing,
            config_path: config_path_text,
            repo_root: repo_root_text,
            details: "Codex user configuration has no matching project trust entry".to_owned(),
        };
    };
    let Some(table) = project_item.as_table() else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Malformed,
            config_path: config_path_text,
            repo_root: repo_root_text,
            details: format!("Codex project trust entry is not a table: {project_path}"),
        };
    };
    let trust_level = table.get("trust_level").and_then(Item::as_str);
    let status = match trust_level {
        Some("trusted") => ProjectTrustStatus::Trusted,
        Some("untrusted") => ProjectTrustStatus::Untrusted,
        Some(_) | None => ProjectTrustStatus::Unknown,
    };
    let details = match status {
        ProjectTrustStatus::Trusted => "Codex user configuration marks the project trusted",
        ProjectTrustStatus::Untrusted => "Codex user configuration marks the project untrusted",
        ProjectTrustStatus::Unknown => {
            "Codex user configuration project entry does not contain a recognized trust_level"
        }
        ProjectTrustStatus::Missing
        | ProjectTrustStatus::Unreadable
        | ProjectTrustStatus::Malformed => {
            "Codex project trust could not be confirmed from user configuration"
        }
    };
    ProjectTrustDiagnostic {
        status,
        config_path: config_path_text,
        repo_root: repo_root_text,
        details: details.to_owned(),
    }
}

fn project_trust_for_plan(env: &CodexEnvironment, plan: &HostPlan) -> ProjectTrustDiagnostic {
    let HostTarget::File(target) = &plan.target else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Unknown,
            config_path: String::new(),
            repo_root: String::new(),
            details: "Codex project trust could not be checked for a non-file target".to_owned(),
        };
    };
    let Some(repo_root) = target.parent().and_then(Path::parent) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Unknown,
            config_path: String::new(),
            repo_root: String::new(),
            details: "Codex project trust could not be checked because the repository root was unavailable".to_owned(),
        };
    };
    project_trust_diagnostic(env, repo_root)
}

fn codex_user_config_path(env: &CodexEnvironment) -> Option<PathBuf> {
    env.codex_home
        .as_ref()
        .map(|path| path.join("config.toml"))
        .or_else(|| {
            env.home
                .as_ref()
                .map(|path| path.join(".codex/config.toml"))
        })
}

fn matching_project_entry<'a>(
    projects: &'a Table,
    repo_root: &Path,
) -> Option<(&'a str, &'a Item)> {
    projects
        .iter()
        .find(|(project_path, _)| project_path_matches(project_path, repo_root))
}

fn project_path_matches(project_path: &str, repo_root: &Path) -> bool {
    if !Path::new(project_path).is_absolute() || !repo_root.is_absolute() {
        return false;
    }
    let normalized_project_path = normalize_trailing_slashes(project_path);
    let repo_root_text = repo_root.display().to_string();
    let normalized_repo_root = normalize_trailing_slashes(&repo_root_text);
    if normalized_project_path == normalized_repo_root {
        return true;
    }
    let project_canonical = fs::canonicalize(project_path);
    let repo_canonical = fs::canonicalize(repo_root);
    matches!(
        (project_canonical, repo_canonical),
        (Ok(project), Ok(repo)) if project == repo
    )
}

fn normalize_trailing_slashes(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn managed_config_status_for_plan(
    plan: &HostPlan,
) -> Result<ManagedConfigStatus, HostConfigError> {
    verify_codex_entry(plan)
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

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

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

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

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

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;

        assert_eq!(
            plan.target,
            HostTarget::File(repo.join(".codex").join("config.toml"))
        );
        assert!(plan.user_actions.is_empty());
        Ok(())
    }

    #[test]
    fn codex_project_trust_reads_trusted() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-trusted")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let codex_home = dir.join("codex-home");
        write_project_trust(&codex_home, &repo, "trusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Trusted);
        Ok(())
    }

    #[test]
    fn codex_project_trust_reads_untrusted() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-untrusted")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let codex_home = dir.join("codex-home");
        write_project_trust(&codex_home, &repo, "untrusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Untrusted);
        Ok(())
    }

    #[test]
    fn codex_project_trust_missing_project_entry_is_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-missing")?;
        let repo = dir.join("product");
        let other = dir.join("other");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&other)?;
        let codex_home = dir.join("codex-home");
        write_project_trust(&codex_home, &other, "trusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Missing);
        Ok(())
    }

    #[test]
    fn codex_project_trust_malformed_config_is_malformed() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = temp_dir("codex-trust-malformed")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let codex_home = dir.join("codex-home");
        fs::create_dir_all(&codex_home)?;
        fs::write(codex_home.join("config.toml"), "[projects.\n")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Malformed);
        Ok(())
    }

    #[test]
    fn codex_project_trust_respects_codex_home() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-codex-home")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let home = dir.join("home");
        let default_codex_home = home.join(".codex");
        let codex_home = dir.join("codex-home");
        write_project_trust(&default_codex_home, &repo, "untrusted")?;
        write_project_trust(&codex_home, &repo, "trusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: Some(home),
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Trusted);
        Ok(())
    }

    #[test]
    fn intent_mapping_rejects_codex_global() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-intent")?;
        let repo = temp_dir("codex-intent-repo")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.clone()),
            codex_home: None,
            path: None,
        });

        let personal = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        let shared = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        let global = adapter
            .plan(HostPlanRequest {
                connection_intent: ConnectionIntent::Global,
                ..request(HostScope::User, None, Path::new("/bin/volicord"))
            })
            .expect_err("Codex global intent should be unsupported");

        assert_eq!(personal.host_scope, HostScope::User);
        assert_eq!(shared.host_scope, HostScope::Project);
        assert!(matches!(global, HostConfigError::Conflict(_)));
        assert!(global
            .to_string()
            .contains("supported connection intents: personal, shared"));
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
            "[mcp_servers.volicord-existing]\ncommand = \"ambient\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.join("home")),
            codex_home: Some(ambient_codex_home),
            path: None,
        });

        let plan = adapter.plan_existing(existing_request(
            HostScope::User,
            &stored_target,
            Path::new("/bin/volicord"),
            Some(Path::new("/runtime")),
        ))?;

        assert_eq!(plan.target, HostTarget::File(stored_target));
        assert_eq!(plan.change, PlannedChange::Noop);
        assert_ne!(plan.fingerprint, "stored-fingerprint");
        assert_eq!(
            plan.entry
                .env
                .get(VOLICORD_MCP_CONNECTION_ID)
                .map(String::as_str),
            Some("int_alpha")
        );
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
            "[mcp_servers.volicord-existing]\ncommand = \"ambient\"\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(ambient_codex_home),
            path: Some(dir.join("empty-path").into_os_string()),
        });
        let plan = adapter.plan_existing(existing_request(
            HostScope::User,
            &stored_target,
            Path::new("/bin/volicord"),
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

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;
        let text = fs::read_to_string(target)?;

        assert!(text.contains("# keep me"));
        assert!(text.contains("model = \"gpt-5.5\""));
        assert!(text.contains("[mcp_servers.other]"));
        assert!(text.contains("[mcp_servers.volicord]"));
        assert!(text.contains("args = [\"mcp\", \"--stdio\", \"--connection\", \"int_alpha\"]"));
        assert!(text.contains("[mcp_servers.volicord.env]"));
        assert!(text.contains("VOLICORD_MCP_LAUNCH = \"managed_host\""));
        assert!(text.contains("VOLICORD_MCP_HOST = \"codex\""));
        assert!(text.contains("VOLICORD_MCP_CONNECTION_ID = \"int_alpha\""));
        Ok(())
    }

    #[test]
    fn project_config_includes_managed_launch_env_markers() -> Result<(), Box<dyn std::error::Error>>
    {
        let repo = temp_dir("codex-project-env")?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let text = fs::read_to_string(repo.join(".codex/config.toml"))?;

        assert_eq!(
            plan.entry.env.get(VOLICORD_MCP_LAUNCH).map(String::as_str),
            Some(MANAGED_HOST_LAUNCH_VALUE)
        );
        assert_eq!(
            plan.entry.env.get(VOLICORD_MCP_HOST).map(String::as_str),
            Some(CODEX_HOST_VALUE)
        );
        assert_eq!(
            plan.entry
                .env
                .get(VOLICORD_MCP_CONNECTION_ID)
                .map(String::as_str),
            Some("int_alpha")
        );
        assert_eq!(
            plan.entry
                .env
                .get(VOLICORD_MCP_PROJECT_ID)
                .map(String::as_str),
            Some("project_alpha")
        );
        assert!(text.contains("[mcp_servers.volicord.env]"));
        assert!(text.contains("VOLICORD_MCP_LAUNCH = \"managed_host\""));
        assert!(text.contains("VOLICORD_MCP_HOST = \"codex\""));
        assert!(text.contains("VOLICORD_MCP_CONNECTION_ID = \"int_alpha\""));
        assert!(text.contains("VOLICORD_MCP_PROJECT_ID = \"project_alpha\""));
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
        let first = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&first)?;

        let second = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&first.fingerprint),
            installation_profile: InstallationProfile {
                volicord_mcp_command: Path::new("/usr/local/bin/volicord"),
                ..request(HostScope::User, None, Path::new("/bin/volicord")).installation_profile
            },
            ..request(HostScope::User, None, Path::new("/bin/volicord"))
        })?;
        assert_eq!(second.change, PlannedChange::Update);
        adapter.apply(&second)?;

        let third = adapter.plan(HostPlanRequest {
            installation_profile: InstallationProfile {
                volicord_mcp_command: Path::new("/usr/local/bin/volicord"),
                ..request(HostScope::User, None, Path::new("/bin/volicord")).installation_profile
            },
            ..request(HostScope::User, None, Path::new("/bin/volicord"))
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
            "[mcp_servers.volicord]\ncommand = \"other\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::UnmanagedNameCollision
        );
        Ok(())
    }

    #[test]
    fn managed_fingerprint_mismatch_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-managed-mismatch")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        fs::write(
            codex_home.join("config.toml"),
            "[mcp_servers.volicord]\ncommand = \"/bin/volicord\"\nargs = [\"mcp\", \"--stdio\", \"--connection\", \"other\"]\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::FingerprintMismatch
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
            .plan(request(HostScope::User, None, Path::new("/bin/volicord")))
            .expect_err("malformed TOML should fail");

        assert!(matches!(error, HostConfigError::Malformed(_)));
        assert_eq!(fs::read_to_string(target)?, "[mcp_servers.\n");
        Ok(())
    }

    #[test]
    fn shared_intent_uses_path_command_and_no_runtime_home(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project-path")?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("/personal/target/debug/volicord"),
        ))?;

        assert_eq!(plan.entry.command, "volicord");
        assert!(!plan.entry.env.contains_key("VOLICORD_HOME"));
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
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("codex target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/volicord", "/tmp/manual"),
        )?;

        let error = adapter
            .remove(HostRemoveRequest {
                host_kind: HostKind::Codex,
                connection_intent: plan.connection_intent,
                host_scope: HostScope::User,
                mode: plan.mode.clone(),
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
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
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
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
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
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
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
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
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
    fn verify_distinguishes_missing_changed_and_project_trust_diagnostics(
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
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
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
            fs::read_to_string(&target)?.replace("/bin/volicord", "/tmp/manual"),
        )?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "changed");

        let repo = temp_dir("codex-project-verify")?;
        let project = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&project)?;
        let verification = adapter.verify(&project)?;
        assert_eq!(verification.status.as_str(), "complete");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Available
        );
        assert_eq!(
            verification
                .project_trust
                .as_ref()
                .expect("project trust diagnostic should be present")
                .status,
            ProjectTrustStatus::Missing
        );
        assert!(verification.user_actions.is_empty());
        assert!(verification.mcp_handshake_allowed);
        Ok(())
    }

    #[test]
    fn verify_treats_missing_managed_launch_markers_as_changed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-missing-launch-markers")?;
        fs::create_dir_all(repo.join(".codex"))?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        fs::write(
            repo.join(".codex/config.toml"),
            "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--connection\", \"int_alpha\", \"--project\", \"project_alpha\"]\n",
        )?;

        let status = managed_config_status_for_plan(&plan)?;

        assert_eq!(status, ManagedConfigStatus::Changed);
        Ok(())
    }

    #[test]
    fn verify_treats_managed_launch_marker_mismatch_as_changed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-launch-marker-mismatch")?;
        fs::create_dir_all(repo.join(".codex"))?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let target = repo.join(".codex/config.toml");
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace(
                "VOLICORD_MCP_PROJECT_ID = \"project_alpha\"",
                "VOLICORD_MCP_PROJECT_ID = \"project_beta\"",
            ),
        )?;

        let status = managed_config_status_for_plan(&plan)?;

        assert_eq!(status, ManagedConfigStatus::Changed);
        Ok(())
    }

    fn request<'a>(
        scope: HostScope,
        repo_root: Option<&'a Path>,
        mcp_command: &'a Path,
    ) -> HostPlanRequest<'a> {
        let connection_intent = match scope {
            HostScope::User => ConnectionIntent::Personal,
            HostScope::Project => ConnectionIntent::Shared,
            _ => ConnectionIntent::Personal,
        };
        HostPlanRequest {
            host_kind: HostKind::Codex,
            connection_intent,
            project: repo_root.map(|repo_root| ProjectContext {
                project_id: "project_alpha",
                project_name: "Alpha",
                repo_root,
            }),
            installation_profile: InstallationProfile {
                runtime_home: Path::new("/runtime"),
                volicord_command: Path::new("/bin/volicord"),
                volicord_mcp_command: mcp_command,
                default_connection_mode: "workflow",
            },
            connection_id: "int_alpha",
            mode: "workflow",
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
            connection_intent: match scope {
                HostScope::Project => ConnectionIntent::Shared,
                _ => ConnectionIntent::Personal,
            },
            scope,
            connection_id: "int_alpha",
            project_id: (scope == HostScope::Project).then_some("project_alpha"),
            server_name: "volicord-existing",
            config_target,
            mcp_command,
            runtime_home,
            mode: "workflow",
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

    fn write_project_trust(
        codex_home: &Path,
        repo_root: &Path,
        trust_level: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(codex_home)?;
        fs::write(
            codex_home.join("config.toml"),
            format!(
                "[projects.\"{}\"]\ntrust_level = \"{}\"\n",
                repo_root.display(),
                trust_level
            ),
        )?;
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
