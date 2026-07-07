use serde::Serialize;
use serde_json::{json, Value};
use volicord_store::bootstrap::{InstallationProfileRecord, RuntimeHomeRecord};

use crate::setup_report::{SetupAction, SetupActionKind, SetupReport, SetupStatus};

use super::{path_text, workflow::OutputFormat, SetupCommandError};

const SETUP_NON_GUARANTEE_TEXT: &str =
    "future shell PATH state, already-running host reload state, host approval, or model behavior";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct DiagnosticCheck {
    pub(super) id: String,
    pub(super) status: String,
    pub(super) summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) details: Option<Value>,
}

impl DiagnosticCheck {
    pub(super) fn passed(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "passed".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    pub(super) fn warning(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "warning".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    pub(super) fn failed(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "failed".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    pub(super) fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

pub(super) fn render_setup_output(
    output: OutputFormat,
    report: &SetupReport,
    runtime_home: &RuntimeHomeRecord,
    profile: Option<&InstallationProfileRecord>,
    checks: &[DiagnosticCheck],
) -> Result<String, SetupCommandError> {
    match output {
        OutputFormat::Json => serde_json::to_string_pretty(&json!({
            "status": report.status.as_str(),
            "status_meaning": setup_status_meaning(report.status),
            "runtime_home": path_text(&runtime_home.runtime_home),
            "registry_db": path_text(&runtime_home.registry_db_path),
            "installation_profile": profile.map(profile_json),
            "states": setup_states_json(report),
            "setup_report": report,
            "commands": &report.commands,
            "checks": checks,
            "actions": &report.actions_required,
            "actions_required": &report.actions_required,
            "actions_optional": &report.actions_optional,
            "actions_performed": &report.actions_performed,
            "primary_next_action": primary_setup_action(report),
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| SetupCommandError::Runtime(error.to_string())),
        OutputFormat::Text => Ok(render_compact_setup_text(
            report,
            runtime_home,
            profile,
            checks,
        )),
    }
}

fn render_compact_setup_text(
    report: &SetupReport,
    runtime_home: &RuntimeHomeRecord,
    profile: Option<&InstallationProfileRecord>,
    checks: &[DiagnosticCheck],
) -> String {
    let mut text = format!("Volicord setup {}\n\n", report.status.as_str());
    text.push_str("Summary:\n");
    text.push_str(&format!(
        "  Status: {}\n  Meaning: {}\n  Runtime Home: {}\n  Installation profile: {}\n  Commands: {}\n  Host reload required: {}\n",
        setup_result_text(report.status),
        setup_status_meaning(report.status),
        display_state_text(report.runtime_home.status.as_str()),
        display_state_text(report.installation_profile.status.as_str()),
        display_state_text(setup_command_state(report)),
        yes_no(setup_host_reload_required(report)),
    ));
    text.push_str(&format!(
        "\nRuntime Home:\n  {}\n  Registry: {}\n",
        runtime_home.runtime_home.display(),
        runtime_home.registry_db_path.display()
    ));
    if let Some(profile) = profile {
        text.push_str(&format!(
            "\nSelected commands:\n  volicord: {}\n  MCP launch: {}\n  Default mode: {}\n",
            profile.volicord_command, profile.volicord_mcp_command, profile.default_connection_mode
        ));
    }
    append_setup_checks(&mut text, checks);
    append_setup_actions(&mut text, "Next", &report.actions_required);
    append_setup_actions(&mut text, "Optional", &report.actions_optional);
    text.push_str(&format!(
        "\nLimits:\n  This does not prove {}.\n\nDiagnostics:\n  Run:\n    volicord doctor --json\n",
        SETUP_NON_GUARANTEE_TEXT
    ));
    text
}

fn append_setup_checks(output: &mut String, checks: &[DiagnosticCheck]) {
    output.push_str("\nChecks:\n");
    let not_passed = checks
        .iter()
        .filter(|check| check.status != "passed")
        .collect::<Vec<_>>();
    if not_passed.is_empty() {
        output.push_str("  All available setup checks passed.\n");
        return;
    }
    for check in not_passed {
        output.push_str(&format!(
            "  {}: {}\n",
            check.summary,
            display_state_text(&check.status)
        ));
    }
}

fn append_setup_actions(output: &mut String, label: &str, actions: &[SetupAction]) {
    if label == "Optional" && actions.is_empty() {
        return;
    }
    output.push_str(&format!("\n{label}:\n"));
    if actions.is_empty() {
        output.push_str("  none\n");
        return;
    }
    for (index, action) in actions.iter().enumerate() {
        output.push_str(&format!(
            "  {}. {}\n",
            index + 1,
            trimmed_sentence(&action.instruction)
        ));
        if let Some(command) = &action.command {
            output.push_str(&format!("     Run:\n       {command}\n"));
        }
    }
}

fn setup_states_json(report: &SetupReport) -> Value {
    json!({
        "runtime_home": report.runtime_home.status.as_str(),
        "installation_profile": report.installation_profile.status.as_str(),
        "command_availability": setup_command_state(report),
        "host_reload_required": setup_host_reload_required(report),
    })
}

fn setup_command_state(report: &SetupReport) -> &'static str {
    if report.commands.iter().any(|command| !command.discovered) {
        "not_found"
    } else if report
        .commands
        .iter()
        .any(|command| !command.selected_path_ready())
    {
        "action_required"
    } else {
        "ready"
    }
}

fn setup_host_reload_required(report: &SetupReport) -> bool {
    report.actions_required.iter().any(|action| {
        matches!(
            action.kind,
            SetupActionKind::CommandAvailability
                | SetupActionKind::CommandLinks
                | SetupActionKind::PathUpdate
                | SetupActionKind::ShellStartup
        )
    })
}

fn primary_setup_action(report: &SetupReport) -> Option<&SetupAction> {
    report.actions_required.first()
}

fn setup_result_text(status: SetupStatus) -> &'static str {
    match status {
        SetupStatus::ActionRequired => "action_required (not a fatal CLI error)",
        SetupStatus::Complete => "complete",
        SetupStatus::Failed => "failed",
    }
}

fn setup_status_meaning(status: SetupStatus) -> &'static str {
    match status {
        SetupStatus::Complete => "installation profile setup is complete",
        SetupStatus::ActionRequired => "installation profile setup needs a named user action",
        SetupStatus::Failed => "installation profile setup could not complete",
    }
}

fn display_state_text(value: &str) -> String {
    value.replace('_', " ")
}

fn trimmed_sentence(value: &str) -> &str {
    value.trim().trim_end_matches('.')
}

pub(super) fn append_interactive_notes(
    mut output: String,
    format: OutputFormat,
    notes: &[String],
) -> String {
    if format == OutputFormat::Text && !notes.is_empty() {
        output.push_str("interactive_setup:\n");
        for note in notes {
            output.push_str(&format!("- {note}\n"));
        }
    }
    output
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

pub(crate) fn profile_json(profile: &InstallationProfileRecord) -> Value {
    json!({
        "installation_id": profile.installation_id,
        "runtime_home_id": profile.runtime_home_id,
        "volicord_command": profile.volicord_command,
        "volicord_mcp_command": profile.volicord_mcp_command,
        "bin_dir": path_text(&profile.bin_dir),
        "default_connection_mode": profile.default_connection_mode,
        "created_at": profile.created_at,
        "updated_at": profile.updated_at,
    })
}
