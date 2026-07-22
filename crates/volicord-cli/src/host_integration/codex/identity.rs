use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use toml_edit::{Item, Table};
use volicord_mcp::ManagedMcpLaunchSpec;
use volicord_types::AgentToolId;

use crate::host_integration::verification::{ManagedConfigDiagnostic, ManagedConfigStatus};
use crate::host_integration::{
    config_edit::read_text_snapshot, HostConfigError, HostConflict, HostConflictKind, HostKind,
    HostPlan, HostScope, HostTarget, PlannedChange,
};

use super::config::parse_document;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCodexManagedIdentity {
    managed_entry: ManagedMcpLaunchSpec,
    enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexManagedIdentityProblem {
    Unmanaged,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexManagedIdentityEvaluation {
    pub(crate) status: ManagedConfigStatus,
    pub(crate) diagnostic: Option<ManagedConfigDiagnostic>,
    pub(crate) details: String,
}

pub(super) fn codex_managed_launch_spec(
    scope: HostScope,
    connection_id: impl Into<String>,
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
    ManagedMcpLaunchSpec::personal(mcp_command, runtime_home, connection_id)
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
    if !parsed.enabled {
        conflicts.push(HostConflict::new(
            HostConflictKind::FingerprintMismatch,
            format!("Codex MCP server entry is disabled: {server_name}"),
        ));
        return PlannedChange::Noop;
    }
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
    let allowed_keys = ["command", "args", "env", "env_vars", "tools", "enabled"];
    if table.iter().any(|(key, _)| !allowed_keys.contains(&key)) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
    if !codex_tool_approval_overlay_is_valid(table) {
        return Err(CodexManagedIdentityProblem::Unmanaged);
    }
    let enabled = match table.get("enabled") {
        None => true,
        Some(item) => item
            .as_bool()
            .ok_or(CodexManagedIdentityProblem::Malformed)?,
    };
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
        enabled,
    })
}

pub(super) fn codex_managed_identity_fingerprint(
    scope: HostScope,
    server_name: &str,
    item: &Item,
) -> Option<String> {
    let parsed = parse_codex_managed_identity(item).ok()?;
    (parsed.enabled && parsed.managed_entry.host_scope() == scope)
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
    AgentToolId::from_wire_name(tool_name).is_ok()
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
        return Ok(managed_config_failure(
            ManagedConfigStatus::Unknown,
            ManagedConfigDiagnostic::Unavailable,
            "Codex managed configuration target is not a file",
        ));
    };
    let (_, text) = match read_text_snapshot(target) {
        Ok(snapshot) => snapshot,
        Err(HostConfigError::Malformed(_)) => {
            return Ok(managed_config_failure(
                ManagedConfigStatus::Malformed,
                ManagedConfigDiagnostic::TomlParseFailure,
                "Codex managed configuration is not valid bounded UTF-8 text",
            ));
        }
        Err(error) => {
            return Ok(managed_config_failure(
                ManagedConfigStatus::Unavailable,
                ManagedConfigDiagnostic::Unavailable,
                error.to_string(),
            ));
        }
    };
    let Some(text) = text else {
        return Ok(managed_config_failure(
            ManagedConfigStatus::Missing,
            ManagedConfigDiagnostic::EntryMissing,
            "Codex configuration target does not exist",
        ));
    };
    let document = match parse_document(Some(&text), target) {
        Ok(document) => document,
        Err(error) => {
            return match error {
                HostConfigError::Malformed(_) => Ok(managed_config_failure(
                    ManagedConfigStatus::Malformed,
                    ManagedConfigDiagnostic::TomlParseFailure,
                    "Codex managed configuration is malformed TOML",
                )),
                other => Err(other),
            };
        }
    };
    let Some(servers) = document.get("mcp_servers") else {
        return Ok(managed_config_failure(
            ManagedConfigStatus::Missing,
            ManagedConfigDiagnostic::EntryMissing,
            "Codex configuration has no mcp_servers table",
        ));
    };
    let Some(servers) = servers.as_table() else {
        return Ok(managed_config_failure(
            ManagedConfigStatus::Malformed,
            ManagedConfigDiagnostic::TomlParseFailure,
            "Codex mcp_servers configuration is not a table",
        ));
    };
    let Some(item) = servers.get(&plan.server_name) else {
        return Ok(managed_config_failure(
            ManagedConfigStatus::Missing,
            ManagedConfigDiagnostic::EntryMissing,
            format!("Codex mcp_servers table has no {} entry", plan.server_name),
        ));
    };
    if let Some(diagnostic) = managed_entry_shape_diagnostic(item, &plan.entry) {
        let (status, details) = match diagnostic {
            ManagedConfigDiagnostic::EntryDisabled => (
                ManagedConfigStatus::Changed,
                "Codex managed MCP server entry is disabled",
            ),
            ManagedConfigDiagnostic::MalformedApprovalOverlay => (
                ManagedConfigStatus::Malformed,
                "Codex managed MCP tool approval overlay is malformed",
            ),
            ManagedConfigDiagnostic::FingerprintMismatch => (
                ManagedConfigStatus::Unmanaged,
                "Codex MCP server entry has an incompatible managed identity",
            ),
            ManagedConfigDiagnostic::CommandDrift
            | ManagedConfigDiagnostic::ArgumentDrift
            | ManagedConfigDiagnostic::StaticEnvironmentDrift
            | ManagedConfigDiagnostic::ForwardedEnvironmentDrift => (
                ManagedConfigStatus::Changed,
                "Codex managed MCP server entry differs from the canonical configuration",
            ),
            ManagedConfigDiagnostic::TomlParseFailure
            | ManagedConfigDiagnostic::EntryMissing
            | ManagedConfigDiagnostic::Unavailable => unreachable!("shape classifier result"),
        };
        return Ok(managed_config_failure(status, diagnostic, details));
    }
    match parse_codex_managed_identity(item) {
        Ok(parsed) => {
            if parsed.managed_entry.host_scope() != plan.host_scope {
                return Ok(managed_config_failure(
                    ManagedConfigStatus::Changed,
                    ManagedConfigDiagnostic::FingerprintMismatch,
                    "Codex managed MCP server entry has the wrong managed scope",
                ));
            }
            let fingerprint = parsed.managed_entry.managed_fingerprint(&plan.server_name);
            if fingerprint == plan.fingerprint {
                Ok(CodexManagedIdentityEvaluation {
                    status: ManagedConfigStatus::Match,
                    diagnostic: None,
                    details: "Codex managed MCP server entry matches the canonical configuration"
                        .to_owned(),
                })
            } else {
                Ok(managed_config_failure(
                    ManagedConfigStatus::Changed,
                    ManagedConfigDiagnostic::FingerprintMismatch,
                    "Codex managed MCP server entry fingerprint differs from the canonical configuration",
                ))
            }
        }
        Err(CodexManagedIdentityProblem::Unmanaged) => Ok(managed_config_failure(
            ManagedConfigStatus::Unmanaged,
            ManagedConfigDiagnostic::FingerprintMismatch,
            "Codex MCP server name is owned by a non-Volicord entry",
        )),
        Err(CodexManagedIdentityProblem::Malformed) => Ok(managed_config_failure(
            ManagedConfigStatus::Malformed,
            ManagedConfigDiagnostic::FingerprintMismatch,
            "Codex managed MCP server entry is malformed",
        )),
    }
}

fn managed_config_failure(
    status: ManagedConfigStatus,
    diagnostic: ManagedConfigDiagnostic,
    details: impl Into<String>,
) -> CodexManagedIdentityEvaluation {
    CodexManagedIdentityEvaluation {
        status,
        diagnostic: Some(diagnostic),
        details: details.into(),
    }
}

fn managed_entry_shape_diagnostic(
    item: &Item,
    expected: &ManagedMcpLaunchSpec,
) -> Option<ManagedConfigDiagnostic> {
    let Some(table) = item.as_table() else {
        return Some(ManagedConfigDiagnostic::FingerprintMismatch);
    };
    if table
        .iter()
        .any(|(key, _)| !["command", "args", "env", "env_vars", "tools", "enabled"].contains(&key))
    {
        return Some(ManagedConfigDiagnostic::FingerprintMismatch);
    }
    match table.get("enabled") {
        Some(item) if item.as_bool() == Some(false) => {
            return Some(ManagedConfigDiagnostic::EntryDisabled);
        }
        Some(item) if item.as_bool().is_none() => {
            return Some(ManagedConfigDiagnostic::FingerprintMismatch);
        }
        _ => {}
    }
    if !codex_tool_approval_overlay_is_valid(table) {
        return Some(ManagedConfigDiagnostic::MalformedApprovalOverlay);
    }
    if table.get("command").and_then(Item::as_str) != Some(expected.command()) {
        return Some(ManagedConfigDiagnostic::CommandDrift);
    }
    let args = table
        .get("args")
        .and_then(Item::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        });
    if args.as_deref() != Some(expected.args()) {
        return Some(ManagedConfigDiagnostic::ArgumentDrift);
    }
    let static_environment = match table.get("env") {
        None => Some(BTreeMap::new()),
        Some(item) => item.as_table().and_then(|items| {
            items
                .iter()
                .map(|(key, item)| {
                    item.as_str()
                        .map(|value| (key.to_owned(), value.to_owned()))
                })
                .collect::<Option<BTreeMap<_, _>>>()
        }),
    };
    if static_environment.as_ref() != Some(expected.environment().static_values()) {
        return Some(ManagedConfigDiagnostic::StaticEnvironmentDrift);
    }
    let forwarded_environment = match table.get("env_vars") {
        None => Some(BTreeSet::new()),
        Some(item) => item.as_array().and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<BTreeSet<_>>>()
        }),
    };
    if forwarded_environment.as_ref() != Some(expected.environment().forwarded_names()) {
        return Some(ManagedConfigDiagnostic::ForwardedEnvironmentDrift);
    }
    None
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

    fn personal_launch() -> ManagedMcpLaunchSpec {
        ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
        )
        .expect("personal launch")
    }

    #[test]
    fn managed_config_drift_and_disabled_entry_are_classified_structurally() {
        let expected = personal_launch();
        let disabled = entry(&personal_text("enabled = false"));
        assert_eq!(
            managed_entry_shape_diagnostic(&disabled, &expected),
            Some(ManagedConfigDiagnostic::EntryDisabled)
        );

        let command_drift = entry(
            &personal_text("").replace("/opt/volicord/bin/volicord", "/opt/other/bin/volicord"),
        );
        assert_eq!(
            managed_entry_shape_diagnostic(&command_drift, &expected),
            Some(ManagedConfigDiagnostic::CommandDrift)
        );

        let argument_drift =
            entry(&personal_text("").replace("connection_alpha\"]", "connection_beta\"]"));
        assert_eq!(
            managed_entry_shape_diagnostic(&argument_drift, &expected),
            Some(ManagedConfigDiagnostic::ArgumentDrift)
        );

        let static_environment_drift = entry(&personal_text("").replace(
            "VOLICORD_HOME = \"/srv/volicord/runtime\"",
            "VOLICORD_HOME = \"/srv/volicord/other\"",
        ));
        assert_eq!(
            managed_entry_shape_diagnostic(&static_environment_drift, &expected),
            Some(ManagedConfigDiagnostic::StaticEnvironmentDrift)
        );

        let malformed_overlay = entry(&format!(
            "{}\n[mcp_servers.volicord.tools.\"volicord.status\"]\napproval_mode = []\n",
            personal_text("")
        ));
        assert_eq!(
            managed_entry_shape_diagnostic(&malformed_overlay, &expected),
            Some(ManagedConfigDiagnostic::MalformedApprovalOverlay)
        );
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
    fn personal_project_argument_and_environment_marker_are_noncanonical() {
        let project_argument = entry(&personal_text("").replace(
            "\"connection_alpha\"]",
            "\"connection_alpha\", \"--project\", \"project_alpha\"]",
        ));
        assert_eq!(
            parse_codex_managed_identity(&project_argument),
            Err(CodexManagedIdentityProblem::Unmanaged)
        );

        let project_marker = entry(&personal_text("").replace(
            "VOLICORD_HOME = \"/srv/volicord/runtime\"",
            "VOLICORD_HOME = \"/srv/volicord/runtime\"\nVOLICORD_MCP_PROJECT_ID = \"project_alpha\"",
        ));
        assert_eq!(
            parse_codex_managed_identity(&project_marker),
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
