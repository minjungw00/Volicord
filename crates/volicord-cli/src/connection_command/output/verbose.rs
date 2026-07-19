use std::collections::BTreeSet;

use serde_json::{Map, Value};
use volicord_types::{
    ConnectionCheck, ConnectionCheckKind, ConnectionCheckStatus, LIST_PROJECTS_TOOL_NAME,
};

use super::{
    human::{headline, CheckCounts},
    report::{ConnectionCommandReport, ConnectionCommandResult},
    PlannedConnectionChangeKind,
};

const MAX_DETAIL_RENDER_DEPTH: usize = 8;
const MAX_INLINE_SCALARS: usize = 8;

pub(super) fn render_command_report_verbose(report: &ConnectionCommandReport) -> String {
    let counts = CheckCounts::from_report(report);
    let mut sections = vec![headline(report, counts), render_connection(report)];
    sections.push(render_summary(report, counts));

    if !report.checks.is_empty() {
        sections.push(render_checks(report));
    }
    if !report.actions.is_empty() {
        sections.push(render_actions(report));
    }
    if let Some(result) = report.result.as_ref() {
        sections.push(render_result(result));
    }
    if let Some(changes) = report
        .planned_changes
        .as_deref()
        .filter(|changes| !changes.is_empty())
    {
        sections.push(render_planned_changes(changes));
    }
    if !report.limits.is_empty() {
        sections.push(render_assurance(report));
    }

    format!("{}\n", sections.join("\n\n"))
}

fn render_connection(report: &ConnectionCommandReport) -> String {
    format!(
        concat!(
            "Connection\n",
            "  ID: {}\n",
            "  Host: {}\n",
            "  Scope: {}\n",
            "  Profile: {}\n",
            "  Mode: {}\n",
            "  Repository: {}\n",
            "  Config target: {}\n",
            "  Runtime home: {}",
        ),
        report.connection.id,
        report.connection.host,
        report.connection.scope,
        report.connection.profile,
        report.connection.mode,
        report.connection.repository,
        report.connection.config_target,
        report.runtime_home,
    )
}

fn render_summary(report: &ConnectionCommandReport, counts: CheckCounts) -> String {
    let mut lines = vec![
        "Summary".to_owned(),
        format!("  Status: {}", report.status.as_str()),
    ];
    if report.dry_run {
        lines.push("  Dry run: yes".to_owned());
    }
    lines.push(format!(
        "  Checks: {} passed, {} pending, {} failed",
        counts.ready, counts.waiting, counts.failed
    ));
    lines.join("\n")
}

fn render_checks(report: &ConnectionCommandReport) -> String {
    let mut blocks = Vec::with_capacity(report.checks.len());
    for check in &report.checks {
        blocks.push(render_check(report, check));
    }
    format!("Checks\n{}", blocks.join("\n\n"))
}

fn render_check(report: &ConnectionCommandReport, check: &ConnectionCheck) -> String {
    let mut lines = vec![format!(
        "  [{}] {}",
        check_status_label(check.status()),
        check_label(check.id())
    )];
    push_multiline(&mut lines, 4, check.summary());
    if let Some(code) = check.code() {
        lines.push(format!("    Code: {code}"));
    }
    if let Some(observed_at) = check.observed_at() {
        lines.push(format!(
            "    Observed at: {}",
            observed_at.to_canonical_string()
        ));
    }

    let mut details = DetailContext::new(report, check);
    render_known_details(&mut details);
    details.render_additional();
    lines.extend(details.lines);
    lines.join("\n")
}

fn check_status_label(status: ConnectionCheckStatus) -> &'static str {
    match status {
        ConnectionCheckStatus::Passed => "pass",
        ConnectionCheckStatus::Pending => "wait",
        ConnectionCheckStatus::Failed => "fail",
    }
}

fn check_label(kind: ConnectionCheckKind) -> &'static str {
    match kind {
        ConnectionCheckKind::VerificationNotRun => "Connection verification",
        ConnectionCheckKind::ManagedConfig => "Managed Codex configuration",
        ConnectionCheckKind::HostExecutable => "Codex executable",
        ConnectionCheckKind::McpServer => "Volicord MCP server",
        ConnectionCheckKind::HostSession => "Codex managed session",
        ConnectionCheckKind::RequiredTools => "Codex required tools",
        ConnectionCheckKind::ToolRoundTrip => "Read-only tool round trip",
        ConnectionCheckKind::ProjectTrust => "Project trust",
        ConnectionCheckKind::GuardFiles => "Guard managed files",
        ConnectionCheckKind::GuardObservation => "Guard hook activity",
        ConnectionCheckKind::SetupPlan => "Setup plan",
        ConnectionCheckKind::ModeTransition => "Connection mode transition",
        ConnectionCheckKind::ConnectionRemoval => "Connection removal",
    }
}

struct DetailContext<'a> {
    report: &'a ConnectionCommandReport,
    check: &'a ConnectionCheck,
    object: Option<&'a Map<String, Value>>,
    consumed: BTreeSet<String>,
    lines: Vec<String>,
}

impl<'a> DetailContext<'a> {
    fn new(report: &'a ConnectionCommandReport, check: &'a ConnectionCheck) -> Self {
        Self {
            report,
            check,
            object: check.details().map(|details| details.as_object()),
            consumed: BTreeSet::new(),
            lines: Vec::new(),
        }
    }

    fn value(&mut self, path: &str) -> Option<&'a Value> {
        self.consumed.insert(path.to_owned());
        value_at_path(self.object?, path)
    }

    fn string(&mut self, path: &str) -> Option<String> {
        self.value(path).and_then(Value::as_str).map(str::to_owned)
    }

    fn boolean(&mut self, path: &str) -> Option<bool> {
        self.value(path).and_then(Value::as_bool)
    }

    fn string_array(&mut self, path: &str) -> Vec<String> {
        self.value(path)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    }

    fn line(&mut self, label: &str, value: impl std::fmt::Display) {
        self.lines.push(format!("    {label}: {value}"));
    }

    fn diagnostic(&mut self, label: &str, value: &str) {
        let Some((prefix, nested)) = split_json_suffix(value) else {
            self.line(label, value);
            return;
        };
        let prefix = prefix.trim().trim_end_matches(':').trim_end();
        if prefix.is_empty() {
            self.lines.push(format!("    {label}"));
        } else {
            self.line(label, prefix);
        }
        render_generic_value(
            "Response details".to_owned(),
            &nested,
            "",
            &BTreeSet::new(),
            6,
            0,
            &mut self.lines,
        );
    }

    fn render_additional(&mut self) {
        let Some(object) = self.object else {
            return;
        };
        if !has_renderable_object(object, "", &self.consumed, 0) {
            return;
        }
        self.lines.push("    Additional details".to_owned());
        render_generic_object(object, "", &self.consumed, 6, 0, &mut self.lines);
    }
}

fn value_at_path<'a>(object: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
    let mut segments = path.split('.');
    let first = segments.next()?;
    let mut value = object.get(first)?;
    for segment in segments {
        value = value.as_object()?.get(segment)?;
    }
    Some(value)
}

fn render_known_details(context: &mut DetailContext<'_>) {
    match context.check.id() {
        ConnectionCheckKind::VerificationNotRun => {}
        ConnectionCheckKind::ManagedConfig => render_managed_config(context),
        ConnectionCheckKind::HostExecutable => render_host_executable(context),
        ConnectionCheckKind::McpServer => render_mcp_server(context),
        ConnectionCheckKind::HostSession => render_host_session(context),
        ConnectionCheckKind::RequiredTools => render_required_tools(context),
        ConnectionCheckKind::ToolRoundTrip => render_tool_round_trip(context),
        ConnectionCheckKind::ProjectTrust => render_project_trust(context),
        ConnectionCheckKind::GuardFiles => render_guard_files(context),
        ConnectionCheckKind::GuardObservation => render_guard_observation(context),
        ConnectionCheckKind::SetupPlan => render_setup_plan(context),
        ConnectionCheckKind::ModeTransition => render_mode_transition(context),
        ConnectionCheckKind::ConnectionRemoval => render_connection_removal(context),
    }
}

fn render_managed_config(context: &mut DetailContext<'_>) {
    if let Some(target) = context.string("target") {
        context.line("Target", target);
    }
    if let Some(state) = context.string("observed_state") {
        context.line("State", state);
    }
    if let Some(code) = context.string("diagnostic_code") {
        context.line("Diagnostic code", code);
    }
    if let Some(diagnostic) = context.string("diagnostic") {
        if diagnostic_adds_information(&diagnostic, context.check.summary()) {
            context.diagnostic("Diagnostic", &diagnostic);
        }
    }
}

fn render_host_executable(context: &mut DetailContext<'_>) {
    if let Some(version) = context.string("version") {
        context.line("Version", version);
    }
    if let Some(path) = context.string("path") {
        context.line("Path", path);
    }
    if let Some(diagnostic) = context.string("diagnostic") {
        if diagnostic_adds_information(&diagnostic, context.check.summary()) {
            context.diagnostic("Probe diagnostic", &diagnostic);
        }
    }
}

fn render_mcp_server(context: &mut DetailContext<'_>) {
    let preflight = context.string("preflight.status");
    let preflight_code = context.string("preflight.code");
    let preflight_diagnostic = context.string("preflight.diagnostic");
    if let Some(status) = preflight.as_deref() {
        context.line("Preflight", status);
    }
    if let Some(storage_read) = context.string("preflight.storage.storage_read") {
        let storage_write = context.string("preflight.storage.storage_write");
        match storage_write {
            Some(write) => context.line(
                "Storage",
                format_args!("read {storage_read}, write {write}"),
            ),
            None => context.line("Storage read", storage_read),
        }
    } else if let Some(storage_write) = context.string("preflight.storage.storage_write") {
        context.line("Storage write", storage_write);
    }
    if let Some(mode) = context.string("preflight.storage.effective_tool_mode") {
        context.line("Effective mode", mode);
    }

    let self_test_status = context.string("self_test.status");
    let self_test_code = context.string("self_test.code");
    let diagnostic = context.string("self_test.diagnostic");
    let initialize = context.boolean("self_test.initialize");
    let tools = context.string_array("self_test.tools_list");
    let safe_tool = context
        .string("self_test.safe_read_only_tool")
        .unwrap_or_else(|| LIST_PROJECTS_TOOL_NAME.to_owned());
    let stage_code = context
        .check
        .code()
        .or(self_test_code.as_deref())
        .unwrap_or_default();
    let preflight_passed = preflight.as_deref() == Some("passed");
    let self_test_passed = self_test_status.as_deref() == Some("passed");

    context.line(
        "Initialize",
        mcp_initialize_result(preflight_passed, self_test_passed, initialize, stage_code),
    );
    context.line(
        "Required tools",
        mcp_required_tools_result(preflight_passed, self_test_passed, stage_code),
    );
    if !tools.is_empty() {
        context.line("Tools returned", tools.len());
    }
    let safe_result = mcp_safe_tool_result(preflight_passed, self_test_passed, stage_code);
    if safe_result == "passed" {
        context.line("Designated read-only tool", safe_tool);
    } else {
        context.line(
            "Designated read-only tool",
            format_args!("{safe_tool} ({safe_result})"),
        );
    }

    if let Some(diagnostic) = diagnostic.as_deref() {
        let missing = diagnostic_tool_names(diagnostic, "missing required tool:");
        let incompatible = diagnostic_tool_names(diagnostic, "incompatible required tool:");
        if !missing.is_empty() {
            context.line("Missing tools", render_string_values(&missing));
        }
        if !incompatible.is_empty() {
            context.line("Incompatible tools", render_string_values(&incompatible));
        }
        if !self_test_passed
            && missing.is_empty()
            && incompatible.is_empty()
            && diagnostic_adds_information(diagnostic, context.check.summary())
        {
            context.diagnostic("Self-test diagnostic", diagnostic);
        }
    }
    if preflight.as_deref() != Some("passed") {
        if let Some(code) = preflight_code {
            context.line("Preflight code", code);
        }
        if let Some(diagnostic) = preflight_diagnostic {
            if diagnostic_adds_information(&diagnostic, context.check.summary()) {
                context.diagnostic("Preflight diagnostic", &diagnostic);
            }
        }
    }
}

fn mcp_initialize_result(
    preflight_passed: bool,
    self_test_passed: bool,
    initialize: Option<bool>,
    code: &str,
) -> &'static str {
    if self_test_passed || code.contains("tools_list") || code.contains("safe_call") {
        "passed"
    } else if !preflight_passed {
        "not run"
    } else if initialize == Some(true) {
        "passed"
    } else if code.contains("initialize") || code.contains("process") {
        "failed"
    } else {
        "not completed"
    }
}

fn mcp_required_tools_result(
    preflight_passed: bool,
    self_test_passed: bool,
    code: &str,
) -> &'static str {
    if self_test_passed || code.contains("safe_call") {
        "passed"
    } else if code.contains("tools_list") {
        "failed"
    } else if preflight_passed {
        "not completed"
    } else {
        "not run"
    }
}

fn mcp_safe_tool_result(
    preflight_passed: bool,
    self_test_passed: bool,
    code: &str,
) -> &'static str {
    if self_test_passed {
        "passed"
    } else if code.contains("safe_call") {
        "failed"
    } else if preflight_passed {
        "not completed"
    } else {
        "not run"
    }
}

fn diagnostic_tool_names(diagnostic: &str, marker: &str) -> Vec<String> {
    diagnostic
        .split(marker)
        .skip(1)
        .filter_map(|suffix| {
            let name = suffix
                .trim_start()
                .split(|character: char| {
                    character.is_whitespace() || character == ',' || character == ';'
                })
                .next()
                .unwrap_or_default()
                .trim_matches(|character: char| {
                    matches!(character, '`' | '"' | '\'' | '.' | ']' | '}')
                });
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

fn split_json_suffix(value: &str) -> Option<(&str, Value)> {
    value.char_indices().find_map(|(index, character)| {
        if !matches!(character, '{' | '[') {
            return None;
        }
        let nested = serde_json::from_str::<Value>(value[index..].trim()).ok()?;
        matches!(nested, Value::Object(_) | Value::Array(_)).then(|| (&value[..index], nested))
    })
}

fn render_host_session(context: &mut DetailContext<'_>) {
    render_revision_pair(context);
    if let Some(version) = context.string("current_host_version") {
        context.line("Current host version", version);
    }
    if let Some(version) = context.string("observed_host_version") {
        context.line("Observed host version", version);
    }
    context.line("Initialize", host_initialize_result(context.check));
    render_terminal_failure(context);
    render_last_observed(context);
}

fn host_initialize_result(check: &ConnectionCheck) -> &'static str {
    match (check.status(), check.code().unwrap_or_default()) {
        (ConnectionCheckStatus::Passed, _) => "completed",
        (ConnectionCheckStatus::Failed, _) => "failed",
        (_, code) if code.contains("not_observed") || code.contains("revision_stale") => {
            "not observed"
        }
        _ => "pending",
    }
}

fn render_required_tools(context: &mut DetailContext<'_>) {
    render_revision_pair(context);
    let observed = context
        .boolean("tools_list_observed")
        .or_else(|| context.string("tools_list_observed_at").map(|_| true))
        .unwrap_or_else(|| {
            matches!(
                context.check.code(),
                Some("required_tools_present" | "required_tools_missing")
            )
        });
    context.line("Tools/list observed", yes_no(observed));
    let explicit_result = context.boolean("required_tools_present");
    let result = match (context.check.status(), explicit_result) {
        (_, Some(true)) | (ConnectionCheckStatus::Passed, _) => "passed",
        (_, Some(false)) | (ConnectionCheckStatus::Failed, _) => "failed",
        _ => "pending",
    };
    context.line("Required tools", result);
    let missing = context.string_array("missing_tools");
    if !missing.is_empty() {
        context.line("Missing tools", render_string_values(&missing));
    }
    render_terminal_failure(context);
    render_last_observed(context);
}

fn render_tool_round_trip(context: &mut DetailContext<'_>) {
    render_revision_pair(context);
    let safe_tool = context
        .string("safe_read_only_tool")
        .unwrap_or_else(|| LIST_PROJECTS_TOOL_NAME.to_owned());
    context.line("Designated read-only tool", safe_tool);
    let completed = context
        .boolean("call_completed")
        .unwrap_or(context.check.status() == ConnectionCheckStatus::Passed);
    context.line("Call completed", yes_no(completed));
    render_terminal_failure(context);
    render_last_observed(context);
}

fn render_revision_pair(context: &mut DetailContext<'_>) {
    if let Some(revision) = context.string("current_integration_revision") {
        context.line("Current revision", revision);
    }
    if let Some(revision) = context.string("observed_integration_revision") {
        context.line("Observed revision", revision);
    }
}

fn render_terminal_failure(context: &mut DetailContext<'_>) {
    if let Some(code) = context.string("terminal_failure_code") {
        context.line("Terminal failure code", code);
    }
    if let Some(details) = context.string("terminal_failure_details") {
        context.diagnostic("Terminal failure", &details);
    }
}

fn render_last_observed(context: &mut DetailContext<'_>) {
    if let Some(last_observed) = context.string("last_observed_at") {
        let duplicate = context
            .check
            .observed_at()
            .is_some_and(|observed_at| observed_at.to_canonical_string() == last_observed);
        if !duplicate {
            context.line("Last observed", last_observed);
        }
    }
}

fn render_project_trust(context: &mut DetailContext<'_>) {
    let applicable = context.boolean("applicable").unwrap_or(true);
    if !applicable {
        return;
    }
    context.line("Applicable", "yes");
    if let Some(state) = context.string("observed_state") {
        context.line("State", state);
    }
    if let Some(target) = context.string("repo_root") {
        context.line("Target", target);
    }
    if let Some(path) = context.string("config_path") {
        context.line("Configuration", path);
    }
    if let Some(diagnostic) = context.string("diagnostic") {
        if diagnostic_adds_information(&diagnostic, context.check.summary()) {
            context.diagnostic("Diagnostic", &diagnostic);
        }
    }
}

fn render_guard_files(context: &mut DetailContext<'_>) {
    let installations = context.string_array("installation_ids");
    if !installations.is_empty() {
        context.line(
            "Guard Installation IDs",
            render_string_values(&installations),
        );
    }
    let affected_paths = context.string_array("affected_paths");
    render_list(&mut context.lines, "Affected paths", &affected_paths);

    let artifact_issues = context.value("artifact_issues").cloned();
    if let Some(issues) = artifact_issues.as_ref().and_then(Value::as_array) {
        let issues = issues
            .iter()
            .filter_map(Value::as_object)
            .filter(|issue| !issue.is_empty())
            .collect::<Vec<_>>();
        if !issues.is_empty() {
            context.lines.push("    Artifact issues".to_owned());
            for (index, issue) in issues.into_iter().enumerate() {
                context.lines.push(format!("      {}", index + 1));
                for (key, label) in [
                    ("artifact", "Artifact"),
                    ("path", "Path"),
                    ("issue", "Issue"),
                    ("details", "Details"),
                ] {
                    if let Some(value) = issue.get(key).and_then(Value::as_str) {
                        push_labeled_multiline(&mut context.lines, 8, label, value);
                    }
                }
            }
        }
    }
    let manifest_issues = context.string_array("manifest_issues");
    if !manifest_issues.is_empty() {
        context.line("Manifest issues", render_string_values(&manifest_issues));
    }
    let mut missing_phases = context.string_array("missing_required_phases");
    sort_phases(&mut missing_phases);
    if !missing_phases.is_empty() {
        context.line(
            "Missing required phases",
            render_string_values(&missing_phases),
        );
    }
}

fn render_guard_observation(context: &mut DetailContext<'_>) {
    for (path, label) in [
        ("required_phases", "Required phases"),
        ("observed_phases", "Observed phases"),
        ("missing_required_phases", "Missing phases"),
    ] {
        let mut phases = context.string_array(path);
        sort_phases(&mut phases);
        if !phases.is_empty() {
            context.line(label, render_string_values(&phases));
        }
    }
    let incompatible = context.string_array("incompatible_event_ids");
    if !incompatible.is_empty() {
        context.line(
            "Incompatible event IDs",
            render_string_values(&incompatible),
        );
    }
    let configured = context.boolean("prompt_capture.configured");
    let supported = context.boolean("prompt_capture.host_supported");
    let observed = context.boolean("prompt_capture.observed");
    if configured.is_some() || supported.is_some() || observed.is_some() {
        context.line(
            "Prompt capture",
            format_args!(
                "{}, {}, {}",
                boolean_word(configured, "configured"),
                boolean_word(supported, "supported"),
                boolean_word(observed, "observed")
            ),
        );
    }
    if let Some(last_observed) = context.string("last_current_observation_at") {
        let duplicate = context
            .check
            .observed_at()
            .is_some_and(|observed_at| observed_at.to_canonical_string() == last_observed);
        if !duplicate {
            context.line("Last current observation", last_observed);
        }
    }
}

fn render_setup_plan(context: &mut DetailContext<'_>) {
    let state = match context.check.status() {
        ConnectionCheckStatus::Passed => "ready",
        ConnectionCheckStatus::Pending => "changes ready to apply",
        ConnectionCheckStatus::Failed => "partial application",
    };
    context.line("Planned state", state);
    let Some(changes) = context.report.planned_changes.as_deref() else {
        return;
    };
    for kind in planned_change_kinds() {
        let count = changes
            .iter()
            .filter(|change| change.kind() == kind)
            .count();
        if count > 0 {
            context.line(kind.as_str(), count);
        }
    }
}

fn render_mode_transition(context: &mut DetailContext<'_>) {
    let Some(ConnectionCommandResult::ModeTransition {
        changed,
        previous_mode,
        current_mode,
        previous_integration_revision,
        current_integration_revision,
        rebound_guard_installation_ids,
    }) = context.report.result.as_ref()
    else {
        return;
    };
    context.line("Previous mode", previous_mode);
    context.line("Current mode", current_mode);
    context.line("Transition", if *changed { "changed" } else { "no-op" });
    context.line("Previous revision", previous_integration_revision);
    context.line("Current revision", current_integration_revision);
    if !rebound_guard_installation_ids.is_empty() {
        context.line(
            "Rebound Guard Installation IDs",
            render_string_values(rebound_guard_installation_ids),
        );
    }
}

fn render_connection_removal(context: &mut DetailContext<'_>) {
    if context.report.dry_run {
        context.line("Membership", "planned for removal");
        context.line("Connection", "retained until changes are applied");
        return;
    }
    let Some(ConnectionCommandResult::Removal {
        membership_removed,
        connection_removed,
        remaining_project_count,
    }) = context.report.result.as_ref()
    else {
        return;
    };
    context.line(
        "Membership",
        if *membership_removed {
            "removed"
        } else {
            "retained"
        },
    );
    context.line(
        "Connection",
        if *connection_removed {
            "removed"
        } else {
            "retained"
        },
    );
    context.line("Remaining project count", remaining_project_count);
}

fn render_actions(report: &ConnectionCommandReport) -> String {
    let mut blocks = Vec::with_capacity(report.actions.len());
    for action in &report.actions {
        let mut lines = vec![format!("  {}", action.id().as_str())];
        push_multiline(&mut lines, 4, action.instruction());
        if let Some(command) = action.command() {
            push_labeled_multiline(&mut lines, 4, "Command", command);
        }
        blocks.push(lines.join("\n"));
    }
    format!("Actions\n{}", blocks.join("\n\n"))
}

fn render_result(result: &ConnectionCommandResult) -> String {
    let mut lines = vec!["Result".to_owned()];
    match result {
        ConnectionCommandResult::Setup { applied } => {
            lines.push(format!("  Applied: {}", yes_no(*applied)));
        }
        ConnectionCommandResult::ModeTransition {
            changed,
            previous_mode,
            current_mode,
            previous_integration_revision,
            current_integration_revision,
            rebound_guard_installation_ids,
        } => {
            lines.push(format!("  Changed: {}", yes_no(*changed)));
            lines.push(format!("  Previous mode: {previous_mode}"));
            lines.push(format!("  Current mode: {current_mode}"));
            lines.push(format!(
                "  Previous revision: {previous_integration_revision}"
            ));
            lines.push(format!(
                "  Current revision: {current_integration_revision}"
            ));
            render_list_at(
                &mut lines,
                2,
                "Rebound Guard Installation IDs",
                rebound_guard_installation_ids,
            );
        }
        ConnectionCommandResult::Removal {
            membership_removed,
            connection_removed,
            remaining_project_count,
        } => {
            lines.push(format!(
                "  Membership removed: {}",
                yes_no(*membership_removed)
            ));
            lines.push(format!(
                "  Connection record removed: {}",
                yes_no(*connection_removed)
            ));
            lines.push(format!(
                "  Remaining project count: {remaining_project_count}"
            ));
        }
    }
    lines.join("\n")
}

fn render_planned_changes(changes: &[super::PlannedConnectionChange]) -> String {
    let mut blocks = Vec::with_capacity(changes.len());
    for (index, change) in changes.iter().enumerate() {
        blocks.push(format!(
            concat!(
                "  Change {}\n",
                "    Kind: {}\n",
                "    Operation: {}\n",
                "    Target: {}",
            ),
            index + 1,
            change.kind().as_str(),
            change.operation().as_str(),
            change.target(),
        ));
    }
    format!("Planned changes\n{}", blocks.join("\n\n"))
}

fn render_assurance(report: &ConnectionCommandReport) -> String {
    let mut lines = vec!["Assurance".to_owned()];
    for limit in &report.limits {
        push_multiline(&mut lines, 2, limit);
    }
    lines.join("\n")
}

fn planned_change_kinds() -> [PlannedConnectionChangeKind; 6] {
    [
        PlannedConnectionChangeKind::ConnectionMembership,
        PlannedConnectionChangeKind::GuardManagedFile,
        PlannedConnectionChangeKind::GuardRegistrySetup,
        PlannedConnectionChangeKind::ManagedHostConfiguration,
        PlannedConnectionChangeKind::ProjectRegistration,
        PlannedConnectionChangeKind::RuntimeHomeInitialization,
    ]
}

fn render_list(lines: &mut Vec<String>, label: &str, values: &[String]) {
    render_list_at(lines, 4, label, values);
}

fn render_list_at(lines: &mut Vec<String>, indent: usize, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    lines.push(format!("{}{label}", " ".repeat(indent)));
    for value in values {
        push_multiline(lines, indent + 2, value);
    }
}

fn sort_phases(phases: &mut [String]) {
    phases.sort_by(|left, right| {
        phase_rank(left)
            .cmp(&phase_rank(right))
            .then_with(|| left.cmp(right))
    });
}

fn phase_rank(phase: &str) -> u8 {
    match phase {
        "pre_tool" => 0,
        "post_tool" => 1,
        "prompt_capture" => 2,
        _ => 3,
    }
}

fn diagnostic_adds_information(diagnostic: &str, summary: &str) -> bool {
    let diagnostic = diagnostic.trim();
    if diagnostic.is_empty() {
        return false;
    }
    let diagnostic_lower = diagnostic.to_lowercase();
    let summary_lower = summary.to_lowercase();
    !summary_lower.contains(&diagnostic_lower) && !diagnostic_lower.contains(&summary_lower)
}

fn boolean_word(value: Option<bool>, word: &str) -> String {
    match value {
        Some(true) => word.to_owned(),
        Some(false) => format!("not {word}"),
        None => format!("{word} unknown"),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn render_string_values(values: &[String]) -> String {
    let visible = values
        .iter()
        .take(MAX_INLINE_SCALARS)
        .cloned()
        .collect::<Vec<_>>();
    if values.len() <= MAX_INLINE_SCALARS {
        visible.join(", ")
    } else {
        format!(
            "{} values: {}; {} more",
            values.len(),
            visible.join(", "),
            values.len() - MAX_INLINE_SCALARS
        )
    }
}

fn push_multiline(lines: &mut Vec<String>, indent: usize, value: &str) {
    for line in value.lines() {
        lines.push(format!("{}{line}", " ".repeat(indent)));
    }
    if value.is_empty() {
        lines.push(" ".repeat(indent));
    }
}

fn push_labeled_multiline(lines: &mut Vec<String>, indent: usize, label: &str, value: &str) {
    let mut value_lines = value.lines();
    let first = value_lines.next().unwrap_or_default();
    lines.push(format!("{}{label}: {first}", " ".repeat(indent)));
    for line in value_lines {
        lines.push(format!("{}{line}", " ".repeat(indent + 2)));
    }
}

fn has_renderable_object(
    object: &Map<String, Value>,
    path: &str,
    consumed: &BTreeSet<String>,
    depth: usize,
) -> bool {
    object.iter().any(|(key, value)| {
        let child_path = joined_path(path, key);
        !consumed.contains(&child_path) && has_renderable_value(value, &child_path, consumed, depth)
    })
}

fn has_renderable_value(
    value: &Value,
    path: &str,
    consumed: &BTreeSet<String>,
    depth: usize,
) -> bool {
    if consumed.contains(path) || value.is_null() {
        return false;
    }
    if depth >= MAX_DETAIL_RENDER_DEPTH {
        return true;
    }
    match value {
        Value::Object(object) => has_renderable_object(object, path, consumed, depth + 1),
        Value::Array(values) => values
            .iter()
            .any(|value| has_renderable_value(value, path, consumed, depth + 1)),
        _ => true,
    }
}

fn render_generic_object(
    object: &Map<String, Value>,
    path: &str,
    consumed: &BTreeSet<String>,
    indent: usize,
    depth: usize,
    lines: &mut Vec<String>,
) {
    let mut keys = object.keys().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        let child_path = joined_path(path, key);
        let value = &object[key];
        if !has_renderable_value(value, &child_path, consumed, depth) {
            continue;
        }
        render_generic_value(
            humanize_key(key),
            value,
            &child_path,
            consumed,
            indent,
            depth,
            lines,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_generic_value(
    label: String,
    value: &Value,
    path: &str,
    consumed: &BTreeSet<String>,
    indent: usize,
    depth: usize,
    lines: &mut Vec<String>,
) {
    if depth >= MAX_DETAIL_RENDER_DEPTH {
        lines.push(format!(
            "{}{label}: [nested details omitted at depth limit]",
            " ".repeat(indent)
        ));
        return;
    }
    match value {
        Value::Null => {}
        Value::Bool(value) => {
            lines.push(format!("{}{label}: {}", " ".repeat(indent), yes_no(*value)))
        }
        Value::Number(value) => lines.push(format!("{}{label}: {value}", " ".repeat(indent))),
        Value::String(value) => push_labeled_multiline(lines, indent, &label, value),
        Value::Object(object) => {
            lines.push(format!("{}{label}", " ".repeat(indent)));
            render_generic_object(object, path, consumed, indent + 2, depth + 1, lines);
        }
        Value::Array(values) if values.iter().all(is_scalar) => {
            let rendered = values.iter().filter_map(render_scalar).collect::<Vec<_>>();
            if !rendered.is_empty() {
                lines.push(format!(
                    "{}{label}: {}",
                    " ".repeat(indent),
                    render_string_values(&rendered)
                ));
            }
        }
        Value::Array(values) => {
            lines.push(format!("{}{label}", " ".repeat(indent)));
            for (index, value) in values.iter().enumerate() {
                if !has_renderable_value(value, path, consumed, depth + 1) {
                    continue;
                }
                match value {
                    Value::Object(object) => {
                        lines.push(format!("{}{}", " ".repeat(indent + 2), index + 1));
                        render_generic_object(object, path, consumed, indent + 4, depth + 1, lines);
                    }
                    _ => render_generic_value(
                        (index + 1).to_string(),
                        value,
                        path,
                        consumed,
                        indent + 2,
                        depth + 1,
                        lines,
                    ),
                }
            }
        }
    }
}

fn joined_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

fn humanize_key(key: &str) -> String {
    let words = key.replace('_', " ");
    let mut characters = words.chars();
    match characters.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), characters.as_str()),
        None => String::new(),
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn render_scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(yes_no(*value).to_owned()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.to_owned()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;
    use volicord_types::{
        ConnectionAction, ConnectionActionKind, ConnectionCheckDetails, ConnectionStatus,
        UtcTimestamp,
    };

    use super::*;
    use crate::connection_command::{
        output::{cooperative_assurance_limits, CommandConnection, CommandOperation},
        planning::{PlannedChangeOperation, PlannedConnectionChange},
    };

    fn connection(mode: &str) -> CommandConnection {
        CommandConnection::new(
            "connection_1",
            "codex",
            "user",
            mode,
            Path::new("/workspace/product"),
            "/home/user/.codex/config.toml",
        )
    }

    fn details(value: Value) -> Option<ConnectionCheckDetails> {
        let Value::Object(object) = value else {
            panic!("test details must be an object")
        };
        Some(ConnectionCheckDetails::try_new(object).unwrap())
    }

    fn check(
        id: ConnectionCheckKind,
        status: ConnectionCheckStatus,
        code: Option<&str>,
        summary: &str,
        detail: Option<Value>,
        observed_at: Option<&str>,
    ) -> ConnectionCheck {
        ConnectionCheck::try_new(
            id,
            status,
            code.map(str::to_owned),
            summary,
            detail.and_then(details),
            observed_at.map(|value| UtcTimestamp::parse(value).unwrap()),
        )
        .unwrap()
    }

    fn action(
        id: ConnectionActionKind,
        instruction: &str,
        command: Option<&str>,
    ) -> ConnectionAction {
        ConnectionAction::try_new(id, instruction, command.map(str::to_owned)).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn report(
        operation: CommandOperation,
        dry_run: bool,
        status: ConnectionStatus,
        mode: &str,
        checks: Vec<ConnectionCheck>,
        actions: Vec<ConnectionAction>,
        result: Option<ConnectionCommandResult>,
        planned_changes: Option<Vec<PlannedConnectionChange>>,
    ) -> ConnectionCommandReport {
        ConnectionCommandReport {
            operation,
            dry_run,
            status,
            runtime_home: "/runtime".to_owned(),
            connection: connection(mode),
            checks,
            actions,
            result,
            planned_changes,
            limits: cooperative_assurance_limits(),
        }
    }

    fn rendered(report: &ConnectionCommandReport) -> String {
        render_command_report_verbose(report)
    }

    #[test]
    fn verbose_representative_report_is_exact_and_uses_every_section() {
        let report = report(
            CommandOperation::Init,
            true,
            ConnectionStatus::Failed,
            "workflow",
            vec![
                check(
                    ConnectionCheckKind::GuardFiles,
                    ConnectionCheckStatus::Passed,
                    None,
                    "Guard managed files match current expectations",
                    Some(json!({
                        "installation_ids": ["guard_1"],
                        "affected_paths": [],
                        "artifact_issues": [],
                        "manifest_issues": [],
                        "missing_required_phases": [],
                    })),
                    None,
                ),
                check(
                    ConnectionCheckKind::HostSession,
                    ConnectionCheckStatus::Pending,
                    Some("host_session_initialize_pending"),
                    "Codex initialize has not completed",
                    Some(json!({
                        "current_integration_revision": "revision_current",
                        "observed_integration_revision": "revision_current",
                        "current_host_version": "1.2.3",
                        "observed_host_version": "1.2.3",
                        "last_observed_at": "2026-07-20T00:00:00Z",
                        "terminal_failure_code": null,
                        "terminal_failure_details": null,
                    })),
                    Some("2026-07-20T00:00:00Z"),
                ),
                check(
                    ConnectionCheckKind::ManagedConfig,
                    ConnectionCheckStatus::Failed,
                    Some("managed_config_mismatch"),
                    "Managed Codex configuration differs from the canonical entry",
                    Some(json!({
                        "target": "/home/user/.codex/config.toml",
                        "observed_state": "changed",
                        "diagnostic_code": "managed_config_mismatch",
                        "diagnostic": "managed command differs",
                    })),
                    None,
                ),
            ],
            vec![action(
                ConnectionActionKind::RepairManagedConfig,
                "Repair the managed Codex configuration",
                Some("volicord init --host codex --repo /workspace/product --profile record"),
            )],
            Some(ConnectionCommandResult::Setup { applied: false }),
            Some(vec![PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ManagedHostConfiguration,
                PlannedChangeOperation::Update,
                "/home/user/.codex/config.toml",
            )]),
        );

        assert_eq!(
            rendered(&report),
            concat!(
                "Volicord setup changes are ready to review.\n\n",
                "Connection\n",
                "  ID: connection_1\n",
                "  Host: codex\n",
                "  Scope: user\n",
                "  Profile: record\n",
                "  Mode: workflow\n",
                "  Repository: /workspace/product\n",
                "  Config target: /home/user/.codex/config.toml\n",
                "  Runtime home: /runtime\n\n",
                "Summary\n",
                "  Status: failed\n",
                "  Dry run: yes\n",
                "  Checks: 1 passed, 1 pending, 1 failed\n\n",
                "Checks\n",
                "  [pass] Guard managed files\n",
                "    Guard managed files match current expectations\n",
                "    Guard Installation IDs: guard_1\n\n",
                "  [wait] Codex managed session\n",
                "    Codex initialize has not completed\n",
                "    Code: host_session_initialize_pending\n",
                "    Observed at: 2026-07-20T00:00:00Z\n",
                "    Current revision: revision_current\n",
                "    Observed revision: revision_current\n",
                "    Current host version: 1.2.3\n",
                "    Observed host version: 1.2.3\n",
                "    Initialize: pending\n\n",
                "  [fail] Managed Codex configuration\n",
                "    Managed Codex configuration differs from the canonical entry\n",
                "    Code: managed_config_mismatch\n",
                "    Target: /home/user/.codex/config.toml\n",
                "    State: changed\n",
                "    Diagnostic code: managed_config_mismatch\n",
                "    Diagnostic: managed command differs\n\n",
                "Actions\n",
                "  repair_managed_config\n",
                "    Repair the managed Codex configuration\n",
                "    Command: volicord init --host codex --repo /workspace/product --profile record\n\n",
                "Result\n",
                "  Applied: no\n\n",
                "Planned changes\n",
                "  Change 1\n",
                "    Kind: managed_host_configuration\n",
                "    Operation: update\n",
                "    Target: /home/user/.codex/config.toml\n\n",
                "Assurance\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );
    }

    #[test]
    fn exact_init_action_required_status_complete_and_verify_failed_outputs() {
        let init = report(
            CommandOperation::Init,
            false,
            ConnectionStatus::ActionRequired,
            "workflow",
            vec![check(
                ConnectionCheckKind::HostSession,
                ConnectionCheckStatus::Pending,
                Some("host_session_not_observed"),
                "Managed host connection use has not been observed",
                Some(json!({
                    "current_integration_revision": "revision_current",
                    "observed_integration_revision": null,
                    "current_host_version": "1.2.3",
                    "observed_host_version": null,
                    "last_observed_at": null,
                    "terminal_failure_code": null,
                    "terminal_failure_details": null,
                })),
                None,
            )],
            vec![action(
                ConnectionActionKind::ObserveCodex,
                "Restart or reload Codex and use the connection",
                None,
            )],
            Some(ConnectionCommandResult::Setup { applied: true }),
            None,
        );
        assert_eq!(
            rendered(&init),
            concat!(
                "Volicord setup was applied and needs one more step.\n\n",
                "Connection\n",
                "  ID: connection_1\n",
                "  Host: codex\n",
                "  Scope: user\n",
                "  Profile: record\n",
                "  Mode: workflow\n",
                "  Repository: /workspace/product\n",
                "  Config target: /home/user/.codex/config.toml\n",
                "  Runtime home: /runtime\n\n",
                "Summary\n",
                "  Status: action_required\n",
                "  Checks: 0 passed, 1 pending, 0 failed\n\n",
                "Checks\n",
                "  [wait] Codex managed session\n",
                "    Managed host connection use has not been observed\n",
                "    Code: host_session_not_observed\n",
                "    Current revision: revision_current\n",
                "    Current host version: 1.2.3\n",
                "    Initialize: not observed\n\n",
                "Actions\n",
                "  observe_codex\n",
                "    Restart or reload Codex and use the connection\n\n",
                "Result\n",
                "  Applied: yes\n\n",
                "Assurance\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );

        let status = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Complete,
            "workflow",
            vec![check(
                ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::Passed,
                None,
                "No separate project trust action applies to this connection scope",
                Some(json!({"applicable": false})),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        assert_eq!(
            rendered(&status),
            concat!(
                "Codex connection is ready.\n\n",
                "Connection\n",
                "  ID: connection_1\n",
                "  Host: codex\n",
                "  Scope: user\n",
                "  Profile: record\n",
                "  Mode: workflow\n",
                "  Repository: /workspace/product\n",
                "  Config target: /home/user/.codex/config.toml\n",
                "  Runtime home: /runtime\n\n",
                "Summary\n",
                "  Status: complete\n",
                "  Checks: 1 passed, 0 pending, 0 failed\n\n",
                "Checks\n",
                "  [pass] Project trust\n",
                "    No separate project trust action applies to this connection scope\n\n",
                "Assurance\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );

        let verify = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Failed,
                Some("mcp_server_tools_list_failed"),
                "Volicord MCP server self-test failed",
                Some(mcp_details(
                    "failed",
                    "MCP tools/list failed: MCP tools/list missing required tool: volicord.close_task",
                    Vec::new(),
                )),
                None,
            )],
            vec![action(
                ConnectionActionKind::RepairMcpServer,
                "Repair the MCP server and verify again",
                Some("volicord connection verify"),
            )],
            None,
            None,
        );
        assert_eq!(
            rendered(&verify),
            concat!(
                "Verification completed: 1 failed.\n\n",
                "Connection\n",
                "  ID: connection_1\n",
                "  Host: codex\n",
                "  Scope: user\n",
                "  Profile: record\n",
                "  Mode: workflow\n",
                "  Repository: /workspace/product\n",
                "  Config target: /home/user/.codex/config.toml\n",
                "  Runtime home: /runtime\n\n",
                "Summary\n",
                "  Status: failed\n",
                "  Checks: 0 passed, 0 pending, 1 failed\n\n",
                "Checks\n",
                "  [fail] Volicord MCP server\n",
                "    Volicord MCP server self-test failed\n",
                "    Code: mcp_server_tools_list_failed\n",
                "    Preflight: passed\n",
                "    Storage: read passed, write passed\n",
                "    Effective mode: workflow\n",
                "    Initialize: passed\n",
                "    Required tools: failed\n",
                "    Designated read-only tool: volicord.list_projects (not completed)\n",
                "    Missing tools: volicord.close_task\n\n",
                "Actions\n",
                "  repair_mcp_server\n",
                "    Repair the MCP server and verify again\n",
                "    Command: volicord connection verify\n\n",
                "Assurance\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );
    }

    #[test]
    fn exact_mode_changed_and_removal_dry_run_outputs_show_typed_facts() {
        let mode = ConnectionCommandReport::mode_transition(
            Path::new("/runtime"),
            connection("read_only"),
            true,
            "workflow".to_owned(),
            "read_only".to_owned(),
            "revision_before".to_owned(),
            "revision_after".to_owned(),
            vec!["guard_1".to_owned()],
        )
        .unwrap();
        assert_eq!(
            rendered(&mode),
            concat!(
                "Connection mode changed from workflow to read_only.\n\n",
                "Connection\n",
                "  ID: connection_1\n",
                "  Host: codex\n",
                "  Scope: user\n",
                "  Profile: record\n",
                "  Mode: read_only\n",
                "  Repository: /workspace/product\n",
                "  Config target: /home/user/.codex/config.toml\n",
                "  Runtime home: /runtime\n\n",
                "Summary\n",
                "  Status: action_required\n",
                "  Checks: 1 passed, 0 pending, 0 failed\n\n",
                "Checks\n",
                "  [pass] Connection mode transition\n",
                "    Connection mode transition was applied\n",
                "    Previous mode: workflow\n",
                "    Current mode: read_only\n",
                "    Transition: changed\n",
                "    Previous revision: revision_before\n",
                "    Current revision: revision_after\n",
                "    Rebound Guard Installation IDs: guard_1\n\n",
                "Actions\n",
                "  reload_host\n",
                "    Restart or reload Codex, then use the current Volicord integration so new runtime and Guard observations bind revision revision_after\n\n",
                "Result\n",
                "  Changed: yes\n",
                "  Previous mode: workflow\n",
                "  Current mode: read_only\n",
                "  Previous revision: revision_before\n",
                "  Current revision: revision_after\n",
                "  Rebound Guard Installation IDs\n",
                "    guard_1\n\n",
                "Assurance\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );

        let removal = ConnectionCommandReport::removal_dry_run(
            Path::new("/runtime"),
            connection("workflow"),
            vec![PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ConnectionMembership,
                PlannedChangeOperation::Remove,
                "/workspace/product",
            )],
        )
        .unwrap();
        assert_eq!(
            rendered(&removal),
            concat!(
                "Connection removal is ready to review.\n\n",
                "Connection\n",
                "  ID: connection_1\n",
                "  Host: codex\n",
                "  Scope: user\n",
                "  Profile: record\n",
                "  Mode: workflow\n",
                "  Repository: /workspace/product\n",
                "  Config target: /home/user/.codex/config.toml\n",
                "  Runtime home: /runtime\n\n",
                "Summary\n",
                "  Status: action_required\n",
                "  Dry run: yes\n",
                "  Checks: 0 passed, 1 pending, 0 failed\n\n",
                "Checks\n",
                "  [wait] Connection removal\n",
                "    Selected Connection membership removal is ready to apply\n",
                "    Code: connection_removal_planned\n",
                "    Membership: planned for removal\n",
                "    Connection: retained until changes are applied\n\n",
                "Actions\n",
                "  apply_removal\n",
                "    Run connection remove without --dry-run to apply the planned removal\n\n",
                "Planned changes\n",
                "  Change 1\n",
                "    Kind: connection_membership\n",
                "    Operation: remove\n",
                "    Target: /workspace/product\n\n",
                "Assurance\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );
    }

    fn mcp_details(status: &str, diagnostic: &str, tools: Vec<String>) -> Value {
        json!({
            "preflight": {
                "status": "passed",
                "code": "mcp_server_preflight_passed",
                "diagnostic": "volicord mcp preflight passed",
                "storage": {
                    "storage_read": "passed",
                    "storage_write": "passed",
                    "effective_tool_mode": "workflow",
                },
            },
            "self_test": {
                "status": status,
                "code": if status == "passed" { "mcp_server_ready" } else { "mcp_server_tools_list_failed" },
                "diagnostic": diagnostic,
                "initialize": status == "passed",
                "tools_list": tools,
                "safe_read_only_tool": LIST_PROJECTS_TOOL_NAME,
            },
        })
    }

    #[test]
    fn focused_mcp_guard_host_and_trust_details_are_human_readable() {
        let tools = (0..13)
            .map(|index| format!("private.tool_{index}"))
            .collect::<Vec<_>>();
        let mcp = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Complete,
            "workflow",
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
                None,
                "Volicord MCP server self-test passed",
                Some(mcp_details("passed", "all stages passed", tools.clone())),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&mcp);
        let machine = serde_json::to_value(&mcp).unwrap();
        assert_eq!(
            machine["checks"][0]["details"]["self_test"]["tools_list"]
                .as_array()
                .map(Vec::len),
            Some(13)
        );
        for expected in [
            "    Preflight: passed\n",
            "    Storage: read passed, write passed\n",
            "    Effective mode: workflow\n",
            "    Initialize: passed\n",
            "    Required tools: passed\n",
            "    Tools returned: 13\n",
            "    Designated read-only tool: volicord.list_projects\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}");
        }
        for tool in tools {
            assert!(!output.contains(&tool), "successful tool inventory leaked");
        }

        let protocol_failure = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Failed,
                Some("mcp_server_protocol_failed"),
                "Volicord MCP server self-test failed",
                Some(mcp_details(
                    "failed",
                    r#"MCP initialize returned error: {"code":-32000,"data":{"phase":"initialize"}}"#,
                    Vec::new(),
                )),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let protocol_output = rendered(&protocol_failure);
        assert!(protocol_output.contains(concat!(
            "    Self-test diagnostic: MCP initialize returned error\n",
            "      Response details\n",
            "        Code: -32000\n",
            "        Data\n",
            "          Phase: initialize\n",
        )));
        assert!(!protocol_output.contains(r#"{"code"#));

        let guards = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![
                check(
                    ConnectionCheckKind::GuardFiles,
                    ConnectionCheckStatus::Failed,
                    Some("guard_files_failed"),
                    "Guard files do not match",
                    Some(json!({
                        "installation_ids": ["guard_1"],
                        "affected_paths": [".codex/hooks.json"],
                        "artifact_issues": [{
                            "artifact": "host_hooks_config",
                            "path": ".codex/hooks.json",
                            "issue": "content_mismatch",
                            "details": "expected current managed content",
                        }],
                        "manifest_issues": ["ownership_mismatch"],
                        "missing_required_phases": ["prompt_capture", "pre_tool"],
                    })),
                    None,
                ),
                check(
                    ConnectionCheckKind::GuardObservation,
                    ConnectionCheckStatus::Failed,
                    Some("guard_observation_failed"),
                    "Guard event is incompatible",
                    Some(json!({
                        "required_phases": ["prompt_capture", "post_tool", "pre_tool"],
                        "observed_phases": ["post_tool", "pre_tool"],
                        "missing_required_phases": ["prompt_capture"],
                        "incompatible_event_ids": ["event_1"],
                        "prompt_capture": {
                            "configured": true,
                            "host_supported": false,
                            "observed": false,
                        },
                        "last_current_observation_at": "2026-07-20T01:00:00Z",
                    })),
                    Some("2026-07-20T01:00:00Z"),
                ),
            ],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&guards);
        for expected in [
            "    Affected paths\n      .codex/hooks.json\n",
            "    Artifact issues\n      1\n        Artifact: host_hooks_config\n",
            "        Issue: content_mismatch\n",
            "    Manifest issues: ownership_mismatch\n",
            "    Missing required phases: pre_tool, prompt_capture\n",
            "    Required phases: pre_tool, post_tool, prompt_capture\n",
            "    Prompt capture: configured, not supported, not observed\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}");
        }
        for forbidden in ["Details: {", "artifact_issues\":", "\":[", "[]"] {
            assert!(!output.contains(forbidden), "raw JSON leaked: {forbidden}");
        }

        let host = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::HostSession,
                ConnectionCheckStatus::Failed,
                Some("host_session_initialize_failed"),
                "Codex initialize failed",
                Some(json!({
                    "current_integration_revision": "revision_current",
                    "observed_integration_revision": "revision_observed",
                    "current_host_version": "2.0",
                    "observed_host_version": "1.0",
                    "runtime_session_id": "session_1",
                    "last_observed_at": "2026-07-20T02:00:00Z",
                    "terminal_failure_code": "protocol_failure",
                    "terminal_failure_details": "initialize response was incompatible",
                })),
                Some("2026-07-20T02:00:00Z"),
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&host);
        assert!(!output.contains("    Previous revision:"));
        for expected in [
            "    Current revision: revision_current\n",
            "    Observed revision: revision_observed\n",
            "    Initialize: failed\n",
            "    Terminal failure code: protocol_failure\n",
            "    Terminal failure: initialize response was incompatible\n",
            "    Additional details\n      Runtime session id: session_1\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}");
        }

        let trust = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Complete,
            "workflow",
            vec![check(
                ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::Passed,
                None,
                "No separate project trust action applies to this connection scope",
                Some(json!({"applicable": false})),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let trust_block = rendered(&trust);
        assert_eq!(
            trust_block
                .matches("No separate project trust action")
                .count(),
            1
        );
        assert!(!trust_block.contains("Applicable:"));
    }

    #[test]
    fn every_check_kind_has_one_exhaustive_human_label() {
        let expected = [
            (ConnectionCheckKind::ConnectionRemoval, "Connection removal"),
            (ConnectionCheckKind::GuardFiles, "Guard managed files"),
            (ConnectionCheckKind::GuardObservation, "Guard hook activity"),
            (ConnectionCheckKind::HostExecutable, "Codex executable"),
            (ConnectionCheckKind::HostSession, "Codex managed session"),
            (
                ConnectionCheckKind::ManagedConfig,
                "Managed Codex configuration",
            ),
            (ConnectionCheckKind::McpServer, "Volicord MCP server"),
            (
                ConnectionCheckKind::ModeTransition,
                "Connection mode transition",
            ),
            (ConnectionCheckKind::ProjectTrust, "Project trust"),
            (ConnectionCheckKind::RequiredTools, "Codex required tools"),
            (ConnectionCheckKind::SetupPlan, "Setup plan"),
            (
                ConnectionCheckKind::ToolRoundTrip,
                "Read-only tool round trip",
            ),
            (
                ConnectionCheckKind::VerificationNotRun,
                "Connection verification",
            ),
        ];
        assert_eq!(expected.len(), ConnectionCheckKind::ALL.len());
        for (kind, label) in expected {
            assert_eq!(check_label(kind), label);
        }
    }

    #[test]
    fn remaining_focused_check_details_use_named_human_facts() {
        let report = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![
                check(
                    ConnectionCheckKind::HostExecutable,
                    ConnectionCheckStatus::Failed,
                    Some("host_executable_probe_failed"),
                    "Codex executable version probe failed",
                    Some(json!({
                        "version": "1.2.3",
                        "path": "/opt/codex/bin/codex",
                        "diagnostic": "process exited with status 1",
                    })),
                    None,
                ),
                check(
                    ConnectionCheckKind::RequiredTools,
                    ConnectionCheckStatus::Failed,
                    Some("required_tools_missing"),
                    "Current managed host is missing required tools",
                    Some(json!({
                        "current_integration_revision": "revision_current",
                        "observed_integration_revision": "revision_observed",
                        "tools_list_observed": true,
                        "required_tools_present": false,
                        "missing_tools": ["volicord.close_task"],
                        "last_observed_at": "2026-07-20T03:00:00Z",
                        "terminal_failure_code": null,
                        "terminal_failure_details": null,
                    })),
                    Some("2026-07-20T03:00:00Z"),
                ),
                check(
                    ConnectionCheckKind::SetupPlan,
                    ConnectionCheckStatus::Pending,
                    Some("setup_changes_planned"),
                    "Setup changes are ready to apply",
                    None,
                    None,
                ),
                check(
                    ConnectionCheckKind::ToolRoundTrip,
                    ConnectionCheckStatus::Failed,
                    Some("tool_round_trip_failed"),
                    "Read-only tool call failed",
                    Some(json!({
                        "current_integration_revision": "revision_current",
                        "observed_integration_revision": "revision_observed",
                        "safe_read_only_tool": "volicord.list_projects",
                        "call_completed": false,
                        "last_observed_at": "2026-07-20T04:00:00Z",
                        "terminal_failure_code": "tool_contract_mismatch",
                        "terminal_failure_details": "response shape was incompatible",
                    })),
                    Some("2026-07-20T04:00:00Z"),
                ),
                check(
                    ConnectionCheckKind::VerificationNotRun,
                    ConnectionCheckStatus::Pending,
                    Some("verification_not_run"),
                    "Connection verification has not been run",
                    None,
                    None,
                ),
            ],
            Vec::new(),
            None,
            Some(vec![
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::GuardManagedFile,
                    PlannedChangeOperation::Create,
                    ".codex/hooks.json",
                ),
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::GuardManagedFile,
                    PlannedChangeOperation::Update,
                    "AGENTS.md",
                ),
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::ManagedHostConfiguration,
                    PlannedChangeOperation::Update,
                    "/home/user/.codex/config.toml",
                ),
            ]),
        );
        let output = rendered(&report);
        for expected in [
            "  [fail] Codex executable\n",
            "    Version: 1.2.3\n",
            "    Path: /opt/codex/bin/codex\n",
            "    Probe diagnostic: process exited with status 1\n",
            "  [fail] Codex required tools\n",
            "    Tools/list observed: yes\n",
            "    Required tools: failed\n",
            "    Missing tools: volicord.close_task\n",
            "  [wait] Setup plan\n",
            "    Planned state: changes ready to apply\n",
            "    guard_managed_file: 2\n",
            "    managed_host_configuration: 1\n",
            "  [fail] Read-only tool round trip\n",
            "    Designated read-only tool: volicord.list_projects\n",
            "    Call completed: no\n",
            "    Terminal failure code: tool_contract_mismatch\n",
            "    Terminal failure: response shape was incompatible\n",
            "  [wait] Connection verification\n",
            "    Connection verification has not been run\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        assert!(!output.contains("Details:"));
    }

    #[test]
    fn summary_uses_canonical_status_without_deriving_it_from_results() {
        let report = report(
            CommandOperation::Mode,
            false,
            ConnectionStatus::Failed,
            "read_only",
            vec![check(
                ConnectionCheckKind::ModeTransition,
                ConnectionCheckStatus::Passed,
                None,
                "Mode transition was applied",
                None,
                None,
            )],
            Vec::new(),
            Some(ConnectionCommandResult::ModeTransition {
                changed: true,
                previous_mode: "workflow".to_owned(),
                current_mode: "read_only".to_owned(),
                previous_integration_revision: "before".to_owned(),
                current_integration_revision: "after".to_owned(),
                rebound_guard_installation_ids: Vec::new(),
            }),
            None,
        );
        let output = rendered(&report);
        assert!(output.contains("Summary\n  Status: failed\n"));
        assert!(output.contains("Result\n  Changed: yes\n"));
    }

    #[test]
    fn unknown_details_render_recursively_with_bounds_and_do_not_mutate_json() {
        let report_value = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::SetupPlan,
                ConnectionCheckStatus::Failed,
                Some("setup_partial_application"),
                "Setup migration could not be completed",
                Some(json!({
                    "failure": "registry conflict",
                    "nested": {
                        "alpha": 1,
                        "items": [
                            {"kind": "first", "value": true},
                            {"kind": "second", "value": false}
                        ],
                        "nothing": null,
                    },
                    "retry_arguments": ["volicord", "init", "--home", "/runtime"],
                    "long_values": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                    "empty_array": [],
                    "empty_object": {},
                })),
                None,
            )],
            Vec::new(),
            Some(ConnectionCommandResult::Setup { applied: false }),
            None,
        );
        let json_before = serde_json::to_string_pretty(&report_value).unwrap();
        let output = rendered(&report_value);
        let json_after = serde_json::to_string_pretty(&report_value).unwrap();
        assert_eq!(json_after, json_before);
        for expected in [
            "    Additional details\n",
            "      Failure: registry conflict\n",
            "      Long values: 10 values: 1, 2, 3, 4, 5, 6, 7, 8; 2 more\n",
            "      Nested\n",
            "        Alpha: 1\n",
            "        Items\n",
            "          1\n",
            "            Kind: first\n",
            "            Value: yes\n",
            "      Retry arguments: volicord, init, --home, /runtime\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        for omitted in [
            "Empty array",
            "Empty object",
            "Nothing",
            "Details: {",
            "\":[",
        ] {
            assert!(!output.contains(omitted), "unexpected {omitted:?}");
        }

        let mut nested = json!("deep");
        for index in 0..12 {
            nested = json!({(format!("level_{index}")): nested});
        }
        let deep = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::SetupPlan,
                ConnectionCheckStatus::Failed,
                Some("setup_partial_application"),
                "Deep details",
                Some(json!({"future": nested})),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        assert!(rendered(&deep).contains("[nested details omitted at depth limit]"));
    }
}
