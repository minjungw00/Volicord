use std::{collections::BTreeMap, process::Command, time::Duration};

use volicord_store::agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW};

use super::{
    failure::{McpProcessFailure, McpStage},
    supervisor::{ChildSupervisor, SupervisorKind, MAX_PREFLIGHT_STDOUT_BYTES},
};
use crate::connection_command::verification::McpPreflightDiagnostics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProcessOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub(super) fn run_preflight_command(
    command: Command,
    timeout: Duration,
) -> Result<ConnectionProcessOutput, McpProcessFailure> {
    let mut supervisor = ChildSupervisor::spawn(command, SupervisorKind::Preflight, timeout)?;
    let status = match supervisor.wait_for_exit(McpStage::Startup) {
        Ok(status) => status,
        Err(failure) => return Err(supervisor.finish_failure(failure)),
    };
    let output = supervisor.finish_success(McpStage::Startup)?;
    debug_assert_eq!(output.status, status);
    let stdout_truncated = output.stdout.truncated;
    let stderr = if stdout_truncated {
        append_diagnostic_context(
            format!(
                "managed MCP preflight stdout exceeded the {MAX_PREFLIGHT_STDOUT_BYTES}-byte limit"
            ),
            &output.stderr.text,
        )
    } else {
        output.stderr.text
    };
    Ok(ConnectionProcessOutput {
        success: status.success() && !stdout_truncated,
        status_code: status.code(),
        stdout: if stdout_truncated {
            String::new()
        } else {
            output.stdout.text
        },
        stderr,
    })
}

pub(super) fn validate_connection_preflight_report(
    stdout: &str,
    connection_id: &str,
    mode: &str,
) -> Result<Option<McpPreflightDiagnostics>, String> {
    let report = parse_colon_report(stdout)?;
    expect_report_field(&report, "configuration", "valid")?;
    expect_report_field(&report, "transport", "stdio")?;
    expect_report_field(&report, "connection_id", connection_id)?;
    expect_report_field(&report, "mode", mode)?;
    expect_report_field(&report, "enabled", "true")?;
    expect_report_field(&report, "registry_read", "passed")?;
    expect_report_field(&report, "project_state_read", "passed")?;
    match mode {
        CONNECTION_MODE_WORKFLOW => {
            expect_report_field(&report, "project_state_write", "passed")?;
            expect_report_field(&report, "effective_tool_mode", "workflow")?;
        }
        CONNECTION_MODE_READ_ONLY => {
            expect_report_field(&report, "project_state_write", "passed")?;
            expect_report_field(&report, "effective_tool_mode", "read_only")?;
        }
        other => return Err(format!("unsupported connection mode: {other}")),
    }
    expect_report_field(&report, "tools_list_schema_validation", "passed")?;
    Ok(McpPreflightDiagnostics::from_preflight_report(&report))
}

fn parse_colon_report(stdout: &str) -> Result<BTreeMap<String, String>, String> {
    let mut report = BTreeMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once(':') {
            report.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    if report.is_empty() {
        Err("preflight did not return a key-value report".to_owned())
    } else {
        Ok(report)
    }
}

fn expect_report_field(
    report: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    match report.get(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!(
            "preflight field {key} was {actual}, expected {expected}"
        )),
        None => Err(format!("preflight field {key} was missing")),
    }
}

fn append_diagnostic_context(summary: String, context: &str) -> String {
    if context.is_empty() {
        summary
    } else {
        format!("{summary}\n{context}")
    }
}

pub(super) fn compact_stream(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::{process::Command, time::Instant};

    use super::*;

    #[test]
    fn preflight_requires_current_storage_and_tool_schema_checks() {
        let report = "configuration: valid\ntransport: stdio\nconnection_id: connection_fixture\nmode: workflow\nenabled: true\nregistry_read: passed\nproject_state_read: passed\nproject_state_write: passed\neffective_tool_mode: workflow\ntools_list_schema_validation: passed\n";
        assert!(validate_connection_preflight_report(
            report,
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_ok());
        assert!(validate_connection_preflight_report(
            &report.replace("registry_read: passed\n", ""),
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_err());
        let read_only = report.replace("mode: workflow", "mode: read_only").replace(
            "effective_tool_mode: workflow",
            "effective_tool_mode: read_only",
        );
        assert!(validate_connection_preflight_report(
            &read_only,
            "connection_fixture",
            CONNECTION_MODE_READ_ONLY
        )
        .is_ok());
        assert!(validate_connection_preflight_report(
            &report.replace(
                "tools_list_schema_validation: passed",
                "tools_list_schema_validation: failed"
            ),
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_err());
    }

    #[cfg(unix)]
    fn shell_command(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    #[cfg(unix)]
    #[test]
    fn successful_preflight_uses_bounded_shared_supervision() {
        let output = run_preflight_command(
            shell_command("printf '%s\\n' 'configuration: valid'"),
            Duration::from_secs(1),
        )
        .expect("successful preflight");
        assert!(output.success);
        assert_eq!(output.stdout, "configuration: valid\n");
    }

    #[cfg(unix)]
    #[test]
    fn preflight_does_not_wait_for_a_descendant_inherited_pipe() {
        let started = Instant::now();
        let output = run_preflight_command(
            shell_command("sleep 30 & printf '%s\\n' 'configuration: valid'"),
            Duration::from_secs(1),
        )
        .expect("contained descendant cleanup");
        assert!(output.success);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn preflight_timeout_uses_the_shared_lifecycle_failure() {
        let failure = run_preflight_command(
            shell_command("printf '%s\\n' 'preflight still running' >&2; while :; do :; done"),
            Duration::from_millis(50),
        )
        .expect_err("preflight must time out");
        match failure {
            McpProcessFailure::Timeout {
                stage,
                timeout,
                stderr,
            } => {
                assert_eq!(stage, McpStage::Startup);
                assert_eq!(timeout, Duration::from_millis(50));
                assert!(stderr.text.contains("preflight still running"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }
}
