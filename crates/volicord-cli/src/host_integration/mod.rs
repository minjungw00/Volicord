use std::{
    fmt,
    path::{Path, PathBuf},
};

use serde::Serialize;
use volicord_mcp::ManagedMcpLaunchSpec;
use volicord_types::GuardHookPhase;
pub use volicord_types::{ConnectionIntent, HostKind, HostScope};

pub mod codex;
pub mod config_edit;
pub mod contracts;
pub mod process;
pub mod verification;

pub const DEFAULT_SERVER_NAME: &str = "volicord";
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
pub struct HostPlan {
    pub host_kind: HostKind,
    pub connection_intent: ConnectionIntent,
    pub host_scope: HostScope,
    pub mode: String,
    pub server_name: String,
    pub target: HostTarget,
    pub entry: ManagedMcpLaunchSpec,
    pub change: PlannedChange,
    pub fingerprint: String,
    pub conflicts: Vec<HostConflict>,
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
