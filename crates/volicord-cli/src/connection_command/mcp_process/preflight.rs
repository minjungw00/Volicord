use std::{process::Command, time::Duration};

use serde_json::Value;
use volicord_store::agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW};

use super::{
    failure::{McpProcessFailure, McpStage},
    supervisor::{ChildSupervisor, SupervisorKind, MAX_PREFLIGHT_STDOUT_BYTES},
};
use crate::connection_command::verification::McpPreflightDiagnostics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProcessOutput {
    pub process_id: u32,
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
    let process_id = supervisor.child_id();
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
        process_id,
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
    let report: Value =
        serde_json::from_str(stdout).map_err(|error| format!("invalid preflight JSON: {error}"))?;
    expect_report_string(&report, "operation", "mcp_preflight")?;
    expect_report_string(&report, "status", "passed")?;
    expect_report_string(&report, "configuration", "valid")?;
    expect_report_string(&report, "canonical_managed_entry", "passed")?;
    expect_report_string(&report, "transport", "stdio")?;
    expect_report_string(&report, "connection_id", connection_id)?;
    expect_report_string(&report, "mode", mode)?;
    expect_report_bool(&report, "enabled", true)?;
    expect_report_string(&report, "registry_read", "passed")?;
    expect_report_string(&report, "project_state_read", "passed")?;
    expect_report_string(&report, "evidence_class", "read_only_preflight")?;
    let side_effects = report
        .get("side_effects")
        .and_then(Value::as_array)
        .ok_or_else(|| "preflight field side_effects was missing or invalid".to_owned())?;
    if !side_effects.is_empty() {
        return Err("preflight declared side effects".to_owned());
    }
    let writeability = report
        .get("writeability")
        .ok_or_else(|| "preflight field writeability was missing".to_owned())?;
    expect_report_string(writeability, "status", "not_checked")?;
    expect_report_string(writeability, "requirement", "requires_active_verification")?;
    match mode {
        CONNECTION_MODE_WORKFLOW | CONNECTION_MODE_READ_ONLY => {}
        other => return Err(format!("unsupported connection mode: {other}")),
    }
    expect_report_string(
        &report,
        "effective_tool_mode",
        "requires_active_verification",
    )?;
    expect_report_string(&report, "tools_list_schema_validation", "passed")?;
    Ok(McpPreflightDiagnostics::from_preflight_report(&report))
}

fn expect_report_string(report: &Value, key: &str, expected: &str) -> Result<(), String> {
    match report.get(key).and_then(Value::as_str) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!(
            "preflight field {key} was {actual}, expected {expected}"
        )),
        None => Err(format!("preflight field {key} was missing")),
    }
}

fn expect_report_bool(report: &Value, key: &str, expected: bool) -> Result<(), String> {
    match report.get(key).and_then(Value::as_bool) {
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

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::connection_command::mcp_process::test_child;

    #[test]
    fn preflight_requires_current_storage_and_tool_schema_checks() {
        let report = r#"{"operation":"mcp_preflight","status":"passed","side_effects":[],"evidence_class":"read_only_preflight","configuration":"valid","canonical_managed_entry":"passed","transport":"stdio","connection_id":"connection_fixture","mode":"workflow","enabled":true,"registry_read":"passed","project_state_read":"passed","writeability":{"status":"not_checked","requirement":"requires_active_verification"},"effective_tool_mode":"requires_active_verification","tools_list_schema_validation":"passed"}"#;
        assert!(validate_connection_preflight_report(
            report,
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_ok());
        assert!(validate_connection_preflight_report(
            &report.replace(r#""registry_read":"passed","#, ""),
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_err());
        let read_only = report.replace(r#""mode":"workflow""#, r#""mode":"read_only""#);
        assert!(validate_connection_preflight_report(
            &read_only,
            "connection_fixture",
            CONNECTION_MODE_READ_ONLY
        )
        .is_ok());
        assert!(validate_connection_preflight_report(
            &report.replace(
                r#""tools_list_schema_validation":"passed""#,
                r#""tools_list_schema_validation":"failed""#
            ),
            "connection_fixture",
            CONNECTION_MODE_WORKFLOW
        )
        .is_err());
    }

    #[test]
    fn successful_preflight_uses_bounded_shared_supervision() {
        let output = run_preflight_command(
            test_child::command("preflight-success"),
            Duration::from_secs(1),
        )
        .expect("successful preflight");
        assert!(output.success);
        assert_eq!(output.stdout, "configuration: valid\n");
    }

    #[test]
    fn preflight_does_not_wait_for_a_descendant_inherited_pipe() {
        let started = Instant::now();
        let output = run_preflight_command(
            test_child::command("preflight-descendant-output-hold"),
            Duration::from_secs(1),
        )
        .expect("contained descendant cleanup");
        assert!(output.success);
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn preflight_timeout_uses_the_shared_lifecycle_failure() {
        let timeout = Duration::from_millis(100);
        let started = Instant::now();
        let failure = run_preflight_command(test_child::command("hang-before-initialize"), timeout)
            .expect_err("preflight must time out");
        let elapsed = started.elapsed();
        assert!(elapsed >= timeout);
        assert!(elapsed < Duration::from_secs(2));
        match failure {
            McpProcessFailure::Timeout {
                stage,
                timeout,
                stderr,
            } => {
                assert_eq!(stage, McpStage::Startup);
                assert_eq!(timeout, Duration::from_millis(100));
                assert!(stderr.text.contains("waiting for initialize"));
            }
            other => panic!("unexpected failure: {other:?}"),
        }
    }
}
