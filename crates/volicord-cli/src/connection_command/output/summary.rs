use super::*;

use crate::summary_card::DIAGNOSTIC_SUMMARY_GUARANTEE;
use volicord_types::SummaryCard;

pub(super) fn connection_diagnostic_summary_card(
    action: &str,
    guard_state: &GuardOperationalState,
    host_display: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) -> Option<SummaryCard> {
    if !matches!(action, "status" | "verified") {
        return None;
    }
    Some(SummaryCard {
        task: "not_selected".to_owned(),
        recording: "diagnostic_observation".to_owned(),
        profile: guard_state.selected_profile().to_owned(),
        write_ticket: "not_selected".to_owned(),
        evidence: "not_selected".to_owned(),
        user_action: "not_selected".to_owned(),
        changes: "not_selected".to_owned(),
        close_status: "not_selected".to_owned(),
        transport: "Agent Connection".to_owned(),
        next: connection_summary_next_text(primary_next_action, host_display),
        next_action: None,
        guarantee: DIAGNOSTIC_SUMMARY_GUARANTEE.to_owned(),
    })
}

fn connection_summary_next_text(
    primary_next_action: Option<&PrimaryNextAction>,
    host_display: &str,
) -> String {
    let Some(action) = primary_next_action else {
        return "none".to_owned();
    };
    match action.id.as_str() {
        "managed_host_startup_not_observed" => format!(
            "Restart, reload, resume, or start a new {host_display} session in this repository, then confirm active Volicord tool exposure."
        ),
        "managed_host_tools_list_not_observed" => format!(
            "Check {host_display} MCP startup/tool-list logs; managed startup was observed but managed tools/list was not."
        ),
        "active_tool_exposure_unconfirmed" => format!(
            "Confirm active {host_display} tool exposure or invoke a read-only Volicord tool from the active {host_display} session."
        ),
        "managed_host_storage_degraded" => format!(
            "Repair managed {host_display} host storage read/write capability or switch to a compatible read-only mode."
        ),
        "host_trust_required" => format!(
            "The project must be trusted before project-scoped {host_display} configuration loads; then rerun verification."
        ),
        "project_approval_required" => format!(
            "The project must be approved before project-scoped {host_display} configuration loads; then rerun verification."
        ),
        "reload_required" => format!(
            "Restart or reload {host_display} so it loads Volicord configuration, then rerun verification."
        ),
        "mcp_config_missing" => {
            "Reinstall missing MCP configuration, then rerun verification.".to_owned()
        }
        "mcp_config_changed" => {
            "Review the changed MCP configuration and repair it if Volicord should manage it, then rerun verification.".to_owned()
        }
        "mcp_config_malformed" => {
            "Repair the malformed MCP configuration, then rerun verification.".to_owned()
        }
        "guard_files_missing" => {
            "Reinstall missing Codex Record Guard files, then rerun verification.".to_owned()
        }
        "guard_files_stale" => {
            "Refresh stale Codex Record Guard files, then rerun verification.".to_owned()
        }
        "guard_files_broken" => {
            "Repair broken Codex Record Guard files, then rerun verification.".to_owned()
        }
        "guard_capability_degraded" => {
            "Repair the required Codex Record Guard hook configuration, then rerun verification.".to_owned()
        }
        _ => action.instruction.clone(),
    }
}
