use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::host_integration::{
    config_edit::read_json_object,
    current_entry_fingerprint_from_json, is_volicord_managed_entry, managed_entry_from_json,
    managed_fingerprint,
    verification::{HostConfigurationStatus, ManagedConfigStatus, Verification},
    HostConfigError, HostConflict, HostConflictKind, HostKind, HostPlan, HostScope,
    ManagedServerEntry, PlannedChange, DEFAULT_MCP_COMMAND,
};

pub(crate) fn project_settings_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".claude").join("settings.json")
}

pub(crate) fn project_local_settings_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".claude").join("settings.local.json")
}

pub(crate) fn project_rule_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".claude").join("rules").join("volicord.md")
}

pub(crate) fn project_rule_block(policy_path: &str, command_lines: &[(String, String)]) -> String {
    let mut block = format!(
        "# Volicord\n\nUse the repository-local `{policy_path}` detective host-hook policy. Do not resolve user-owned actions through the Agent Connection.\n\nConfigured local detective host-hook commands:\n"
    );
    for (phase, command) in command_lines {
        block.push_str(&format!("- `{phase}`: `{command}`\n"));
    }
    block
}

pub(super) fn classify_existing_json_entry(
    scope: HostScope,
    server_name: &str,
    value: &Value,
    desired_fingerprint: &str,
    expected_fingerprint: Option<&str>,
    conflicts: &mut Vec<HostConflict>,
    label: &str,
) -> PlannedChange {
    let Some(entry) = managed_entry_from_json(value) else {
        conflicts.push(HostConflict::new(
            HostConflictKind::UnmanagedNameCollision,
            format!("{label} is already configured by an unmanaged entry: {server_name}"),
        ));
        return PlannedChange::Noop;
    };
    if !is_claude_managed_identity_candidate(&entry) {
        conflicts.push(HostConflict::new(
            HostConflictKind::UnmanagedNameCollision,
            format!("{label} is already configured by an unmanaged entry: {server_name}"),
        ));
        return PlannedChange::Noop;
    };
    let current = managed_fingerprint(HostKind::ClaudeCode, scope, server_name, &entry);
    if current == desired_fingerprint {
        PlannedChange::Noop
    } else if expected_fingerprint == Some(current.as_str()) {
        PlannedChange::Update
    } else {
        conflicts.push(HostConflict::new(
            HostConflictKind::FingerprintMismatch,
            format!(
                "{label} is already configured by a different Volicord-managed entry: {server_name}"
            ),
        ));
        PlannedChange::Noop
    }
}

pub(super) fn is_claude_managed_identity_candidate(entry: &ManagedServerEntry) -> bool {
    is_volicord_managed_entry(entry)
        || command_is_volicord(entry)
        || args_have_volicord_mcp_binding(&entry.args)
}

fn command_is_volicord(entry: &ManagedServerEntry) -> bool {
    Path::new(&entry.command)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == DEFAULT_MCP_COMMAND)
}

fn args_have_volicord_mcp_binding(args: &[String]) -> bool {
    (args.len() == 4 || args.len() == 6)
        && args[0] == "mcp"
        && args[1] == "--stdio"
        && args[2] == "--connection"
        && !args[3].trim().is_empty()
        && (args.len() == 4 || (args[4] == "--project" && !args[5].trim().is_empty()))
}

pub(super) fn validate_mcp_command(
    scope: HostScope,
    command: &Path,
) -> Result<(), HostConfigError> {
    if scope == HostScope::Project {
        if command == Path::new(DEFAULT_MCP_COMMAND) {
            return Ok(());
        }
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Claude Code project-scoped configuration must use volicord from PATH",
        )));
    }
    if command.is_absolute() {
        Ok(())
    } else {
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::InvalidCommand,
            "Claude Code local and user scopes require an absolute volicord command path",
        )))
    }
}

pub(super) fn upsert_project_entry(
    object: &mut serde_json::Map<String, Value>,
    server_name: &str,
    entry: &ManagedServerEntry,
) -> Result<(), HostConfigError> {
    let servers = object
        .entry("mcpServers".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            HostConfigError::Malformed(
                "Claude Code .mcp.json mcpServers must be an object".to_owned(),
            )
        })?;
    servers.insert(server_name.to_owned(), entry.to_json_value());
    Ok(())
}

pub(super) fn remove_project_entry(
    object: &mut serde_json::Map<String, Value>,
    server_name: &str,
) -> Result<(), HostConfigError> {
    let Some(servers) = object.get_mut("mcpServers").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    servers.remove(server_name);
    Ok(())
}

pub(super) fn current_project_entry_fingerprint(
    server_name: &str,
    value: &Value,
) -> Option<String> {
    current_entry_fingerprint_from_json(
        HostKind::ClaudeCode,
        HostScope::Project,
        server_name,
        value,
    )
}

pub(super) fn verify_claude_project_entry(
    plan: &HostPlan,
) -> Result<ManagedConfigStatus, HostConfigError> {
    let crate::host_integration::HostTarget::File(target) = &plan.target else {
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
    let Some(entry) = managed_entry_from_json(existing) else {
        return Ok(ManagedConfigStatus::Malformed);
    };
    if !is_claude_managed_identity_candidate(&entry) {
        return Ok(ManagedConfigStatus::Unmanaged);
    }
    let current = managed_fingerprint(
        HostKind::ClaudeCode,
        HostScope::Project,
        &plan.server_name,
        &entry,
    );
    if current == plan.fingerprint {
        Ok(ManagedConfigStatus::Match)
    } else {
        Ok(ManagedConfigStatus::Changed)
    }
}

pub(super) fn verification_from_managed_status(
    status: ManagedConfigStatus,
    details: String,
) -> Verification {
    match status {
        ManagedConfigStatus::Missing => Verification::missing(details),
        ManagedConfigStatus::Unmanaged => {
            Verification::changed(details).with_managed_config(ManagedConfigStatus::Unmanaged)
        }
        ManagedConfigStatus::Changed => Verification::changed(details),
        ManagedConfigStatus::Malformed => Verification::failed(details)
            .with_managed_config(ManagedConfigStatus::Malformed)
            .with_host_configuration(HostConfigurationStatus::Malformed),
        ManagedConfigStatus::Match => Verification::configured_ready(details),
        ManagedConfigStatus::NotApplicable | ManagedConfigStatus::Unknown => {
            Verification::unknown(details)
        }
    }
}
