use std::path::Path;

use toml_edit::{value, Array, DocumentMut, Item, Table};
use volicord_mcp::ManagedMcpLaunchSpec;

use crate::host_integration::{config_edit::FileSnapshot, HostConfigError};

use super::identity::accepted_codex_tool_approval_overlay_item;

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
    entry: &ManagedMcpLaunchSpec,
) -> Result<(), HostConfigError> {
    if !document.as_table().contains_key("mcp_servers") {
        let mut servers = Table::new();
        servers.set_implicit(true);
        document["mcp_servers"] = Item::Table(servers);
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

fn server_table(entry: &ManagedMcpLaunchSpec) -> Table {
    let mut table = Table::new();
    table["command"] = value(entry.command());
    let mut args = Array::default();
    for arg in entry.args() {
        args.push(arg.as_str());
    }
    table["args"] = value(args);
    if !entry.environment().forwarded_names().is_empty() {
        let mut env_vars = Array::default();
        for env_var in entry.environment().forwarded_names() {
            env_vars.push(env_var.as_str());
        }
        table["env_vars"] = value(env_vars);
    }
    if !entry.environment().static_values().is_empty() {
        let mut env = Table::new();
        for (key, value_text) in entry.environment().static_values() {
            env[key] = value(value_text.clone());
        }
        table["env"] = Item::Table(env);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::HostKind;

    fn personal_launch() -> ManagedMcpLaunchSpec {
        ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
        )
        .expect("personal launch")
    }

    #[test]
    fn codex_toml_generation_has_exact_personal_and_shared_shapes() {
        let mut personal = DocumentMut::new();
        upsert_server_table(&mut personal, "volicord", &personal_launch()).expect("personal table");
        assert_eq!(
            personal.to_string(),
            "[mcp_servers.volicord]\ncommand = \"/opt/volicord/bin/volicord\"\nargs = [\"_host-launch\", \"codex\", \"--connection\", \"connection_alpha\"]\n\n[mcp_servers.volicord.env]\nVOLICORD_HOME = \"/srv/volicord/runtime\"\n"
        );

        let mut shared = DocumentMut::new();
        let shared_launch =
            ManagedMcpLaunchSpec::shared_repository(HostKind::Codex).expect("shared launch");
        upsert_server_table(&mut shared, "volicord", &shared_launch).expect("shared table");
        assert_eq!(
            shared.to_string(),
            "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"_host-launch\", \"codex\", \"--discover-repository\"]\nenv_vars = [\"VOLICORD_HOME\"]\n"
        );
    }

    #[test]
    fn codex_toml_generation_preserves_only_a_valid_tool_approval_overlay() {
        let mut document = "[mcp_servers.volicord]\ncommand = \"changed\"\nargs = []\n\n[mcp_servers.volicord.tools.\"volicord.status\"]\napproval_mode = \"auto\"\n"
            .parse::<DocumentMut>()
            .expect("Codex configuration");
        upsert_server_table(&mut document, "volicord", &personal_launch())
            .expect("managed replacement");

        let tools = document["mcp_servers"]["volicord"]["tools"]
            .as_table()
            .expect("preserved tools overlay");
        assert_eq!(
            tools["volicord.status"]["approval_mode"].as_str(),
            Some("auto")
        );
    }
}
