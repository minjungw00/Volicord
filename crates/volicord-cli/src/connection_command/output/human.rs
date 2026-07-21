use std::path::Path;

use serde_json::Value;
use volicord_types::{
    ConnectionCheck, ConnectionCheckKind, ConnectionCheckStatus, ConnectionStatus, HostKind,
    HostScope,
};

use super::report::{
    projected_actions, projected_check_root_cause_ids, projected_root_cause_ids, CommandOperation,
    ConnectionCommandReport, ConnectionCommandResult,
};
use crate::connection_command::{
    guidance::{ConnectionUserInvocation, DiagnosticOperation},
    ConnectionCommandError, PlannedConnectionChange, PlannedConnectionChangeKind,
};

pub(super) fn render_command_report_concise(
    report: &ConnectionCommandReport,
) -> Result<String, ConnectionCommandError> {
    let counts = CheckCounts::from_report(report);
    let mut sections = vec![headline(report, counts)];
    sections.push(format!(
        "Repository: {}\nMode: {}\nChecks: {}",
        report.connection.repository,
        report.connection.mode,
        counts.render(true)
    ));

    if let Some(planned_changes) = report
        .planned_changes
        .as_deref()
        .filter(|changes| !changes.is_empty())
    {
        sections.push(render_planned_changes(planned_changes));
    }

    let problems = render_root_problems(report)?;
    if !problems.is_empty() {
        sections.push(format!("Problems\n{}", problems.join("\n")));
    }

    let waiting = render_waiting_checks(&report.checks);
    if !waiting.is_empty() {
        sections.push(format!("Waiting\n{}", waiting.join("\n")));
    }

    let projected_actions = projected_actions(report)?;
    if !projected_actions.is_empty() {
        let numbered = projected_actions.len() > 1;
        let actions = projected_actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                let instruction = action.summary();
                let code = action.code().as_str();
                if numbered {
                    format!("  {}. {code}: {instruction}", index + 1)
                } else {
                    format!("  {code}: {instruction}")
                }
            })
            .collect::<Vec<_>>();
        sections.push(format!("Next\n{}", actions.join("\n")));
    }

    if let Some(hint) = concise_diagnostic_hint(report) {
        sections.push(hint);
    }
    Ok(format!("{}\n", sections.join("\n\n")))
}

fn render_root_problems(
    report: &ConnectionCommandReport,
) -> Result<Vec<String>, ConnectionCommandError> {
    let roots = projected_root_cause_ids(report)?;
    if roots.is_empty() {
        return Ok(report
            .checks
            .iter()
            .filter(|check| check.status() == ConnectionCheckStatus::Failed)
            .map(|check| format!("  {}", check.summary()))
            .collect());
    }
    let projected = roots
        .into_iter()
        .filter_map(|root_id| {
            let finding = report
                .findings
                .iter()
                .find(|finding| finding.id() == &root_id)?;
            let facts = finding.facts().data();
            let summary = facts
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or_else(|| finding.code().as_str());
            let mut lines = vec![format!("  {}: {summary}", finding.code())];
            if let Some(value) = combined_client_info(facts) {
                lines.push(format!("    Actual MCP client: {value}"));
            }
            push_fact_line(
                &mut lines,
                facts,
                "requested_revision",
                "Requested protocol",
            );
            push_array_fact_line(
                &mut lines,
                facts,
                "production_supported_revisions",
                "Supported protocols",
            );
            push_fact_line(&mut lines, facts, "actual", "Actual");
            push_fact_line(&mut lines, facts, "expected", "Expected");
            push_fact_line(&mut lines, facts, "observed_state", "Observation");
            push_fact_line(&mut lines, facts, "observed_revision", "Actual revision");
            push_fact_line(&mut lines, facts, "expected_revision", "Expected revision");
            push_array_fact_line(&mut lines, facts, "missing_tools", "Missing tools");
            push_fact_line(&mut lines, facts, "timeout_ms", "Timeout (ms)");
            push_fact_line(&mut lines, facts, "exit_code", "Process exit");
            let blocked = report
                .checks
                .iter()
                .filter(|check| check.status() == ConnectionCheckStatus::Blocked)
                .filter_map(|check| {
                    projected_check_root_cause_ids(report, check)
                        .ok()
                        .filter(|roots| roots.contains(&root_id))
                        .map(|_| check.id().as_str())
                })
                .collect::<Vec<_>>();
            if !blocked.is_empty() {
                lines.push(format!("    Blocked checks: {}", blocked.join(", ")));
            }
            if let Some(runtime_session_id) = finding.runtime_session_id() {
                lines.push(format!("    Runtime session: {runtime_session_id}"));
            }
            lines.push(format!("    Finding: {}", finding.id()));
            Some(lines.join("\n"))
        })
        .collect::<Vec<_>>();
    Ok(if projected.is_empty() {
        report
            .checks
            .iter()
            .filter(|check| check.status() == ConnectionCheckStatus::Failed)
            .map(|check| format!("  {}", check.summary()))
            .collect()
    } else {
        projected
    })
}

fn combined_client_info(facts: &std::collections::BTreeMap<String, Value>) -> Option<String> {
    let name = facts.get("attempted_client_name").and_then(Value::as_str);
    let version = facts
        .get("attempted_client_version")
        .and_then(Value::as_str);
    match (name, version) {
        (Some(name), Some(version)) => Some(format!("{name} {version}")),
        (Some(name), None) => Some(name.to_owned()),
        (None, Some(version)) => Some(version.to_owned()),
        (None, None) => None,
    }
}

fn push_fact_line(
    lines: &mut Vec<String>,
    facts: &std::collections::BTreeMap<String, Value>,
    key: &str,
    label: &str,
) {
    let Some(value) = facts.get(key).filter(|value| !value.is_null()) else {
        return;
    };
    let rendered = value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    lines.push(format!("    {label}: {rendered}"));
}

fn push_array_fact_line(
    lines: &mut Vec<String>,
    facts: &std::collections::BTreeMap<String, Value>,
    key: &str,
    label: &str,
) {
    let Some(values) = facts.get(key).and_then(Value::as_array) else {
        return;
    };
    let rendered = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned)
        })
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!("    {label}: {rendered}"));
}

fn concise_diagnostic_hint(report: &ConnectionCommandReport) -> Option<String> {
    let has_nonpassing_check = report.checks.iter().any(|check| {
        matches!(
            check.status(),
            ConnectionCheckStatus::Pending
                | ConnectionCheckStatus::Failed
                | ConnectionCheckStatus::Blocked
        )
    });

    match report.operation {
        CommandOperation::Status => {
            has_nonpassing_check.then(|| current_status_diagnostic_hint(report))
        }
        CommandOperation::Verify => has_nonpassing_check.then(|| {
            diagnostic_invocation_from_report(report, DiagnosticOperation::Verify).render_guidance()
        }),
        CommandOperation::Init | CommandOperation::Add if report.dry_run => {
            Some("Run the same dry-run command with --verbose for detailed diagnostics.".to_owned())
        }
        CommandOperation::Init | CommandOperation::Add => match report.result.as_ref() {
            Some(ConnectionCommandResult::Setup { applied: true }) if has_nonpassing_check => {
                Some(current_status_diagnostic_hint(report))
            }
            Some(ConnectionCommandResult::Setup { applied: false })
                if report.status == ConnectionStatus::Failed && has_nonpassing_check =>
            {
                Some(
                    "Run the same setup command with --verbose for detailed diagnostics."
                        .to_owned(),
                )
            }
            _ => None,
        },
        CommandOperation::Mode => match report.result.as_ref() {
            Some(ConnectionCommandResult::ModeTransition { changed: true, .. })
                if has_nonpassing_check =>
            {
                Some(current_status_diagnostic_hint(report))
            }
            None if report.status == ConnectionStatus::Failed && has_nonpassing_check => Some(
                "Run the same connection mode command with --verbose for detailed diagnostics."
                    .to_owned(),
            ),
            _ => None,
        },
        CommandOperation::Remove if report.dry_run => {
            Some("Run the same dry-run command with --verbose for detailed diagnostics.".to_owned())
        }
        CommandOperation::Remove => match report.result.as_ref() {
            None if report.status == ConnectionStatus::Failed && has_nonpassing_check => Some(
                "Run the same connection remove command with --verbose for detailed diagnostics."
                    .to_owned(),
            ),
            _ => None,
        },
    }
}

fn current_status_diagnostic_hint(report: &ConnectionCommandReport) -> String {
    diagnostic_invocation_from_report(report, DiagnosticOperation::Status).render_guidance()
}

fn diagnostic_invocation_from_report(
    report: &ConnectionCommandReport,
    operation: DiagnosticOperation,
) -> ConnectionUserInvocation {
    let host = report
        .connection
        .host
        .parse::<HostKind>()
        .expect("Connection command report host must be canonical");
    let scope = match report.connection.scope.as_str() {
        scope if scope == HostScope::User.as_str() => HostScope::User,
        scope if scope == HostScope::Project.as_str() => HostScope::Project,
        _ => unreachable!("Connection command reports contain a typed host scope"),
    };
    ConnectionUserInvocation::diagnostic(
        operation,
        host,
        Path::new(&report.connection.repository),
        Path::new(&report.runtime_home),
        scope,
    )
}

#[derive(Clone, Copy)]
pub(super) struct CheckCounts {
    pub(super) ready: usize,
    pub(super) blocked: usize,
    pub(super) waiting: usize,
    pub(super) failed: usize,
    pub(super) not_applicable: usize,
}

impl CheckCounts {
    fn from_checks(checks: &[ConnectionCheck]) -> Self {
        let mut counts = Self {
            ready: 0,
            blocked: 0,
            waiting: 0,
            failed: 0,
            not_applicable: 0,
        };
        for check in checks {
            match check.status() {
                ConnectionCheckStatus::Passed => counts.ready += 1,
                ConnectionCheckStatus::Pending => counts.waiting += 1,
                ConnectionCheckStatus::Failed => counts.failed += 1,
                ConnectionCheckStatus::Blocked => counts.blocked += 1,
                ConnectionCheckStatus::NotApplicable => counts.not_applicable += 1,
            }
        }
        counts
    }

    pub(super) fn from_report(report: &ConnectionCommandReport) -> Self {
        Self::from_checks(&report.checks)
    }

    fn render(self, always_show_ready: bool) -> String {
        if always_show_ready {
            return format!(
                "{} ready, {} blocked, {} waiting, {} failed",
                self.ready, self.blocked, self.waiting, self.failed
            );
        }
        let mut parts = Vec::new();
        if self.ready > 0 {
            parts.push(format!("{} ready", self.ready));
        }
        if self.blocked > 0 {
            parts.push(format!("{} blocked", self.blocked));
        }
        if self.waiting > 0 {
            parts.push(format!("{} waiting", self.waiting));
        }
        if self.failed > 0 {
            parts.push(format!("{} failed", self.failed));
        }
        parts.join(", ")
    }
}

pub(super) fn headline(report: &ConnectionCommandReport, counts: CheckCounts) -> String {
    match report.operation {
        CommandOperation::Init | CommandOperation::Add => setup_headline(report),
        CommandOperation::Status => match report.status {
            ConnectionStatus::Complete => "Codex connection is ready.".to_owned(),
            ConnectionStatus::ActionRequired => {
                "Codex connection is configured and waiting for activity.".to_owned()
            }
            ConnectionStatus::Failed => "Codex connection needs attention.".to_owned(),
        },
        CommandOperation::Verify => {
            let result = counts.render(false);
            if result.is_empty() {
                "Verification completed.".to_owned()
            } else {
                format!("Verification completed: {result}.")
            }
        }
        CommandOperation::Mode => mode_headline(report),
        CommandOperation::Remove => removal_headline(report),
    }
}

fn setup_headline(report: &ConnectionCommandReport) -> String {
    if report.dry_run {
        return if report
            .planned_changes
            .as_ref()
            .is_some_and(|changes| !changes.is_empty())
        {
            "Volicord setup changes are ready to review.".to_owned()
        } else {
            "No Volicord setup changes are required.".to_owned()
        };
    }

    let applied = matches!(
        report.result,
        Some(ConnectionCommandResult::Setup { applied: true })
    );
    match (report.status, applied) {
        (ConnectionStatus::Complete, _) => "Volicord setup is ready.".to_owned(),
        (ConnectionStatus::ActionRequired, true) => {
            "Volicord setup was applied and needs one more step.".to_owned()
        }
        (ConnectionStatus::ActionRequired, false) => {
            "Volicord setup needs one more step.".to_owned()
        }
        (ConnectionStatus::Failed, true) => {
            "Volicord setup was applied, but verification failed.".to_owned()
        }
        (ConnectionStatus::Failed, false) => "Volicord setup could not be applied.".to_owned(),
    }
}

fn mode_headline(report: &ConnectionCommandReport) -> String {
    let Some(ConnectionCommandResult::ModeTransition {
        changed,
        previous_mode,
        current_mode,
        ..
    }) = report.result.as_ref()
    else {
        return "Connection mode needs attention.".to_owned();
    };
    if *changed {
        format!("Connection mode changed from {previous_mode} to {current_mode}.")
    } else {
        format!("Connection mode is already {current_mode}.")
    }
}

fn removal_headline(report: &ConnectionCommandReport) -> String {
    if report.dry_run {
        return if report
            .planned_changes
            .as_ref()
            .is_some_and(|changes| !changes.is_empty())
        {
            "Connection removal is ready to review.".to_owned()
        } else {
            "No Connection removal is required.".to_owned()
        };
    }
    match report.result.as_ref() {
        Some(ConnectionCommandResult::Removal {
            membership_removed: true,
            connection_removed: true,
            ..
        }) => "Connection membership and Connection record were removed.".to_owned(),
        Some(ConnectionCommandResult::Removal {
            membership_removed: true,
            connection_removed: false,
            ..
        }) => "Connection membership was removed; the shared Connection remains in use.".to_owned(),
        _ => "Connection removal needs attention.".to_owned(),
    }
}

fn render_waiting_checks(checks: &[ConnectionCheck]) -> Vec<String> {
    let mut waiting = Vec::new();
    let pending_activity = PendingCodexActivity::from_checks(checks);
    if let Some(activity) = pending_activity.render() {
        waiting.push(format!("  {activity}"));
    }

    if let Some(check) = checks.iter().find(|check| {
        check.id() == ConnectionCheckKind::GuardObservation
            && check.status() == ConnectionCheckStatus::Pending
    }) {
        let missing = guard_missing_phases(check);
        if missing.is_empty() {
            waiting.push("  Guard hook activity".to_owned());
        } else {
            waiting.push(format!("  Guard hook activity: {}", missing.join(", ")));
        }
    }

    waiting.extend(
        checks
            .iter()
            .filter(|check| {
                check.status() == ConnectionCheckStatus::Pending
                    && !matches!(
                        check.id(),
                        ConnectionCheckKind::HostSession
                            | ConnectionCheckKind::RequiredTools
                            | ConnectionCheckKind::ToolRoundTrip
                            | ConnectionCheckKind::GuardObservation
                    )
            })
            .map(|check| format!("  {}", check.summary())),
    );
    waiting
}

#[derive(Default)]
struct PendingCodexActivity {
    host_session: bool,
    required_tools: bool,
    tool_round_trip: bool,
}

impl PendingCodexActivity {
    fn from_checks(checks: &[ConnectionCheck]) -> Self {
        let mut pending = Self::default();
        for check in checks
            .iter()
            .filter(|check| check.status() == ConnectionCheckStatus::Pending)
        {
            match check.id() {
                ConnectionCheckKind::HostSession => pending.host_session = true,
                ConnectionCheckKind::RequiredTools => pending.required_tools = true,
                ConnectionCheckKind::ToolRoundTrip => pending.tool_round_trip = true,
                _ => {}
            }
        }
        pending
    }

    fn render(&self) -> Option<&'static str> {
        match (
            self.host_session,
            self.required_tools,
            self.tool_round_trip,
        ) {
            (true, true, true) => Some(
                "Codex session and tool activity: initialize, tools/list, and the designated read-only tool call",
            ),
            (true, true, false) => {
                Some("Codex session and tool activity: initialize and tools/list")
            }
            (true, false, true) => Some(
                "Codex session and tool activity: initialize and the designated read-only tool call",
            ),
            (false, true, true) => Some(
                "Codex tool activity: tools/list and the designated read-only tool call",
            ),
            (true, false, false) => Some("Codex managed session"),
            (false, true, false) => Some("Codex tools/list"),
            (false, false, true) => Some("Read-only Volicord tool call"),
            (false, false, false) => None,
        }
    }
}

fn guard_missing_phases(check: &ConnectionCheck) -> Vec<String> {
    let mut phases = check
        .details()
        .and_then(|details| details.as_object().get("missing_required_phases"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    phases.sort_by_key(|phase| match phase.as_str() {
        "pre_tool" => 0,
        "post_tool" => 1,
        "prompt_capture" => 2,
        _ => 3,
    });
    phases
}

fn render_planned_changes(changes: &[PlannedConnectionChange]) -> String {
    const KINDS: [PlannedConnectionChangeKind; 6] = [
        PlannedConnectionChangeKind::RuntimeHomeInitialization,
        PlannedConnectionChangeKind::ProjectRegistration,
        PlannedConnectionChangeKind::ManagedHostConfiguration,
        PlannedConnectionChangeKind::GuardManagedFile,
        PlannedConnectionChangeKind::GuardRegistrySetup,
        PlannedConnectionChangeKind::ConnectionMembership,
    ];
    let lines = KINDS
        .into_iter()
        .filter_map(|kind| {
            let count = changes
                .iter()
                .filter(|change| change.kind() == kind)
                .count();
            (count > 0).then(|| format!("  {count} {}", planned_change_label(kind, count)))
        })
        .collect::<Vec<_>>();
    format!("Planned changes\n{}", lines.join("\n"))
}

fn planned_change_label(kind: PlannedConnectionChangeKind, count: usize) -> &'static str {
    match (kind, count == 1) {
        (PlannedConnectionChangeKind::RuntimeHomeInitialization, true) => {
            "local Volicord storage initialization"
        }
        (PlannedConnectionChangeKind::RuntimeHomeInitialization, false) => {
            "local Volicord storage initialization changes"
        }
        (PlannedConnectionChangeKind::ProjectRegistration, true) => {
            "Product Repository registration"
        }
        (PlannedConnectionChangeKind::ProjectRegistration, false) => {
            "Product Repository registrations"
        }
        (PlannedConnectionChangeKind::ManagedHostConfiguration, true) => {
            "managed Codex configuration change"
        }
        (PlannedConnectionChangeKind::ManagedHostConfiguration, false) => {
            "managed Codex configuration changes"
        }
        (PlannedConnectionChangeKind::GuardManagedFile, true) => "Guard managed-file change",
        (PlannedConnectionChangeKind::GuardManagedFile, false) => "Guard managed-file changes",
        (PlannedConnectionChangeKind::GuardRegistrySetup, true) => "Guard Registry change",
        (PlannedConnectionChangeKind::GuardRegistrySetup, false) => "Guard Registry changes",
        (PlannedConnectionChangeKind::ConnectionMembership, true) => "Connection membership change",
        (PlannedConnectionChangeKind::ConnectionMembership, false) => {
            "Connection membership changes"
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{json, Value};
    use volicord_types::{
        ConnectionAction, ConnectionActionKind, ConnectionCheck, ConnectionCheckDetails,
        ConnectionCheckKind, ConnectionCheckStatus, ConnectionStatus, ConnectionVerificationReport,
        UtcTimestamp,
    };

    use crate::connection_command::{
        args::{HumanOutputDetail, OutputFormat},
        output::report::{
            render_command_report, CommandConnection, CommandOperation, ConnectionCommandReport,
        },
        planning::{PlannedChangeOperation, PlannedConnectionChange, PlannedConnectionChangeKind},
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

    fn check(
        id: ConnectionCheckKind,
        status: ConnectionCheckStatus,
        summary: &str,
        details: Option<Value>,
    ) -> ConnectionCheck {
        let details = details.map(|details| {
            let Value::Object(object) = details else {
                panic!("test check details must be an object")
            };
            ConnectionCheckDetails::try_new(object).unwrap()
        });
        ConnectionCheck::try_new(
            id,
            status,
            Vec::new(),
            (status != ConnectionCheckStatus::Passed)
                .then(|| format!("{}_diagnostic", id.as_str())),
            summary,
            details,
            None,
        )
        .unwrap()
    }

    fn action(id: ConnectionActionKind, instruction: &str) -> ConnectionAction {
        ConnectionAction::try_new(id, instruction).unwrap()
    }

    fn verification(
        mut checks: Vec<ConnectionCheck>,
        mut actions: Vec<ConnectionAction>,
    ) -> ConnectionVerificationReport {
        checks.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        actions.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-20T00:00:00Z").unwrap(),
            checks,
            actions,
        )
        .unwrap()
    }

    fn report(
        operation: CommandOperation,
        setup_applied: Option<bool>,
        checks: Vec<ConnectionCheck>,
        actions: Vec<ConnectionAction>,
    ) -> ConnectionCommandReport {
        ConnectionCommandReport::from_verification(
            operation,
            setup_applied,
            Path::new("/runtime"),
            connection("workflow"),
            &verification(checks, actions),
        )
    }

    fn concise(report: &ConnectionCommandReport) -> String {
        render_command_report(OutputFormat::Human(HumanOutputDetail::Concise), report)
            .unwrap()
            .output
    }

    fn ready_check() -> ConnectionCheck {
        check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Passed,
            "Managed configuration is ready",
            None,
        )
    }

    fn failed_check() -> ConnectionCheck {
        check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Failed,
            "Managed Codex configuration is unavailable",
            Some(json!({
                "config_target": "/home/user/.codex/config.toml",
                "integration_revision": "revision_secret",
            })),
        )
    }

    fn activity_checks() -> Vec<ConnectionCheck> {
        vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
                "Managed configuration is ready",
                None,
            ),
            check(
                ConnectionCheckKind::HostExecutable,
                ConnectionCheckStatus::Passed,
                "Codex is available",
                None,
            ),
            check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
                "MCP self-test passed",
                None,
            ),
            check(
                ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::Passed,
                "No separate trust action applies",
                Some(json!({"applicable": false})),
            ),
            check(
                ConnectionCheckKind::GuardFiles,
                ConnectionCheckStatus::Passed,
                "Guard files are ready",
                None,
            ),
            check(
                ConnectionCheckKind::HostSession,
                ConnectionCheckStatus::Pending,
                "Managed host connection use has not been observed",
                None,
            ),
            check(
                ConnectionCheckKind::RequiredTools,
                ConnectionCheckStatus::Pending,
                "Current managed host has not reported tools/list",
                Some(json!({"required_tools": ["volicord.list_projects"]})),
            ),
            check(
                ConnectionCheckKind::ToolRoundTrip,
                ConnectionCheckStatus::Pending,
                "Current managed host has not completed the designated read-only call",
                None,
            ),
            check(
                ConnectionCheckKind::GuardObservation,
                ConnectionCheckStatus::Pending,
                "Required Guard hook phases have not been observed",
                Some(json!({
                    "installation_ids": ["guard_installation_secret"],
                    "missing_required_phases": ["post_tool", "prompt_capture", "pre_tool"],
                })),
            ),
        ]
    }

    fn observe_action() -> ConnectionAction {
        action(
            ConnectionActionKind::ObserveCodex,
            "Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.",
        )
    }

    #[test]
    fn concise_init_outputs_are_exact_for_complete_action_required_and_applied_failure() {
        let complete = report(
            CommandOperation::Init,
            Some(true),
            vec![ready_check()],
            Vec::new(),
        );
        assert_eq!(
            concise(&complete),
            concat!(
                "Volicord setup is ready.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n",
            )
        );

        let action_required = report(
            CommandOperation::Init,
            Some(true),
            activity_checks(),
            vec![observe_action()],
        );
        assert_eq!(
            concise(&action_required),
            concat!(
                "Volicord setup was applied and needs one more step.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 0 blocked, 4 waiting, 0 failed\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Next\n",
                "  action.host.observe_activity: Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics.\n",
            )
        );

        let failed = report(
            CommandOperation::Init,
            Some(true),
            vec![failed_check()],
            vec![action(
                ConnectionActionKind::RepairManagedConfig,
                "Repair the managed Codex configuration",
            )],
        );
        assert_eq!(
            concise(&failed),
            concat!(
                "Volicord setup was applied, but verification failed.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 0 waiting, 1 failed\n\n",
                "Problems\n",
                "  Managed Codex configuration is unavailable\n\n",
                "Next\n",
                "  action.managed_config.repair: Repair the managed Codex configuration\n\n",
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics.\n",
            )
        );

        let not_applied = ConnectionCommandReport::setup_failure(
            CommandOperation::Init,
            Path::new("/runtime"),
            connection("workflow"),
            "Setup migration could not be completed",
            json!({"retry_arguments": ["init", "--verbose"]}),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            concise(&not_applied),
            concat!(
                "Volicord setup could not be applied.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 0 waiting, 1 failed\n\n",
                "Problems\n",
                "  setup.partial_application: Setup migration could not be completed\n",
                "    Actual: partial setup application\n",
                "    Expected: complete setup application\n",
                "    Finding: finding.setup.partial_application\n\n",
                "Next\n",
                "  action.connection.retry_setup: Resolve the typed setup failure and rerun the setup operation\n\n",
                "Run the same setup command with --verbose for detailed diagnostics.\n",
            )
        );
    }

    #[test]
    fn concise_status_outputs_are_exact_for_all_aggregate_states() {
        let complete = report(
            CommandOperation::Status,
            None,
            vec![ready_check()],
            Vec::new(),
        );
        assert_eq!(
            concise(&complete),
            concat!(
                "Codex connection is ready.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n",
            )
        );

        let action_required = report(
            CommandOperation::Status,
            None,
            activity_checks(),
            vec![observe_action()],
        );
        assert_eq!(
            concise(&action_required),
            concat!(
                "Codex connection is configured and waiting for activity.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 0 blocked, 4 waiting, 0 failed\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Next\n",
                "  action.host.observe_activity: Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics.\n",
            )
        );

        let failed = report(
            CommandOperation::Status,
            None,
            vec![failed_check()],
            Vec::new(),
        );
        assert_eq!(
            concise(&failed),
            concat!(
                "Codex connection needs attention.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 0 waiting, 1 failed\n\n",
                "Problems\n",
                "  Managed Codex configuration is unavailable\n\n",
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics.\n",
            )
        );
    }

    #[test]
    fn concise_verify_action_required_output_has_an_active_verification_headline() {
        let action_required = report(
            CommandOperation::Verify,
            None,
            activity_checks(),
            vec![observe_action()],
        );
        assert_eq!(
            concise(&action_required),
            concat!(
                "Verification completed: 5 ready, 4 waiting.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 0 blocked, 4 waiting, 0 failed\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Next\n",
                "  action.host.observe_activity: Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Rerun active verification with `volicord connection verify codex --repo /workspace/product --home /runtime --verbose` for detailed diagnostics.\n",
            )
        );

        let failed = report(
            CommandOperation::Verify,
            None,
            vec![failed_check()],
            Vec::new(),
        );
        let failed_output = concise(&failed);
        assert!(failed_output.contains(
            "Rerun active verification with `volicord connection verify codex --repo /workspace/product --home /runtime --verbose` for detailed diagnostics."
        ));

        let complete = report(
            CommandOperation::Verify,
            None,
            vec![ready_check()],
            Vec::new(),
        );
        assert!(!concise(&complete).contains("--verbose"));
    }

    #[test]
    fn concise_setup_guidance_distinguishes_applied_dry_run_and_pre_apply_results() {
        for operation in [CommandOperation::Init, CommandOperation::Add] {
            let applied = report(
                operation,
                Some(true),
                vec![check(
                    ConnectionCheckKind::HostSession,
                    ConnectionCheckStatus::Pending,
                    "Managed host connection use has not been observed",
                    None,
                )],
                Vec::new(),
            );
            let output = concise(&applied);
            assert!(output.contains(
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics."
            ));
            assert!(!output.contains("same setup command"));

            let not_applied = ConnectionCommandReport::setup_failure(
                operation,
                Path::new("/runtime"),
                connection("workflow"),
                "Setup could not be applied",
                json!({"retryable": true}),
                Vec::new(),
            )
            .unwrap();
            assert!(concise(&not_applied)
                .contains("Run the same setup command with --verbose for detailed diagnostics."));

            let dry_run = ConnectionCommandReport::setup_dry_run(
                operation,
                Path::new("/runtime"),
                connection("workflow"),
                None,
                Vec::new(),
                &[],
            )
            .unwrap();
            assert!(concise(&dry_run)
                .contains("Run the same dry-run command with --verbose for detailed diagnostics."));
        }
    }

    #[test]
    fn concise_connection_diagnostic_uses_shared_structured_guidance() {
        let mut report = report(
            CommandOperation::Status,
            None,
            vec![failed_check()],
            Vec::new(),
        );
        report.connection = CommandConnection::new(
            "connection_1",
            "codex",
            "project",
            "workflow",
            Path::new("/workspace/product repo's"),
            "/workspace/product repo's/.codex/config.toml",
        );

        assert!(concise(&report).contains(concat!(
            "For detailed current Connection diagnostics, run the verbose status command with:\n\n",
            "  Host: codex\n",
            "  Repository: /workspace/product repo's\n",
            "  Runtime home: /runtime\n",
            "  Scope: shared\n",
            "  Verbose output: required."
        )));

        report.operation = CommandOperation::Verify;
        assert!(concise(&report).contains(concat!(
            "For detailed diagnostics, rerun active verification with:\n\n",
            "  Host: codex\n",
            "  Repository: /workspace/product repo's\n",
            "  Runtime home: /runtime\n",
            "  Scope: shared\n",
            "  Verbose output: required."
        )));
    }

    #[test]
    fn concise_mode_outputs_use_the_typed_transition_result() {
        let changed = ConnectionCommandReport::mode_transition(
            Path::new("/runtime"),
            connection("read_only"),
            true,
            "workflow".to_owned(),
            "read_only".to_owned(),
            "revision_before".to_owned(),
            "revision_after".to_owned(),
            vec!["guard_installation_secret".to_owned()],
        )
        .unwrap();
        assert_eq!(
            concise(&changed),
            concat!(
                "Connection mode changed from workflow to read_only.\n\n",
                "Repository: /workspace/product\n",
                "Mode: read_only\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n\n",
                "Next\n",
                "  action.host.reload_after_configuration_change: Restart or reload Codex, then use the current Volicord integration so new runtime and Guard observations bind revision revision_after\n",
            )
        );

        let no_op = ConnectionCommandReport::mode_transition(
            Path::new("/runtime"),
            connection("workflow"),
            false,
            "workflow".to_owned(),
            "workflow".to_owned(),
            "revision_same".to_owned(),
            "revision_same".to_owned(),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            concise(&no_op),
            concat!(
                "Connection mode is already workflow.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n",
            )
        );

        let mut changed_with_diagnostics = changed.clone();
        changed_with_diagnostics.status = ConnectionStatus::Failed;
        changed_with_diagnostics.checks = vec![failed_check()];
        changed_with_diagnostics.actions.clear();
        let diagnostics_output = concise(&changed_with_diagnostics);
        assert!(diagnostics_output.contains(
            "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics."
        ));
        assert!(!diagnostics_output.contains("same connection mode command"));

        let pre_mutation_failure = report(
            CommandOperation::Mode,
            None,
            vec![failed_check()],
            Vec::new(),
        );
        assert!(concise(&pre_mutation_failure).contains(
            "Run the same connection mode command with --verbose for detailed diagnostics."
        ));
    }

    #[test]
    fn concise_removal_outputs_distinguish_final_and_shared_membership() {
        let final_membership = ConnectionCommandReport::removal(
            Path::new("/runtime"),
            connection("workflow"),
            true,
            true,
            0,
        )
        .unwrap();
        assert_eq!(
            concise(&final_membership),
            concat!(
                "Connection membership and Connection record were removed.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n",
            )
        );

        let shared_membership = ConnectionCommandReport::removal(
            Path::new("/runtime"),
            connection("workflow"),
            true,
            false,
            1,
        )
        .unwrap();
        assert_eq!(
            concise(&shared_membership),
            concat!(
                "Connection membership was removed; the shared Connection remains in use.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n",
            )
        );

        for applied in [&final_membership, &shared_membership] {
            let output = concise(applied);
            assert!(!output.contains("--verbose"));
            assert!(!output.contains("connection status"));
        }

        let pre_mutation_failure = report(
            CommandOperation::Remove,
            None,
            vec![failed_check()],
            Vec::new(),
        );
        assert!(concise(&pre_mutation_failure).contains(
            "Run the same connection remove command with --verbose for detailed diagnostics."
        ));
    }

    #[test]
    fn concise_dry_runs_group_typed_changes_without_rendering_targets() {
        let setup = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Init,
            Path::new("/runtime"),
            connection("workflow"),
            None,
            vec![
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::ManagedHostConfiguration,
                    PlannedChangeOperation::Update,
                    "/home/user/.codex/config.toml",
                ),
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::GuardManagedFile,
                    PlannedChangeOperation::Create,
                    "/workspace/product/.codex/hooks.json",
                ),
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::GuardManagedFile,
                    PlannedChangeOperation::Update,
                    "/workspace/product/AGENTS.md",
                ),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(
            concise(&setup),
            concat!(
                "Volicord setup changes are ready to review.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 5 waiting, 0 failed\n\n",
                "Planned changes\n",
                "  1 managed Codex configuration change\n",
                "  2 Guard managed-file changes\n\n",
                "Waiting\n",
                "  Codex managed session\n",
                "  Guard hook activity\n",
                "  Guard managed-file plan was inspected\n",
                "  Managed Codex configuration plan was inspected\n",
                "  Setup changes are ready to apply\n\n",
                "Next\n",
                "  1. action.connection.apply_setup: Run init without --dry-run to apply the planned setup changes\n",
                "  2. action.host.observe_activity: After setup is applied, restart or reload Codex and use the connection so actual Codex and Guard activity can be observed\n\n",
                "Run the same dry-run command with --verbose for detailed diagnostics.\n",
            )
        );

        let removal = ConnectionCommandReport::removal_dry_run(
            Path::new("/runtime"),
            connection("workflow"),
            vec![
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::ManagedHostConfiguration,
                    PlannedChangeOperation::Remove,
                    "/home/user/.codex/config.toml",
                ),
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::GuardRegistrySetup,
                    PlannedChangeOperation::Remove,
                    "guard_installation_secret",
                ),
                PlannedConnectionChange::new(
                    PlannedConnectionChangeKind::ConnectionMembership,
                    PlannedChangeOperation::Remove,
                    "/workspace/product",
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            concise(&removal),
            concat!(
                "Connection removal is ready to review.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 1 waiting, 0 failed\n\n",
                "Planned changes\n",
                "  1 managed Codex configuration change\n",
                "  1 Guard Registry change\n",
                "  1 Connection membership change\n\n",
                "Waiting\n",
                "  Selected Connection membership removal is ready to apply\n\n",
                "Next\n",
                "  action.connection.apply_removal: Run connection remove without --dry-run to apply the planned removal\n\n",
                "Run the same dry-run command with --verbose for detailed diagnostics.\n",
            )
        );
    }

    #[test]
    fn concise_output_orders_problems_before_waiting_and_hides_support_data() {
        let report = report(
            CommandOperation::Status,
            None,
            vec![
                failed_check(),
                check(
                    ConnectionCheckKind::HostSession,
                    ConnectionCheckStatus::Pending,
                    "Managed host activity is pending",
                    Some(json!({
                        "runtime_session_id": "runtime_session_secret",
                        "required_tools": ["volicord.list_projects"],
                    })),
                ),
                check(
                    ConnectionCheckKind::GuardObservation,
                    ConnectionCheckStatus::Pending,
                    "Guard activity is pending",
                    Some(json!({
                        "installation_ids": ["guard_installation_secret"],
                        "missing_required_phases": ["pre_tool"],
                    })),
                ),
            ],
            vec![
                action(ConnectionActionKind::ObserveCodex, "Observe Codex activity"),
                action(
                    ConnectionActionKind::RepairManagedConfig,
                    "Repair managed configuration",
                ),
            ],
        );
        let output = concise(&report);
        assert!(output.find("Problems\n").unwrap() < output.find("Waiting\n").unwrap());
        assert_eq!(output.matches("Codex managed session").count(), 1);
        assert_eq!(output.matches("Guard hook activity").count(), 1);
        for hidden in [
            "Operation:",
            "Status:",
            "Dry run:",
            "Runtime home:",
            "Runtime Home",
            "connection_1",
            "/home/user/.codex/config.toml",
            "managed_config_diagnostic",
            "revision_secret",
            "runtime_session_secret",
            "guard_installation_secret",
            "volicord.list_projects",
            "observe_codex",
            "repair_managed_config",
            "Details: {",
        ] {
            assert!(
                !output.contains(hidden),
                "unexpected support data: {hidden}"
            );
        }
        assert!(output.ends_with("diagnostics.\n"));
        assert!(!output.ends_with("\n\n"));
    }

    #[test]
    fn concise_waiting_uses_only_canonically_pending_codex_activities() {
        let cases = [
            (
                Some(ConnectionCheckStatus::Pending),
                Some(ConnectionCheckStatus::Pending),
                Some(ConnectionCheckStatus::Pending),
                Some(
                    "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call",
                ),
            ),
            (
                Some(ConnectionCheckStatus::Passed),
                Some(ConnectionCheckStatus::Failed),
                Some(ConnectionCheckStatus::Pending),
                Some("  Read-only Volicord tool call"),
            ),
            (
                Some(ConnectionCheckStatus::Failed),
                Some(ConnectionCheckStatus::Pending),
                Some(ConnectionCheckStatus::Pending),
                Some(
                    "  Codex tool activity: tools/list and the designated read-only tool call",
                ),
            ),
            (
                Some(ConnectionCheckStatus::Passed),
                Some(ConnectionCheckStatus::Passed),
                Some(ConnectionCheckStatus::Pending),
                Some("  Read-only Volicord tool call"),
            ),
            (
                Some(ConnectionCheckStatus::Pending),
                None,
                None,
                Some("  Codex managed session"),
            ),
            (
                Some(ConnectionCheckStatus::Passed),
                Some(ConnectionCheckStatus::Passed),
                Some(ConnectionCheckStatus::Passed),
                None,
            ),
            (
                Some(ConnectionCheckStatus::Failed),
                Some(ConnectionCheckStatus::Failed),
                Some(ConnectionCheckStatus::Failed),
                None,
            ),
        ];

        for (host, tools, round_trip, expected) in cases {
            let checks = [
                (ConnectionCheckKind::HostSession, host),
                (ConnectionCheckKind::RequiredTools, tools),
                (ConnectionCheckKind::ToolRoundTrip, round_trip),
            ]
            .into_iter()
            .filter_map(|(id, status)| status.map(|status| check(id, status, id.as_str(), None)))
            .collect::<Vec<_>>();
            let waiting = super::render_waiting_checks(&checks);
            assert_eq!(
                waiting,
                expected.into_iter().map(str::to_owned).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn concise_waiting_keeps_guard_phase_order_and_blocked_checks_out_of_waiting() {
        let all_guard_phases = check(
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Pending,
            "Guard activity is pending",
            Some(json!({
                "missing_required_phases": ["prompt_capture", "post_tool", "pre_tool"],
            })),
        );
        assert_eq!(
            super::render_waiting_checks(&[all_guard_phases]),
            vec!["  Guard hook activity: pre_tool, post_tool, prompt_capture"]
        );

        let subset = check(
            ConnectionCheckKind::GuardObservation,
            ConnectionCheckStatus::Pending,
            "Guard activity is pending",
            Some(json!({
                "missing_required_phases": ["prompt_capture", "post_tool"],
            })),
        );
        assert_eq!(
            super::render_waiting_checks(&[subset]),
            vec!["  Guard hook activity: post_tool, prompt_capture"]
        );

        let cause =
            volicord_types::DiagnosticFindingId::parse("finding.initialize_failed").unwrap();
        let failed = check(
            ConnectionCheckKind::HostSession,
            ConnectionCheckStatus::Failed,
            "MCP initialize failed",
            None,
        )
        .with_cause_finding_ids(vec![cause.clone()])
        .unwrap();
        let blocked_tools = check(
            ConnectionCheckKind::RequiredTools,
            ConnectionCheckStatus::Pending,
            "tools/list is pending",
            None,
        )
        .blocked_by(vec![cause.clone()])
        .unwrap();
        let blocked_round_trip = check(
            ConnectionCheckKind::ToolRoundTrip,
            ConnectionCheckStatus::Pending,
            "Read-only call is pending",
            None,
        )
        .blocked_by(vec![cause])
        .unwrap();
        let checks = vec![failed, blocked_tools, blocked_round_trip];
        assert!(super::render_waiting_checks(&checks).is_empty());
        let counts = super::CheckCounts::from_checks(&checks);
        assert_eq!(
            (counts.ready, counts.blocked, counts.waiting, counts.failed),
            (0, 2, 0, 1)
        );
    }

    #[test]
    fn json_rendering_remains_the_pretty_serialized_report_plus_one_newline() {
        let report = report(
            CommandOperation::Verify,
            None,
            activity_checks(),
            vec![observe_action()],
        );
        let expected = format!(
            "{}\n",
            serde_json::to_string_pretty(&report.diagnostic_report().unwrap()).unwrap()
        );
        let rendered = render_command_report(OutputFormat::Json, &report).unwrap();
        assert_eq!(rendered.output, expected);
    }
}
