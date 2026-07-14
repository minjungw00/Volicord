use std::collections::BTreeMap;

use crate::host_integration::{
    managed_fingerprint,
    verification::{
        HostConfigurationStatus, HostExecutableStatus, HostGateStatus, ManagedConfigStatus,
        Verification,
    },
    HostKind, HostPlan, HostScope, ManagedServerEntry, UserAction, UserActionKind,
};

use super::{cli::CommandOutput, config::is_claude_managed_identity_candidate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClaudeMcpState {
    Connected,
    PendingApproval,
    Rejected,
    Missing,
    CommandFailed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClaudeMcpInspection {
    pub(super) state: ClaudeMcpState,
    pub(super) scope: Option<HostScope>,
    pub(super) command: Option<String>,
    pub(super) args: Option<Vec<String>>,
    pub(super) env: BTreeMap<String, String>,
    pub(super) diagnostic: Option<String>,
}

pub(super) fn inspection_is_volicord_managed(inspection: &ClaudeMcpInspection) -> bool {
    let Some(command) = &inspection.command else {
        return false;
    };
    let Some(args) = &inspection.args else {
        return false;
    };
    is_claude_managed_identity_candidate(&ManagedServerEntry {
        command: command.clone(),
        args: args.clone(),
        env: inspection.env.clone(),
        env_vars: Vec::new(),
    })
}

pub(super) fn verification_from_claude_output(
    plan: &HostPlan,
    output: &CommandOutput,
) -> Verification {
    let inspection = parse_claude_mcp_get_output(output);
    // Claude Code exposes configuration and approval state through `claude mcp get`.
    // Active tool exposure, managed lifecycle, and storage capability stay unknown
    // here because they require separate managed-host runtime evidence.
    match inspection.state {
        ClaudeMcpState::Connected => {
            let Some(current) =
                fingerprint_from_claude_inspection(plan.host_scope, &plan.server_name, &inspection)
            else {
                return Verification::unknown(format!(
                    "Claude Code command `claude mcp get {}` returned connected output, but command, args, env, or scope could not be parsed reliably",
                    plan.server_name
                ))
                .with_managed_config(ManagedConfigStatus::Match)
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_configuration(HostConfigurationStatus::Discovered)
                .with_diagnostic(inspection.diagnostic.unwrap_or_default());
            };
            if current == plan.fingerprint {
                Verification::configured_ready(
                    "Claude Code reports the managed MCP server is connected and matches Volicord configuration",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_gate(HostGateStatus::Ready)
                .with_mcp_handshake_allowed(true)
            } else {
                Verification::changed(
                    "Claude Code reports an MCP server with that name, but command, args, env, or scope differ from Volicord-managed configuration",
                )
                .with_host_executable(HostExecutableStatus::Available)
                .with_host_configuration(HostConfigurationStatus::Changed)
            }
        }
        ClaudeMcpState::PendingApproval => Verification::action_required(
            "Claude Code reports the MCP server is pending project approval",
        )
        .with_host_executable(HostExecutableStatus::Available)
        .with_host_gate(HostGateStatus::ActionRequired)
        .with_mcp_handshake_allowed(true)
        .with_user_actions(vec![UserAction::new(
            UserActionKind::ProjectApprovalRequired,
            "Claude Code requires user approval before the MCP server is available",
        )]),
        ClaudeMcpState::Rejected => {
            Verification::rejected("Claude Code reports the MCP server was rejected")
        }
        ClaudeMcpState::Missing => Verification::missing(
            "Claude Code did not report a configured MCP server with that name",
        )
        .with_host_executable(HostExecutableStatus::Available),
        ClaudeMcpState::CommandFailed => Verification::failed(format!(
            "Claude Code command `claude mcp get {}` failed with status {}; host output was not echoed",
            plan.server_name,
            output
                .status_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        ))
        .with_host_executable(HostExecutableStatus::Available),
        ClaudeMcpState::Unknown => Verification::unknown(format!(
            "Claude Code command `claude mcp get {}` returned unsupported output; cannot interpret host state",
            plan.server_name
        ))
        .with_host_executable(HostExecutableStatus::Available)
        .with_diagnostic(inspection.diagnostic.unwrap_or_default()),
    }
}

pub(super) fn fingerprint_from_claude_inspection(
    scope: HostScope,
    server_name: &str,
    inspection: &ClaudeMcpInspection,
) -> Option<String> {
    if inspection.scope.is_some_and(|actual| actual != scope) {
        return Some(managed_fingerprint(
            HostKind::ClaudeCode,
            inspection.scope.unwrap(),
            server_name,
            &ManagedServerEntry {
                command: inspection.command.clone()?,
                args: inspection.args.clone()?,
                env: inspection.env.clone(),
                env_vars: Vec::new(),
            },
        ));
    }
    Some(managed_fingerprint(
        HostKind::ClaudeCode,
        scope,
        server_name,
        &ManagedServerEntry {
            command: inspection.command.clone()?,
            args: inspection.args.clone()?,
            env: inspection.env.clone(),
            env_vars: Vec::new(),
        },
    ))
}

pub(super) fn parse_claude_mcp_get_output(output: &CommandOutput) -> ClaudeMcpInspection {
    let combined = format!("{}\n{}", output.stdout, output.stderr);
    let mut state = None;
    let mut scope = None;
    let mut command = None;
    let mut args = None;
    let mut env = BTreeMap::new();
    let mut in_env = false;

    for line in combined.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_pending_marker(trimmed) {
            state = Some(ClaudeMcpState::PendingApproval);
        } else if is_rejected_marker(trimmed) {
            state = Some(ClaudeMcpState::Rejected);
        } else if is_missing_marker(trimmed) {
            state = Some(ClaudeMcpState::Missing);
        } else if is_connected_marker(trimmed) && state.is_none() {
            state = Some(ClaudeMcpState::Connected);
        }

        if let Some(value) = field_value(trimmed, "scope") {
            scope = parse_scope(value);
            in_env = false;
        } else if let Some(value) = field_value(trimmed, "command") {
            command = Some(value.to_owned());
            in_env = false;
        } else if let Some(value) = field_value(trimmed, "args") {
            args = parse_args(value);
            in_env = false;
        } else if let Some(value) = field_value(trimmed, "environment") {
            in_env = true;
            parse_env_assignment(value, &mut env);
        } else if let Some(value) = field_value(trimmed, "env") {
            in_env = true;
            parse_env_assignment(value, &mut env);
        } else if in_env {
            parse_env_assignment(trimmed, &mut env);
        }
    }

    let state = state.unwrap_or({
        if output.success {
            ClaudeMcpState::Unknown
        } else {
            ClaudeMcpState::CommandFailed
        }
    });
    ClaudeMcpInspection {
        state,
        scope,
        command,
        args,
        env,
        diagnostic: Some(host_output_summary(output)),
    }
}

fn host_output_summary(output: &CommandOutput) -> String {
    format!(
        "claude mcp get output summary: stdout_lines={}, stderr_lines={}, stderr_present={}",
        output.stdout.lines().count(),
        output.stderr.lines().count(),
        !output.stderr.trim().is_empty()
    )
}

fn field_value<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    let (actual, value) = line.split_once(':')?;
    if actual.trim().eq_ignore_ascii_case(label) {
        Some(value.trim())
    } else {
        None
    }
}

fn parse_scope(value: &str) -> Option<HostScope> {
    match value.trim().to_ascii_lowercase().as_str() {
        "local" => Some(HostScope::Local),
        "project" => Some(HostScope::Project),
        "user" => Some(HostScope::User),
        _ => None,
    }
}

fn parse_args(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Some(Vec::new());
    }
    if value.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(value).ok();
    }
    if value.contains('"') || value.contains('\'') {
        return None;
    }
    Some(value.split_whitespace().map(str::to_owned).collect())
}

fn parse_env_assignment(value: &str, env: &mut BTreeMap<String, String>) {
    let value = value.trim().trim_start_matches('-').trim();
    let Some((key, value)) = value.split_once('=') else {
        return;
    };
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return;
    }
    env.insert(key.to_owned(), value.trim().to_owned());
}

fn is_pending_marker(line: &str) -> bool {
    line == "⏸ Pending approval"
        || line == "Pending approval"
        || line == "Status: ⏸ Pending approval"
        || line.eq_ignore_ascii_case("Status: Pending approval")
}

fn is_rejected_marker(line: &str) -> bool {
    line == "✗ Rejected"
        || line == "Rejected"
        || line == "Status: ✗ Rejected"
        || line.eq_ignore_ascii_case("Status: Rejected")
}

fn is_missing_marker(line: &str) -> bool {
    line == "Server not found"
        || line == "No MCP server found"
        || line == "MCP server not found"
        || line.eq_ignore_ascii_case("Error: Server not found")
}

fn is_connected_marker(line: &str) -> bool {
    line == "✓ Connected"
        || line == "Connected"
        || line == "Status: ✓ Connected"
        || line.eq_ignore_ascii_case("Status: Connected")
}
