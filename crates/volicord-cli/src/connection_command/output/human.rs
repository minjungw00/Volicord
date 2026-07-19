use volicord_types::{
    ConnectionAction, ConnectionActionKind, ConnectionCheck, ConnectionCheckKind,
    ConnectionCheckStatus, ConnectionStatus,
};

use super::report::{CommandOperation, ConnectionCommandReport, ConnectionCommandResult};
use crate::connection_command::{PlannedConnectionChange, PlannedConnectionChangeKind};

pub(super) fn render_command_report_concise(report: &ConnectionCommandReport) -> String {
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

    let problems = report
        .checks
        .iter()
        .filter(|check| check.status() == ConnectionCheckStatus::Failed)
        .map(|check| format!("  {}", check.summary()))
        .collect::<Vec<_>>();
    if !problems.is_empty() {
        sections.push(format!("Problems\n{}", problems.join("\n")));
    }

    let waiting = render_waiting_checks(&report.checks);
    if !waiting.is_empty() {
        sections.push(format!("Waiting\n{}", waiting.join("\n")));
    }

    if !report.actions.is_empty() {
        let numbered = report.actions.len() > 1;
        let actions = report
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                let instruction = concise_action_instruction(report.operation, action);
                if numbered {
                    format!("  {}. {instruction}", index + 1)
                } else {
                    format!("  {instruction}")
                }
            })
            .collect::<Vec<_>>();
        sections.push(format!("Next\n{}", actions.join("\n")));
    }

    sections.push("Run again with --verbose for detailed diagnostics.".to_owned());
    format!("{}\n", sections.join("\n\n"))
}

#[derive(Clone, Copy)]
pub(super) struct CheckCounts {
    pub(super) ready: usize,
    pub(super) waiting: usize,
    pub(super) failed: usize,
}

impl CheckCounts {
    pub(super) fn from_report(report: &ConnectionCommandReport) -> Self {
        let mut counts = Self {
            ready: 0,
            waiting: 0,
            failed: 0,
        };
        for check in &report.checks {
            match check.status() {
                ConnectionCheckStatus::Passed => counts.ready += 1,
                ConnectionCheckStatus::Pending => counts.waiting += 1,
                ConnectionCheckStatus::Failed => counts.failed += 1,
            }
        }
        counts
    }

    fn render(self, always_show_ready: bool) -> String {
        let mut parts = Vec::new();
        if always_show_ready || self.ready > 0 {
            parts.push(format!("{} ready", self.ready));
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
    if checks.iter().any(|check| {
        check.status() == ConnectionCheckStatus::Pending
            && matches!(
                check.id(),
                ConnectionCheckKind::HostSession
                    | ConnectionCheckKind::RequiredTools
                    | ConnectionCheckKind::ToolRoundTrip
            )
    }) {
        waiting.push(
            "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call"
                .to_owned(),
        );
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

fn concise_action_instruction(operation: CommandOperation, action: &ConnectionAction) -> &str {
    if operation == CommandOperation::Mode && action.id() == ConnectionActionKind::ReloadHost {
        "Restart or reload Codex, then use the current Volicord integration"
    } else {
        action.instruction()
    }
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
        ConnectionCheckKind, ConnectionCheckStatus, ConnectionVerificationReport, UtcTimestamp,
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
            (status != ConnectionCheckStatus::Passed)
                .then(|| format!("{}_diagnostic", id.as_str())),
            summary,
            details,
            None,
        )
        .unwrap()
    }

    fn action(id: ConnectionActionKind, instruction: &str) -> ConnectionAction {
        ConnectionAction::try_new(id, instruction, None).unwrap()
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
                "Checks: 1 ready\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 5 ready, 4 waiting\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Next\n",
                "  Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 0 ready, 1 failed\n\n",
                "Problems\n",
                "  Managed Codex configuration is unavailable\n\n",
                "Next\n",
                "  Repair the managed Codex configuration\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 0 ready, 1 failed\n\n",
                "Problems\n",
                "  Setup migration could not be completed\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 1 ready\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 5 ready, 4 waiting\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Next\n",
                "  Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 0 ready, 1 failed\n\n",
                "Problems\n",
                "  Managed Codex configuration is unavailable\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
            )
        );
    }

    #[test]
    fn concise_verify_action_required_output_has_an_active_verification_headline() {
        let report = report(
            CommandOperation::Verify,
            None,
            activity_checks(),
            vec![observe_action()],
        );
        assert_eq!(
            concise(&report),
            concat!(
                "Verification completed: 5 ready, 4 waiting.\n\n",
                "Repository: /workspace/product\n",
                "Mode: workflow\n",
                "Checks: 5 ready, 4 waiting\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity: pre_tool, post_tool, prompt_capture\n\n",
                "Next\n",
                "  Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
            )
        );
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
                "Checks: 1 ready\n\n",
                "Next\n",
                "  Restart or reload Codex, then use the current Volicord integration\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 1 ready\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
            )
        );
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
                "Checks: 1 ready\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 1 ready\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
            )
        );
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
                "Checks: 0 ready, 5 waiting\n\n",
                "Planned changes\n",
                "  1 managed Codex configuration change\n",
                "  2 Guard managed-file changes\n\n",
                "Waiting\n",
                "  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call\n",
                "  Guard hook activity\n",
                "  Guard managed-file plan was inspected\n",
                "  Managed Codex configuration plan was inspected\n",
                "  Setup changes are ready to apply\n\n",
                "Next\n",
                "  1. Run init without --dry-run to apply the planned setup changes\n",
                "  2. After setup is applied, restart or reload Codex and use the connection so actual Codex and Guard activity can be observed\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
                "Checks: 0 ready, 1 waiting\n\n",
                "Planned changes\n",
                "  1 managed Codex configuration change\n",
                "  1 Guard Registry change\n",
                "  1 Connection membership change\n\n",
                "Waiting\n",
                "  Selected Connection membership removal is ready to apply\n\n",
                "Next\n",
                "  Run connection remove without --dry-run to apply the planned removal\n\n",
                "Run again with --verbose for detailed diagnostics.\n",
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
        assert_eq!(output.matches("Codex session and tool activity").count(), 1);
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
    fn json_rendering_remains_the_pretty_serialized_report_plus_one_newline() {
        let report = report(
            CommandOperation::Verify,
            None,
            activity_checks(),
            vec![observe_action()],
        );
        let expected = format!("{}\n", serde_json::to_string_pretty(&report).unwrap());
        let rendered = render_command_report(OutputFormat::Json, &report).unwrap();
        assert_eq!(rendered.output, expected);
    }
}
