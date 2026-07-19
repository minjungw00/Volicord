use std::{path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use volicord_types::{
    ConnectionAction, ConnectionActionKind, ConnectionCheck, ConnectionCheckDetails,
    ConnectionCheckKind, ConnectionCheckStatus, ConnectionStatus, ConnectionVerificationReport,
    IntegrationProfile, UtcTimestamp,
};

use super::{
    cooperative_assurance_limits, human::render_command_report_concise, path_text,
    verbose::render_command_report_verbose, ConnectionCommandError, OutputFormat,
    PlannedConnectionChange, PlannedConnectionChangeKind,
};
use crate::connection_command::args::HumanOutputDetail;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::connection_command) enum CommandOperation {
    Init,
    Add,
    Status,
    Verify,
    Mode,
    Remove,
}

impl CommandOperation {
    #[cfg(test)]
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Add => "add",
            Self::Status => "status",
            Self::Verify => "verify",
            Self::Mode => "mode",
            Self::Remove => "remove",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(in crate::connection_command) struct CommandConnection {
    pub(super) id: String,
    pub(super) host: String,
    pub(super) scope: String,
    pub(super) profile: String,
    pub(super) mode: String,
    pub(super) repository: String,
    pub(super) config_target: String,
}

impl CommandConnection {
    pub(in crate::connection_command) fn new(
        id: impl Into<String>,
        host: impl Into<String>,
        scope: impl Into<String>,
        mode: impl Into<String>,
        repository: &Path,
        config_target: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            host: host.into(),
            scope: scope.into(),
            profile: IntegrationProfile::Record.as_str().to_owned(),
            mode: mode.into(),
            repository: path_text(repository),
            config_target: config_target.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ConnectionCommandResult {
    Setup {
        applied: bool,
    },
    ModeTransition {
        changed: bool,
        previous_mode: String,
        current_mode: String,
        previous_integration_revision: String,
        current_integration_revision: String,
        rebound_guard_installation_ids: Vec<String>,
    },
    Removal {
        membership_removed: bool,
        connection_removed: bool,
        remaining_project_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(in crate::connection_command) struct ConnectionCommandReport {
    pub(super) operation: CommandOperation,
    pub(super) dry_run: bool,
    pub(super) status: ConnectionStatus,
    pub(super) runtime_home: String,
    pub(super) connection: CommandConnection,
    pub(super) checks: Vec<ConnectionCheck>,
    pub(super) actions: Vec<ConnectionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result: Option<ConnectionCommandResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) planned_changes: Option<Vec<PlannedConnectionChange>>,
    pub(super) limits: Vec<String>,
}

impl ConnectionCommandReport {
    pub(in crate::connection_command) fn from_verification(
        operation: CommandOperation,
        setup_result: Option<bool>,
        runtime_home: &Path,
        connection: CommandConnection,
        verification: &ConnectionVerificationReport,
    ) -> Self {
        Self {
            operation,
            dry_run: false,
            status: command_status(verification),
            runtime_home: path_text(runtime_home),
            connection,
            checks: verification.checks().to_vec(),
            actions: verification.actions().to_vec(),
            result: setup_result.map(|applied| ConnectionCommandResult::Setup { applied }),
            planned_changes: None,
            limits: cooperative_assurance_limits(),
        }
    }

    pub(in crate::connection_command) fn setup_dry_run(
        operation: CommandOperation,
        runtime_home: &Path,
        connection: CommandConnection,
        current: Option<&ConnectionVerificationReport>,
        planned_changes: Vec<PlannedConnectionChange>,
        plan_actions: &[ConnectionAction],
    ) -> Result<Self, ConnectionCommandError> {
        let has_changes = !planned_changes.is_empty();
        let mut checks = current
            .map(|report| {
                report
                    .checks()
                    .iter()
                    .filter(|check| {
                        !matches!(
                            check.id(),
                            ConnectionCheckKind::ManagedConfig
                                | ConnectionCheckKind::GuardFiles
                                | ConnectionCheckKind::SetupPlan
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        checks.extend([
            command_check(
                ConnectionCheckKind::ManagedConfig,
                if planned_changes.iter().any(|change| {
                    change.kind() == PlannedConnectionChangeKind::ManagedHostConfiguration
                }) {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "managed_config_change_planned",
                "Managed Codex configuration plan was inspected",
                None,
            )?,
            command_check(
                ConnectionCheckKind::GuardFiles,
                if planned_changes
                    .iter()
                    .any(|change| change.kind() == PlannedConnectionChangeKind::GuardManagedFile)
                {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "guard_file_change_planned",
                "Guard managed-file plan was inspected",
                None,
            )?,
            command_check(
                ConnectionCheckKind::SetupPlan,
                if has_changes {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "setup_changes_planned",
                if has_changes {
                    "Setup changes are ready to apply"
                } else {
                    "Setup already matches the requested configuration"
                },
                None,
            )?,
        ]);
        if current.is_none() {
            checks.extend([
                command_check(
                    ConnectionCheckKind::HostSession,
                    ConnectionCheckStatus::Pending,
                    "host_session_not_observed",
                    "Actual Codex connection activity has not been observed",
                    None,
                )?,
                command_check(
                    ConnectionCheckKind::GuardObservation,
                    ConnectionCheckStatus::Pending,
                    "guard_observation_pending",
                    "Actual Codex Guard activity has not been observed",
                    None,
                )?,
            ]);
        }

        let mut actions = plan_actions.to_vec();
        if let Some(current) = current {
            actions.extend(current.actions().iter().cloned());
        }
        if has_changes {
            actions.push(ConnectionAction::try_new(
                ConnectionActionKind::ApplySetup,
                match operation {
                    CommandOperation::Init => {
                        "Run init without --dry-run to apply the planned setup changes"
                    }
                    CommandOperation::Add => {
                        "Run connection add without --dry-run to apply the planned setup changes"
                    }
                    _ => "Apply the planned setup changes without --dry-run",
                },
                None,
            )?);
        }
        if current.is_none() {
            actions.push(ConnectionAction::try_new(
                ConnectionActionKind::ObserveCodex,
                "After setup is applied, restart or reload Codex and use the connection so actual Codex and Guard activity can be observed",
                None,
            )?);
        }
        Self::from_components(
            operation,
            true,
            runtime_home,
            connection,
            checks,
            actions,
            Some(ConnectionCommandResult::Setup { applied: false }),
            Some(planned_changes),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::connection_command) fn mode_transition(
        runtime_home: &Path,
        connection: CommandConnection,
        changed: bool,
        previous_mode: String,
        current_mode: String,
        previous_integration_revision: String,
        current_integration_revision: String,
        rebound_guard_installation_ids: Vec<String>,
    ) -> Result<Self, ConnectionCommandError> {
        let actions = if changed {
            vec![ConnectionAction::try_new(
                ConnectionActionKind::ReloadHost,
                format!(
                    "Restart or reload Codex, then use the current Volicord integration so new runtime and Guard observations bind revision {current_integration_revision}"
                ),
                None,
            )?]
        } else {
            Vec::new()
        };
        let checks = vec![command_check(
            ConnectionCheckKind::ModeTransition,
            ConnectionCheckStatus::Passed,
            "mode_transition_applied",
            if changed {
                "Connection mode transition was applied"
            } else {
                "Connection mode already matched the requested mode"
            },
            None,
        )?];
        Self::from_components(
            CommandOperation::Mode,
            false,
            runtime_home,
            connection,
            checks,
            actions,
            Some(ConnectionCommandResult::ModeTransition {
                changed,
                previous_mode,
                current_mode,
                previous_integration_revision,
                current_integration_revision,
                rebound_guard_installation_ids,
            }),
            None,
        )
    }

    pub(in crate::connection_command) fn removal(
        runtime_home: &Path,
        connection: CommandConnection,
        membership_removed: bool,
        connection_removed: bool,
        remaining_project_count: usize,
    ) -> Result<Self, ConnectionCommandError> {
        Self::from_components(
            CommandOperation::Remove,
            false,
            runtime_home,
            connection,
            vec![command_check(
                ConnectionCheckKind::ConnectionRemoval,
                ConnectionCheckStatus::Passed,
                "connection_removal_applied",
                "Selected Connection membership removal was applied",
                None,
            )?],
            Vec::new(),
            Some(ConnectionCommandResult::Removal {
                membership_removed,
                connection_removed,
                remaining_project_count,
            }),
            None,
        )
    }

    pub(in crate::connection_command) fn removal_dry_run(
        runtime_home: &Path,
        connection: CommandConnection,
        planned_changes: Vec<PlannedConnectionChange>,
    ) -> Result<Self, ConnectionCommandError> {
        let has_changes = !planned_changes.is_empty();
        let checks = vec![command_check(
            ConnectionCheckKind::ConnectionRemoval,
            if has_changes {
                ConnectionCheckStatus::Pending
            } else {
                ConnectionCheckStatus::Passed
            },
            "connection_removal_planned",
            if has_changes {
                "Selected Connection membership removal is ready to apply"
            } else {
                "No Connection removal is required"
            },
            None,
        )?];
        let actions = if has_changes {
            vec![ConnectionAction::try_new(
                ConnectionActionKind::ApplyRemoval,
                "Run connection remove without --dry-run to apply the planned removal",
                None,
            )?]
        } else {
            Vec::new()
        };
        Self::from_components(
            CommandOperation::Remove,
            true,
            runtime_home,
            connection,
            checks,
            actions,
            None,
            Some(planned_changes),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::connection_command) fn setup_failure(
        operation: CommandOperation,
        runtime_home: &Path,
        connection: CommandConnection,
        summary: &str,
        details: Value,
        actions: Vec<ConnectionAction>,
    ) -> Result<Self, ConnectionCommandError> {
        Self::from_components(
            operation,
            false,
            runtime_home,
            connection,
            vec![command_check(
                ConnectionCheckKind::SetupPlan,
                ConnectionCheckStatus::Failed,
                "setup_partial_application",
                summary,
                Some(details),
            )?],
            actions,
            Some(ConnectionCommandResult::Setup { applied: false }),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_components(
        operation: CommandOperation,
        dry_run: bool,
        runtime_home: &Path,
        connection: CommandConnection,
        checks: Vec<ConnectionCheck>,
        actions: Vec<ConnectionAction>,
        result: Option<ConnectionCommandResult>,
        planned_changes: Option<Vec<PlannedConnectionChange>>,
    ) -> Result<Self, ConnectionCommandError> {
        let canonical =
            ConnectionVerificationReport::try_new(current_timestamp(), checks, actions)?;
        let status = command_status(&canonical);
        Ok(Self {
            operation,
            dry_run,
            status,
            runtime_home: path_text(runtime_home),
            connection,
            checks: canonical.checks().to_vec(),
            actions: canonical.actions().to_vec(),
            result,
            planned_changes,
            limits: cooperative_assurance_limits(),
        })
    }

    pub(super) const fn status(&self) -> ConnectionStatus {
        self.status
    }
}

pub(in crate::connection_command) struct RenderedCommandReport {
    pub(in crate::connection_command) output: String,
    pub(in crate::connection_command) status: ConnectionStatus,
}

pub(in crate::connection_command) fn render_command_report(
    format: OutputFormat,
    report: &ConnectionCommandReport,
) -> Result<RenderedCommandReport, ConnectionCommandError> {
    let output = match format {
        OutputFormat::Json => serde_json::to_string_pretty(report)
            .map(|output| format!("{output}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        OutputFormat::Human(HumanOutputDetail::Concise) => render_command_report_concise(report),
        OutputFormat::Human(HumanOutputDetail::Verbose) => render_command_report_verbose(report),
    };
    Ok(RenderedCommandReport {
        output,
        status: report.status(),
    })
}

fn command_status(report: &ConnectionVerificationReport) -> ConnectionStatus {
    if report.status() == ConnectionStatus::Complete && !report.actions().is_empty() {
        ConnectionStatus::ActionRequired
    } else {
        report.status()
    }
}

fn command_check(
    id: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    details: Option<Value>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let details = details
        .map(|details| {
            let Value::Object(details) = details else {
                return Err(ConnectionCommandError::runtime(
                    "command report check details must be an object",
                ));
            };
            ConnectionCheckDetails::try_new(details).map_err(ConnectionCommandError::from)
        })
        .transpose()?;
    ConnectionCheck::try_new(
        id,
        status,
        (status != ConnectionCheckStatus::Passed).then(|| code.to_owned()),
        summary,
        details,
        None,
    )
    .map_err(ConnectionCommandError::from)
}

fn current_timestamp() -> UtcTimestamp {
    let timestamp: DateTime<Utc> = SystemTime::now().into();
    UtcTimestamp::from_str(&timestamp.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .expect("current UTC timestamp must be canonical")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use crate::connection_command::planning::PlannedChangeOperation;

    use super::*;

    fn verification(status: ConnectionCheckStatus) -> ConnectionVerificationReport {
        ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            vec![ConnectionCheck::try_new(
                ConnectionCheckKind::ManagedConfig,
                status,
                (status != ConnectionCheckStatus::Passed)
                    .then(|| "managed_config_failed".to_owned()),
                "Managed configuration check",
                None,
                None,
            )
            .unwrap()],
            Vec::new(),
        )
        .unwrap()
    }

    fn connection() -> CommandConnection {
        CommandConnection::new(
            "connection_1",
            "codex",
            "user",
            "workflow",
            Path::new("/workspace/product"),
            "/home/user/.codex/config.toml",
        )
    }

    fn assert_top_level_keys(value: &Value, optional: &[&str]) {
        let mut expected = BTreeSet::from([
            "actions",
            "checks",
            "connection",
            "dry_run",
            "limits",
            "operation",
            "runtime_home",
            "status",
        ]);
        expected.extend(optional.iter().copied());
        assert_eq!(
            value
                .as_object()
                .expect("command report object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected
        );
    }

    #[test]
    fn every_operation_uses_the_same_exact_top_level_shape() {
        for operation in [
            CommandOperation::Init,
            CommandOperation::Add,
            CommandOperation::Status,
            CommandOperation::Verify,
        ] {
            let report = ConnectionCommandReport::from_verification(
                operation,
                matches!(operation, CommandOperation::Init | CommandOperation::Add).then_some(true),
                Path::new("/runtime"),
                connection(),
                &verification(ConnectionCheckStatus::Passed),
            );
            let value = serde_json::to_value(&report).unwrap();
            assert_eq!(value["operation"], operation.as_str());
            assert_eq!(value["status"], "complete");
            assert_eq!(value["checks"].as_array().map(Vec::len), Some(1));
            assert_eq!(value["actions"], json!([]));
            assert!(value.get("planned_changes").is_none());
            if matches!(operation, CommandOperation::Init | CommandOperation::Add) {
                assert_top_level_keys(&value, &["result"]);
                assert_eq!(value["result"], json!({"kind": "setup", "applied": true}));
            } else {
                assert_top_level_keys(&value, &[]);
                assert!(value.get("result").is_none());
            }
        }

        let mode = serde_json::to_value(
            ConnectionCommandReport::mode_transition(
                Path::new("/runtime"),
                connection(),
                false,
                "workflow".to_owned(),
                "workflow".to_owned(),
                "revision_1".to_owned(),
                "revision_1".to_owned(),
                Vec::new(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_top_level_keys(&mode, &["result"]);
        assert_eq!(mode["operation"], "mode");
        assert_eq!(mode["status"], "complete");
        assert_eq!(
            mode["result"],
            json!({
                "kind": "mode_transition",
                "changed": false,
                "previous_mode": "workflow",
                "current_mode": "workflow",
                "previous_integration_revision": "revision_1",
                "current_integration_revision": "revision_1",
                "rebound_guard_installation_ids": [],
            })
        );

        let removal = serde_json::to_value(
            ConnectionCommandReport::removal(Path::new("/runtime"), connection(), true, false, 1)
                .unwrap(),
        )
        .unwrap();
        assert_top_level_keys(&removal, &["result"]);
        assert_eq!(removal["operation"], "remove");
        assert_eq!(removal["status"], "complete");
        assert_eq!(
            removal["result"],
            json!({
                "kind": "removal",
                "membership_removed": true,
                "connection_removed": false,
                "remaining_project_count": 1,
            })
        );
    }

    #[test]
    fn dry_run_and_mode_status_come_from_typed_checks_and_actions() {
        let action_only_verification = ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            verification(ConnectionCheckStatus::Passed)
                .checks()
                .to_vec(),
            vec![
                ConnectionAction::try_new(ConnectionActionKind::ReloadHost, "Reload Codex", None)
                    .unwrap(),
            ],
        )
        .unwrap();
        let action_only = ConnectionCommandReport::from_verification(
            CommandOperation::Verify,
            None,
            Path::new("/runtime"),
            connection(),
            &action_only_verification,
        );
        assert_eq!(action_only.status(), ConnectionStatus::ActionRequired);

        let changed = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            vec![PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ManagedHostConfiguration,
                PlannedChangeOperation::Update,
                "/home/user/.codex/config.toml",
            )],
            &[],
        )
        .unwrap();
        let changed = serde_json::to_value(changed).unwrap();
        assert_top_level_keys(&changed, &["planned_changes", "result"]);
        assert_eq!(changed["operation"], "add");
        assert_eq!(changed["status"], "action_required");
        assert_eq!(
            changed["result"],
            json!({"kind": "setup", "applied": false})
        );

        let mode = ConnectionCommandReport::mode_transition(
            Path::new("/runtime"),
            connection(),
            true,
            "workflow".to_owned(),
            "read_only".to_owned(),
            "before".to_owned(),
            "after".to_owned(),
            vec!["guard_1".to_owned()],
        )
        .unwrap();
        let mode = serde_json::to_value(mode).unwrap();
        assert_top_level_keys(&mode, &["result"]);
        assert_eq!(mode["status"], "action_required");
        assert_eq!(mode["checks"][0]["status"], "passed");
        assert_eq!(mode["actions"][0]["id"], "reload_host");
        assert_eq!(mode["result"]["changed"], true);

        let removal = ConnectionCommandReport::removal_dry_run(
            Path::new("/runtime"),
            connection(),
            vec![PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ConnectionMembership,
                PlannedChangeOperation::Remove,
                "connection_1:project_1",
            )],
        )
        .unwrap();
        let removal = serde_json::to_value(removal).unwrap();
        assert_top_level_keys(&removal, &["planned_changes"]);
        assert_eq!(removal["operation"], "remove");
        assert_eq!(removal["status"], "action_required");
        assert_eq!(removal["checks"][0]["status"], "pending");
        assert_eq!(removal["actions"][0]["id"], "apply_removal");
        assert!(removal.get("result").is_none());
    }

    #[test]
    fn setup_dry_run_preserves_canonical_host_actions_and_rejects_duplicate_kinds() {
        let host_action = ConnectionAction::try_new(
            ConnectionActionKind::RunVerification,
            "Run connection verification",
            Some("volicord connection verify".to_owned()),
        )
        .unwrap();
        let report = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            Vec::new(),
            std::slice::from_ref(&host_action),
        )
        .unwrap();
        let action = report
            .actions
            .iter()
            .find(|action| action.id() == ConnectionActionKind::RunVerification)
            .expect("host-supplied action");
        assert_eq!(action.instruction(), "Run connection verification");
        assert_eq!(action.command(), Some("volicord connection verify"));

        let error = ConnectionCommandReport::setup_dry_run(
            CommandOperation::Add,
            Path::new("/runtime"),
            connection(),
            None,
            Vec::new(),
            &[host_action.clone(), host_action],
        )
        .expect_err("duplicate action kinds must fail");
        assert!(error.to_string().contains("duplicate action"));
    }

    #[test]
    fn json_and_verbose_human_render_the_same_typed_status_and_actions() {
        let report = ConnectionCommandReport::from_verification(
            CommandOperation::Verify,
            None,
            Path::new("/runtime"),
            connection(),
            &verification(ConnectionCheckStatus::Failed),
        );
        let json = render_command_report(OutputFormat::Json, &report).unwrap();
        let text = render_command_report(OutputFormat::Human(HumanOutputDetail::Verbose), &report)
            .unwrap();
        assert_eq!(json.status, ConnectionStatus::Failed);
        assert_eq!(text.status, ConnectionStatus::Failed);
        assert_eq!(
            serde_json::from_str::<Value>(&json.output).unwrap()["status"],
            "failed"
        );
        assert_eq!(
            text.output,
            format!(
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
                    "  [fail] Managed Codex configuration\n",
                    "    Managed configuration check\n",
                    "    Code: managed_config_failed\n",
                    "\nAssurance\n",
                    "  {}\n",
                ),
                super::super::common::COOPERATIVE_ASSURANCE_LIMIT
            )
        );
    }
}
