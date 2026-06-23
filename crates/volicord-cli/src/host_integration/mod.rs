use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub mod claude_code;
pub mod codex;
pub mod config_edit;
pub mod generic;
pub mod verification;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedServerEntry {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl ManagedServerEntry {
    pub fn new(
        integration_id: impl Into<String>,
        mcp_command: &Path,
        runtime_home: Option<&Path>,
    ) -> Self {
        let mut env = BTreeMap::new();
        if let Some(runtime_home) = runtime_home {
            env.insert(
                "HARNESS_HOME".to_owned(),
                runtime_home.display().to_string(),
            );
        }
        Self {
            command: mcp_command.display().to_string(),
            args: vec!["--integration".to_owned(), integration_id.into()],
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
    pub host_scope: HostScope,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionKind {
    HostTrustRequired,
    ProjectApprovalRequired,
    ReloadRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEffect {
    pub host_kind: HostKind,
    pub host_scope: HostScope,
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
    pub host_scope: HostScope,
    pub server_name: String,
    pub target: HostTarget,
    pub expected_fingerprint: String,
}

pub fn default_server_name(integration_id: &str) -> String {
    let sanitized = sanitize_identifier(integration_id);
    let suffix = short_hash(integration_id);
    if sanitized.is_empty() {
        return format!("harness-{suffix}");
    }
    let base = format!("harness-{sanitized}");
    if base.len() <= 48 {
        base
    } else {
        let keep = 48usize.saturating_sub(suffix.len() + 1);
        format!(
            "{}-{suffix}",
            base.trim_end_matches('-')
                .chars()
                .take(keep)
                .collect::<String>()
        )
    }
}

pub fn export_file_name(integration_id: &str) -> String {
    let sanitized = sanitize_identifier(integration_id);
    let stem = if sanitized.is_empty() {
        short_hash(integration_id)
    } else {
        sanitized
    };
    format!("harness-{stem}.mcp.json")
}

pub fn validated_server_name(
    integration_id: &str,
    explicit: Option<&str>,
) -> Result<String, HostConflict> {
    let name = explicit
        .map(str::to_owned)
        .unwrap_or_else(|| default_server_name(integration_id));
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
        "format": "harness-host-entry-v1",
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
        "format": "harness-host-entry-v1",
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
    let object = value.as_object()?;
    let allowed_keys = ["command", "args", "env"];
    if object
        .keys()
        .any(|key| !allowed_keys.contains(&key.as_str()))
    {
        return Some(unmanaged_fingerprint(
            host_kind,
            host_scope,
            server_name,
            &value.to_string(),
        ));
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
    Some(managed_fingerprint(
        host_kind,
        host_scope,
        server_name,
        &ManagedServerEntry { command, args, env },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_server_name_uses_integration_identity() {
        let first = default_server_name("integration/Alpha:One");
        let second = default_server_name("integration/Alpha:One");
        let other = default_server_name("integration/Alpha:Two");

        assert_eq!(first, second);
        assert_ne!(first, other);
        assert!(first.starts_with("harness-integration-alpha-one"));
        assert_ne!(first, "harness-agent");
    }

    #[test]
    fn explicit_server_name_is_validated() {
        assert_eq!(
            validated_server_name("integration", Some("harness_custom-1")).unwrap(),
            "harness_custom-1"
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
        let entry = ManagedServerEntry::new("integration", Path::new("/bin/harness-mcp"), None);
        let mut changed = entry.clone();
        changed.args.push("--extra".to_owned());

        assert_ne!(
            managed_fingerprint(
                HostKind::Generic,
                HostScope::Export,
                "harness-integration",
                &entry
            ),
            managed_fingerprint(
                HostKind::Generic,
                HostScope::Export,
                "harness-integration",
                &changed
            )
        );
    }
}
