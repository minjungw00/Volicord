use std::{collections::BTreeMap, path::Path};

use toml_edit::{Item, Table};
use volicord_mcp::ManagedMcpLaunchSpec;
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES, WORKFLOW_METHOD_TOOL_NAMES,
};

use crate::host_integration::verification::ManagedConfigStatus;
use crate::host_integration::{
    config_edit::read_text_snapshot, HostConfigError, HostConflict, HostConflictKind, HostKind,
    HostPlan, HostScope, HostTarget, PlannedChange,
};

use super::config::parse_document;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCodexManagedIdentity {
    managed_entry: ManagedMcpLaunchSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexManagedIdentityProblem {
    Unmanaged,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexManagedIdentityEvaluation {
    pub(crate) status: ManagedConfigStatus,
    pub(crate) details: String,
}

pub(super) fn codex_managed_launch_spec(
    scope: HostScope,
    connection_id: impl Into<String>,
    project_id: Option<&str>,
    mcp_command: &Path,
    runtime_home: Option<&Path>,
) -> Result<ManagedMcpLaunchSpec, HostConfigError> {
    if scope == HostScope::Project {
        let spec = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .map_err(managed_launch_error)?;
        if mcp_command != Path::new(spec.command()) || runtime_home.is_some() {
            return Err(managed_launch_error(
                "shared Codex managed MCP launch requires PATH-resolved volicord and no static Runtime Home",
            ));
        }
        return Ok(spec);
    }
    let runtime_home = runtime_home.ok_or_else(|| {
        managed_launch_error("personal Codex managed MCP launch requires a selected Runtime Home")
    })?;
    ManagedMcpLaunchSpec::personal(mcp_command, runtime_home, connection_id, project_id)
        .map_err(managed_launch_error)
}

pub(super) fn classify_existing_codex_entry(
    scope: HostScope,
    server_name: &str,
    item: &Item,
    desired_fingerprint: &str,
    expected_fingerprint: Option<&str>,
    conflicts: &mut Vec<HostConflict>,
) -> PlannedChange {
    let parsed = match parse_codex_managed_identity(item) {
        Ok(parsed) => parsed,
        Err(_) => {
            conflicts.push(HostConflict::new(
                HostConflictKind::UnmanagedNameCollision,
                format!(
                    "Codex MCP server name is already configured by an unmanaged entry: {server_name}"
                ),
            ));
            return PlannedChange::Noop;
        }
    };
    let entry = parsed.managed_entry;
    if entry.host_scope() != scope {
        conflicts.push(HostConflict::new(
            HostConflictKind::FingerprintMismatch,
            format!(
                "Codex MCP server name is already configured with the wrong managed scope: {server_name}"
            ),
        ));
        return PlannedChange::Noop;
    }
    let current = entry.managed_fingerprint(server_name);
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

fn parse_codex_managed_identity(
    item: &Item,
) -> Result<ParsedCodexManagedIdentity, CodexManagedIdentityProblem> {
    let table = item
        .as_table()
        .ok_or(CodexManagedIdentityProblem::Malformed)?;
    let allowed_keys = ["command", "args", "env", "env_vars", "tools"];
    if table.iter().any(|(key, _)| !allowed_keys.contains(&key)) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
    if !codex_tool_approval_overlay_is_valid(table) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
    let command = table
        .get("command")
        .and_then(Item::as_str)
        .ok_or(CodexManagedIdentityProblem::Malformed)?
        .to_owned();
    let args = match table.get("args") {
        None => Vec::new(),
        Some(item) => item
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or(CodexManagedIdentityProblem::Malformed)?,
    };
    let env = match table.get("env") {
        None => BTreeMap::new(),
        Some(item) => item
            .as_table()
            .and_then(|items| {
                items
                    .iter()
                    .map(|(key, item)| {
                        item.as_str()
                            .map(|value| (key.to_owned(), value.to_owned()))
                    })
                    .collect::<Option<BTreeMap<_, _>>>()
            })
            .ok_or(CodexManagedIdentityProblem::Malformed)?,
    };
    let env_vars = match table.get("env_vars") {
        None => Vec::new(),
        Some(item) => item
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or(CodexManagedIdentityProblem::Malformed)?,
    };
    let entry = ManagedMcpLaunchSpec::try_from_host_projection(command, args, env, env_vars)
        .map_err(|_| CodexManagedIdentityProblem::Unmanaged)?;
    Ok(ParsedCodexManagedIdentity {
        managed_entry: entry,
    })
}

pub(super) fn codex_managed_identity_fingerprint(
    scope: HostScope,
    server_name: &str,
    item: &Item,
) -> Option<String> {
    let parsed = parse_codex_managed_identity(item).ok()?;
    (parsed.managed_entry.host_scope() == scope)
        .then(|| parsed.managed_entry.managed_fingerprint(server_name))
}

pub(super) fn accepted_codex_tool_approval_overlay_item(item: &Item) -> Option<Item> {
    let table = item.as_table()?;
    let tools = table.get("tools")?;
    codex_tool_approval_overlay_is_valid(table).then(|| tools.clone())
}

fn codex_tool_approval_overlay_is_valid(table: &Table) -> bool {
    let Some(item) = table.get("tools") else {
        return true;
    };
    let Some(tools) = item.as_table() else {
        return false;
    };
    for (tool_name, item) in tools.iter() {
        if !is_known_volicord_tool(tool_name) {
            return false;
        }
        let Some(tool) = item.as_table() else {
            return false;
        };
        if tool.iter().any(|(key, _)| key != "approval_mode") {
            return false;
        }
        let Some(approval) = tool.get("approval_mode").and_then(Item::as_str) else {
            return false;
        };
        if approval.trim().is_empty() {
            return false;
        }
    }
    true
}

fn is_known_volicord_tool(tool_name: &str) -> bool {
    WORKFLOW_METHOD_TOOL_NAMES.contains(&tool_name)
        || READ_ONLY_METHOD_TOOL_NAMES.contains(&tool_name)
        || ADAPTER_UTILITY_TOOL_NAMES.contains(&tool_name)
}

pub(crate) fn managed_identity_evaluation_for_plan(
    plan: &HostPlan,
) -> Result<CodexManagedIdentityEvaluation, HostConfigError> {
    evaluate_codex_managed_identity(plan)
}

pub(super) fn evaluate_codex_managed_identity(
    plan: &HostPlan,
) -> Result<CodexManagedIdentityEvaluation, HostConfigError> {
    let HostTarget::File(target) = &plan.target else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Unknown,
            details: "Codex managed configuration target is not a file".to_owned(),
        });
    };
    let (_, text) = match read_text_snapshot(target) {
        Ok(snapshot) => snapshot,
        Err(HostConfigError::Malformed(details)) => {
            return Ok(CodexManagedIdentityEvaluation {
                status: ManagedConfigStatus::Malformed,
                details,
            });
        }
        Err(error) => {
            return Ok(CodexManagedIdentityEvaluation {
                status: ManagedConfigStatus::Unavailable,
                details: error.to_string(),
            });
        }
    };
    let Some(text) = text else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Missing,
            details: "Codex configuration target does not exist".to_owned(),
        });
    };
    let document = match parse_document(Some(&text), target) {
        Ok(document) => document,
        Err(error) => {
            return match error {
                HostConfigError::Malformed(details) => Ok(CodexManagedIdentityEvaluation {
                    status: ManagedConfigStatus::Malformed,
                    details,
                }),
                other => Err(other),
            };
        }
    };
    let Some(servers) = document.get("mcp_servers") else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Missing,
            details: "Codex configuration has no mcp_servers table".to_owned(),
        });
    };
    let Some(servers) = servers.as_table() else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Malformed,
            details: "Codex mcp_servers configuration is not a table".to_owned(),
        });
    };
    let Some(item) = servers.get(&plan.server_name) else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Missing,
            details: format!("Codex mcp_servers table has no {} entry", plan.server_name),
        });
    };
    match parse_codex_managed_identity(item) {
        Ok(parsed) => {
            if parsed.managed_entry.host_scope() != plan.host_scope {
                return Ok(CodexManagedIdentityEvaluation {
                    status: ManagedConfigStatus::Changed,
                    details: "Codex managed MCP server entry has the wrong managed scope"
                        .to_owned(),
                });
            }
            let fingerprint = parsed.managed_entry.managed_fingerprint(&plan.server_name);
            Ok(CodexManagedIdentityEvaluation {
                status: if fingerprint == plan.fingerprint {
                    ManagedConfigStatus::Match
                } else {
                    ManagedConfigStatus::Changed
                },
                details: if fingerprint == plan.fingerprint {
                    "Codex managed MCP server entry matches the canonical configuration".to_owned()
                } else {
                    "Codex managed MCP server entry differs from the canonical configuration"
                        .to_owned()
                },
            })
        }
        Err(CodexManagedIdentityProblem::Unmanaged) => Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Unmanaged,
            details: "Codex MCP server name is owned by a non-Volicord entry".to_owned(),
        }),
        Err(CodexManagedIdentityProblem::Malformed) => Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Malformed,
            details: "Codex managed MCP server entry is malformed".to_owned(),
        }),
    }
}

fn managed_launch_error(error: impl ToString) -> HostConfigError {
    HostConfigError::Conflict(HostConflict::new(
        HostConflictKind::InvalidCommand,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::DocumentMut;

    fn entry(text: &str) -> Item {
        let document = text
            .parse::<DocumentMut>()
            .expect("test Codex configuration");
        document["mcp_servers"]["volicord"].clone()
    }

    fn personal_text(extra: &str) -> String {
        format!(
            "[mcp_servers.volicord]\ncommand = \"/opt/volicord/bin/volicord\"\nargs = [\"mcp\", \"--stdio\", \"--connection\", \"connection_alpha\"]\n{extra}\n[mcp_servers.volicord.env]\nVOLICORD_HOME = \"/srv/volicord/runtime\"\nVOLICORD_MCP_CONNECTION_ID = \"connection_alpha\"\nVOLICORD_MCP_HOST = \"codex\"\nVOLICORD_MCP_LAUNCH = \"managed_host\"\n"
        )
    }

    #[test]
    fn strict_codex_parsing_reconstructs_current_personal_and_shared_launches() {
        let personal = parse_codex_managed_identity(&entry(&personal_text("")))
            .expect("personal managed identity")
            .managed_entry;
        assert_eq!(personal.host_scope(), HostScope::User);
        assert_eq!(
            personal.binding().runtime_home().map(|home| home.as_str()),
            Some("/srv/volicord/runtime")
        );

        let shared = entry(
            "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]\nenv_vars = [\"VOLICORD_HOME\"]\n",
        );
        let shared = parse_codex_managed_identity(&shared)
            .expect("shared managed identity")
            .managed_entry;
        assert_eq!(shared.host_scope(), HostScope::Project);
        assert!(shared.environment().static_values().is_empty());
    }

    #[test]
    fn formatting_and_valid_tool_approval_do_not_change_launch_identity() {
        let compact = entry(&personal_text(""));
        let formatted = entry(
            "[mcp_servers.volicord]\nargs=[\"mcp\",\"--stdio\",\"--connection\",\"connection_alpha\"]\ncommand='/opt/volicord/bin/volicord'\n\n[mcp_servers.volicord.env]\nVOLICORD_MCP_LAUNCH='managed_host'\nVOLICORD_MCP_HOST='codex'\nVOLICORD_MCP_CONNECTION_ID='connection_alpha'\nVOLICORD_HOME='/srv/volicord/runtime'\n\n[mcp_servers.volicord.tools.\"volicord.status\"]\napproval_mode='auto'\n",
        );
        assert_eq!(
            codex_managed_identity_fingerprint(HostScope::User, "volicord", &compact),
            codex_managed_identity_fingerprint(HostScope::User, "volicord", &formatted)
        );
        assert!(accepted_codex_tool_approval_overlay_item(&formatted).is_some());
    }

    #[test]
    fn unknown_and_malformed_shapes_are_noncanonical() {
        let unknown = entry(&personal_text("timeout_sec = 5"));
        assert_eq!(
            parse_codex_managed_identity(&unknown),
            Err(CodexManagedIdentityProblem::Unmanaged)
        );

        let malformed =
            entry("[mcp_servers.volicord]\ncommand = 5\nargs = [\"mcp\", \"--stdio\"]\n");
        assert_eq!(
            parse_codex_managed_identity(&malformed),
            Err(CodexManagedIdentityProblem::Malformed)
        );

        let unknown_tool = entry(&format!(
            "{}\n[mcp_servers.volicord.tools.\"volicord.unknown\"]\napproval_mode = \"auto\"\n",
            personal_text("")
        ));
        assert_eq!(
            parse_codex_managed_identity(&unknown_tool),
            Err(CodexManagedIdentityProblem::Unmanaged)
        );
    }

    #[test]
    fn semantic_drift_uses_the_current_fingerprint_repair_path() {
        let current = entry(&personal_text(""));
        let current_fingerprint =
            codex_managed_identity_fingerprint(HostScope::User, "volicord", &current)
                .expect("current fingerprint");
        let desired = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_beta",
            None,
        )
        .expect("desired launch");
        let desired_fingerprint = desired.managed_fingerprint("volicord");
        let mut conflicts = Vec::new();

        assert_eq!(
            classify_existing_codex_entry(
                HostScope::User,
                "volicord",
                &current,
                &desired_fingerprint,
                Some(&current_fingerprint),
                &mut conflicts,
            ),
            PlannedChange::Update
        );
        assert!(conflicts.is_empty());
    }
}
