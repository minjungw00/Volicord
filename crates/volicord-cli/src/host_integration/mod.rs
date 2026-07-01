use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub mod claude_code;
pub mod codex;
pub mod config_edit;
pub mod generic;
pub mod verification;

pub const DEFAULT_SERVER_NAME: &str = "volicord";
pub const DEFAULT_MCP_COMMAND: &str = "volicord";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    Codex,
    ClaudeCode,
    Generic,
}

impl HostKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostScope {
    User,
    Project,
    Local,
    Export,
}

impl HostScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
            Self::Export => "export",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionIntent {
    Personal,
    Shared,
    Global,
}

static CODEX_SUPPORTED_CONNECTION_INTENTS: [ConnectionIntent; 2] =
    [ConnectionIntent::Personal, ConnectionIntent::Shared];
static CLAUDE_CODE_SUPPORTED_CONNECTION_INTENTS: [ConnectionIntent; 3] = [
    ConnectionIntent::Personal,
    ConnectionIntent::Shared,
    ConnectionIntent::Global,
];
static GENERIC_SUPPORTED_CONNECTION_INTENTS: [ConnectionIntent; 0] = [];

impl ConnectionIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Shared => "shared",
            Self::Global => "global",
        }
    }
}

pub fn supported_connection_intents(host_kind: HostKind) -> &'static [ConnectionIntent] {
    match host_kind {
        HostKind::Codex => &CODEX_SUPPORTED_CONNECTION_INTENTS,
        HostKind::ClaudeCode => &CLAUDE_CODE_SUPPORTED_CONNECTION_INTENTS,
        HostKind::Generic => &GENERIC_SUPPORTED_CONNECTION_INTENTS,
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
    pub http_mcp: bool,
    pub session_start_hook: bool,
    pub pre_tool_hook: bool,
    pub post_tool_hook: bool,
    pub user_prompt_submit_hook: bool,
    pub stop_hook: bool,
    pub rule_file_support: bool,
    pub project_local_configuration: bool,
}

impl HostCapabilities {
    pub fn supports_phase(self, phase: HostLifecyclePhase) -> bool {
        match phase {
            HostLifecyclePhase::SessionStart => self.session_start_hook,
            HostLifecyclePhase::PreTool => self.pre_tool_hook,
            HostLifecyclePhase::PostTool => self.post_tool_hook,
            HostLifecyclePhase::UserPromptSubmit => self.user_prompt_submit_hook,
            HostLifecyclePhase::Stop => self.stop_hook,
        }
    }

    pub fn missing_required_guard_phases(self) -> Vec<HostLifecyclePhase> {
        REQUIRED_GUARD_PHASES
            .iter()
            .copied()
            .filter(|phase| !self.supports_phase(*phase))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostLifecyclePhase {
    SessionStart,
    PreTool,
    PostTool,
    UserPromptSubmit,
    Stop,
}

impl HostLifecyclePhase {
    pub fn policy_key(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::PreTool => "pre_tool",
            Self::PostTool => "post_tool",
            Self::UserPromptSubmit => "prompt_capture",
            Self::Stop => "stop",
        }
    }

    pub fn command_name(self) -> &'static str {
        match self {
            Self::SessionStart => "session-start",
            Self::PreTool => "pre-tool",
            Self::PostTool => "post-tool",
            Self::UserPromptSubmit => "prompt-capture",
            Self::Stop => "stop",
        }
    }

    pub fn capability_name(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start_hook",
            Self::PreTool => "pre_tool_hook",
            Self::PostTool => "post_tool_hook",
            Self::UserPromptSubmit => "user_prompt_submit_hook",
            Self::Stop => "stop_hook",
        }
    }
}

pub const REQUIRED_GUARD_PHASES: [HostLifecyclePhase; 5] = [
    HostLifecyclePhase::SessionStart,
    HostLifecyclePhase::PreTool,
    HostLifecyclePhase::PostTool,
    HostLifecyclePhase::UserPromptSubmit,
    HostLifecyclePhase::Stop,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostIntegrationFileKind {
    VolicordPolicy,
    HostMcpConfig,
    HostHookConfig,
    HostRuleInstruction,
    AgentsManagedBlock,
}

impl HostIntegrationFileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VolicordPolicy => "volicord_policy",
            Self::HostMcpConfig => "host_mcp_config",
            Self::HostHookConfig => "host_hook_config",
            Self::HostRuleInstruction => "host_rule_instruction",
            Self::AgentsManagedBlock => "agents_managed_block",
        }
    }
}

pub fn host_capabilities(host_kind: HostKind) -> HostCapabilities {
    match host_kind {
        HostKind::Codex => codex::capabilities(),
        HostKind::ClaudeCode => claude_code::capabilities(),
        HostKind::Generic => generic::capabilities(),
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
}

impl ManagedServerEntry {
    pub fn new(
        connection_id: impl Into<String>,
        mcp_command: &Path,
        runtime_home: Option<&Path>,
    ) -> Self {
        let mut env = BTreeMap::new();
        if let Some(runtime_home) = runtime_home {
            env.insert(
                "VOLICORD_HOME".to_owned(),
                runtime_home.display().to_string(),
            );
        }
        Self {
            command: mcp_command.display().to_string(),
            args: vec![
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--connection".to_owned(),
                connection_id.into(),
            ],
            env,
        }
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
        Value::Object(entry)
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAction {
    pub kind: UserActionKind,
    pub message: String,
}

impl UserAction {
    pub fn new(kind: UserActionKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserActionKind {
    HostTrustRequired,
    ProjectApprovalRequired,
    ReloadRequired,
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

pub fn export_file_name(connection_id: &str) -> String {
    let sanitized = sanitize_identifier(connection_id);
    let stem = if sanitized.is_empty() {
        short_hash(connection_id)
    } else {
        sanitized
    };
    format!("volicord-{stem}.mcp.json")
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

pub fn managed_fingerprint(
    host_kind: HostKind,
    host_scope: HostScope,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> String {
    let payload = json!({
        "format": "volicord-host-entry-v1",
        "host_kind": host_kind.as_str(),
        "host_scope": host_scope.as_str(),
        "server_name": server_name,
        "entry": {
            "command": entry.command,
            "args": entry.args,
            "env": entry.env,
        },
    });
    digest_json(&payload)
}

pub(crate) fn unmanaged_fingerprint(
    host_kind: HostKind,
    host_scope: HostScope,
    server_name: &str,
    raw: &str,
) -> String {
    let payload = json!({
        "format": "volicord-host-entry-v1",
        "host_kind": host_kind.as_str(),
        "host_scope": host_scope.as_str(),
        "server_name": server_name,
        "raw": raw,
    });
    digest_json(&payload)
}

fn digest_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("JSON fingerprint payload should serialize");
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn sanitize_identifier(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars().flat_map(char::to_lowercase) {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' {
            Some(ch)
        } else if ch == '-' || ch == '.' || ch == '/' || ch == ':' {
            Some('-')
        } else {
            None
        };
        if let Some(ch) = next {
            if ch == '-' {
                if last_dash {
                    continue;
                }
                last_dash = true;
            } else {
                last_dash = false;
            }
            out.push(ch);
        }
    }
    out.trim_matches('-').to_owned()
}

fn short_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut text = String::new();
    for byte in digest.iter().take(6) {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

pub(crate) fn current_entry_fingerprint_from_json(
    host_kind: HostKind,
    host_scope: HostScope,
    server_name: &str,
    value: &Value,
) -> Option<String> {
    let entry = managed_entry_from_json(value)?;
    Some(managed_fingerprint(
        host_kind,
        host_scope,
        server_name,
        &entry,
    ))
}

pub(crate) fn managed_entry_from_json(value: &Value) -> Option<ManagedServerEntry> {
    let object = value.as_object()?;
    let allowed_keys = ["command", "args", "env"];
    if object
        .keys()
        .any(|key| !allowed_keys.contains(&key.as_str()))
    {
        return None;
    }
    let command = object.get("command")?.as_str()?.to_owned();
    let args = object
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
                .map(|items| items.into_iter().map(str::to_owned).collect::<Vec<_>>())
        })
        .unwrap_or_else(|| Some(Vec::new()))?;
    let env = object
        .get("env")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_owned())))
                .collect::<Option<BTreeMap<_, _>>>()
        })
        .unwrap_or_else(|| Some(BTreeMap::new()))?;
    Some(ManagedServerEntry { command, args, env })
}

pub(crate) fn is_volicord_managed_entry(entry: &ManagedServerEntry) -> bool {
    if entry.args.len() != 4
        || entry.args[0] != "mcp"
        || entry.args[1] != "--stdio"
        || entry.args[2] != "--connection"
        || entry.args[3].trim().is_empty()
    {
        return false;
    }
    Path::new(&entry.command)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == DEFAULT_MCP_COMMAND)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_server_name_is_internal_host_key() {
        let first = default_server_name("integration/Alpha:One");
        let second = default_server_name("integration/Alpha:One");
        let other = default_server_name("integration/Alpha:Two");

        assert_eq!(first, second);
        assert_eq!(first, other);
        assert_eq!(first, DEFAULT_SERVER_NAME);
    }

    #[test]
    fn explicit_server_name_is_validated() {
        assert_eq!(
            validated_server_name("integration", Some("volicord_custom-1")).unwrap(),
            "volicord_custom-1"
        );
        assert_eq!(
            validated_server_name("integration", Some("-bad"))
                .expect_err("leading hyphen should fail")
                .kind,
            HostConflictKind::InvalidServerName
        );
        assert_eq!(
            validated_server_name("integration", Some("bad.name"))
                .expect_err("dot should fail")
                .kind,
            HostConflictKind::InvalidServerName
        );
    }

    #[test]
    fn fingerprint_changes_when_entry_changes() {
        let entry = ManagedServerEntry::new("integration", Path::new("/bin/volicord"), None);
        let mut changed = entry.clone();
        changed.args.push("--extra".to_owned());

        assert_ne!(
            managed_fingerprint(
                HostKind::Generic,
                HostScope::Export,
                "volicord-integration",
                &entry
            ),
            managed_fingerprint(
                HostKind::Generic,
                HostScope::Export,
                "volicord-integration",
                &changed
            )
        );
    }

    #[test]
    fn host_connection_intent_support_matrix_is_centralized() {
        assert_eq!(
            supported_connection_intents(HostKind::Codex),
            &[ConnectionIntent::Personal, ConnectionIntent::Shared]
        );
        assert_eq!(
            supported_connection_intents(HostKind::ClaudeCode),
            &[
                ConnectionIntent::Personal,
                ConnectionIntent::Shared,
                ConnectionIntent::Global
            ]
        );
        assert_eq!(supported_connection_intents(HostKind::Generic), &[]);

        assert!(supports_connection_intent(
            HostKind::Codex,
            ConnectionIntent::Personal
        ));
        assert!(supports_connection_intent(
            HostKind::Codex,
            ConnectionIntent::Shared
        ));
        assert!(!supports_connection_intent(
            HostKind::Codex,
            ConnectionIntent::Global
        ));
        assert!(supports_connection_intent(
            HostKind::ClaudeCode,
            ConnectionIntent::Global
        ));
        assert_eq!(
            format_supported_connection_intents(HostKind::Codex),
            "personal, shared"
        );
    }

    #[test]
    fn host_capability_matrix_is_explicit() {
        let codex = host_capabilities(HostKind::Codex);
        assert!(codex.stdio_mcp);
        assert!(codex.project_local_configuration);
        assert!(!codex.rule_file_support);
        assert_eq!(codex.missing_required_guard_phases(), REQUIRED_GUARD_PHASES);

        let claude = host_capabilities(HostKind::ClaudeCode);
        assert!(claude.stdio_mcp);
        assert!(claude.project_local_configuration);
        assert!(claude.rule_file_support);
        assert!(!claude.user_prompt_submit_hook);
        assert_eq!(
            claude.missing_required_guard_phases(),
            REQUIRED_GUARD_PHASES
        );
    }
}
