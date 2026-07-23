use std::collections::BTreeSet;

use serde_json::{Map, Value};
use volicord_types::{
    ConnectionCheck, ConnectionCheckKind, ConnectionCheckStatus, DiagnosticFindingId,
    DiagnosticReportAction,
};

use crate::connection_command::managed_host_round_trip_tool;

use super::{
    human::{headline, CheckCounts},
    report::{
        projected_actions, projected_root_cause_ids, ConnectionCommandReport,
        ConnectionCommandResult,
    },
    ConnectionCommandError, PlannedConnectionChangeKind,
};

const MAX_DETAIL_RENDER_DEPTH: usize = 8;
const MAX_INLINE_SCALARS: usize = 8;

pub(super) fn render_command_report_verbose(
    report: &ConnectionCommandReport,
) -> Result<String, ConnectionCommandError> {
    let counts = CheckCounts::from_report(report);
    let roots = projected_root_cause_ids(report)?;
    let actions = projected_actions(report)?;
    let mut sections = vec![headline(report, counts), render_connection(report)?];
    sections.push(render_summary(report, counts));

    if !report.checks.is_empty() {
        sections.push(render_checks(report));
    }
    if !report.findings.is_empty() {
        sections.push(render_findings(report, &roots));
    }
    if !actions.is_empty() {
        sections.push(render_actions(&actions));
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

    Ok(format!("{}\n", sections.join("\n\n")))
}

fn render_connection(report: &ConnectionCommandReport) -> Result<String, ConnectionCommandError> {
    let mut output = format!(
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
    );
    if let Some(revision) = report.integration_revision.as_ref() {
        output.push_str(&format!("\n  Integration revision: {}", revision.as_str()));
    }
    let runtime_sessions = report.role_bearing_runtime_sessions()?;
    let role_ids = runtime_sessions
        .iter()
        .map(|session| session.id().as_str())
        .collect::<BTreeSet<_>>();
    let related_sessions = report
        .findings
        .iter()
        .filter_map(|finding| finding.runtime_session_id())
        .filter(|session_id| !role_ids.contains(session_id.as_str()))
        .map(|session_id| session_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if !runtime_sessions.is_empty() || !related_sessions.is_empty() {
        let mut rendered_sessions = runtime_sessions
            .iter()
            .map(|session| {
                format!(
                    "{} ({})",
                    session.id().as_str(),
                    session
                        .roles()
                        .iter()
                        .map(|role| role.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>();
        rendered_sessions.extend(related_sessions);
        output.push_str(&format!(
            "\n  Runtime sessions: {}",
            rendered_sessions.join(", ")
        ));
    }
    Ok(output)
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
        "  Checks: {} passed, {} blocked, {} pending, {} failed, {} not applicable",
        counts.ready, counts.blocked, counts.waiting, counts.failed, counts.not_applicable
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
    if !check.depends_on().is_empty() {
        let dependencies = check
            .depends_on()
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("    Depends on: {}", dependencies));
        if check.status() == ConnectionCheckStatus::Blocked {
            lines.push(format!("    Blocked by: {dependencies}"));
        }
    }
    if !check.cause_finding_ids().is_empty() {
        lines.push(format!(
            "    Root findings: {}",
            check
                .cause_finding_ids()
                .iter()
                .map(|finding_id| finding_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
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
        ConnectionCheckStatus::Blocked => "blocked",
        ConnectionCheckStatus::NotApplicable => "n/a",
    }
}

fn check_label(kind: ConnectionCheckKind) -> &'static str {
    match kind {
        ConnectionCheckKind::DiagnosticLookup => "Diagnostic finding lookup",
        ConnectionCheckKind::VerificationNotRun => "Connection verification",
        ConnectionCheckKind::ManagedConfig => "Managed Codex configuration",
        ConnectionCheckKind::HostExecutable => "Codex executable",
        ConnectionCheckKind::McpServer => "Volicord MCP server",
        ConnectionCheckKind::ProcessStartup => "Managed MCP process startup",
        ConnectionCheckKind::HostSession => "Codex managed session",
        ConnectionCheckKind::RequiredTools => "Codex required tools",
        ConnectionCheckKind::ToolRoundTrip => "Read-only tool round trip",
        ConnectionCheckKind::ProjectTrust => "Project trust",
        ConnectionCheckKind::GuardFiles => "Guard managed files",
        ConnectionCheckKind::GuardHookExecution => "Guard hook execution",
        ConnectionCheckKind::GuardObservation => "Guard hook activity",
        ConnectionCheckKind::GuardVerification => "Guard integration verification",
        ConnectionCheckKind::SetupPlan => "Setup plan",
        ConnectionCheckKind::ModeTransition => "Connection mode transition",
        ConnectionCheckKind::ConnectionRemoval => "Connection removal",
        ConnectionCheckKind::RuntimeSessionLookup => "Runtime-session lookup",
    }
}

struct DetailContext<'a> {
    report: &'a ConnectionCommandReport,
    check: &'a ConnectionCheck,
    object: Option<&'a Map<String, Value>>,
    consumed: BTreeSet<DetailPath>,
    lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DetailPathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct DetailPath(Vec<DetailPathSegment>);

impl DetailPath {
    fn from_dotted_keys(path: &str) -> Self {
        Self(
            path.split('.')
                .map(|key| DetailPathSegment::Key(key.to_owned()))
                .collect(),
        )
    }

    fn key(&self, key: &str) -> Self {
        let mut path = self.clone();
        path.0.push(DetailPathSegment::Key(key.to_owned()));
        path
    }

    fn index(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.0.push(DetailPathSegment::Index(index));
        path
    }
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

    fn peek(&self, path: &DetailPath) -> Option<&'a Value> {
        value_at_path(self.object?, path)
    }

    fn take_string(&mut self, path: &str) -> Option<String> {
        self.take_string_at(&DetailPath::from_dotted_keys(path))
    }

    fn take_string_at(&mut self, path: &DetailPath) -> Option<String> {
        let value = self.peek(path)?.as_str()?.to_owned();
        self.consume(path);
        Some(value)
    }

    fn take_bool(&mut self, path: &str) -> Option<bool> {
        let path = DetailPath::from_dotted_keys(path);
        let value = self.peek(&path)?.as_bool()?;
        self.consume(&path);
        Some(value)
    }

    fn take_string_array(&mut self, path: &str) -> Option<Vec<String>> {
        let path = DetailPath::from_dotted_keys(path);
        let values = self.peek(&path)?.as_array()?;
        let strings = values
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .map(str::to_owned)
            .collect();
        self.consume(&path);
        Some(strings)
    }

    fn take_value(&mut self, path: &str) -> Option<Value> {
        let path = DetailPath::from_dotted_keys(path);
        let value = self.peek(&path)?.clone();
        self.consume(&path);
        Some(value)
    }

    fn consume(&mut self, path: &DetailPath) {
        self.consumed.insert(path.clone());
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
            &DetailPath::default(),
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
        let root = DetailPath::default();
        if !has_renderable_object(object, &root, &self.consumed, 0) {
            return;
        }
        self.lines.push("    Additional details".to_owned());
        render_generic_object(object, &root, &self.consumed, 6, 0, &mut self.lines);
    }
}

fn value_at_path<'a>(object: &'a Map<String, Value>, path: &DetailPath) -> Option<&'a Value> {
    let mut segments = path.0.iter();
    let DetailPathSegment::Key(first) = segments.next()? else {
        return None;
    };
    let mut value = object.get(first)?;
    for segment in segments {
        value = match segment {
            DetailPathSegment::Key(key) => value.as_object()?.get(key)?,
            DetailPathSegment::Index(index) => value.as_array()?.get(*index)?,
        };
    }
    Some(value)
}

fn render_known_details(context: &mut DetailContext<'_>) {
    match context.check.id() {
        ConnectionCheckKind::DiagnosticLookup | ConnectionCheckKind::RuntimeSessionLookup => {}
        ConnectionCheckKind::VerificationNotRun => {}
        ConnectionCheckKind::ManagedConfig => render_managed_config(context),
        ConnectionCheckKind::HostExecutable => render_host_executable(context),
        ConnectionCheckKind::McpServer => render_mcp_server(context),
        ConnectionCheckKind::ProcessStartup => render_process_startup(context),
        ConnectionCheckKind::HostSession => render_host_session(context),
        ConnectionCheckKind::RequiredTools => render_required_tools(context),
        ConnectionCheckKind::ToolRoundTrip => render_tool_round_trip(context),
        ConnectionCheckKind::ProjectTrust => render_project_trust(context),
        ConnectionCheckKind::GuardFiles => render_guard_files(context),
        ConnectionCheckKind::GuardHookExecution => render_guard_observation(context),
        ConnectionCheckKind::GuardObservation => render_guard_observation(context),
        ConnectionCheckKind::GuardVerification => render_guard_verification(context),
        ConnectionCheckKind::SetupPlan => render_setup_plan(context),
        ConnectionCheckKind::ModeTransition => render_mode_transition(context),
        ConnectionCheckKind::ConnectionRemoval => render_connection_removal(context),
    }
}

fn render_guard_verification(context: &mut DetailContext<'_>) {
    if let Some(id) = context.take_string("verification_id") {
        context.line("Verification ID", id);
    }
    if let Some(status) = context.take_string("verification_status") {
        context.line("Verification status", status);
    }
    if let Some(runtime) = context.take_string("runtime_session_id") {
        context.line("Runtime session", runtime);
    }
    if let Some(turn) = context.take_string("host_turn_id") {
        context.line("Host turn", turn);
    }
}

fn render_managed_config(context: &mut DetailContext<'_>) {
    if let Some(target) = context.take_string("target") {
        context.line("Target", target);
    }
    if let Some(state) = context.take_string("observed_state") {
        context.line("State", state);
    }
    if let Some(code) = context.take_string("diagnostic_code") {
        context.line("Diagnostic code", code);
    }
    if let Some(diagnostic) = context.take_string("diagnostic") {
        if diagnostic_adds_information(&diagnostic, context.check.summary()) {
            context.diagnostic("Diagnostic", &diagnostic);
        }
    }
}

fn render_host_executable(context: &mut DetailContext<'_>) {
    let _status = context.take_string("status");
    if let Some(version) = context.take_string("probe.version") {
        context.line("Version", version);
    }
    if let Some(path) = context.take_string("probe.discovered_path") {
        context.line("Path", path);
    }
    if let Some(diagnostic) = context.take_string("diagnostic") {
        if diagnostic_adds_information(&diagnostic, context.check.summary()) {
            context.diagnostic("Probe diagnostic", &diagnostic);
        }
    }
}

fn render_process_startup(context: &mut DetailContext<'_>) {
    render_runtime_evidence_identity(context);
    render_revision_pair(context);
    if let Some(started_at) = context.take_string("process_started_at") {
        context.line("Process started at", started_at);
    }
    render_managed_peer_and_probe(context);
    render_terminal_finding(context);
    render_last_observed(context);
}

fn render_mcp_server(context: &mut DetailContext<'_>) {
    let preflight = context.take_string("preflight.status");
    let preflight_code = context.take_string("preflight.code");
    let preflight_diagnostic = context.take_string("preflight.diagnostic");
    let preflight_finding_id = context.take_string("preflight.finding_id");
    let preflight_diagnostic_code = context.take_string("preflight.diagnostic_code");
    let _preflight_failure_stage = context.take_string("preflight.failure_stage");
    if let Some(status) = preflight.as_deref() {
        context.line("Preflight", status);
    }
    if let Some(storage_read) = context.take_string("preflight.storage.storage_read") {
        let storage_write = context.take_string("preflight.storage.storage_write");
        match storage_write {
            Some(write) => context.line(
                "Storage",
                format_args!("read {storage_read}, write {write}"),
            ),
            None => context.line("Storage read", storage_read),
        }
    } else if let Some(storage_write) = context.take_string("preflight.storage.storage_write") {
        context.line("Storage write", storage_write);
    }
    if let Some(mode) = context.take_string("preflight.storage.effective_tool_mode") {
        context.line("Effective mode", mode);
    }
    if let Some(code) = preflight_diagnostic_code {
        context.line("Preflight diagnostic code", code);
    }
    if let Some(finding_id) = preflight_finding_id {
        context.line("Preflight finding", finding_id);
    }

    let self_test_status = context.take_string("self_test.status");
    let _self_test_code = context.take_string("self_test.code");
    let diagnostic = context.take_string("self_test.diagnostic");
    let diagnostic_code = context.take_string("self_test.diagnostic_code");
    let finding_id = context.take_string("self_test.finding_id");
    let failure_stage = context.take_string("self_test.failure_stage");
    let production_revisions = context
        .take_string_array("self_test.production_supported_revisions")
        .unwrap_or_default();
    if !production_revisions.is_empty() {
        let conformance = context
            .take_value("self_test.conformance")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let host_profiles = context
            .take_string_array("self_test.host_compatibility_profiles")
            .unwrap_or_default();
        let host_compatibility = context
            .take_value("self_test.host_compatibility")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        let safe_tool = context
            .take_string("self_test.safe_read_only_tool")
            .unwrap_or_else(|| managed_host_round_trip_tool().wire_name().to_owned());
        let tools = context
            .take_string_array("self_test.tools_list")
            .unwrap_or_default();
        render_mcp_probe_matrix(
            context,
            &production_revisions,
            &conformance,
            &host_profiles,
            &host_compatibility,
            &safe_tool,
            &tools,
        );
        if let Some(code) = diagnostic_code {
            context.line("Self-test diagnostic code", code);
        }
        if let Some(finding_id) = finding_id {
            context.line("Self-test finding", finding_id);
        }
        if self_test_status.as_deref() != Some("passed")
            && diagnostic.as_deref().is_some_and(|diagnostic| {
                diagnostic_adds_information(diagnostic, context.check.summary())
            })
        {
            context.diagnostic(
                "Self-test diagnostic",
                diagnostic.as_deref().expect("diagnostic was checked"),
            );
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
        return;
    }
    let initialize = context.take_bool("self_test.initialize");
    let tools_list_observed = context.take_bool("self_test.tools_list_observed");
    let tools = context
        .take_string_array("self_test.tools_list")
        .unwrap_or_default();
    let required_tools_validated = context.take_bool("self_test.required_tools_validated");
    let safe_tool = context
        .take_string("self_test.safe_read_only_tool")
        .unwrap_or_else(|| managed_host_round_trip_tool().wire_name().to_owned());
    let safe_tool_completed = context.take_bool("self_test.safe_read_only_tool_completed");
    let shutdown_completed = context.take_bool("self_test.shutdown_completed");
    let preflight_passed = preflight.as_deref() == Some("passed");
    let self_test_passed = self_test_status.as_deref() == Some("passed");

    context.line(
        "Initialize",
        mcp_initialize_result(preflight_passed, initialize, failure_stage.as_deref()),
    );
    context.line(
        "Required tools",
        mcp_required_tools_result(
            preflight_passed,
            tools_list_observed,
            required_tools_validated,
        ),
    );
    if tools_list_observed == Some(true) {
        context.line("Tools returned", tools.len());
    }
    let safe_result = mcp_safe_tool_result(
        preflight_passed,
        safe_tool_completed,
        failure_stage.as_deref(),
    );
    if safe_result == "passed" {
        context.line("Designated read-only tool", safe_tool);
    } else {
        context.line(
            "Designated read-only tool",
            format_args!("{safe_tool} ({safe_result})"),
        );
    }
    context.line(
        "Shutdown",
        mcp_shutdown_result(
            preflight_passed,
            shutdown_completed,
            failure_stage.as_deref(),
        ),
    );

    if let Some(code) = diagnostic_code {
        context.line("Self-test diagnostic code", code);
    }
    if let Some(finding_id) = finding_id {
        context.line("Self-test finding", finding_id);
    }
    if failure_stage.is_none()
        && !self_test_passed
        && diagnostic.as_deref().is_some_and(|diagnostic| {
            diagnostic_adds_information(diagnostic, context.check.summary())
        })
    {
        context.diagnostic(
            "Self-test diagnostic",
            diagnostic.as_deref().expect("diagnostic was checked"),
        );
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

fn render_mcp_probe_matrix(
    context: &mut DetailContext<'_>,
    production_revisions: &[String],
    conformance: &[Value],
    host_profiles: &[String],
    host_compatibility: &[Value],
    safe_tool: &str,
    tools: &[String],
) {
    context.line(
        "Production revisions",
        render_string_values(production_revisions),
    );
    let passed = conformance
        .iter()
        .filter(|probe| probe["status"] == "passed")
        .count();
    context.line(
        "Server conformance",
        format_args!("{passed}/{} passed", conformance.len()),
    );
    for probe in conformance {
        let revision = probe["revision"].as_str().unwrap_or("unknown");
        let status = probe["status"].as_str().unwrap_or("unknown");
        let negotiated = probe["negotiated_revision"]
            .as_str()
            .unwrap_or("not negotiated");
        let tools = probe["tools_returned"].as_u64().unwrap_or(0);
        let shutdown = if probe["shutdown_completed"] == true {
            "graceful"
        } else {
            "incomplete"
        };
        context.line(
            &format!("Revision {revision}"),
            format_args!("{status}; negotiated {negotiated}; {tools} tools; {shutdown} shutdown"),
        );
        if let Some(stage) = probe["failure_stage"].as_str() {
            context.line(
                "Revision failure",
                format_args!("{revision} during {stage}"),
            );
        }
        if let Some(code) = probe["diagnostic_code"].as_str() {
            context.line("Revision diagnostic code", code);
        }
    }
    context.line("Tools returned", tools.len());
    context.line("Designated read-only tool", safe_tool);
    context.line(
        "Host compatibility profiles",
        render_string_values(host_profiles),
    );
    for probe in host_compatibility {
        let profile = probe["profile"].as_str().unwrap_or("unknown");
        let fixture = probe["fixture"].as_str().unwrap_or("unknown");
        let status = probe["status"].as_str().unwrap_or("unknown");
        let negotiated = probe["negotiated_revision"]
            .as_str()
            .unwrap_or("not negotiated");
        context.line(
            &format!("Host profile {profile}"),
            format_args!("{status}; {fixture}; negotiated {negotiated}"),
        );
    }
}

fn mcp_initialize_result(
    preflight_passed: bool,
    initialize: Option<bool>,
    failure_stage: Option<&str>,
) -> &'static str {
    if initialize == Some(true) {
        "passed"
    } else if !preflight_passed {
        "not run"
    } else if matches!(failure_stage, Some("startup" | "initialize")) {
        "failed"
    } else {
        "not completed"
    }
}

fn mcp_required_tools_result(
    preflight_passed: bool,
    tools_list_observed: Option<bool>,
    required_tools_validated: Option<bool>,
) -> &'static str {
    if required_tools_validated == Some(true) {
        "passed"
    } else if tools_list_observed == Some(true) {
        "failed"
    } else if preflight_passed {
        "not completed"
    } else {
        "not run"
    }
}

fn mcp_safe_tool_result(
    preflight_passed: bool,
    safe_tool_completed: Option<bool>,
    failure_stage: Option<&str>,
) -> &'static str {
    if safe_tool_completed == Some(true) {
        "passed"
    } else if failure_stage == Some("safe_tool_call") {
        "failed"
    } else if preflight_passed {
        "not completed"
    } else {
        "not run"
    }
}

fn mcp_shutdown_result(
    preflight_passed: bool,
    shutdown_completed: Option<bool>,
    failure_stage: Option<&str>,
) -> &'static str {
    if shutdown_completed == Some(true) {
        "passed"
    } else if failure_stage == Some("shutdown") {
        "failed"
    } else if preflight_passed {
        "not completed"
    } else {
        "not run"
    }
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
    render_runtime_evidence_identity(context);
    render_revision_pair(context);
    render_managed_peer_and_probe(context);
    context.line("Initialize", host_initialize_result(context.check));
    render_terminal_finding(context);
    render_last_observed(context);
}

fn host_initialize_result(check: &ConnectionCheck) -> &'static str {
    match (check.status(), check.code().unwrap_or_default()) {
        (ConnectionCheckStatus::Passed, _) => "completed",
        (ConnectionCheckStatus::Failed, _) => "failed",
        (ConnectionCheckStatus::Blocked, _) => "blocked",
        (ConnectionCheckStatus::NotApplicable, _) => "not applicable",
        (_, "host_session_not_observed" | "host_session_revision_stale") => "not observed",
        _ => "pending",
    }
}

fn render_required_tools(context: &mut DetailContext<'_>) {
    render_runtime_evidence_identity(context);
    render_revision_pair(context);
    render_managed_peer_and_probe(context);
    let tools_observed_at = context.take_string("required_tools.tools_list_observed_at");
    if let Some(observed_at) = tools_observed_at {
        context.line("Tools/list observed at", observed_at);
    }
    let returned_tools = context
        .take_string_array("required_tools.returned_tool_identities")
        .unwrap_or_default();
    if !returned_tools.is_empty() {
        context.line("Returned tools", returned_tools.len());
    }
    let explicit_result = context.take_bool("required_tools.required_tools_present");
    let result = match (context.check.status(), explicit_result) {
        (ConnectionCheckStatus::Blocked, _) => "blocked",
        (ConnectionCheckStatus::NotApplicable, _) => "not applicable",
        (_, Some(true)) | (ConnectionCheckStatus::Passed, _) => "passed",
        (_, Some(false)) | (ConnectionCheckStatus::Failed, _) => "failed",
        _ => "pending",
    };
    context.line("Required tools", result);
    if let Some(validated_at) = context.take_string("required_tools.required_tools_validated_at") {
        context.line("Required tools validated at", validated_at);
    }
    render_terminal_finding(context);
    render_last_observed(context);
}

fn render_tool_round_trip(context: &mut DetailContext<'_>) {
    render_runtime_evidence_identity(context);
    render_revision_pair(context);
    render_managed_peer_and_probe(context);
    let expected_tool = context
        .take_string("verification_tool.expected_tool_identity")
        .unwrap_or_else(|| managed_host_round_trip_tool().wire_name().to_owned());
    context.line("Expected verification tool", expected_tool);
    let observed_tool = context.take_string("verification_tool.observed_tool_identity");
    if let Some(observed_tool) = observed_tool.as_deref() {
        context.line("Observed verification tool", observed_tool);
    }
    if let Some(observed_at) = context.take_string("verification_tool.observed_at") {
        context.line("Verification tool observed at", observed_at);
    }
    if observed_tool.is_some() || context.check.status() == ConnectionCheckStatus::Passed {
        context.line("Call completed", "yes");
    }
    render_terminal_finding(context);
    render_last_observed(context);
}

fn render_runtime_evidence_identity(context: &mut DetailContext<'_>) {
    if let Some(role) = context.take_string("evidence_role") {
        context.line("Evidence role", role);
    }
    if let Some(runtime_session_id) = context.take_string("runtime_session_id") {
        context.line("Runtime session", runtime_session_id);
    }
    if let Some(source) = context.take_string("source") {
        context.line("Session source", source);
    }
}

fn render_managed_peer_and_probe(context: &mut DetailContext<'_>) {
    if let Some(path) = context.take_string("host_executable_probe.discovered_path") {
        context.line("PATH executable", path);
    }
    if let Some(version) = context.take_string("host_executable_probe.version") {
        context.line("PATH executable version", version);
    }
    if let Some(name) = context.take_string("managed_peer.client_info.name") {
        context.line("Actual MCP peer", name);
    }
    if let Some(version) = context.take_string("managed_peer.client_info.version") {
        context.line("Actual MCP peer version", version);
    }
    if let Some(revision) = context.take_string("managed_peer.requested_protocol_revision") {
        context.line("Requested protocol", revision);
    }
    if let Some(revision) = context.take_string("managed_peer.selected_protocol_revision") {
        context.line("Selected protocol", revision);
    }
    if let Some(revision) = context.take_string("managed_peer.negotiated_protocol_revision") {
        context.line("Negotiated protocol", revision);
    }
}

fn render_revision_pair(context: &mut DetailContext<'_>) {
    if let Some(revision) = context.take_string("current_integration_revision") {
        context.line("Current revision", revision);
    }
    if let Some(revision) = context.take_string("observed_integration_revision") {
        context.line("Observed revision", revision);
    }
}

fn render_terminal_finding(context: &mut DetailContext<'_>) {
    if let Some(finding_id) = context.take_string("terminal_finding_id") {
        context.line("Terminal finding", finding_id);
    }
}

fn render_last_observed(context: &mut DetailContext<'_>) {
    if let Some(last_observed) = context.take_string("last_observed_at") {
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
    let applicable_path = DetailPath::from_dotted_keys("applicable");
    match context.take_bool("applicable") {
        Some(false) => return,
        Some(true) => context.line("Applicable", "yes"),
        None if context.peek(&applicable_path).is_none() => context.line("Applicable", "yes"),
        None => {}
    }
    if let Some(state) = context.take_string("observed_state") {
        context.line("State", state);
    }
    if let Some(target) = context.take_string("repo_root") {
        context.line("Target", target);
    }
    if let Some(path) = context.take_string("config_path") {
        context.line("Configuration", path);
    }
    if let Some(diagnostic) = context.take_string("diagnostic") {
        if diagnostic_adds_information(&diagnostic, context.check.summary()) {
            context.diagnostic("Diagnostic", &diagnostic);
        }
    }
}

fn render_guard_files(context: &mut DetailContext<'_>) {
    let installations = context
        .take_string_array("installation_ids")
        .unwrap_or_default();
    if !installations.is_empty() {
        context.line(
            "Guard Installation IDs",
            render_string_values(&installations),
        );
    }
    let affected_paths = context
        .take_string_array("affected_paths")
        .unwrap_or_default();
    render_list(&mut context.lines, "Affected paths", &affected_paths);

    render_artifact_issues(context);
    let manifest_issues = context
        .take_string_array("manifest_issues")
        .unwrap_or_default();
    if !manifest_issues.is_empty() {
        context.line("Manifest issues", render_string_values(&manifest_issues));
    }
    let mut missing_phases = context
        .take_string_array("missing_required_phases")
        .unwrap_or_default();
    sort_phases(&mut missing_phases);
    if !missing_phases.is_empty() {
        context.line(
            "Missing required phases",
            render_string_values(&missing_phases),
        );
    }
}

fn render_artifact_issues(context: &mut DetailContext<'_>) {
    let array_path = DetailPath::from_dotted_keys("artifact_issues");
    let Some(Value::Array(issues)) = context.peek(&array_path) else {
        return;
    };
    if issues.is_empty() {
        context.consume(&array_path);
        return;
    }

    let mut rendered_heading = false;
    for (index, value) in issues.iter().enumerate() {
        let Value::Object(issue) = value else {
            continue;
        };
        if !rendered_heading {
            context.lines.push("    Artifact issues".to_owned());
            rendered_heading = true;
        }

        let item_path = array_path.index(index);
        context.lines.push(format!("      {}", index + 1));
        if issue.is_empty() {
            context.lines.push("        Empty object".to_owned());
            context.consume(&item_path);
            continue;
        }

        for (key, label) in [
            ("artifact", "Artifact"),
            ("path", "Path"),
            ("issue", "Issue"),
            ("details", "Details"),
        ] {
            let field_path = item_path.key(key);
            if let Some(value) = context.take_string_at(&field_path) {
                push_labeled_multiline(&mut context.lines, 8, label, &value);
            }
        }

        if has_renderable_object(issue, &item_path, &context.consumed, 1) {
            context.lines.push("        Additional details".to_owned());
            render_generic_object(
                issue,
                &item_path,
                &context.consumed,
                10,
                1,
                &mut context.lines,
            );
        }
        context.consume(&item_path);
    }
}

fn render_guard_observation(context: &mut DetailContext<'_>) {
    for (path, label) in [
        ("required_phases", "Required phases"),
        ("observed_phases", "Observed phases"),
        ("missing_required_phases", "Missing phases"),
    ] {
        let mut phases = context.take_string_array(path).unwrap_or_default();
        sort_phases(&mut phases);
        if !phases.is_empty() {
            context.line(label, render_string_values(&phases));
        }
    }
    let incompatible = context
        .take_string_array("incompatible_event_ids")
        .unwrap_or_default();
    if !incompatible.is_empty() {
        context.line(
            "Incompatible event IDs",
            render_string_values(&incompatible),
        );
    }
    let configured = context.take_bool("prompt_capture.configured");
    let supported = context.take_bool("prompt_capture.host_supported");
    let observed = context.take_bool("prompt_capture.observed");
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
    if let Some(last_observed) = context.take_string("last_current_observation_at") {
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
        ConnectionCheckStatus::Blocked => "blocked",
        ConnectionCheckStatus::NotApplicable => "not applicable",
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

fn render_actions(actions: &[DiagnosticReportAction]) -> String {
    let mut blocks = Vec::with_capacity(actions.len());
    for action in actions {
        let mut lines = vec![format!("  {}", action.code())];
        push_multiline(&mut lines, 4, action.summary());
        if !action.root_cause_ids().is_empty() {
            lines.push(format!(
                "    Root findings: {}",
                action
                    .root_cause_ids()
                    .iter()
                    .map(|finding_id| finding_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        blocks.push(lines.join("\n"));
    }
    format!("Actions\n{}", blocks.join("\n\n"))
}

fn render_findings(report: &ConnectionCommandReport, roots: &[DiagnosticFindingId]) -> String {
    let mut blocks = Vec::with_capacity(report.findings.len());
    for finding in &report.findings {
        let role = if roots.contains(finding.id()) {
            "root"
        } else {
            "related"
        };
        let severity = match finding.severity() {
            volicord_types::DiagnosticSeverity::Info => "info",
            volicord_types::DiagnosticSeverity::Warning => "warning",
            volicord_types::DiagnosticSeverity::Error => "error",
        };
        let mut lines = vec![
            format!("  [{role}] {}", finding.id()),
            format!("    Code: {}", finding.code()),
            format!("    Domain: {}", finding.domain()),
            format!("    Stage: {}", finding.stage()),
            format!("    Severity: {severity}"),
            format!("    Source: {}", finding.source()),
            format!(
                "    Subject: {} {}",
                finding.subject().kind(),
                finding.subject().reference()
            ),
            format!(
                "    Observed at: {}",
                finding.observed_at().to_canonical_string()
            ),
        ];
        if let Some(correlation_id) = finding.correlation_id() {
            lines.push(format!("    Correlation: {correlation_id}"));
        }
        if let Some(connection_id) = finding.connection_id() {
            lines.push(format!("    Connection: {connection_id}"));
        }
        if let Some(project_id) = finding.project_id() {
            lines.push(format!("    Project: {project_id}"));
        }
        if let Some(runtime_session_id) = finding.runtime_session_id() {
            lines.push(format!("    Runtime session: {runtime_session_id}"));
        }
        if let Some(revision) = finding.integration_revision() {
            lines.push(format!("    Integration revision: {}", revision.as_str()));
        }
        if !finding.causes().is_empty() {
            lines.push(format!(
                "    Caused by: {}",
                finding
                    .causes()
                    .iter()
                    .map(|cause| cause.finding_id().as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        lines.push("    Bounded typed facts".to_owned());
        let object = finding
            .facts()
            .data()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Map<_, _>>();
        render_generic_object(
            &object,
            &DetailPath::default(),
            &BTreeSet::new(),
            6,
            0,
            &mut lines,
        );
        if !finding.facts().redacted_fields().is_empty() {
            lines.push(format!(
                "    Redacted fields: {}",
                finding.facts().redacted_fields().join(", ")
            ));
        }
        lines.push(format!(
            "    Facts truncated: {}",
            yes_no(finding.facts().truncated())
        ));
        blocks.push(lines.join("\n"));
    }
    format!("Findings\n{}", blocks.join("\n\n"))
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
    let mut lines = vec!["Report limits".to_owned()];
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
    path: &DetailPath,
    consumed: &BTreeSet<DetailPath>,
    depth: usize,
) -> bool {
    object.iter().any(|(key, value)| {
        let child_path = path.key(key);
        has_renderable_value(value, &child_path, consumed, depth)
    })
}

fn has_renderable_value(
    value: &Value,
    path: &DetailPath,
    consumed: &BTreeSet<DetailPath>,
    depth: usize,
) -> bool {
    if consumed.contains(path) || value.is_null() {
        return false;
    }
    if matches!(value, Value::Object(object) if object.is_empty())
        || matches!(value, Value::Array(values) if values.is_empty())
    {
        return false;
    }
    if depth >= MAX_DETAIL_RENDER_DEPTH {
        return true;
    }
    match value {
        Value::Object(object) => has_renderable_object(object, path, consumed, depth + 1),
        Value::Array(values) => values.iter().enumerate().any(|(index, value)| {
            has_renderable_value(value, &path.index(index), consumed, depth + 1)
        }),
        _ => true,
    }
}

fn render_generic_object(
    object: &Map<String, Value>,
    path: &DetailPath,
    consumed: &BTreeSet<DetailPath>,
    indent: usize,
    depth: usize,
    lines: &mut Vec<String>,
) {
    let mut keys = object.keys().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        let child_path = path.key(key);
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
    path: &DetailPath,
    consumed: &BTreeSet<DetailPath>,
    indent: usize,
    depth: usize,
    lines: &mut Vec<String>,
) {
    if consumed.contains(path) {
        return;
    }
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
            let rendered = values
                .iter()
                .enumerate()
                .filter(|(index, value)| {
                    has_renderable_value(value, &path.index(*index), consumed, depth + 1)
                })
                .filter_map(|(_, value)| render_scalar(value))
                .collect::<Vec<_>>();
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
            let renderable = values
                .iter()
                .enumerate()
                .filter(|(index, value)| {
                    has_renderable_value(value, &path.index(*index), consumed, depth + 1)
                })
                .collect::<Vec<_>>();
            for (index, value) in renderable.iter().take(MAX_INLINE_SCALARS).copied() {
                let item_path = path.index(index);
                match value {
                    Value::Object(object) => {
                        lines.push(format!("{}{}", " ".repeat(indent + 2), index + 1));
                        render_generic_object(
                            object,
                            &item_path,
                            consumed,
                            indent + 4,
                            depth + 1,
                            lines,
                        );
                    }
                    _ => render_generic_value(
                        (index + 1).to_string(),
                        value,
                        &item_path,
                        consumed,
                        indent + 2,
                        depth + 1,
                        lines,
                    ),
                }
            }
            if renderable.len() > MAX_INLINE_SCALARS {
                lines.push(format!(
                    "{}...: {} more items",
                    " ".repeat(indent + 2),
                    renderable.len() - MAX_INLINE_SCALARS
                ));
            }
        }
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
        mcp_process::McpVerification,
        output::{cooperative_assurance_limits, CommandConnection, CommandOperation},
        planning::{PlannedChangeOperation, PlannedConnectionChange},
        verification::{mcp_server_check, VerificationStep},
        McpExchangeOutcome, McpExchangeProgress, McpProcessFailure, McpStage,
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
            Vec::new(),
            code.map(str::to_owned),
            summary,
            detail.and_then(details),
            observed_at.map(|value| UtcTimestamp::parse(value).unwrap()),
        )
        .unwrap()
    }

    fn action(id: ConnectionActionKind, instruction: &str) -> ConnectionAction {
        ConnectionAction::try_new(id, instruction).unwrap()
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
            generated_at: UtcTimestamp::parse("2026-07-22T00:00:00Z").unwrap(),
            findings: Vec::new(),
            integration_revision: None,
            result,
            planned_changes,
            limits: cooperative_assurance_limits(),
        }
    }

    fn rendered(report: &ConnectionCommandReport) -> String {
        render_command_report_verbose(report).unwrap()
    }

    #[test]
    fn blocked_protocol_details_are_never_rendered_as_pending() {
        let cause = volicord_types::DiagnosticFindingId::parse("finding.initialize_failed")
            .expect("cause id");
        let host = check(
            ConnectionCheckKind::HostSession,
            ConnectionCheckStatus::Pending,
            Some("host_session_initialize_pending"),
            "Codex initialize has not completed",
            None,
            None,
        )
        .blocked_by(vec![cause.clone()])
        .expect("blocked host check");
        assert_eq!(host_initialize_result(&host), "blocked");

        let required_tools = check(
            ConnectionCheckKind::RequiredTools,
            ConnectionCheckStatus::Pending,
            Some("required_tools_not_observed"),
            "Tools/list has not been observed",
            Some(json!({"required_tools_present": null})),
            None,
        )
        .blocked_by(vec![cause])
        .expect("blocked tools check");
        let report = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![required_tools],
            Vec::new(),
            None,
            None,
        );
        let output = render_checks(&report);
        assert!(output.contains("    Required tools: blocked"));
        assert!(!output.contains("Required tools: pending"));
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
                    "evidence_role": "latest_attempt",
                    "runtime_session_id": "session_1",
                    "source": "managed_host",
                    "host_executable_probe": {"discovered_path": "/opt/codex", "version": "1.2.3"},
                    "managed_peer": {
                        "client_info": {"name": "codex", "version": "1.2.3"},
                        "requested_protocol_revision": "2025-11-25",
                    },
                        "last_observed_at": "2026-07-20T00:00:00Z",
                        "terminal_finding_id": null,
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
                "  Runtime home: /runtime\n",
                "  Runtime sessions: session_1 (latest_attempt)\n\n",
                "Summary\n",
                "  Status: failed\n",
                "  Dry run: yes\n",
                "  Checks: 1 passed, 0 blocked, 1 pending, 1 failed, 0 not applicable\n\n",
                "Checks\n",
                "  [pass] Guard managed files\n",
                "    Guard managed files match current expectations\n",
                "    Guard Installation IDs: guard_1\n\n",
                "  [wait] Codex managed session\n",
                "    Codex initialize has not completed\n",
                "    Code: host_session_initialize_pending\n",
                "    Observed at: 2026-07-20T00:00:00Z\n",
                "    Depends on: process_startup\n",
                "    Evidence role: latest_attempt\n",
                "    Runtime session: session_1\n",
                "    Session source: managed_host\n",
                "    Current revision: revision_current\n",
                "    Observed revision: revision_current\n",
                "    PATH executable: /opt/codex\n",
                "    PATH executable version: 1.2.3\n",
                "    Actual MCP peer: codex\n",
                "    Actual MCP peer version: 1.2.3\n",
                "    Requested protocol: 2025-11-25\n",
                "    Initialize: pending\n\n",
                "  [fail] Managed Codex configuration\n",
                "    Managed Codex configuration differs from the canonical entry\n",
                "    Code: managed_config_mismatch\n",
                "    Target: /home/user/.codex/config.toml\n",
                "    State: changed\n",
                "    Diagnostic code: managed_config_mismatch\n",
                "    Diagnostic: managed command differs\n\n",
                "Actions\n",
                "  action.managed_config.repair\n",
                "    Repair the managed Codex configuration\n",
                "\n",
                "Result\n",
                "  Applied: no\n\n",
                "Planned changes\n",
                "  Change 1\n",
                "    Kind: managed_host_configuration\n",
                "    Operation: update\n",
                "    Target: /home/user/.codex/config.toml\n\n",
                "Report limits\n",
                "  Diagnostic cause traversal is bounded to 32 edges and 128 findings.\n",
                "  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.\n",
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
                    "evidence_role": "latest_attempt",
                    "host_executable_probe": {"discovered_path": "/opt/codex", "version": "1.2.3"},
                    "last_observed_at": null,
                    "terminal_finding_id": null,
                })),
                None,
            )],
            vec![action(
                ConnectionActionKind::ObserveCodex,
                "Restart or reload Codex and use the connection",
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
                "  Checks: 0 passed, 0 blocked, 1 pending, 0 failed, 0 not applicable\n\n",
                "Checks\n",
                "  [wait] Codex managed session\n",
                "    Managed host connection use has not been observed\n",
                "    Code: host_session_not_observed\n",
                "    Depends on: process_startup\n",
                "    Evidence role: latest_attempt\n",
                "    Current revision: revision_current\n",
                "    PATH executable: /opt/codex\n",
                "    PATH executable version: 1.2.3\n",
                "    Initialize: not observed\n\n",
                "Actions\n",
                "  action.host.observe_activity\n",
                "    Restart or reload Codex and use the connection\n\n",
                "Result\n",
                "  Applied: yes\n\n",
                "Report limits\n",
                "  Diagnostic cause traversal is bounded to 32 edges and 128 findings.\n",
                "  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.\n",
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
                ConnectionCheckStatus::NotApplicable,
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
                "  Checks: 0 passed, 0 blocked, 0 pending, 0 failed, 1 not applicable\n\n",
                "Checks\n",
                "  [n/a] Project trust\n",
                "    No separate project trust action applies to this connection scope\n\n",
                "Report limits\n",
                "  Diagnostic cause traversal is bounded to 32 edges and 128 findings.\n",
                "  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.\n",
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
                "  Checks: 0 passed, 0 blocked, 0 pending, 1 failed, 0 not applicable\n\n",
                "Checks\n",
                "  [fail] Volicord MCP server\n",
                "    Volicord MCP server self-test failed\n",
                "    Code: mcp_server_tools_list_failed\n",
                "    Depends on: managed_config\n",
                "    Preflight: passed\n",
                "    Storage: read passed, write passed\n",
                "    Effective mode: workflow\n",
                "    Initialize: passed\n",
                "    Required tools: failed\n",
                "    Tools returned: 0\n",
                "    Designated read-only tool: volicord.list_projects (not completed)\n",
                "    Shutdown: not completed\n",
                "    Self-test diagnostic code: mcp.tools.required_missing\n",
                "    Self-test finding: finding.tools.required_missing\n\n",
                "Actions\n",
                "  action.mcp.repair_server\n",
                "    Repair the MCP server and verify again\n",
                "\n",
                "Report limits\n",
                "  Diagnostic cause traversal is bounded to 32 edges and 128 findings.\n",
                "  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.\n",
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
                "  Checks: 1 passed, 0 blocked, 0 pending, 0 failed, 0 not applicable\n\n",
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
                "  action.host.reload_after_configuration_change\n",
                "    Restart or reload Codex, then use the current Volicord integration so new runtime and Guard observations bind revision revision_after\n\n",
                "Result\n",
                "  Changed: yes\n",
                "  Previous mode: workflow\n",
                "  Current mode: read_only\n",
                "  Previous revision: revision_before\n",
                "  Current revision: revision_after\n",
                "  Rebound Guard Installation IDs\n",
                "    guard_1\n\n",
                "Report limits\n",
                "  Diagnostic cause traversal is bounded to 32 edges and 128 findings.\n",
                "  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.\n",
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
                "  Checks: 0 passed, 0 blocked, 1 pending, 0 failed, 0 not applicable\n\n",
                "Checks\n",
                "  [wait] Connection removal\n",
                "    Selected Connection membership removal is ready to apply\n",
                "    Code: connection_removal_planned\n",
                "    Membership: planned for removal\n",
                "    Connection: retained until changes are applied\n\n",
                "Actions\n",
                "  action.connection.apply_removal\n",
                "    Run connection remove without --dry-run to apply the planned removal\n\n",
                "Planned changes\n",
                "  Change 1\n",
                "    Kind: connection_membership\n",
                "    Operation: remove\n",
                "    Target: /workspace/product\n\n",
                "Report limits\n",
                "  Diagnostic cause traversal is bounded to 32 edges and 128 findings.\n",
                "  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.\n",
                "  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.\n",
            )
        );
    }

    fn mcp_details(status: &str, diagnostic: &str, tools: Vec<String>) -> Value {
        let mut details = json!({
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
                "initialize": true,
                "tools_list_observed": true,
                "tools_list": tools,
                "required_tools_validated": status == "passed",
                "safe_read_only_tool": managed_host_round_trip_tool().wire_name(),
                "safe_read_only_tool_completed": status == "passed",
                "shutdown_completed": status == "passed",
            },
        });
        if status != "passed" {
            details["self_test"]["diagnostic_code"] = json!("mcp.tools.required_missing");
            details["self_test"]["failure_stage"] = json!("tools_list");
            details["self_test"]["finding_id"] = json!("finding.tools.required_missing");
        }
        details
    }

    fn rendered_mcp_progress(
        progress: McpExchangeProgress,
        failure: Option<McpProcessFailure>,
    ) -> String {
        let exchange = match failure {
            Some(failure) => McpExchangeOutcome::failed(progress, failure),
            None => McpExchangeOutcome::completed(progress),
        };
        let check = mcp_server_check(
            &VerificationStep::passed_with_code("mcp_preflight_ready", "ready"),
            &McpVerification::from_exchange(exchange),
        )
        .expect("MCP server check");
        let status = match check.status() {
            ConnectionCheckStatus::Passed => ConnectionStatus::Complete,
            ConnectionCheckStatus::Pending => ConnectionStatus::ActionRequired,
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked => {
                ConnectionStatus::Failed
            }
            ConnectionCheckStatus::NotApplicable => ConnectionStatus::Complete,
        };
        rendered(&report(
            CommandOperation::Verify,
            false,
            status,
            "workflow",
            vec![check],
            Vec::new(),
            None,
            None,
        ))
    }

    #[test]
    fn human_mcp_projection_uses_explicit_progress_for_all_terminal_stages() {
        let before_initialize = rendered_mcp_progress(
            McpExchangeProgress::not_started(),
            Some(McpProcessFailure::protocol(
                McpStage::Startup,
                "startup failed",
            )),
        );
        assert!(before_initialize.contains("    Initialize: failed\n"));
        assert!(before_initialize.contains("    Required tools: not completed\n"));
        assert!(!before_initialize.contains("    Tools returned:"));

        let tools_list_failed = rendered_mcp_progress(
            McpExchangeProgress::observed(true, None, false, false, false),
            Some(McpProcessFailure::protocol(
                McpStage::ToolsList,
                "tools/list failed",
            )),
        );
        assert!(tools_list_failed.contains("    Initialize: passed\n"));
        assert!(tools_list_failed.contains("    Required tools: not completed\n"));
        assert!(!tools_list_failed.contains("    Tools returned:"));

        let required_tools_failed = rendered_mcp_progress(
            McpExchangeProgress::observed(
                true,
                Some(vec!["fixture.alpha".to_owned(), "fixture.beta".to_owned()]),
                false,
                false,
                false,
            ),
            Some(McpProcessFailure::protocol(
                McpStage::ToolsList,
                "required tools failed",
            )),
        );
        assert!(required_tools_failed.contains("    Required tools: failed\n"));
        assert!(required_tools_failed.contains("    Tools returned: 2\n"));

        let safe_call_failed = rendered_mcp_progress(
            McpExchangeProgress::observed(
                true,
                Some(vec![managed_host_round_trip_tool().wire_name().to_owned()]),
                true,
                false,
                false,
            ),
            Some(McpProcessFailure::protocol(
                McpStage::SafeToolCall,
                "designated read-only tool call failed",
            )),
        );
        assert!(safe_call_failed.contains("    Required tools: passed\n"));
        assert!(safe_call_failed.contains("    Tools returned: 1\n"));
        assert!(safe_call_failed
            .contains("    Designated read-only tool: volicord.list_projects (failed)\n"));
        assert!(safe_call_failed.contains("    Shutdown: not completed\n"));

        let shutdown_failed = rendered_mcp_progress(
            McpExchangeProgress::observed(
                true,
                Some(vec![managed_host_round_trip_tool().wire_name().to_owned()]),
                true,
                true,
                false,
            ),
            Some(McpProcessFailure::protocol(
                McpStage::Shutdown,
                "shutdown failed",
            )),
        );
        assert!(shutdown_failed.contains("    Designated read-only tool: volicord.list_projects\n"));
        assert!(shutdown_failed.contains("    Shutdown: failed\n"));

        let completed = rendered_mcp_progress(
            McpExchangeProgress::observed(true, Some(Vec::new()), true, true, true),
            None,
        );
        assert!(completed.contains("    Initialize: passed\n"));
        assert!(completed.contains("    Required tools: passed\n"));
        assert!(completed.contains("    Tools returned: 0\n"));
        assert!(completed.contains("    Shutdown: passed\n"));
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
        let machine = serde_json::to_value(mcp.diagnostic_report().unwrap()).unwrap();
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
            "    Shutdown: passed\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}");
        }
        for tool in tools {
            assert!(!output.contains(&tool), "successful tool inventory leaked");
        }

        let mut protocol_details = mcp_details(
            "failed",
            "MCP protocol failed during initialize",
            Vec::new(),
        );
        protocol_details["self_test"]["initialize"] = json!(false);
        protocol_details["self_test"]["tools_list_observed"] = json!(false);
        protocol_details["self_test"]
            .as_object_mut()
            .expect("self-test details")
            .remove("tools_list");
        protocol_details["self_test"]["diagnostic_code"] = json!("mcp.json_rpc.error_response");
        protocol_details["self_test"]["failure_stage"] = json!("initialize");
        protocol_details["self_test"]["finding_id"] = json!("finding.protocol_failure");
        let protocol_failure = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Failed,
                Some("mcp_server_initialize_failed"),
                "Volicord MCP server self-test failed",
                Some(protocol_details),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let protocol_output = rendered(&protocol_failure);
        let protocol_machine =
            serde_json::to_value(protocol_failure.diagnostic_report().unwrap()).unwrap();
        assert_eq!(
            protocol_machine["checks"][0]["details"]["self_test"]["diagnostic_code"],
            "mcp.json_rpc.error_response"
        );
        assert_eq!(
            protocol_machine["checks"][0]["details"]["self_test"]["failure_stage"],
            "initialize"
        );
        assert!(protocol_output
            .contains("    Self-test diagnostic code: mcp.json_rpc.error_response\n"));
        assert!(protocol_output.contains("    Self-test finding: finding.protocol_failure\n"));
        assert!(!protocol_output.contains("Phase:"));

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
        assert!(output.contains(concat!(
            "    Artifact issues\n",
            "      1\n",
            "        Artifact: host_hooks_config\n",
            "        Path: .codex/hooks.json\n",
            "        Issue: content_mismatch\n",
            "        Details: expected current managed content\n",
        )));
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
                    "evidence_role": "latest_attempt",
                    "host_executable_probe": {"discovered_path": "/opt/codex", "version": "2.0"},
                    "managed_peer": {
                        "client_info": {"name": "codex", "version": "1.0"},
                        "requested_protocol_revision": "2025-11-25",
                    },
                    "runtime_session_id": "session_1",
                    "last_observed_at": "2026-07-20T02:00:00Z",
                    "terminal_finding_id": "finding.protocol_failure",
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
            "    Terminal finding: finding.protocol_failure\n",
            "    Evidence role: latest_attempt\n",
            "    Runtime session: session_1\n",
            "    PATH executable: /opt/codex\n",
            "    PATH executable version: 2.0\n",
            "    Actual MCP peer: codex\n",
            "    Actual MCP peer version: 1.0\n",
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
            (
                ConnectionCheckKind::DiagnosticLookup,
                "Diagnostic finding lookup",
            ),
            (ConnectionCheckKind::GuardFiles, "Guard managed files"),
            (
                ConnectionCheckKind::GuardHookExecution,
                "Guard hook execution",
            ),
            (ConnectionCheckKind::GuardObservation, "Guard hook activity"),
            (
                ConnectionCheckKind::GuardVerification,
                "Guard integration verification",
            ),
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
            (
                ConnectionCheckKind::ProcessStartup,
                "Managed MCP process startup",
            ),
            (ConnectionCheckKind::ProjectTrust, "Project trust"),
            (ConnectionCheckKind::RequiredTools, "Codex required tools"),
            (
                ConnectionCheckKind::RuntimeSessionLookup,
                "Runtime-session lookup",
            ),
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
                        "status": "unavailable",
                        "probe": {
                            "version": "1.2.3",
                            "discovered_path": "/opt/codex/bin/codex",
                        },
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
                        "evidence_role": "latest_attempt",
                        "runtime_session_id": "session_tools",
                        "source": "managed_host",
                        "required_tools": {
                            "tools_list_observed_at": "2026-07-20T03:00:00Z",
                            "returned_tool_identities": ["volicord.status"],
                            "required_tools_present": false,
                        },
                        "last_observed_at": "2026-07-20T03:00:00Z",
                        "terminal_finding_id": null,
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
                        "evidence_role": "latest_attempt",
                        "runtime_session_id": "session_tools",
                        "source": "managed_host",
                        "verification_tool": {
                            "expected_tool_identity": "volicord.list_projects",
                            "observed_tool_identity": "volicord.status",
                            "observed_at": "2026-07-20T04:00:00Z",
                        },
                        "last_observed_at": "2026-07-20T04:00:00Z",
                        "terminal_finding_id": "finding.tool_contract_mismatch",
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
            "    Tools/list observed at: 2026-07-20T03:00:00Z\n",
            "    Returned tools: 1\n",
            "    Required tools: failed\n",
            "  [wait] Setup plan\n",
            "    Planned state: changes ready to apply\n",
            "    guard_managed_file: 2\n",
            "    managed_host_configuration: 1\n",
            "  [fail] Read-only tool round trip\n",
            "    Expected verification tool: volicord.list_projects\n",
            "    Observed verification tool: volicord.status\n",
            "    Verification tool observed at: 2026-07-20T04:00:00Z\n",
            "    Call completed: yes\n",
            "    Terminal finding: finding.tool_contract_mismatch\n",
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
        let json_before =
            serde_json::to_string_pretty(&report_value.diagnostic_report().unwrap()).unwrap();
        let output = rendered(&report_value);
        let json_after =
            serde_json::to_string_pretty(&report_value.diagnostic_report().unwrap()).unwrap();
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

    #[test]
    fn detail_context_consumes_only_successfully_interpreted_values() {
        let report = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Complete,
            "workflow",
            vec![check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
                None,
                "Managed configuration details",
                Some(json!({
                    "string": "value",
                    "number": 7,
                    "boolean": true,
                    "boolean_string": "true",
                    "strings": ["one", "two"],
                    "empty_strings": [],
                    "mixed": ["one", 2],
                    "explicit": {"handled": true},
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let mut context = DetailContext::new(&report, &report.checks[0]);

        let string = DetailPath::from_dotted_keys("string");
        assert_eq!(context.peek(&string), Some(&json!("value")));
        assert_eq!(context.take_string("string").as_deref(), Some("value"));
        assert!(context.consumed.contains(&string));

        for mismatched in ["number", "boolean_string"] {
            assert_eq!(context.take_bool(mismatched), None);
            assert!(!context
                .consumed
                .contains(&DetailPath::from_dotted_keys(mismatched)));
        }
        assert_eq!(context.take_string("number"), None);
        assert_eq!(context.take_bool("boolean"), Some(true));
        assert_eq!(
            context.take_string_array("strings"),
            Some(vec!["one".to_owned(), "two".to_owned()])
        );
        assert_eq!(context.take_string_array("empty_strings"), Some(Vec::new()));
        assert_eq!(context.take_string_array("mixed"), None);
        assert!(!context
            .consumed
            .contains(&DetailPath::from_dotted_keys("mixed")));

        let explicit = DetailPath::from_dotted_keys("explicit");
        assert!(context.peek(&explicit).is_some());
        context.consume(&explicit);
        assert!(context.consumed.contains(&explicit));
    }

    #[test]
    fn mismatched_known_scalars_remain_in_additional_details() {
        let managed = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Failed,
                Some("managed_config_mismatch"),
                "Managed configuration mismatch",
                Some(json!({
                    "target": 17,
                    "observed_state": {"actual": "changed"},
                    "diagnostic_code": "managed_config_mismatch",
                    "diagnostic": "Managed configuration mismatch",
                    "future_field": "future value",
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&managed);
        for expected in [
            "    Diagnostic code: managed_config_mismatch\n",
            "    Additional details\n",
            "      Future field: future value\n",
            "      Observed state\n        Actual: changed\n",
            "      Target: 17\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        assert_eq!(
            output
                .matches("Diagnostic code: managed_config_mismatch")
                .count(),
            1
        );
        assert_eq!(output.matches("Managed configuration mismatch").count(), 1);

        let trust = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::Failed,
                Some("project_trust_invalid"),
                "Project trust details are invalid",
                Some(json!({"applicable": "not-a-boolean"})),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&trust);
        assert!(output.contains("    Additional details\n      Applicable: not-a-boolean\n"));
        assert!(!output.contains("    Applicable: yes\n"));
    }

    #[test]
    fn string_arrays_are_taken_only_when_every_element_is_a_string() {
        let focused = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::RequiredTools,
                ConnectionCheckStatus::Failed,
                Some("required_tools_missing"),
                "Required tools are missing",
                Some(json!({
                    "required_tools": {
                        "tools_list_observed_at": "2026-07-20T03:00:00Z",
                        "returned_tool_identities": ["volicord.close_task", "volicord.record_evidence"],
                    },
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&focused);
        assert!(output.contains("    Returned tools: 2\n"));
        assert!(!output.contains("Additional details"));

        let mixed = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::GuardObservation,
                ConnectionCheckStatus::Failed,
                Some("guard_observation_failed"),
                "Guard phase details are invalid",
                Some(json!({
                    "required_phases": ["pre_tool", 7, {"phase": "future"}],
                    "observed_phases": [],
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&mixed);
        for expected in [
            "    Additional details\n",
            "      Required phases\n",
            "        1: pre_tool\n",
            "        2: 7\n",
            "        3\n          Phase: future\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        assert!(!output.contains("    Required phases: pre_tool\n"));

        let long_mixed = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::GuardFiles,
                ConnectionCheckStatus::Failed,
                Some("guard_files_failed"),
                "Guard installation details are invalid",
                Some(json!({
                    "installation_ids": ["one", 2, "three", 4, "five", 6, "seven", 8, {"nine": true}, 10],
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&long_mixed);
        assert!(output.contains("      Installation ids\n"));
        assert!(output.contains("        1: one\n"));
        assert!(output.contains("        8: 8\n"));
        assert!(output.contains("        ...: 2 more items\n"));
    }

    #[test]
    fn nested_extensions_survive_leaf_consumption_without_known_duplicates() {
        let mut details = mcp_details(
            "passed",
            "Volicord MCP server self-test passed",
            vec!["private.tool".to_owned()],
        );
        details["preflight"]["storage"]["future_storage"] = json!({"replica": "ready"});
        details["self_test"]["future_self_test"] = json!({"attempt": 2});
        details["future_top_level"] = json!("visible");
        let extended_report = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Complete,
            "workflow",
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
                None,
                "Volicord MCP server self-test passed",
                Some(details),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&extended_report);
        for expected in [
            "    Storage: read passed, write passed\n",
            "    Tools returned: 1\n",
            "    Additional details\n",
            "      Future top level: visible\n",
            "      Preflight\n        Storage\n          Future storage\n            Replica: ready\n",
            "      Self test\n        Future self test\n          Attempt: 2\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        assert_eq!(
            output.matches("Storage: read passed, write passed").count(),
            1
        );
        assert!(!output.contains("private.tool"));

        let scalar_parent = report(
            CommandOperation::Verify,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Failed,
                Some("mcp_server_preflight_failed"),
                "MCP preflight failed",
                Some(json!({
                    "preflight": {
                        "status": "failed",
                        "code": "mcp_server_preflight_failed",
                        "diagnostic": "storage unavailable",
                        "storage": "not-an-object",
                    },
                    "self_test": {
                        "status": "failed",
                        "code": "mcp_server_self_test_not_run",
                        "diagnostic": "not run",
                        "initialize": false,
                        "tools_list_observed": false,
                        "required_tools_validated": false,
                        "safe_read_only_tool": "volicord.list_projects",
                        "safe_read_only_tool_completed": false,
                        "shutdown_completed": false,
                    },
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        assert!(rendered(&scalar_parent)
            .contains("    Additional details\n      Preflight\n        Storage: not-an-object\n"));
    }

    #[test]
    fn artifact_issue_extensions_and_malformed_elements_remain_indexed_and_visible() {
        let report = report(
            CommandOperation::Status,
            false,
            ConnectionStatus::Failed,
            "workflow",
            vec![check(
                ConnectionCheckKind::GuardFiles,
                ConnectionCheckStatus::Failed,
                Some("guard_files_failed"),
                "Guard files do not match",
                Some(json!({
                    "artifact_issues": [
                        {
                            "artifact": "host_hooks_config",
                            "path": ".codex/hooks.json",
                            "issue": "content_mismatch",
                            "details": "expected current managed content",
                            "extra_scalar": "kept",
                            "extra_nested": {"owner": "future"},
                        },
                        {
                            "artifact": "guard_wrapper",
                            "path": 42,
                            "issue": "mode_mismatch",
                            "details": "expected executable behavior",
                        },
                        "non-object issue",
                        {},
                    ],
                })),
                None,
            )],
            Vec::new(),
            None,
            None,
        );
        let output = rendered(&report);
        for expected in [
            concat!(
                "    Artifact issues\n",
                "      1\n",
                "        Artifact: host_hooks_config\n",
                "        Path: .codex/hooks.json\n",
                "        Issue: content_mismatch\n",
                "        Details: expected current managed content\n",
                "        Additional details\n",
                "          Extra nested\n",
                "            Owner: future\n",
                "          Extra scalar: kept\n",
            ),
            concat!(
                "      2\n",
                "        Artifact: guard_wrapper\n",
                "        Issue: mode_mismatch\n",
                "        Details: expected executable behavior\n",
                "        Additional details\n",
                "          Path: 42\n",
            ),
            "      4\n        Empty object\n",
            "    Additional details\n      Artifact issues\n        3: non-object issue\n",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        for value in [
            "host_hooks_config",
            ".codex/hooks.json",
            "content_mismatch",
            "expected current managed content",
            "kept",
            "future",
            "guard_wrapper",
            "42",
            "mode_mismatch",
            "expected executable behavior",
            "non-object issue",
        ] {
            assert!(output.contains(value), "dropped {value:?}\n{output}");
        }
        assert_eq!(output.matches("host_hooks_config").count(), 1);
        assert_eq!(output.matches("guard_wrapper").count(), 1);
    }
}
