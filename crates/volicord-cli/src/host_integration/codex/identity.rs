use std::{collections::BTreeMap, path::Path};

use toml_edit::{Item, Table};
use volicord_mcp::RepositoryDiscoveryHost;
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES, WORKFLOW_METHOD_TOOL_NAMES,
};

use crate::host_integration::verification::ManagedConfigStatus;
use crate::host_integration::{
    config_edit::read_text_snapshot, managed_configuration_digest, HostConfigError, HostConflict,
    HostConflictKind, HostKind, HostPlan, HostScope, HostTarget, ManagedServerEntry, PlannedChange,
};

use super::config::parse_document;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCodexManagedIdentity {
    managed_entry: ManagedServerEntry,
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

pub(super) fn codex_managed_server_entry(
    scope: HostScope,
    connection_id: impl Into<String>,
    project_id: Option<&str>,
    mcp_command: &Path,
    _runtime_home: Option<&Path>,
) -> ManagedServerEntry {
    if scope == HostScope::Project {
        return ManagedServerEntry::new_repository_discovery(RepositoryDiscoveryHost::Codex);
    }
    ManagedServerEntry::new_project_bound(connection_id, project_id, mcp_command)
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
    let current = managed_configuration_digest(HostKind::Codex, scope, server_name, &entry);
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
    let entry = ManagedServerEntry {
        command,
        args,
        env,
        env_vars,
    };
    if !has_codex_managed_identity_markers(&entry) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
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
    Some(managed_configuration_digest(
        HostKind::Codex,
        scope,
        server_name,
        &parsed.managed_entry,
    ))
}

fn has_codex_managed_identity_markers(entry: &ManagedServerEntry) -> bool {
    crate::host_integration::is_volicord_managed_entry(entry)
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
            let fingerprint = managed_configuration_digest(
                HostKind::Codex,
                plan.host_scope,
                &plan.server_name,
                &parsed.managed_entry,
            );
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
