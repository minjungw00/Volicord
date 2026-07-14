use std::path::Path;

use toml_edit::{value, Array, DocumentMut, Item, Table};

use crate::host_integration::{
    config_edit::FileSnapshot, HostConfigError, HostConflict, HostConflictKind, HostScope,
    ManagedServerEntry, DEFAULT_MCP_COMMAND,
};

use super::identity::accepted_codex_tool_approval_overlay_item;

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

pub(super) fn parse_document(
    text: Option<&str>,
    target: &Path,
) -> Result<DocumentMut, HostConfigError> {
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

pub(super) fn document_from_snapshot(
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

pub(super) fn upsert_server_table(
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
    let preserved_tools = servers
        .get(server_name)
        .and_then(accepted_codex_tool_approval_overlay_item);
    let mut table = server_table(entry);
    if let Some(tools) = preserved_tools {
        table["tools"] = tools;
    }
    servers.insert(server_name, Item::Table(table));
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
    if !entry.env_vars.is_empty() {
        let mut env_vars = Array::default();
        for env_var in &entry.env_vars {
            env_vars.push(env_var.as_str());
        }
        table["env_vars"] = value(env_vars);
    }
    if !entry.env.is_empty() {
        let mut env = Table::new();
        for (key, value_text) in &entry.env {
            env[key] = value(value_text.clone());
        }
        table["env"] = Item::Table(env);
    }
    table
}
