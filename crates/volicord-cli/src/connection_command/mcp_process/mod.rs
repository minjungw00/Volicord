use std::{ffi::OsString, path::PathBuf, time::Duration};

use volicord_mcp::MaterializedManagedMcpLaunch;

use super::verification::VerificationStep;

mod failure;
mod host_compatibility;
mod launch;
mod pinned_schema;
mod preflight;
mod stdio_probe;
mod supervisor;
#[cfg(test)]
mod test_child;

pub use failure::{McpProcessFailure, McpStage};
pub use host_compatibility::HostCompatibilityProfile;
pub(super) use launch::materialize_connection_invocation;
pub use preflight::ConnectionProcessOutput;
pub use stdio_probe::{McpExchangeOutcome, McpExchangeProgress};

use preflight::{compact_stream, run_preflight_command, validate_connection_preflight_report};
use stdio_probe::verify_mcp_stdio_process;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub trait ConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
    ) -> Result<ConnectionProcessOutput, McpProcessFailure>;
    fn verify_mcp_stdio(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
        mode: &str,
    ) -> McpExchangeOutcome;
}

pub struct ProductionConnectionProcess;

impl ConnectionProcess for ProductionConnectionProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }

    fn run_preflight(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
    ) -> Result<ConnectionProcessOutput, McpProcessFailure> {
        run_preflight_command(launch.process_command(), DEFAULT_TIMEOUT)
    }

    fn verify_mcp_stdio(
        &mut self,
        launch: &MaterializedManagedMcpLaunch,
        mode: &str,
    ) -> McpExchangeOutcome {
        verify_mcp_stdio_process(launch, mode, DEFAULT_TIMEOUT)
    }
}

#[derive(Debug, Clone)]
pub(super) struct McpVerification {
    pub(super) step: VerificationStep,
    pub(super) exchange: Option<McpExchangeOutcome>,
}

impl McpVerification {
    pub(super) fn from_exchange(exchange: McpExchangeOutcome) -> Self {
        let step = match &exchange.failure {
            Some(failure) => VerificationStep::failed_with_code(
                failure.check_code(),
                exchange.failure_summary(failure),
            ),
            None => VerificationStep::passed_with_code(
                "mcp_server_ready",
                if exchange.conformance.is_empty() && exchange.host_compatibility.is_empty() {
                    format!(
                        "MCP initialize, tools/list, required-tool validation, designated read-only tool call, and graceful shutdown succeeded; tools/list returned {} tools",
                        exchange
                            .progress
                            .tools_list
                            .as_ref()
                            .expect("completed MCP exchange observed tools/list")
                            .len()
                    )
                } else {
                    format!(
                        "MCP server conformance passed for {} production revisions and {} independent host compatibility fixtures",
                        exchange.conformance.len(),
                        exchange.host_compatibility.len()
                    )
                },
            ),
        };
        Self {
            step,
            exchange: Some(exchange),
        }
    }

    pub(super) fn not_run() -> Self {
        Self {
            step: VerificationStep::pending(
                "MCP server self-test did not run after failed preflight",
            ),
            exchange: None,
        }
    }
}

pub(super) fn run_connection_preflight(
    process: &mut impl ConnectionProcess,
    launch: &MaterializedManagedMcpLaunch,
    connection_id: &str,
    mode: &str,
) -> VerificationStep {
    match process.run_preflight(launch) {
        Ok(output) if output.success => {
            match validate_connection_preflight_report(&output.stdout, connection_id, mode) {
                Ok(diagnostics) => VerificationStep::passed_with_code(
                    "mcp_server_preflight_passed",
                    "volicord mcp preflight passed",
                )
                .with_preflight_diagnostics(diagnostics),
                Err(message) => {
                    VerificationStep::failed_with_code("mcp_server_preflight_invalid", message)
                }
            }
        }
        Ok(output) => VerificationStep::failed_with_code(
            "mcp_server_preflight_failed",
            format!(
                "volicord mcp preflight failed with status {}; stderr: {}",
                status_text(output.status_code),
                compact_stream(&output.stderr)
            ),
        ),
        Err(failure) => VerificationStep::failed_with_code(failure.check_code(), failure.summary()),
    }
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_command::mcp_process::failure::{bounded_io_text, BoundedText};

    #[test]
    fn typed_failures_map_directly_to_current_check_codes() {
        let cases = [
            (
                McpProcessFailure::Spawn {
                    stage: McpStage::Startup,
                    io_detail: bounded_io_text("not found"),
                },
                "mcp_server_process_failed",
            ),
            (
                McpProcessFailure::Timeout {
                    stage: McpStage::Initialize,
                    timeout: Duration::from_millis(50),
                    stderr: BoundedText::empty(),
                },
                "mcp_server_initialize_failed",
            ),
            (
                McpProcessFailure::protocol(McpStage::ToolsList, "tools/list response was invalid"),
                "mcp_server_tools_list_failed",
            ),
            (
                McpProcessFailure::protocol(
                    McpStage::SafeToolCall,
                    "designated read-only tool response was invalid",
                ),
                "mcp_server_safe_call_failed",
            ),
            (
                McpProcessFailure::Cleanup {
                    stage: McpStage::Shutdown,
                    io_detail: bounded_io_text("cleanup failed"),
                    stderr: BoundedText::empty(),
                },
                "mcp_server_process_failed",
            ),
        ];
        for (failure, expected) in cases {
            assert_eq!(failure.check_code(), expected);
        }
    }
}
