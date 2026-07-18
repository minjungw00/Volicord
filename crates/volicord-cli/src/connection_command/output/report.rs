use std::{collections::BTreeMap, path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, Utc};
use serde::Serialize;
use volicord_types::{
    ConnectionAction, ConnectionCheck, ConnectionCheckDetails, ConnectionCheckId,
    ConnectionCheckStatus, ConnectionStatus, ConnectionVerificationReport, IntegrationProfile,
    UtcTimestamp,
};

use super::*;

const COOPERATIVE_ASSURANCE_LIMIT: &str = "Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::connection_command) enum CommandOperation {
    Init,
    Status,
    Verify,
}

impl CommandOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Status => "status",
            Self::Verify => "verify",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CommandConnection {
    id: String,
    host: String,
    scope: String,
    profile: String,
    mode: String,
    repository: String,
    config_target: String,
}

impl CommandConnection {
    pub(super) fn new(
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
pub(super) struct PlannedConnectionChange {
    change: String,
    target: String,
}

impl PlannedConnectionChange {
    pub(super) fn new(change: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            change: change.into(),
            target: target.into(),
        }
    }

    pub(super) fn change(&self) -> &str {
        &self.change
    }

    pub(super) fn target(&self) -> &str {
        &self.target
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct ConnectionCommandReport {
    operation: CommandOperation,
    dry_run: bool,
    status: ConnectionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    setup_applied: Option<bool>,
    runtime_home: String,
    connection: CommandConnection,
    checks: Vec<ConnectionCheck>,
    actions: Vec<ConnectionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    planned_changes: Option<Vec<PlannedConnectionChange>>,
    limits: Vec<String>,
}

impl ConnectionCommandReport {
    pub(super) fn from_verification(
        operation: CommandOperation,
        setup_applied: Option<bool>,
        runtime_home: &Path,
        connection: CommandConnection,
        verification: &ConnectionVerificationReport,
    ) -> Self {
        Self {
            operation,
            dry_run: false,
            status: verification.status(),
            setup_applied,
            runtime_home: path_text(runtime_home),
            connection,
            checks: verification.checks().to_vec(),
            actions: verification.actions().to_vec(),
            planned_changes: None,
            limits: vec![COOPERATIVE_ASSURANCE_LIMIT.to_owned()],
        }
    }

    pub(super) fn dry_run(
        runtime_home: &Path,
        connection: CommandConnection,
        current: Option<&ConnectionVerificationReport>,
        planned_changes: Vec<PlannedConnectionChange>,
        plan_actions: &[UserAction],
    ) -> Result<Self, ConnectionCommandError> {
        let has_changes = !planned_changes.is_empty();
        let mut checks = current
            .map(|report| {
                report
                    .checks()
                    .iter()
                    .filter(|check| {
                        !matches!(check.id().as_str(), "managed_config" | "guard_files")
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        checks.extend([
            command_check(
                "managed_config",
                if planned_changes
                    .iter()
                    .any(|change| change.target() == connection.config_target)
                {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "managed_config_change_planned",
                "Managed Codex configuration plan was inspected",
                None,
            )?,
            command_check(
                "guard_files",
                if planned_changes.iter().any(|change| {
                    change.target() != connection.config_target
                        && change.target() != runtime_home.display().to_string()
                }) {
                    ConnectionCheckStatus::Pending
                } else {
                    ConnectionCheckStatus::Passed
                },
                "guard_file_change_planned",
                "Guard managed-file plan was inspected",
                None,
            )?,
            command_check(
                "setup_plan",
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
                    "host_session",
                    ConnectionCheckStatus::Pending,
                    "host_session_not_observed",
                    "Actual Codex connection activity has not been observed",
                    None,
                )?,
                command_check(
                    "guard_observation",
                    ConnectionCheckStatus::Pending,
                    "guard_observation_pending",
                    "Actual Codex Guard activity has not been observed",
                    None,
                )?,
            ]);
        }

        let mut actions = BTreeMap::<String, ConnectionAction>::new();
        if let Some(current) = current {
            for action in current.actions() {
                actions.insert(action.id().to_owned(), action.clone());
            }
        }
        for action in plan_actions {
            let id = user_action_id(action.kind).to_owned();
            actions.insert(
                id.clone(),
                ConnectionAction::try_new(id, &action.message, None)
                    .map_err(ConnectionCommandError::from)?,
            );
        }
        if has_changes {
            actions.insert(
                "apply_setup".to_owned(),
                ConnectionAction::try_new(
                    "apply_setup",
                    "Run init without --dry-run to apply the planned setup changes",
                    None,
                )?,
            );
        }
        if current.is_none() {
            actions.insert(
                "observe_codex".to_owned(),
                ConnectionAction::try_new(
                    "observe_codex",
                    "After setup is applied, restart or reload Codex and use the connection so actual Codex and Guard activity can be observed",
                    None,
                )?,
            );
        }
        let canonical = ConnectionVerificationReport::try_new(
            current_timestamp(),
            checks,
            actions.into_values().collect(),
        )?;
        Ok(Self {
            operation: CommandOperation::Init,
            dry_run: true,
            status: canonical.status(),
            setup_applied: Some(false),
            runtime_home: path_text(runtime_home),
            connection,
            checks: canonical.checks().to_vec(),
            actions: canonical.actions().to_vec(),
            planned_changes: Some(planned_changes),
            limits: vec![COOPERATIVE_ASSURANCE_LIMIT.to_owned()],
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

pub(super) fn render_command_report(
    format: OutputFormat,
    report: &ConnectionCommandReport,
) -> Result<RenderedCommandReport, ConnectionCommandError> {
    let output = match format {
        OutputFormat::Json => serde_json::to_string_pretty(report)
            .map(|output| format!("{output}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        OutputFormat::Text => render_command_report_text(report),
    };
    Ok(RenderedCommandReport {
        output,
        status: report.status(),
    })
}

fn render_command_report_text(report: &ConnectionCommandReport) -> String {
    let mut output = String::new();
    output.push_str(&format!("Operation: {}\n", report.operation.as_str()));
    output.push_str(&format!("Status: {}\n", report.status.as_str()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    if let Some(setup_applied) = report.setup_applied {
        output.push_str(&format!("Setup applied: {setup_applied}\n"));
    }
    output.push_str(&format!("Runtime home: {}\n", report.runtime_home));
    output.push_str("Connection:\n");
    output.push_str(&format!("  ID: {}\n", report.connection.id));
    output.push_str(&format!("  Host: {}\n", report.connection.host));
    output.push_str(&format!("  Scope: {}\n", report.connection.scope));
    output.push_str(&format!("  Profile: {}\n", report.connection.profile));
    output.push_str(&format!("  Mode: {}\n", report.connection.mode));
    output.push_str(&format!("  Repository: {}\n", report.connection.repository));
    output.push_str(&format!(
        "  Config target: {}\n",
        report.connection.config_target
    ));
    output.push_str("Checks:\n");
    for check in &report.checks {
        output.push_str(&format!(
            "  [{}] {}: {}\n",
            check.status().as_str(),
            check.id().as_str(),
            check.summary()
        ));
        if let Some(code) = check.code() {
            output.push_str(&format!("    Code: {code}\n"));
        }
    }
    output.push_str("Actions:\n");
    if report.actions.is_empty() {
        output.push_str("  none\n");
    } else {
        for action in &report.actions {
            output.push_str(&format!("  {}: {}\n", action.id(), action.instruction()));
            if let Some(command) = action.command() {
                output.push_str(&format!("    Command: {command}\n"));
            }
        }
    }
    if let Some(planned_changes) = &report.planned_changes {
        output.push_str("Planned changes:\n");
        if planned_changes.is_empty() {
            output.push_str("  none\n");
        } else {
            for change in planned_changes {
                output.push_str(&format!("  {}: {}\n", change.change(), change.target()));
            }
        }
    }
    output.push_str("Limits:\n");
    for limit in &report.limits {
        output.push_str(&format!("  {limit}\n"));
    }
    output
}

fn command_check(
    id: &str,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    details: Option<serde_json::Value>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let details = details
        .map(|details| {
            let serde_json::Value::Object(details) = details else {
                return Err(ConnectionCommandError::runtime(
                    "command report check details must be an object",
                ));
            };
            ConnectionCheckDetails::try_new(details).map_err(ConnectionCommandError::from)
        })
        .transpose()?;
    ConnectionCheck::try_new(
        ConnectionCheckId::new(id),
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
    use serde_json::json;

    use super::*;

    fn verification(status: ConnectionCheckStatus) -> ConnectionVerificationReport {
        ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap(),
            vec![ConnectionCheck::try_new(
                ConnectionCheckId::new("managed_config"),
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

    fn assert_no_obsolete_tree(value: &serde_json::Value) {
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    assert_no_obsolete_tree(value);
                }
            }
            serde_json::Value::Object(object) => {
                for obsolete in [
                    "states",
                    "verification",
                    "verification_report",
                    "verification_status",
                    "host_hook",
                    "summary_card",
                    "primary_next_action",
                    "disclosure",
                    "host_gate",
                    "approval",
                    "mcp_handshake_allowed",
                    "generated_config_verified",
                    "configuration_health",
                    "effective_health",
                    "observation_health",
                    "stale_files",
                    "broken_files",
                ] {
                    assert!(!object.contains_key(obsolete), "unexpected {obsolete}");
                }
                for value in object.values() {
                    assert_no_obsolete_tree(value);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn exact_applied_report_shape_has_one_status_tree() {
        for operation in [
            CommandOperation::Init,
            CommandOperation::Status,
            CommandOperation::Verify,
        ] {
            let report = ConnectionCommandReport::from_verification(
                operation,
                (operation == CommandOperation::Init).then_some(true),
                Path::new("/runtime"),
                connection(),
                &verification(ConnectionCheckStatus::Passed),
            );
            let value = serde_json::to_value(&report).unwrap();
            let expected = json!({
                "operation": operation.as_str(),
                "dry_run": false,
                "status": "complete",
                "runtime_home": "/runtime",
                "connection": {
                    "id": "connection_1",
                    "host": "codex",
                    "scope": "user",
                    "profile": "record",
                    "mode": "workflow",
                    "repository": "/workspace/product",
                    "config_target": "/home/user/.codex/config.toml"
                },
                "checks": [{
                    "id": "managed_config",
                    "status": "passed",
                    "summary": "Managed configuration check"
                }],
                "actions": [],
                "limits": [COOPERATIVE_ASSURANCE_LIMIT]
            });
            let mut expected = expected;
            if operation == CommandOperation::Init {
                expected["setup_applied"] = json!(true);
            }
            assert_eq!(value, expected);
            assert_no_obsolete_tree(&value);
        }
    }

    #[test]
    fn dry_run_status_comes_from_the_typed_plan_report() {
        let current = verification(ConnectionCheckStatus::Passed);
        let unchanged = ConnectionCommandReport::dry_run(
            Path::new("/runtime"),
            connection(),
            Some(&current),
            Vec::new(),
            &[],
        )
        .unwrap();
        let unchanged = serde_json::to_value(unchanged).unwrap();
        assert_eq!(unchanged["dry_run"], true);
        assert_eq!(unchanged["setup_applied"], false);
        assert_eq!(unchanged["status"], "complete");
        assert_eq!(unchanged["planned_changes"], json!([]));
        assert_eq!(unchanged["actions"], json!([]));

        let changed = ConnectionCommandReport::dry_run(
            Path::new("/runtime"),
            connection(),
            Some(&current),
            vec![PlannedConnectionChange::new(
                "update",
                "/home/user/.codex/config.toml",
            )],
            &[],
        )
        .unwrap();
        let changed = serde_json::to_value(changed).unwrap();
        assert_eq!(changed["status"], "action_required");
        assert_eq!(changed["actions"][0]["id"], "apply_setup");
        assert_no_obsolete_tree(&changed);
    }

    #[test]
    fn json_and_text_render_the_same_typed_status_and_actions() {
        let report = ConnectionCommandReport::from_verification(
            CommandOperation::Verify,
            None,
            Path::new("/runtime"),
            connection(),
            &verification(ConnectionCheckStatus::Failed),
        );
        let json = render_command_report(OutputFormat::Json, &report).unwrap();
        let text = render_command_report(OutputFormat::Text, &report).unwrap();
        assert_eq!(json.status, ConnectionStatus::Failed);
        assert_eq!(text.status, ConnectionStatus::Failed);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json.output).unwrap()["status"],
            "failed"
        );
        assert!(text.output.contains("Status: failed\n"));
        assert!(text.output.contains("Checks:\n"));
    }
}
