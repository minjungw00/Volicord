use std::path::Path;

use serde_json::Value;
use volicord_types::{
    ConnectionCheck, ConnectionCheckKind, ConnectionCheckStatus, ConnectionStatus, HostKind,
    HostScope,
};

use super::report::{
    projected_activation_plan, projected_check_root_cause_ids, projected_root_cause_ids,
    CommandOperation, ConnectionCommandReport, ConnectionCommandResult, SetupDisposition,
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
    if report.operation == CommandOperation::Verify {
        sections.push(
            "Operation: active verification\nEvidence class: active_verification\nSide effects: rollback-only Store writeability probes, disposable protocol conformance, diagnostic reconciliation, verification-report persistence\nDoes not prove: managed-host operation, future launch availability, Product Repository correctness outside checked contracts"
                .to_owned(),
        );
    }
    sections.push(format!(
        "Repository: {}\nMode: {}\nActivation: {}\nHook activation: {}\nChecks: {}",
        report.connection.repository,
        report.connection.mode,
        report.activation_state.as_str(),
        report.hook_activation_state.as_str(),
        counts.render(true)
    ));
    if let Some(mcp) = render_mcp_verification_summary(&report.checks) {
        sections.push(mcp);
    }
    if let Some(guard) = render_guard_verification_summary(&report.checks) {
        sections.push(guard);
    }

    if let Some(planned_changes) = report
        .planned_changes
        .as_deref()
        .filter(|changes| !changes.is_empty())
    {
        sections.push(render_planned_changes(planned_changes));
    }
    if let Some(plan) = render_activation_plan(report)? {
        sections.push(plan);
    }

    let problems = render_root_problems(report)?;
    if !problems.is_empty() {
        sections.push(format!("Problems\n{}", problems.join("\n")));
    }

    let waiting = render_waiting_checks(&report.checks);
    if !waiting.is_empty() {
        sections.push(format!("Waiting\n{}", waiting.join("\n")));
    }

    if let Some(hint) = concise_diagnostic_hint(report) {
        sections.push(hint);
    }
    Ok(format!("{}\n", sections.join("\n\n")))
}

fn render_mcp_verification_summary(checks: &[ConnectionCheck]) -> Option<String> {
    let details = checks
        .iter()
        .find(|check| check.id() == ConnectionCheckKind::McpServer)?
        .details()?
        .as_object();
    if !details.contains_key("preflight") {
        return None;
    }
    let Some(active) = details
        .get("last_active_verification")
        .and_then(Value::as_object)
    else {
        return Some("Storage writeability: not checked".to_owned());
    };
    let observed_at = active
        .get("observed_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let source = active
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let registry_write = active
        .get("registry_write")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let project_writes = active
        .get("project_writes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|project| {
            Some(format!(
                "{}={}",
                project.get("project_id")?.as_str()?,
                project.get("state_write")?.as_str()?
            ))
        })
        .collect::<Vec<_>>();
    let writeability = if project_writes.is_empty() {
        registry_write.to_owned()
    } else {
        format!(
            "Registry={registry_write}; projects {}",
            project_writes.join(", ")
        )
    };
    Some(format!(
        "Active verification: {observed_at} ({source})\nStorage writeability: {writeability}"
    ))
}

fn render_guard_verification_summary(checks: &[ConnectionCheck]) -> Option<String> {
    let ambient = checks
        .iter()
        .find(|check| check.id() == ConnectionCheckKind::AmbientHookCoverage);
    let correlated = checks
        .iter()
        .find(|check| check.id() == ConnectionCheckKind::CorrelatedGuardVerification);
    if ambient.is_none() && correlated.is_none() {
        return None;
    }
    let mut lines = Vec::new();
    if let Some(check) = ambient {
        lines.push(format!(
            "Hook installation and ambient execution: {}",
            check.status().as_str()
        ));
    }
    if let Some(check) = correlated {
        lines.push(format!(
            "Correlated Guard verification: {}",
            check.status().as_str()
        ));
        if let Some(reason) = check
            .details()
            .and_then(|details| details.as_object().get("latest_attempt"))
            .and_then(Value::as_object)
            .and_then(|attempt| attempt.get("repair_reason"))
            .and_then(Value::as_str)
        {
            lines.push(format!("Reason: {reason}"));
        }
    }
    Some(lines.join("\n"))
}

pub(super) fn render_activation_plan(
    report: &ConnectionCommandReport,
) -> Result<Option<String>, ConnectionCommandError> {
    let plan = projected_activation_plan(report)?;
    if plan.required_steps().is_empty() && plan.optional_diagnostics().is_empty() {
        return Ok(None);
    }
    let mut sections = Vec::new();
    if !plan.required_steps().is_empty() {
        let numbered = plan.required_steps().len() > 1;
        let steps = plan
            .required_steps()
            .iter()
            .enumerate()
            .map(|(index, step)| {
                if numbered {
                    format!("  {}. {}", index + 1, step.instruction())
                } else {
                    format!("  {}", step.instruction())
                }
            })
            .collect::<Vec<_>>();
        sections.push(format!("Required next steps\n{}", steps.join("\n")));
    }
    if !plan.optional_diagnostics().is_empty() {
        sections.push(format!(
            "Optional active diagnostics\n{}",
            plan.optional_diagnostics()
                .iter()
                .map(|step| format!("  {}", step.instruction()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    Ok(Some(sections.join("\n\n")))
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
            Some(ConnectionCommandResult::Setup {
                disposition: SetupDisposition::Committed,
                ..
            }) if has_nonpassing_check => Some(current_status_diagnostic_hint(report)),
            Some(ConnectionCommandResult::Setup { disposition, .. })
                if *disposition != SetupDisposition::Committed
                    && report.status == ConnectionStatus::Failed
                    && has_nonpassing_check =>
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
            ConnectionStatus::ActionRequired if counts.failed > 0 => {
                "Codex connection needs a repair action.".to_owned()
            }
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

    let disposition = match report.result {
        Some(ConnectionCommandResult::Setup { disposition, .. }) => disposition,
        _ => SetupDisposition::Preserved,
    };
    match (report.status, disposition) {
        (ConnectionStatus::Complete, _) => "Volicord setup is ready.".to_owned(),
        (ConnectionStatus::ActionRequired, SetupDisposition::Committed) => format!(
            "Setup committed; {} host-owned activation {} {}.",
            report.activation_plan.required_steps().len(),
            if report.activation_plan.required_steps().len() == 1 {
                "step"
            } else {
                "steps"
            },
            if report.activation_plan.required_steps().len() == 1 {
                "remains"
            } else {
                "remain"
            },
        ),
        (ConnectionStatus::ActionRequired, _) => format!(
            "Volicord setup requires {} activation {}.",
            report.activation_plan.required_steps().len(),
            if report.activation_plan.required_steps().len() == 1 {
                "step"
            } else {
                "steps"
            }
        ),
        (ConnectionStatus::Failed, SetupDisposition::Committed) => {
            "Volicord setup was committed, but verification failed.".to_owned()
        }
        (ConnectionStatus::Failed, SetupDisposition::RolledBack) => {
            "Volicord setup failed; committed changes were rolled back.".to_owned()
        }
        (ConnectionStatus::Failed, SetupDisposition::Preserved) => {
            "Volicord setup failed before commit; existing state was preserved.".to_owned()
        }
        (ConnectionStatus::Failed, SetupDisposition::PartiallyRolledBack) => {
            "Volicord setup failed and was only partially rolled back.".to_owned()
        }
        (ConnectionStatus::Failed, SetupDisposition::Planned) => {
            "Volicord setup plan could not be committed.".to_owned()
        }
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
    const KINDS: [PlannedConnectionChangeKind; 7] = [
        PlannedConnectionChangeKind::RuntimeHomeInitialization,
        PlannedConnectionChangeKind::ProjectRegistration,
        PlannedConnectionChangeKind::ManagedHostConfiguration,
        PlannedConnectionChangeKind::HookDefinition,
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
        (PlannedConnectionChangeKind::HookDefinition, true) => "project hook-definition change",
        (PlannedConnectionChangeKind::HookDefinition, false) => "project hook-definition changes",
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
        derive_integration_activation_state, ActivationStep, ActivationStepId, ConnectionCheck,
        ConnectionCheckDetails, ConnectionCheckKind, ConnectionCheckStatus, ConnectionStatus,
        ConnectionVerificationReport, HookActivationState, IntegrationActivationPlan,
        IntegrationActivationState, UtcTimestamp,
    };

    use crate::connection_command::{
        args::{HumanOutputDetail, OutputFormat},
        output::report::{
            render_command_report, CommandConnection, CommandOperation, ConnectionCommandReport,
            RuntimeHomePublicationStatus, SetupDisposition, SetupFailureDiagnostic,
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

    fn action(id: ActivationStepId, instruction: &str) -> ActivationStep {
        ActivationStep::try_new(id, Vec::new(), instruction).unwrap()
    }

    fn verification(
        mut checks: Vec<ConnectionCheck>,
        actions: Vec<ActivationStep>,
    ) -> ConnectionVerificationReport {
        checks.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        let state = derive_integration_activation_state(&checks, HookActivationState::Unknown);
        let activation_plan =
            IntegrationActivationPlan::try_new(state, actions, Vec::new()).unwrap();
        ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-20T00:00:00Z").unwrap(),
            checks,
            activation_plan,
        )
        .unwrap()
    }

    fn report(
        operation: CommandOperation,
        setup_disposition: Option<SetupDisposition>,
        checks: Vec<ConnectionCheck>,
        actions: Vec<ActivationStep>,
    ) -> ConnectionCommandReport {
        ConnectionCommandReport::from_verification(
            operation,
            setup_disposition,
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

    macro_rules! assert_current_concise {
        ($actual:expr, $previous:expr $(,)?) => {{
            let actual = $actual;
            let previous = $previous;
            assert_eq!(actual.lines().next(), previous.lines().next());
            assert!(actual.contains("\nActivation: "), "{actual}");
            assert!(actual.contains("\nHook activation: "), "{actual}");
            assert!(actual.contains("\nChecks: "), "{actual}");
            assert!(!actual.contains("action.host.observe_activity"), "{actual}");
            assert!(!actual.contains("\nNext\n"), "{actual}");
            assert!(!actual.contains("Host-owned activation steps"), "{actual}");
        }};
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

    fn observe_action() -> ActivationStep {
        action(
            ActivationStepId::RequestIntegrationVerification,
            "Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.",
        )
    }

    #[test]
    fn concise_init_outputs_are_exact_for_complete_action_required_and_applied_failure() {
        let complete = report(
            CommandOperation::Init,
            Some(SetupDisposition::Committed),
            vec![ready_check()],
            Vec::new(),
        );
        assert_current_concise!(
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
            Some(SetupDisposition::Committed),
            activity_checks(),
            vec![observe_action()],
        );
        assert_current_concise!(
            concise(&action_required),
            concat!(
                "Setup committed; 1 host-owned activation step remains.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 0 blocked, 4 waiting, 0 failed\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Required next steps\n",
                "  action.host.observe_activity: Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics.\n",
            )
        );

        let failed = report(
            CommandOperation::Init,
            Some(SetupDisposition::Committed),
            vec![failed_check()],
            vec![action(
                ActivationStepId::RepairManagedConfiguration,
                "Repair the managed Codex configuration",
            )],
        );
        assert_current_concise!(
            concise(&failed),
            concat!(
                "Volicord setup was committed, but verification failed.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 0 waiting, 1 failed\n\n",
                "Problems\n",
                "  Managed Codex configuration is unavailable\n\n",
                "Required next steps\n",
                "  action.managed_config.repair: Repair the managed Codex configuration\n\n",
                "Run `volicord connection status codex --repo /workspace/product --home /runtime --verbose` for detailed current Connection diagnostics.\n",
            )
        );

        let not_applied = ConnectionCommandReport::setup_failure(
            CommandOperation::Init,
            Path::new("/runtime"),
            connection("workflow"),
            SetupDisposition::Preserved,
            RuntimeHomePublicationStatus::NotPublished,
            SetupFailureDiagnostic::TransactionFailed,
            "Setup migration could not be completed",
            json!({"retry_arguments": ["init", "--verbose"]}),
            IntegrationActivationPlan::empty(IntegrationActivationState::Failed),
        )
        .unwrap();
        assert_current_concise!(
            concise(&not_applied),
            concat!(
                "Volicord setup failed before commit; existing state was preserved.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 0 waiting, 1 failed\n\n",
                "Problems\n",
                "  setup.transaction_failed: Setup migration could not be completed\n",
                "    Actual: preserved\n",
                "    Expected: committed setup transaction\n",
                "    Finding: finding.setup.transaction_failed\n\n",
                "Required next steps\n",
                "  action.connection.retry_setup: Resolve the typed setup failure and rerun the setup operation\n\n",
                "Run the same setup command with --verbose for detailed diagnostics.\n",
            )
        );
    }

    #[test]
    fn changed_hook_init_renders_the_exact_host_owned_activation_sequence() {
        let activation_plan = IntegrationActivationPlan::try_new(
            IntegrationActivationState::HostReloadRequired,
            vec![
                ActivationStep::try_new(
                    ActivationStepId::ReadConnectionStatus,
                    vec![ActivationStepId::RequestIntegrationVerification],
                    "After the agent finishes, read connection status.",
                )
                .unwrap(),
                ActivationStep::try_new(
                    ActivationStepId::RequestIntegrationVerification,
                    vec![ActivationStepId::ReviewProjectHooks],
                    "Start a new Codex conversation and request: \"Run the Volicord integration verification.\"",
                )
                .unwrap(),
                ActivationStep::try_new(
                    ActivationStepId::ReviewProjectHooks,
                    vec![ActivationStepId::ReloadCodex],
                    "Review the current project hooks.",
                )
                .unwrap(),
                ActivationStep::try_new(
                    ActivationStepId::ReloadCodex,
                    Vec::new(),
                    "Restart or reload Codex in this repository.",
                )
                .unwrap(),
            ],
            vec![ActivationStep::try_new(
                ActivationStepId::RunOptionalActiveDiagnostics,
                Vec::new(),
                "Run `volicord connection verify` only when optional active diagnostics are needed",
            )
            .unwrap()],
        )
        .unwrap();
        let expected_required_steps = activation_plan.required_steps().len();
        let verification = ConnectionVerificationReport::try_new_with_hook_activation(
            UtcTimestamp::parse("2026-07-20T00:00:00Z").unwrap(),
            vec![ready_check()],
            HookActivationState::ReviewRequiredBySetup,
            activation_plan,
        )
        .unwrap();
        let report = ConnectionCommandReport::from_verification(
            CommandOperation::Init,
            Some(SetupDisposition::Committed),
            Path::new("/runtime"),
            connection("workflow"),
            &verification,
        );
        let output = concise(&report);
        assert!(output.starts_with("Setup committed; 4 host-owned activation steps remain.\n\n"));
        assert_eq!(output.matches("Required next steps\n").count(), 1);
        assert_eq!(
            output
                .lines()
                .filter(|line| {
                    line.trim_start()
                        .split_once(". ")
                        .is_some_and(|(number, _)| number.parse::<usize>().is_ok())
                })
                .count(),
            expected_required_steps
        );
        assert!(!output.contains("one more step"));
        assert!(!output.contains("\nNext\n"));
        assert!(!output.contains("Host-owned activation steps"));
        let expected = [
            "Required next steps",
            "1. Restart or reload Codex in this repository.",
            "2. Review the current project hooks.",
            "3. Start a new Codex conversation and request: \"Run the Volicord integration verification.\"",
            "4. After the agent finishes, read connection status.",
            "Optional active diagnostics",
            "`volicord connection verify`",
        ];
        let mut prior = 0;
        for text in expected {
            let offset = output[prior..]
                .find(text)
                .unwrap_or_else(|| panic!("missing activation instruction {text:?}: {output}"));
            prior += offset + text.len();
        }
    }

    #[test]
    fn concise_status_outputs_are_exact_for_all_aggregate_states() {
        let complete = report(
            CommandOperation::Status,
            None,
            vec![ready_check()],
            Vec::new(),
        );
        assert_current_concise!(
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
        assert_current_concise!(
            concise(&action_required),
            concat!(
                "Codex connection is configured and waiting for activity.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 0 blocked, 4 waiting, 0 failed\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Required next steps\n",
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
        assert_current_concise!(
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
        assert_current_concise!(
            concise(&action_required),
            concat!(
                "Verification completed: 5 ready, 4 waiting.\n\n",
                "Operation: active verification\n",
                "Evidence class: active_verification\n",
                "Side effects: rollback-only Store writeability probes, disposable protocol conformance, diagnostic reconciliation, verification-report persistence\n",
                "Does not prove: managed-host operation, future launch availability, Product Repository correctness outside checked contracts\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 0 blocked, 4 waiting, 0 failed\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Required next steps\n",
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
                Some(SetupDisposition::Committed),
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
                SetupDisposition::Preserved,
                RuntimeHomePublicationStatus::NotPublished,
                SetupFailureDiagnostic::TransactionFailed,
                "Setup could not be applied",
                json!({"retryable": true}),
                IntegrationActivationPlan::empty(IntegrationActivationState::Failed),
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
        assert_current_concise!(
            concise(&changed),
            concat!(
                "Connection mode changed from workflow to read_only.\n\n",
                "Repository: /workspace/product\n",
                "Mode: read_only\n",
                "Checks: 1 ready, 0 blocked, 0 waiting, 0 failed\n\n",
                "Required next steps\n",
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
        assert_current_concise!(
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
        changed_with_diagnostics.activation_plan =
            IntegrationActivationPlan::empty(IntegrationActivationState::Failed);
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
        assert_current_concise!(
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
        assert_current_concise!(
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
        )
        .unwrap();
        assert_current_concise!(
            concise(&setup),
            concat!(
                "Volicord setup changes are ready to review.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 0 ready, 0 blocked, 6 waiting, 0 failed\n\n",
                "Planned changes\n",
                "  1 managed Codex configuration change\n",
                "  2 Guard managed-file changes\n\n",
                "Waiting\n",
                "  Codex managed session\n",
                "  Guard hook activity\n",
                "  Guard managed-file plan was inspected\n",
                "  In-chat MCP and Guard integration verification has not completed\n",
                "  Managed Codex configuration plan was inspected\n",
                "  Setup changes are ready to apply\n\n",
                "Required next steps\n",
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
        assert_current_concise!(
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
                "Required next steps\n",
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
                action(
                    ActivationStepId::RequestIntegrationVerification,
                    "Observe Codex activity",
                ),
                action(
                    ActivationStepId::RepairManagedConfiguration,
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

    #[test]
    fn mcp_preflight_and_active_evidence_have_human_verbose_and_json_parity() {
        let preflight = json!({
            "status": "passed",
            "code": "mcp_server_preflight_passed",
            "diagnostic": "volicord mcp preflight passed",
            "evidence": {
                "configuration": "passed",
                "registry_read": "passed",
                "project_reads": [{
                    "project_id": "project_1",
                    "state_read": "passed"
                }],
                "schema_validation": "passed",
                "protocol_profiles": "passed",
                "host_contracts": "passed",
                "writeability": {
                    "status": "not_checked",
                    "requires": "connection_verify"
                },
                "side_effects": []
            }
        });
        let before = report(
            CommandOperation::Verify,
            None,
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
                "MCP preflight passed; active verification has not run",
                Some(json!({
                    "preflight": preflight.clone(),
                    "last_active_verification": null
                })),
            )],
            Vec::new(),
        );
        let before_human = concise(&before);
        let before_verbose =
            render_command_report(OutputFormat::Human(HumanOutputDetail::Verbose), &before)
                .unwrap()
                .output;
        let before_json: Value = serde_json::from_str(
            &render_command_report(OutputFormat::Json, &before)
                .unwrap()
                .output,
        )
        .unwrap();
        assert!(before_human.contains("Storage writeability: not checked"));
        assert!(before_verbose.contains("Storage writeability: not checked"));
        let before_details = &before_json["checks"][0]["details"];
        assert_eq!(before_details["preflight"], preflight);
        assert_eq!(before_details["last_active_verification"], Value::Null);

        let active = json!({
            "registry_write": "passed",
            "project_writes": [{
                "project_id": "project_1",
                "state_write": "passed"
            }],
            "protocol_conformance": [],
            "host_compatibility": [],
            "observed_at": "2026-07-25T01:02:03Z",
            "source": "connection_verify",
            "side_effects": [
                "rollback_only_registry_write_probe",
                "rollback_only_project_write_probe"
            ]
        });
        let after = report(
            CommandOperation::Verify,
            None,
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
                "MCP active verification passed",
                Some(json!({
                    "preflight": preflight.clone(),
                    "last_active_verification": active.clone()
                })),
            )],
            Vec::new(),
        );
        let after_human = concise(&after);
        let after_verbose =
            render_command_report(OutputFormat::Human(HumanOutputDetail::Verbose), &after)
                .unwrap()
                .output;
        let after_json: Value = serde_json::from_str(
            &render_command_report(OutputFormat::Json, &after)
                .unwrap()
                .output,
        )
        .unwrap();
        assert!(
            after_human.contains("Active verification: 2026-07-25T01:02:03Z (connection_verify)")
        );
        assert!(after_human
            .contains("Storage writeability: Registry=passed; projects project_1=passed"));
        assert!(after_verbose.contains("Active verification observed at: 2026-07-25T01:02:03Z"));
        assert!(after_verbose.contains("Active verification source: connection_verify"));
        assert!(after_verbose.contains("Registry writeability: passed"));
        assert!(after_verbose.contains("Project project_1 writeability: passed"));
        let after_details = &after_json["checks"][0]["details"];
        assert_eq!(after_details["preflight"], preflight);
        assert_eq!(after_details["last_active_verification"], active);
        assert!(after_details.get("self_test").is_none());
        assert!(after_details["preflight"].get("storage").is_none());
        assert_eq!(
            after_details["preflight"]["evidence"]["writeability"]["status"],
            "not_checked"
        );
    }
}
