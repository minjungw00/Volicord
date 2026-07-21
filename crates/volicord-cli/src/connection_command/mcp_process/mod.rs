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

pub use failure::{
    McpProcessDiagnosticContext, McpProcessFailure, McpProtocolFailureKind, McpStage,
};
pub use host_compatibility::HostCompatibilityProfile;
pub(super) use launch::materialize_connection_invocation;
pub use preflight::ConnectionProcessOutput;
pub use stdio_probe::{McpExchangeOutcome, McpExchangeProgress, McpPersistedDiagnostic};

use preflight::{run_preflight_command, validate_connection_preflight_report};
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
                Err(message) => VerificationStep::failed_with_code(
                    "mcp_server_preflight_invalid",
                    message.clone(),
                )
                .with_process_failure(
                    Some(output.process_id),
                    McpProcessFailure::typed_protocol(
                        McpStage::Startup,
                        McpProtocolFailureKind::PreflightReportInvalid,
                        message,
                    ),
                ),
            }
        }
        Ok(output) => {
            let failure = McpProcessFailure::exited_with_stderr(
                McpStage::Startup,
                output.status_code,
                &output.stderr,
            );
            VerificationStep::failed_with_code(
                "mcp_server_preflight_failed",
                format!(
                    "volicord mcp preflight failed with status {}",
                    status_text(output.status_code)
                ),
            )
            .with_process_failure(Some(output.process_id), failure)
        }
        Err(failure) => VerificationStep::failed_with_code(failure.check_code(), failure.summary())
            .with_process_failure(None, failure),
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
    use volicord_types::{IntegrationRevision, UtcTimestamp};

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

    #[test]
    fn process_and_protocol_variants_map_without_prose_classification() {
        let protocol_cases = [
            (
                McpProtocolFailureKind::MalformedResponse,
                "mcp.json_rpc.malformed_response",
            ),
            (
                McpProtocolFailureKind::FramingFailure,
                "mcp.json_rpc.framing_failure",
            ),
            (
                McpProtocolFailureKind::MessageSizeExceeded,
                "mcp.json_rpc.message_size_exceeded",
            ),
            (
                McpProtocolFailureKind::JsonRpcError,
                "mcp.json_rpc.error_response",
            ),
            (
                McpProtocolFailureKind::MalformedProtocolVersion,
                "mcp.protocol.malformed_version",
            ),
            (
                McpProtocolFailureKind::UnsupportedProtocolRevision,
                "mcp.protocol.unsupported_version",
            ),
            (
                McpProtocolFailureKind::CounterOffer,
                "mcp.protocol.counter_offer",
            ),
            (
                McpProtocolFailureKind::CounterOfferRejectedOrDisconnected,
                "mcp.protocol.counter_offer_rejected",
            ),
            (
                McpProtocolFailureKind::GenerationMismatch,
                "mcp.protocol.generation_mismatch",
            ),
            (
                McpProtocolFailureKind::CapabilityShapeFailure,
                "mcp.protocol.capability_shape_invalid",
            ),
            (
                McpProtocolFailureKind::RevisionSchemaProjectionFailure,
                "mcp.protocol.schema_projection_failed",
            ),
            (
                McpProtocolFailureKind::ToolListProtocolError,
                "mcp.tools.protocol_error",
            ),
            (
                McpProtocolFailureKind::ToolListSchemaFailure,
                "mcp.tools.schema_failure",
            ),
            (
                McpProtocolFailureKind::RequiredToolMissing,
                "mcp.tools.required_missing",
            ),
            (
                McpProtocolFailureKind::InvalidToolDefinitionProjection,
                "mcp.tools.definition_projection_invalid",
            ),
            (
                McpProtocolFailureKind::SafeToolProtocolError,
                "mcp.tool_call.protocol_error",
            ),
            (
                McpProtocolFailureKind::OutputSchemaFailure,
                "mcp.tool_call.output_schema_failed",
            ),
            (
                McpProtocolFailureKind::SafeReadOnlyToolFailure,
                "mcp.tool_call.safe_read_only_failed",
            ),
            (
                McpProtocolFailureKind::SessionCorrelationInvalid,
                "mcp.tool_call.session_correlation_invalid",
            ),
            (
                McpProtocolFailureKind::PreflightReportInvalid,
                "process.preflight.report_invalid",
            ),
            (
                McpProtocolFailureKind::Unexpected,
                volicord_types::INTERNAL_UNEXPECTED_FAILURE_CODE,
            ),
        ];
        for (kind, code) in protocol_cases {
            let failure = McpProcessFailure::typed_protocol(McpStage::Initialize, kind, "ignored");
            assert_eq!(failure.diagnostic_code(), code);
        }

        let process_cases = [
            McpProcessFailure::Spawn {
                stage: McpStage::Startup,
                io_detail: bounded_io_text("spawn"),
            },
            McpProcessFailure::PipeAcquisition {
                stage: McpStage::Startup,
                io_detail: bounded_io_text("pipe"),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Timeout {
                stage: McpStage::Initialize,
                timeout: Duration::from_millis(5),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Timeout {
                stage: McpStage::ToolsList,
                timeout: Duration::from_millis(5),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Timeout {
                stage: McpStage::SafeToolCall,
                timeout: Duration::from_millis(5),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Read {
                stage: McpStage::Initialize,
                io_detail: bounded_io_text("read"),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Write {
                stage: McpStage::Initialize,
                io_detail: bounded_io_text("write"),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::ExitedBeforeResponse {
                stage: McpStage::Initialize,
                exit_code: Some(23),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::ExitedBeforeResponse {
                stage: McpStage::Initialize,
                exit_code: None,
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Cleanup {
                stage: McpStage::Shutdown,
                io_detail: bounded_io_text("cleanup"),
                stderr: BoundedText::empty(),
            },
            McpProcessFailure::Wait {
                stage: McpStage::Shutdown,
                io_detail: bounded_io_text("wait"),
                stderr: BoundedText::empty(),
            },
        ];
        let expected = [
            "process.spawn.failed",
            "process.pipe_acquisition.failed",
            "process.initialize.timeout",
            "process.tools_list.timeout",
            "process.safe_tool_call.timeout",
            "process.pipe.read_failed",
            "process.pipe.write_failed",
            "process.child.exited",
            "process.child.signaled",
            "process.cleanup.failed",
            "process.child.wait_failed",
        ];
        for (failure, code) in process_cases.into_iter().zip(expected) {
            assert_eq!(failure.diagnostic_code(), code);
        }
    }

    #[test]
    fn process_negotiation_finding_keeps_distinct_safe_revision_facts() {
        let finding = McpProcessFailure::typed_protocol(
            McpStage::Initialize,
            McpProtocolFailureKind::UnsupportedProtocolRevision,
            "selected revision did not match",
        )
        .to_diagnostic_finding(McpProcessDiagnosticContext {
            finding_id: "finding.runtime_process.unsupported".to_owned(),
            observed_at: UtcTimestamp::parse("2026-07-22T01:02:03Z").unwrap(),
            connection_id: "connection_test".to_owned(),
            integration_revision: IntegrationRevision::parse(format!("sha256:{}", "0".repeat(64)))
                .unwrap(),
            runtime_session_id: Some("runtime_process".to_owned()),
            requested_revision: Some("2025-06-18".to_owned()),
            selected_revision: Some("2025-11-25".to_owned()),
            negotiated_revision: None,
            production_supported_revisions: vec![
                "2025-03-26".to_owned(),
                "2025-06-18".to_owned(),
                "2025-11-25".to_owned(),
            ],
            attempted_client_name: Some("volicord-conformance-probe".to_owned()),
            attempted_client_version: Some("0.9.1".to_owned()),
        })
        .unwrap();

        let facts = finding.facts().data();
        assert_eq!(finding.code().as_str(), "mcp.protocol.unsupported_version");
        assert_eq!(facts["requested_revision"], "2025-06-18");
        assert_eq!(facts["selected_revision"], "2025-11-25");
        assert!(facts["negotiated_revision"].is_null());
        assert_eq!(facts["attempted_client_name"], "volicord-conformance-probe");
        assert!(facts["production_supported_revisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|revision| revision == "2025-06-18"));
    }
}
