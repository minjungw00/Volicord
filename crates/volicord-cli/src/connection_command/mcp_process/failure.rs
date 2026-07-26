use std::time::Duration;

use serde::Serialize;
use volicord_types::diagnostics::{
    DiagnosticAction, DiagnosticCode, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts,
    DiagnosticFindingData, DiagnosticSeverity, DiagnosticSource, DiagnosticStage,
    DiagnosticSubject,
};
use volicord_types::ids::AgentConnectionId;
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::values::UtcTimestamp;

pub(super) const MAX_CAPTURED_STDERR_BYTES: usize = 2 * 1024;
pub(super) const MAX_PROTOCOL_DETAIL_BYTES: usize = 2 * 1024;
pub(super) const MAX_IO_DETAIL_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStage {
    Startup,
    Initialize,
    ToolsList,
    SafeToolCall,
    Shutdown,
}

impl McpStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Initialize => "initialize",
            Self::ToolsList => "tools_list",
            Self::SafeToolCall => "safe_tool_call",
            Self::Shutdown => "shutdown",
        }
    }

    const fn check_code(self) -> &'static str {
        match self {
            Self::Startup | Self::Shutdown => "mcp_server_process_failed",
            Self::Initialize => "mcp_server_initialize_failed",
            Self::ToolsList => "mcp_server_tools_list_failed",
            Self::SafeToolCall => "mcp_server_safe_call_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProtocolFailureKind {
    MalformedResponse,
    FramingFailure,
    MessageSizeExceeded,
    JsonRpcError,
    MalformedProtocolVersion,
    UnsupportedProtocolRevision,
    CapabilityShapeFailure,
    RevisionSchemaProjectionFailure,
    ToolListProtocolError,
    ToolListSchemaFailure,
    RequiredToolMissing,
    InvalidToolDefinitionProjection,
    SafeToolProtocolError,
    OutputSchemaFailure,
    SafeReadOnlyToolFailure,
    SessionCorrelationInvalid,
    PreflightReportInvalid,
    Unexpected,
}

impl McpProtocolFailureKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MalformedResponse => "mcp.json_rpc.malformed_response",
            Self::FramingFailure => "mcp.json_rpc.framing_failure",
            Self::MessageSizeExceeded => "mcp.json_rpc.message_size_exceeded",
            Self::JsonRpcError => "mcp.json_rpc.error_response",
            Self::MalformedProtocolVersion => "mcp.protocol.malformed_version",
            Self::UnsupportedProtocolRevision => "mcp.protocol.unsupported_version",
            Self::CapabilityShapeFailure => "mcp.protocol.capability_shape_invalid",
            Self::RevisionSchemaProjectionFailure => "mcp.protocol.schema_projection_failed",
            Self::ToolListProtocolError => "mcp.tools.protocol_error",
            Self::ToolListSchemaFailure => "mcp.tools.schema_failure",
            Self::RequiredToolMissing => "mcp.tools.required_missing",
            Self::InvalidToolDefinitionProjection => "mcp.tools.definition_projection_invalid",
            Self::SafeToolProtocolError => "mcp.tool_call.protocol_error",
            Self::OutputSchemaFailure => "mcp.tool_call.output_schema_failed",
            Self::SafeReadOnlyToolFailure => "mcp.tool_call.safe_read_only_failed",
            Self::SessionCorrelationInvalid => "mcp.tool_call.session_correlation_invalid",
            Self::PreflightReportInvalid => "process.preflight.report_invalid",
            Self::Unexpected => volicord_types::diagnostics::INTERNAL_UNEXPECTED_FAILURE_CODE,
        }
    }

    const fn summary(self) -> &'static str {
        match self {
            Self::MalformedResponse => "the child returned a malformed JSON-RPC response",
            Self::FramingFailure => "the child returned invalid newline framing",
            Self::MessageSizeExceeded => "the child exceeded a protocol message budget",
            Self::JsonRpcError => "the child returned a JSON-RPC error response",
            Self::MalformedProtocolVersion => "the child returned a malformed protocol version",
            Self::UnsupportedProtocolRevision => {
                "the child did not select the requested protocol revision"
            }
            Self::CapabilityShapeFailure => "the initialize capability shape was invalid",
            Self::RevisionSchemaProjectionFailure => {
                "the initialize result failed its revision schema"
            }
            Self::ToolListProtocolError => "tools/list returned a protocol error",
            Self::ToolListSchemaFailure => "tools/list failed its revision schema",
            Self::RequiredToolMissing => "tools/list omitted a required tool",
            Self::InvalidToolDefinitionProjection => {
                "tools/list returned an invalid tool definition"
            }
            Self::SafeToolProtocolError => {
                "the designated read-only tool returned a protocol error"
            }
            Self::OutputSchemaFailure => "the designated read-only tool failed its output schema",
            Self::SafeReadOnlyToolFailure => "the designated read-only tool reported failure",
            Self::SessionCorrelationInvalid => {
                "the designated tool-call session correlation was invalid"
            }
            Self::PreflightReportInvalid => "the MCP preflight report was invalid",
            Self::Unexpected => "an unexpected internal child-protocol failure occurred",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedText {
    pub(super) text: String,
    pub(super) truncated: bool,
    pub(super) omitted_bytes: usize,
}

impl BoundedText {
    pub(super) fn empty() -> Self {
        Self {
            text: String::new(),
            truncated: false,
            omitted_bytes: 0,
        }
    }

    pub(super) fn from_utf8(value: impl AsRef<str>, limit: usize, label: &str) -> Self {
        let value = value.as_ref();
        if value.len() <= limit {
            return Self {
                text: value.to_owned(),
                truncated: false,
                omitted_bytes: 0,
            };
        }
        let mut end = limit;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        let omitted_bytes = value.len() - end;
        Self {
            text: format!(
                "{}\n...[{label} truncated; {omitted_bytes} bytes omitted]",
                &value[..end]
            ),
            truncated: true,
            omitted_bytes,
        }
    }

    pub(super) fn from_bytes(bytes: Vec<u8>, omitted_bytes: usize, label: &str) -> Self {
        let mut text = String::from_utf8_lossy(&bytes)
            .chars()
            .map(|character| {
                if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
                    '\u{fffd}'
                } else {
                    character
                }
            })
            .collect::<String>();
        if omitted_bytes > 0 {
            text.push_str(&format!(
                "\n...[{label} truncated; {omitted_bytes} bytes omitted]"
            ));
        }
        Self {
            text,
            truncated: omitted_bytes > 0,
            omitted_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProcessFailure {
    Spawn {
        stage: McpStage,
        io_detail: BoundedText,
    },
    PipeAcquisition {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    ExitedBeforeResponse {
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: BoundedText,
    },
    Timeout {
        stage: McpStage,
        timeout: Duration,
        stderr: BoundedText,
    },
    Read {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Write {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Protocol {
        stage: McpStage,
        kind: McpProtocolFailureKind,
        protocol_detail: BoundedText,
        json_rpc_error_code: Option<i64>,
        missing_tools: Vec<String>,
        stderr: BoundedText,
    },
    Wait {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Cleanup {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Shutdown {
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: BoundedText,
    },
}

impl McpProcessFailure {
    pub fn exited_with_stderr(
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: impl AsRef<str>,
    ) -> Self {
        Self::ExitedBeforeResponse {
            stage,
            exit_code,
            stderr: BoundedText::from_utf8(stderr, MAX_CAPTURED_STDERR_BYTES, "stderr"),
        }
    }

    pub fn protocol(stage: McpStage, detail: impl Into<String>) -> Self {
        let kind = match stage {
            McpStage::Startup => McpProtocolFailureKind::PreflightReportInvalid,
            McpStage::Initialize => McpProtocolFailureKind::CapabilityShapeFailure,
            McpStage::ToolsList => McpProtocolFailureKind::ToolListProtocolError,
            McpStage::SafeToolCall => McpProtocolFailureKind::SafeToolProtocolError,
            McpStage::Shutdown => McpProtocolFailureKind::Unexpected,
        };
        Self::typed_protocol(stage, kind, detail)
    }

    pub fn typed_protocol(
        stage: McpStage,
        kind: McpProtocolFailureKind,
        detail: impl Into<String>,
    ) -> Self {
        Self::Protocol {
            stage,
            kind,
            protocol_detail: bounded_protocol_detail(detail.into()),
            json_rpc_error_code: None,
            missing_tools: Vec::new(),
            stderr: BoundedText::empty(),
        }
    }

    pub const fn stage(&self) -> McpStage {
        match self {
            Self::Spawn { stage, .. }
            | Self::PipeAcquisition { stage, .. }
            | Self::ExitedBeforeResponse { stage, .. }
            | Self::Timeout { stage, .. }
            | Self::Read { stage, .. }
            | Self::Write { stage, .. }
            | Self::Protocol { stage, .. }
            | Self::Wait { stage, .. }
            | Self::Cleanup { stage, .. }
            | Self::Shutdown { stage, .. } => *stage,
        }
    }

    pub const fn check_code(&self) -> &'static str {
        self.stage().check_code()
    }

    pub const fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "process.spawn.failed",
            Self::PipeAcquisition { .. } => "process.pipe_acquisition.failed",
            Self::ExitedBeforeResponse {
                exit_code: Some(_), ..
            } => "process.child.exited",
            Self::ExitedBeforeResponse {
                exit_code: None, ..
            } => "process.child.signaled",
            Self::Timeout { stage, .. } => match stage {
                McpStage::Startup => "process.startup.timeout",
                McpStage::Initialize => "process.initialize.timeout",
                McpStage::ToolsList => "process.tools_list.timeout",
                McpStage::SafeToolCall => "process.safe_tool_call.timeout",
                McpStage::Shutdown => "process.shutdown.timeout",
            },
            Self::Read { .. } => "process.pipe.read_failed",
            Self::Write { .. } => "process.pipe.write_failed",
            Self::Protocol { kind, .. } => kind.code(),
            Self::Wait { .. } => "process.child.wait_failed",
            Self::Cleanup { .. } => "process.cleanup.failed",
            Self::Shutdown {
                exit_code: Some(_), ..
            } => "process.child.exited",
            Self::Shutdown {
                exit_code: None, ..
            } => "process.child.signaled",
        }
    }

    pub(super) fn with_stderr(mut self, stderr: BoundedText) -> Self {
        match &mut self {
            Self::Spawn { .. } => {}
            Self::PipeAcquisition {
                stderr: captured, ..
            }
            | Self::ExitedBeforeResponse {
                stderr: captured, ..
            }
            | Self::Timeout {
                stderr: captured, ..
            }
            | Self::Read {
                stderr: captured, ..
            }
            | Self::Write {
                stderr: captured, ..
            }
            | Self::Protocol {
                stderr: captured, ..
            }
            | Self::Wait {
                stderr: captured, ..
            }
            | Self::Cleanup {
                stderr: captured, ..
            }
            | Self::Shutdown {
                stderr: captured, ..
            } => *captured = stderr,
        }
        self
    }

    pub(super) fn summary(&self) -> String {
        match self {
            Self::Spawn { io_detail, .. } => {
                format!("MCP process spawn failed: {}", io_detail.text)
            }
            Self::PipeAcquisition { io_detail, .. } => {
                format!("MCP process pipe acquisition failed: {}", io_detail.text)
            }
            Self::ExitedBeforeResponse {
                stage, exit_code, ..
            } => format!(
                "MCP process exited before a response during {} with exit code {}",
                stage.as_str(),
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unavailable".to_owned())
            ),
            Self::Timeout { stage, .. } => {
                format!("MCP process timed out during {}", stage.as_str())
            }
            Self::Read {
                stage, io_detail, ..
            } => format!(
                "MCP process read failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Write {
                stage, io_detail, ..
            } => format!(
                "MCP process write failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Protocol {
                stage,
                protocol_detail,
                ..
            } => format!(
                "MCP protocol failed during {}: {}",
                stage.as_str(),
                protocol_detail.text
            ),
            Self::Wait {
                stage, io_detail, ..
            } => format!(
                "MCP process wait failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Cleanup {
                stage, io_detail, ..
            } => format!(
                "MCP process cleanup failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Shutdown { exit_code, .. } => format!(
                "MCP process shutdown failed with exit code {}",
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unavailable".to_owned())
            ),
        }
    }

    const fn kind(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "spawn",
            Self::PipeAcquisition { .. } => "pipe_acquisition",
            Self::ExitedBeforeResponse { .. } => "exited_before_response",
            Self::Timeout { .. } => "timeout",
            Self::Read { .. } => "read",
            Self::Write { .. } => "write",
            Self::Protocol { .. } => "protocol",
            Self::Wait { .. } => "wait",
            Self::Cleanup { .. } => "cleanup",
            Self::Shutdown { .. } => "shutdown",
        }
    }

    pub fn to_diagnostic_data(
        &self,
        context: McpProcessDiagnosticContext,
    ) -> Result<DiagnosticFindingData, volicord_types::diagnostics::DiagnosticError> {
        let (io_detail, protocol_detail, json_rpc_error_code, missing_tools, stderr) = match self {
            Self::Spawn { io_detail, .. } => (Some(io_detail), None, None, &[][..], None),
            Self::PipeAcquisition {
                io_detail, stderr, ..
            }
            | Self::Read {
                io_detail, stderr, ..
            }
            | Self::Write {
                io_detail, stderr, ..
            }
            | Self::Wait {
                io_detail, stderr, ..
            }
            | Self::Cleanup {
                io_detail, stderr, ..
            } => (Some(io_detail), None, None, &[][..], Some(stderr)),
            Self::Protocol {
                protocol_detail,
                json_rpc_error_code,
                missing_tools,
                stderr,
                ..
            } => (
                None,
                Some(protocol_detail),
                *json_rpc_error_code,
                missing_tools.as_slice(),
                Some(stderr),
            ),
            Self::ExitedBeforeResponse { stderr, .. }
            | Self::Timeout { stderr, .. }
            | Self::Shutdown { stderr, .. } => (None, None, None, &[][..], Some(stderr)),
        };
        let exit_code = match self {
            Self::ExitedBeforeResponse { exit_code, .. } | Self::Shutdown { exit_code, .. } => {
                *exit_code
            }
            _ => None,
        };
        let timeout_ms = match self {
            Self::Timeout { timeout, .. } => Some(timeout.as_millis() as u64),
            _ => None,
        };
        let facts = DiagnosticFacts::project(&McpProcessDiagnosticFacts {
            summary: self.safe_summary(),
            process_failure_kind: self.kind(),
            runtime_session_id: context.runtime_session_id.as_deref(),
            requested_revision: context.requested_revision.as_deref(),
            selected_revision: context.selected_revision.as_deref(),
            negotiated_revision: context.negotiated_revision.as_deref(),
            production_supported_revisions: &context.production_supported_revisions,
            attempted_client_name: context.attempted_client_name.as_deref(),
            attempted_client_version: context.attempted_client_version.as_deref(),
            exit_code,
            signal_termination: matches!(
                self,
                Self::ExitedBeforeResponse {
                    exit_code: None,
                    ..
                } | Self::Shutdown {
                    exit_code: None,
                    ..
                }
            ),
            timeout_ms,
            json_rpc_error_code,
            io_detail: io_detail.map(|detail| detail.text.as_str()),
            io_detail_truncated: io_detail.is_some_and(|detail| detail.truncated),
            protocol_detail: protocol_detail.map(|detail| detail.text.as_str()),
            protocol_detail_truncated: protocol_detail.is_some_and(|detail| detail.truncated),
            missing_tools,
            bounded_stderr_excerpt: stderr.map(|captured| captured.text.as_str()),
            bounded_stderr_truncated: stderr.is_some_and(|captured| captured.truncated),
            bounded_stderr_omitted_bytes: stderr.map_or(0, |captured| captured.omitted_bytes),
        })?;
        let subject_reference = context
            .runtime_session_id
            .as_deref()
            .unwrap_or(context.connection_id.as_str());
        let data = DiagnosticFindingData::try_new(
            DiagnosticCode::parse(self.diagnostic_code())?,
            DiagnosticDomain::parse(if self.diagnostic_code().starts_with("mcp.") {
                "mcp"
            } else if self.diagnostic_code().starts_with("internal.") {
                "internal"
            } else {
                "process"
            })?,
            DiagnosticStage::parse(self.stage().as_str())?,
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("cli_process_supervisor")?,
            DiagnosticSubject::try_new(
                if context.runtime_session_id.is_some() {
                    "runtime_session"
                } else {
                    "connection"
                },
                subject_reference,
            )?,
            facts,
            context.observed_at,
        )?
        .with_actions(vec![self.recommended_action()?])?
        .with_connection_id(AgentConnectionId::new(context.connection_id))?
        .with_integration_revision(context.integration_revision);
        Ok(data)
    }

    fn recommended_action(
        &self,
    ) -> Result<DiagnosticAction, volicord_types::diagnostics::DiagnosticError> {
        let (code, summary) = match self {
            Self::Spawn { .. } => (
                "action.process.repair_launch",
                "Repair the managed MCP executable or launch configuration",
            ),
            Self::PipeAcquisition { .. } | Self::Read { .. } | Self::Write { .. } => (
                "action.process.repair_stdio",
                "Repair the managed MCP stdio process boundary",
            ),
            Self::Timeout { .. } => (
                "action.process.resolve_timeout",
                "Resolve the stage timeout and rerun active verification",
            ),
            Self::ExitedBeforeResponse { .. } | Self::Shutdown { .. } => (
                "action.process.repair_child_exit",
                "Repair the child-process exit condition and rerun active verification",
            ),
            Self::Protocol { kind, .. } => match kind {
                McpProtocolFailureKind::MalformedProtocolVersion
                | McpProtocolFailureKind::UnsupportedProtocolRevision => (
                    "action.mcp.use_supported_protocol_revision",
                    "Configure the MCP peer to request one supported protocol revision",
                ),
                McpProtocolFailureKind::RequiredToolMissing
                | McpProtocolFailureKind::InvalidToolDefinitionProjection
                | McpProtocolFailureKind::ToolListProtocolError
                | McpProtocolFailureKind::ToolListSchemaFailure => (
                    "action.mcp.restore_required_tools",
                    "Restore the required tools/list projection for the selected revision",
                ),
                McpProtocolFailureKind::SafeToolProtocolError
                | McpProtocolFailureKind::OutputSchemaFailure
                | McpProtocolFailureKind::SafeReadOnlyToolFailure
                | McpProtocolFailureKind::SessionCorrelationInvalid => (
                    "action.mcp.repair_read_only_tool",
                    "Repair the designated read-only tool call for the selected revision",
                ),
                _ => (
                    "action.mcp.repair_protocol_exchange",
                    "Repair the typed MCP protocol failure and rerun active verification",
                ),
            },
            Self::Wait { .. } | Self::Cleanup { .. } => (
                "action.process.repair_cleanup",
                "Repair child-process cleanup and rerun active verification",
            ),
        };
        DiagnosticAction::try_new(DiagnosticCode::parse(code)?, summary)
    }

    fn safe_summary(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "the MCP child process could not be spawned",
            Self::PipeAcquisition { .. } => "an MCP child-process pipe was unavailable",
            Self::ExitedBeforeResponse {
                exit_code: Some(_), ..
            } => "the MCP child process exited before the expected response",
            Self::ExitedBeforeResponse {
                exit_code: None, ..
            } => "the MCP child process was terminated by a signal",
            Self::Timeout { .. } => "the MCP child process exceeded its stage timeout",
            Self::Read { .. } => "an MCP child-process pipe read failed",
            Self::Write { .. } => "an MCP child-process pipe write failed",
            Self::Protocol { kind, .. } => kind.summary(),
            Self::Wait { .. } => "waiting for the MCP child process failed",
            Self::Cleanup { .. } => "MCP child-process cleanup or descendant termination failed",
            Self::Shutdown {
                exit_code: Some(_), ..
            } => "the MCP child process exited unsuccessfully during shutdown",
            Self::Shutdown {
                exit_code: None, ..
            } => "the MCP child process was terminated by a signal during shutdown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpProcessDiagnosticContext {
    pub observed_at: UtcTimestamp,
    pub connection_id: String,
    pub integration_revision: IntegrationRevision,
    pub runtime_session_id: Option<String>,
    pub requested_revision: Option<String>,
    pub selected_revision: Option<String>,
    pub negotiated_revision: Option<String>,
    pub production_supported_revisions: Vec<String>,
    pub attempted_client_name: Option<String>,
    pub attempted_client_version: Option<String>,
}

#[derive(Serialize)]
struct McpProcessDiagnosticFacts<'a> {
    summary: &'static str,
    process_failure_kind: &'static str,
    runtime_session_id: Option<&'a str>,
    requested_revision: Option<&'a str>,
    selected_revision: Option<&'a str>,
    negotiated_revision: Option<&'a str>,
    production_supported_revisions: &'a [String],
    attempted_client_name: Option<&'a str>,
    attempted_client_version: Option<&'a str>,
    exit_code: Option<i32>,
    signal_termination: bool,
    timeout_ms: Option<u64>,
    json_rpc_error_code: Option<i64>,
    io_detail: Option<&'a str>,
    io_detail_truncated: bool,
    protocol_detail: Option<&'a str>,
    protocol_detail_truncated: bool,
    missing_tools: &'a [String],
    bounded_stderr_excerpt: Option<&'a str>,
    bounded_stderr_truncated: bool,
    bounded_stderr_omitted_bytes: usize,
}

impl DiagnosticFactSource for McpProcessDiagnosticFacts<'_> {}

pub(super) fn bounded_protocol_detail(detail: impl AsRef<str>) -> BoundedText {
    BoundedText::from_utf8(detail, MAX_PROTOCOL_DETAIL_BYTES, "protocol detail")
}

pub(super) fn bounded_io_detail(error: impl std::fmt::Display) -> BoundedText {
    bounded_io_text(error.to_string())
}

pub(super) fn bounded_io_text(detail: impl AsRef<str>) -> BoundedText {
    BoundedText::from_utf8(detail, MAX_IO_DETAIL_BYTES, "I/O detail")
}
