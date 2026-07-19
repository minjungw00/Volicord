use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_mcp::{RepositoryDiscoveryDescriptor, RepositoryDiscoveryHost};
use volicord_types::GuardHookPhase;
pub use volicord_types::{ConnectionIntent, HostKind, HostScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionKind {
    HostTrustRequired,
    RepairManagedConfig,
    InstallOrRepairCodex,
    RepairMcpServer,
    ReloadHost,
    UseVolicordTool,
    ReloadGuard,
    RepairGuard,
    ReloadRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAction {
    pub kind: UserActionKind,
    pub message: String,
}

impl UserAction {
    pub fn new(kind: UserActionKind, message: impl Into<String>) -> Self {
        let message = message.into();
        assert!(
            !message.is_empty() && message.len() <= 4_096 && !message.as_bytes().contains(&0),
            "adapter-owned user-action text must satisfy the canonical report bounds"
        );
        Self { kind, message }
    }
}

pub mod codex;
pub mod config_edit;
pub mod contracts;
pub mod process;
pub mod verification;

pub const DEFAULT_SERVER_NAME: &str = "volicord";
pub const DEFAULT_MCP_COMMAND: &str = "volicord";
pub const MANAGED_WRAPPER_ENV: &str = "VOLICORD_MANAGED_WRAPPER";
pub const MANAGED_WRAPPER_VALUE: &str = "codex-record";
static CODEX_SUPPORTED_CONNECTION_INTENTS: [ConnectionIntent; 2] =
    [ConnectionIntent::Personal, ConnectionIntent::Shared];

pub fn supported_connection_intents(host_kind: HostKind) -> &'static [ConnectionIntent] {
    match host_kind {
        HostKind::Codex => &CODEX_SUPPORTED_CONNECTION_INTENTS,
    }
}

pub fn supports_connection_intent(host_kind: HostKind, intent: ConnectionIntent) -> bool {
    supported_connection_intents(host_kind).contains(&intent)
}

pub fn format_supported_connection_intents(host_kind: HostKind) -> String {
    let intents = supported_connection_intents(host_kind);
    if intents.is_empty() {
        return "none".to_owned();
    }
    intents
        .iter()
        .map(|intent| intent.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone, Copy)]
pub struct InstallationProfile<'a> {
    pub runtime_home: &'a Path,
    pub volicord_command: &'a Path,
    pub volicord_mcp_command: &'a Path,
    pub default_connection_mode: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectContext<'a> {
    pub project_id: &'a str,
    pub project_name: &'a str,
    pub repo_root: &'a Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HostCapabilities {
    pub stdio_mcp: bool,
    pub pre_tool_hook: bool,
    pub post_tool_hook: bool,
    pub user_prompt_submit_hook: bool,
    pub rule_file_support: bool,
    pub project_local_configuration: bool,
}

impl HostCapabilities {
    pub fn supports_phase(self, phase: GuardHookPhase) -> bool {
        match phase {
            GuardHookPhase::PreTool => self.pre_tool_hook,
            GuardHookPhase::PostTool => self.post_tool_hook,
            GuardHookPhase::PromptCapture => self.user_prompt_submit_hook,
        }
    }

    pub fn missing_required_hook_phases(self) -> Vec<GuardHookPhase> {
        GuardHookPhase::REQUIRED
            .iter()
            .copied()
            .filter(|phase| !self.supports_phase(*phase))
            .collect()
    }
}

pub fn guard_phase_capability_name(phase: GuardHookPhase) -> &'static str {
    match phase {
        GuardHookPhase::PreTool => "pre_tool_hook",
        GuardHookPhase::PostTool => "post_tool_hook",
        GuardHookPhase::PromptCapture => "user_prompt_submit_hook",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostIntegrationFileKind {
    VolicordPolicy,
    GitInfoExclude,
    HostMcpConfig,
    HostHookConfig,
    HostHookDispatch,
    HostHookWrapper,
    HostRuleInstruction,
    AgentsManagedBlock,
}

impl HostIntegrationFileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VolicordPolicy => "volicord_policy",
            Self::GitInfoExclude => "git_info_exclude",
            Self::HostMcpConfig => "host_mcp_config",
            Self::HostHookConfig => "host_hook_config",
            Self::HostHookDispatch => "host_hook_dispatch",
            Self::HostHookWrapper => "host_hook_wrapper",
            Self::HostRuleInstruction => "host_rule_instruction",
            Self::AgentsManagedBlock => "agents_managed_block",
        }
    }
}

pub fn host_capabilities(host_kind: HostKind) -> HostCapabilities {
    match host_kind {
        HostKind::Codex => codex::capabilities(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HostPlanRequest<'a> {
    pub host_kind: HostKind,
    pub connection_intent: ConnectionIntent,
    pub project: Option<ProjectContext<'a>>,
    pub installation_profile: InstallationProfile<'a>,
    pub connection_id: &'a str,
    pub mode: &'a str,
    pub expected_fingerprint: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedServerEntry {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub env_vars: Vec<String>,
}

impl ManagedServerEntry {
    pub fn new(connection_id: impl Into<String>, mcp_command: &Path) -> Self {
        Self::new_project_bound(connection_id, None, mcp_command)
    }

    pub fn new_project_bound(
        connection_id: impl Into<String>,
        project_id: Option<&str>,
        mcp_command: &Path,
    ) -> Self {
        let connection_id = connection_id.into();
        let mut args = vec![
            "mcp".to_owned(),
            "--stdio".to_owned(),
            "--connection".to_owned(),
            connection_id.clone(),
        ];
        if let Some(project_id) = project_id {
            args.push("--project".to_owned());
            args.push(project_id.to_owned());
        }
        let mut env = BTreeMap::from([
            ("VOLICORD_MCP_LAUNCH".to_owned(), "managed_host".to_owned()),
            ("VOLICORD_MCP_HOST".to_owned(), "codex".to_owned()),
            ("VOLICORD_MCP_CONNECTION_ID".to_owned(), connection_id),
        ]);
        if let Some(project_id) = project_id {
            env.insert("VOLICORD_MCP_PROJECT_ID".to_owned(), project_id.to_owned());
        }
        Self {
            command: mcp_command.display().to_string(),
            args,
            env,
            env_vars: vec!["VOLICORD_HOME".to_owned()],
        }
    }

    pub fn new_repository_discovery(host: RepositoryDiscoveryHost) -> Self {
        let descriptor = RepositoryDiscoveryDescriptor::new(host);
        Self {
            command: RepositoryDiscoveryDescriptor::COMMAND.to_owned(),
            args: descriptor.args(),
            env: descriptor.env(),
            env_vars: descriptor.env_vars(),
        }
    }

    pub fn validate_repository_discovery(
        &self,
        host: RepositoryDiscoveryHost,
    ) -> Result<(), HostConfigError> {
        RepositoryDiscoveryDescriptor::new(host)
            .validate_entry(&self.command, &self.args, &self.env, &self.env_vars)
            .map_err(|error| {
                HostConfigError::Conflict(HostConflict::new(
                    HostConflictKind::InvalidCommand,
                    error.to_string(),
                ))
            })
    }

    pub fn to_json_value(&self) -> Value {
        let mut entry = serde_json::Map::new();
        entry.insert("command".to_owned(), Value::String(self.command.clone()));
        entry.insert(
            "args".to_owned(),
            Value::Array(self.args.iter().cloned().map(Value::String).collect()),
        );
        if !self.env.is_empty() {
            entry.insert(
                "env".to_owned(),
                Value::Object(
                    self.env
                        .iter()
                        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                        .collect(),
                ),
            );
        }
        if !self.env_vars.is_empty() {
            entry.insert(
                "env_vars".to_owned(),
                Value::Array(self.env_vars.iter().cloned().map(Value::String).collect()),
            );
        }
        Value::Object(entry)
    }
}

pub(crate) fn validate_managed_server_entry_schema(
    host_kind: HostKind,
    host_scope: HostScope,
    entry: &ManagedServerEntry,
) -> Result<(), HostConfigError> {
    let _ = host_kind;
    let discovery_host = RepositoryDiscoveryHost::Codex;
    if host_scope == HostScope::Project {
        return entry.validate_repository_discovery(discovery_host);
    }
    if entry
        .validate_repository_discovery(RepositoryDiscoveryHost::Codex)
        .is_ok()
    {
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "local host configuration must use an explicit local connection binding",
        )));
    }
    if !is_volicord_managed_entry(entry) {
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "local host configuration requires a connection-bound Volicord MCP command",
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPlan {
    pub host_kind: HostKind,
    pub connection_intent: ConnectionIntent,
    pub host_scope: HostScope,
    pub mode: String,
    pub server_name: String,
    pub target: HostTarget,
    pub entry: ManagedServerEntry,
    pub change: PlannedChange,
    pub fingerprint: String,
    pub conflicts: Vec<HostConflict>,
    pub user_actions: Vec<UserAction>,
    pub(crate) file_snapshot: Option<config_edit::FileSnapshot>,
}

impl HostPlan {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostTarget {
    File(PathBuf),
    ExternalCli {
        program: String,
        cwd: Option<PathBuf>,
    },
    Export(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedChange {
    Create,
    Update,
    Remove,
    Noop,
    ExternalCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostConflict {
    pub kind: HostConflictKind,
    pub message: String,
}

impl HostConflict {
    pub fn new(kind: HostConflictKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostConflictKind {
    InvalidScope,
    InvalidServerName,
    InvalidCommand,
    UnsafeTarget,
    MalformedConfiguration,
    UnmanagedNameCollision,
    FingerprintMismatch,
    StalePlan,
    ExternalCommandFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEffect {
    pub host_kind: HostKind,
    pub connection_intent: ConnectionIntent,
    pub host_scope: HostScope,
    pub mode: String,
    pub server_name: String,
    pub target: HostTarget,
    pub change: PlannedChange,
    pub fingerprint: String,
    pub user_actions: Vec<UserAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostConfigError {
    Conflict(HostConflict),
    Io(String),
    Malformed(String),
    StalePlan(String),
    ExternalCommand(String),
}

impl fmt::Display for HostConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict(conflict) => formatter.write_str(&conflict.message),
            Self::Io(message)
            | Self::Malformed(message)
            | Self::StalePlan(message)
            | Self::ExternalCommand(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for HostConfigError {}

impl From<HostConflict> for HostConfigError {
    fn from(conflict: HostConflict) -> Self {
        Self::Conflict(conflict)
    }
}

pub trait HostAdapter {
    fn capabilities(&self) -> HostCapabilities;
    fn detect(&self) -> Result<HostDetection, HostConfigError>;
    fn apply(&mut self, plan: &HostPlan) -> Result<HostEffect, HostConfigError>;
    fn verify(&mut self, plan: &HostPlan) -> Result<verification::Verification, HostConfigError>;
    fn remove(&mut self, request: HostRemoveRequest) -> Result<HostEffect, HostConfigError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDetection {
    pub host_kind: HostKind,
    pub available: bool,
    pub host_version: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRemoveRequest {
    pub host_kind: HostKind,
    pub connection_intent: ConnectionIntent,
    pub host_scope: HostScope,
    pub mode: String,
    pub server_name: String,
    pub target: HostTarget,
    pub expected_fingerprint: String,
}

pub fn default_server_name(connection_id: &str) -> String {
    let _ = connection_id;
    DEFAULT_SERVER_NAME.to_owned()
}

pub fn validated_server_name(
    connection_id: &str,
    explicit: Option<&str>,
) -> Result<String, HostConflict> {
    let name = explicit
        .map(str::to_owned)
        .unwrap_or_else(|| default_server_name(connection_id));
    if is_valid_server_name(&name) {
        Ok(name)
    } else {
        Err(HostConflict::new(
            HostConflictKind::InvalidServerName,
            format!(
                "server name must use ASCII letters, numbers, hyphen, or underscore and start with a letter or number: {name}"
            ),
        ))
    }
}

pub fn is_valid_server_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub fn managed_configuration_digest(
    host_kind: HostKind,
    host_scope: HostScope,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"volicord.codex-managed-configuration\0");
    digest.update(host_kind.as_str().as_bytes());
    digest.update([0]);
    digest.update(host_scope.as_str().as_bytes());
    digest.update([0]);
    digest.update(server_name.as_bytes());
    digest.update([0]);
    digest.update(
        serde_json::to_vec(&entry.to_json_value())
            .expect("managed configuration projection should serialize"),
    );
    format!("sha256:{:x}", digest.finalize())
}

pub(crate) fn is_volicord_managed_entry(entry: &ManagedServerEntry) -> bool {
    if entry
        .validate_repository_discovery(RepositoryDiscoveryHost::Codex)
        .is_ok()
    {
        return true;
    }
    if !matches!(entry.args.len(), 4 | 6)
        || entry.env_vars != ["VOLICORD_HOME"]
        || entry.args[0] != "mcp"
        || entry.args[1] != "--stdio"
        || entry.args[2] != "--connection"
        || entry.args[3].trim().is_empty()
    {
        return false;
    }
    let expected_env = BTreeMap::from([
        ("VOLICORD_MCP_LAUNCH".to_owned(), "managed_host".to_owned()),
        ("VOLICORD_MCP_HOST".to_owned(), "codex".to_owned()),
        (
            "VOLICORD_MCP_CONNECTION_ID".to_owned(),
            entry.args[3].clone(),
        ),
    ]);
    let expected_env = if entry.args.len() == 6
        && entry.args[4] == "--project"
        && !entry.args[5].trim().is_empty()
    {
        let mut expected_env = expected_env;
        expected_env.insert("VOLICORD_MCP_PROJECT_ID".to_owned(), entry.args[5].clone());
        expected_env
    } else if entry.args.len() == 4 {
        expected_env
    } else {
        return false;
    };
    if entry.env != expected_env {
        return false;
    }
    let command = Path::new(&entry.command);
    command.is_absolute()
        && command
            .file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == DEFAULT_MCP_COMMAND)
        && command
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| extension.eq_ignore_ascii_case("exe"))
}
