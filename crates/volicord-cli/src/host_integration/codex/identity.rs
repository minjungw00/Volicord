use std::{collections::BTreeMap, path::Path};

use toml_edit::{Item, Table};
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES, WORKFLOW_METHOD_TOOL_NAMES,
};

use crate::host_integration::verification::{
    HostConfigurationStatus, HostPolicyOverlayDiagnostic, HostPolicyOverlayEntryDiagnostic,
    ManagedConfigStatus, Verification,
};
use crate::host_integration::{
    config_edit::read_text_snapshot, managed_fingerprint, HostConfigError, HostConflict,
    HostConflictKind, HostKind, HostPlan, HostScope, HostTarget, ManagedServerEntry, PlannedChange,
};

use super::{
    config::parse_document, CODEX_HOST_VALUE, CODEX_TOOL_APPROVAL_OVERLAY_KIND,
    MANAGED_HOST_LAUNCH_VALUE, VOLICORD_MCP_CONNECTION_ID, VOLICORD_MCP_HOST, VOLICORD_MCP_LAUNCH,
    VOLICORD_MCP_PROJECT_ID,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCodexManagedIdentity {
    managed_entry: ManagedServerEntry,
    host_policy_overlay: Option<HostPolicyOverlayDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexManagedIdentityProblem {
    Unmanaged,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexManagedIdentityEvaluation {
    pub(crate) status: ManagedConfigStatus,
    pub(crate) host_policy_overlay: Option<HostPolicyOverlayDiagnostic>,
}

pub(super) fn codex_managed_server_entry(
    connection_id: impl Into<String>,
    project_id: Option<&str>,
    mcp_command: &Path,
    runtime_home: Option<&Path>,
) -> ManagedServerEntry {
    let connection_id = connection_id.into();
    let mut entry = ManagedServerEntry::new_project_bound(
        connection_id.clone(),
        project_id,
        mcp_command,
        runtime_home,
    );
    entry.env.insert(
        VOLICORD_MCP_LAUNCH.to_owned(),
        MANAGED_HOST_LAUNCH_VALUE.to_owned(),
    );
    entry
        .env
        .insert(VOLICORD_MCP_HOST.to_owned(), CODEX_HOST_VALUE.to_owned());
    entry
        .env
        .insert(VOLICORD_MCP_CONNECTION_ID.to_owned(), connection_id);
    if let Some(project_id) = project_id {
        entry
            .env
            .insert(VOLICORD_MCP_PROJECT_ID.to_owned(), project_id.to_owned());
    }
    entry
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
    let current = managed_fingerprint(HostKind::Codex, scope, server_name, &entry);
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
    let allowed_keys = ["command", "args", "env", "tools"];
    if table.iter().any(|(key, _)| !allowed_keys.contains(&key)) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
    let host_policy_overlay =
        codex_tool_approval_overlay(table).ok_or(CodexManagedIdentityProblem::Unmanaged)?;
    let command = table
        .get("command")
        .and_then(Item::as_str)
        .ok_or(CodexManagedIdentityProblem::Malformed)?
        .to_owned();
    let args = table
        .get("args")
        .and_then(Item::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_else(|| Some(Vec::new()))
        .ok_or(CodexManagedIdentityProblem::Malformed)?;
    let env = table
        .get("env")
        .and_then(Item::as_table)
        .map(|items| {
            items
                .iter()
                .map(|(key, item)| {
                    item.as_str()
                        .map(|value| (key.to_owned(), value.to_owned()))
                })
                .collect::<Option<BTreeMap<_, _>>>()
        })
        .unwrap_or_else(|| Some(BTreeMap::new()))
        .ok_or(CodexManagedIdentityProblem::Malformed)?;
    let entry = ManagedServerEntry { command, args, env };
    if !has_codex_managed_identity_markers(&entry) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
    Ok(ParsedCodexManagedIdentity {
        managed_entry: entry,
        host_policy_overlay,
    })
}

pub(super) fn codex_managed_identity_fingerprint(
    scope: HostScope,
    server_name: &str,
    item: &Item,
) -> Option<String> {
    let parsed = parse_codex_managed_identity(item).ok()?;
    Some(managed_fingerprint(
        HostKind::Codex,
        scope,
        server_name,
        &parsed.managed_entry,
    ))
}

fn has_codex_managed_identity_markers(entry: &ManagedServerEntry) -> bool {
    entry.env.contains_key(VOLICORD_MCP_LAUNCH)
        && entry.env.contains_key(VOLICORD_MCP_HOST)
        && entry.env.contains_key(VOLICORD_MCP_CONNECTION_ID)
        && (!entry.args.iter().any(|arg| arg == "--project")
            || entry.env.contains_key(VOLICORD_MCP_PROJECT_ID))
}

pub(super) fn accepted_codex_tool_approval_overlay_item(item: &Item) -> Option<Item> {
    let table = item.as_table()?;
    codex_tool_approval_overlay(table).flatten()?;
    table.get("tools").cloned()
}

fn codex_tool_approval_overlay(table: &Table) -> Option<Option<HostPolicyOverlayDiagnostic>> {
    let Some(item) = table.get("tools") else {
        return Some(None);
    };
    let tools = item.as_table()?;
    let mut approvals = BTreeMap::new();
    for (tool_name, item) in tools.iter() {
        if !is_known_volicord_tool(tool_name) {
            return None;
        }
        let tool = item.as_table()?;
        if tool.iter().any(|(key, _)| key != "approval_mode") {
            return None;
        }
        let approval = tool.get("approval_mode").and_then(Item::as_str)?;
        if approval.trim().is_empty() {
            return None;
        }
        approvals.insert(tool_name.to_owned(), approval.to_owned());
    }
    let entries = approvals
        .into_iter()
        .map(|(tool, approval_mode)| HostPolicyOverlayEntryDiagnostic {
            tool,
            approval_mode,
        })
        .collect::<Vec<_>>();
    let tools = entries
        .iter()
        .map(|entry| entry.tool.clone())
        .collect::<Vec<_>>();
    let tool_count = entries.len();
    Some(Some(HostPolicyOverlayDiagnostic {
        present: true,
        accepted: true,
        kind: CODEX_TOOL_APPROVAL_OVERLAY_KIND.to_owned(),
        tool_count,
        tools,
        entries,
        details: if tool_count == 0 {
            "Codex tool approval policy overlay is present and accepted".to_owned()
        } else {
            format!(
                "Codex tool approval policy overlay is present and accepted for {tool_count} Volicord tool(s)"
            )
        },
    }))
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
            host_policy_overlay: None,
        });
    };
    let (_, text) = read_text_snapshot(target)?;
    let Some(text) = text else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Missing,
            host_policy_overlay: None,
        });
    };
    let document = match parse_document(Some(&text), target) {
        Ok(document) => document,
        Err(error) => {
            return match error {
                HostConfigError::Malformed(_) => Ok(CodexManagedIdentityEvaluation {
                    status: ManagedConfigStatus::Malformed,
                    host_policy_overlay: None,
                }),
                other => Err(other),
            };
        }
    };
    let Some(item) = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .and_then(|servers| servers.get(&plan.server_name))
    else {
        return Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Missing,
            host_policy_overlay: None,
        });
    };
    match parse_codex_managed_identity(item) {
        Ok(parsed) => {
            let fingerprint = managed_fingerprint(
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
                host_policy_overlay: parsed.host_policy_overlay,
            })
        }
        Err(CodexManagedIdentityProblem::Unmanaged) => Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Unmanaged,
            host_policy_overlay: None,
        }),
        Err(CodexManagedIdentityProblem::Malformed) => Ok(CodexManagedIdentityEvaluation {
            status: ManagedConfigStatus::Malformed,
            host_policy_overlay: None,
        }),
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
