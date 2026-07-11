use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};
use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        files::VOLICORD_POLICY_SCHEMA, hooks::GuardCommandSpec, public_host_label,
        GuardIntegrationError,
    },
    host_integration::{HostKind, HostLifecyclePhase, ManagedServerEntry, REQUIRED_GUARD_PHASES},
};

pub(crate) fn policy_json(
    host_kind: HostKind,
    profile: IntegrationProfile,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
) -> Result<Value, GuardIntegrationError> {
    validate_policy_mcp_environment(&mcp_entry.env)?;
    let commands = guard_commands
        .iter()
        .map(|(phase, spec)| {
            (
                phase.clone(),
                json!({
                    "command": &spec.command,
                    "args": &spec.args,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    Ok(json!({
        "schema": VOLICORD_POLICY_SCHEMA,
        "managed_by": "volicord",
        "host": public_host_label(host_kind),
        "repo_root": path_text(repo_root),
        "connection_id": connection_id,
        "guard_installation_id": guard_installation_id,
        "selected_profile": profile.as_str(),
        "mcp": {
            "command": &mcp_entry.command,
            "args": &mcp_entry.args,
            "env": &mcp_entry.env,
        },
        "host_hook": {
            "enabled": profile != IntegrationProfile::Record,
            "commands": commands,
        },
    }))
}

fn validate_policy_mcp_environment(
    environment: &BTreeMap<String, String>,
) -> Result<(), GuardIntegrationError> {
    const ALLOWED_KEYS: &[&str] = &[
        "VOLICORD_HOME",
        "VOLICORD_MCP_LAUNCH",
        "VOLICORD_MCP_HOST",
        "VOLICORD_MCP_CONNECTION_ID",
        "VOLICORD_MCP_PROJECT_ID",
    ];
    for key in environment.keys() {
        if ALLOWED_KEYS.contains(&key.as_str()) {
            continue;
        }
        let description = if secret_like_env_key(key) {
            "secret-like"
        } else {
            "unapproved"
        };
        return Err(GuardIntegrationError::runtime(format!(
            "policy serialization refuses {description} MCP environment key {key}; only Volicord-owned launch metadata keys are allowed"
        )));
    }
    Ok(())
}

fn secret_like_env_key(key: &str) -> bool {
    let normalized = key.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "API_KEY",
        "PRIVATE_KEY",
        "CREDENTIAL",
        "AUTH",
        "COOKIE",
        "BEARER",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

pub(crate) fn required_guard_phase_names() -> Vec<&'static str> {
    REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| phase.capability_name())
        .collect()
}

pub(crate) fn lifecycle_phase_names(phases: &[HostLifecyclePhase]) -> Vec<&'static str> {
    phases.iter().map(|phase| phase.capability_name()).collect()
}

pub(crate) fn guard_has_prompt_capture_commands(policy: &Value) -> bool {
    policy
        .get("host_hook")
        .and_then(|guard| guard.get("commands"))
        .and_then(|commands| commands.get("prompt_capture"))
        .is_some()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with_env(key: &str, value: &str) -> ManagedServerEntry {
        ManagedServerEntry {
            command: "volicord".to_owned(),
            args: vec![
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--connection".to_owned(),
                "conn_test".to_owned(),
            ],
            env: BTreeMap::from([(key.to_owned(), value.to_owned())]),
        }
    }

    #[test]
    fn policy_accepts_only_volicord_launch_environment() -> Result<(), Box<dyn std::error::Error>> {
        let policy = policy_json(
            HostKind::Codex,
            IntegrationProfile::Record,
            Path::new("/repo"),
            "conn_test",
            "guard_test",
            &entry_with_env("VOLICORD_HOME", "/runtime"),
            &BTreeMap::new(),
        )?;

        assert_eq!(policy["mcp"]["env"]["VOLICORD_HOME"], "/runtime");
        Ok(())
    }

    #[test]
    fn policy_rejects_secret_like_environment_without_echoing_value() {
        let error = policy_json(
            HostKind::Codex,
            IntegrationProfile::Record,
            Path::new("/repo"),
            "conn_test",
            "guard_test",
            &entry_with_env("SERVICE_API_TOKEN", "do-not-print-this"),
            &BTreeMap::new(),
        )
        .expect_err("secret-like environment must be rejected");

        assert!(error
            .to_string()
            .contains("secret-like MCP environment key"));
        assert!(error.to_string().contains("SERVICE_API_TOKEN"));
        assert!(!error.to_string().contains("do-not-print-this"));
    }

    #[test]
    fn policy_rejects_other_unapproved_environment_keys() {
        let error = policy_json(
            HostKind::ClaudeCode,
            IntegrationProfile::Record,
            Path::new("/repo"),
            "conn_test",
            "guard_test",
            &entry_with_env("RUST_LOG", "debug"),
            &BTreeMap::new(),
        )
        .expect_err("unapproved environment must be rejected");

        assert!(error
            .to_string()
            .contains("unapproved MCP environment key RUST_LOG"));
    }
}
